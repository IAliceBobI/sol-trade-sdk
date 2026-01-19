//! DEX 交易解析器
//!
//! 主解析器，负责根据协议类型分发到对应的子解析器

use std::sync::Arc;
use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcTransactionConfig;
use solana_rpc_client_api::response::RpcConfirmedTransactionWithStatus;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatus,
    UiTransactionEncoding,
};
use solana_commitment_config::CommitmentConfig;

use super::{
    transaction_adapter::TransactionAdapter,
    base_parser::{DexParserTrait, ParseError},
    types::{ParseResult, ParserConfig, DexProtocol},
};

// TODO: 导入协议解析器（待实现）
// use super::pumpswap::PumpswapParser;
// use super::raydium::{RaydiumV4Parser, RaydiumClmmParser, RaydiumCpmmParser};

/// DEX 解析器
///
/// 主入口，负责获取交易并分发到对应的协议解析器
#[derive(Clone)]
pub struct DexParser {
    config: ParserConfig,
    rpc_client: Arc<RpcClient>,
    // TODO: 添加解析器实例
}

impl DexParser {
    /// 创建新的 DEX 解析器
    pub fn new(config: ParserConfig) -> Self {
        let rpc_client = Arc::new(RpcClient::new(config.rpc_url.clone()));

        Self {
            config,
            rpc_client,
        }
    }

    /// 使用默认配置创建解析器
    pub fn default() -> Self {
        Self::new(ParserConfig::default())
    }

    /// 解析交易
    ///
    /// # 参数
    /// - `signature`: 交易签名
    ///
    /// # 返回
    /// 解析结果
    pub async fn parse_transaction(&self, signature: &str) -> ParseResult {
        // 1. 获取交易数据
        let (tx, slot, block_time) = match self.fetch_transaction(signature).await {
            Ok(data) => data,
            Err(e) => {
                return ParseResult {
                    success: false,
                    trades: vec![],
                    error: Some(format!("获取交易失败: {}", e)),
                };
            }
        };

        // 2. 创建交易适配器
        let adapter = match TransactionAdapter::from_confirmed_transaction(&tx, slot, block_time).await {
            Ok(adapter) => adapter,
            Err(e) => {
                return ParseResult {
                    success: false,
                    trades: vec![],
                    error: Some(format!("创建适配器失败: {}", e)),
                };
            }
        };

        // 3. 识别协议并分发到对应的解析器
        // TODO: 实现协议识别和解析
        ParseResult {
            success: false,
            trades: vec![],
            error: Some("解析器未实现".to_string()),
        }
    }

    /// 获取交易数据
    async fn fetch_transaction(
        &self,
        signature: &str,
    ) -> Result<
        (RpcConfirmedTransactionWithStatus, u64, Option<i64>),
        Box<dyn std::error::Error + Send + Sync>,
    > {
        // 使用 tokio 运行阻塞的 RPC 调用
        let rpc_client = self.rpc_client.clone();
        let signature = signature.to_string();

        let (tx, slot, block_time) = tokio::task::spawn_blocking(move || {
            let config = RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::JsonParsed),
                commitment: Some(CommitmentConfig::confirmed()),
                max_supported_transaction_version: Some(0),
                ..RpcTransactionConfig::default()
            };

            let tx = rpc_client.get_transaction_with_config(&signature, config)
                .map_err(|e| format!("RPC 调用失败: {}", e))?;

            let slot = tx.slot;
            let block_time = tx.block_time;

            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((tx, slot, block_time))
        })
        .await
        .map_err(|e| format!("任务执行失败: {}", e))??;

        Ok((tx, slot, block_time))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dex_parser_creation() {
        let parser = DexParser::default();
        assert!(!parser.config.rpc_url.is_empty());
    }

    #[tokio::test]
    async fn test_parse_result() {
        let result = ParseResult {
            success: true,
            trades: vec![],
            error: None,
        };

        assert!(result.success);
        assert!(result.trades.is_empty());
        assert!(result.error.is_none());
    }
}
