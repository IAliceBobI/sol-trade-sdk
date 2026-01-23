//! DEX 交易解析器
//!
//! 主解析器，负责根据协议类型分发到对应的子解析器

use std::sync::Arc;
use std::collections::HashMap;
use std::str::FromStr;
use solana_rpc_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::{UiTransactionEncoding, EncodedConfirmedTransactionWithStatusMeta};
use solana_commitment_config::CommitmentConfig;

use crate::common::rpc_client_wrapper::RpcClientWrapper;

use super::{
    transaction_adapter::TransactionAdapter,
    base_parser::{DexParserTrait, ParseError},
    types::{ParseResult, ParserConfig, DexProtocol},
    pumpswap::PumpswapParser,
    raydium::{
        clmm::RaydiumClmmParser,
        v4::RaydiumV4Parser,
        cpmm::RaydiumCpmmParser,
    },
};

/// DEX 解析器
///
/// 主入口，负责获取交易并分发到对应的协议解析器
#[derive(Clone)]
pub struct DexParser {
    /// 解析器配置
    pub config: ParserConfig,
    /// RPC 客户端包装器
    rpc_client: RpcClientWrapper,
    /// 已注册的协议解析器（key: program_id）
    pub parsers: HashMap<String, Arc<dyn DexParserTrait>>,
}

impl DexParser {
    /// 创建新的 DEX 解析器
    pub fn new(config: ParserConfig) -> Self {
        let rpc_client = RpcClientWrapper::Standard(
            Arc::new(RpcClient::new(config.rpc_url.clone()))
        );

        let mut parsers: HashMap<String, Arc<dyn DexParserTrait>> = HashMap::new();

        // 注册协议解析器
        // Pumpswap
        parsers.insert(
            DexProtocol::PumpSwap.program_id().to_string(),
            Arc::new(PumpswapParser) as Arc<dyn DexParserTrait>
        );
        // Raydium CLMM
        parsers.insert(
            DexProtocol::RaydiumClmm.program_id().to_string(),
            Arc::new(RaydiumClmmParser) as Arc<dyn DexParserTrait>
        );
        // Raydium V4
        parsers.insert(
            DexProtocol::RaydiumV4.program_id().to_string(),
            Arc::new(RaydiumV4Parser) as Arc<dyn DexParserTrait>
        );
        // Raydium CPMM
        parsers.insert(
            DexProtocol::RaydiumCpmm.program_id().to_string(),
            Arc::new(RaydiumCpmmParser) as Arc<dyn DexParserTrait>
        );

        Self {
            config,
            rpc_client,
            parsers,
        }
    }

    /// 使用默认配置创建解析器
    pub fn default() -> Self {
        Self::new(ParserConfig::default())
    }

    /// 使用 Auto Mock 模式创建解析器（测试环境）
    pub fn new_with_mock(config: ParserConfig) -> Self {
        use crate::common::auto_mock_rpc::AutoMockRpcClient;

        let rpc_client = RpcClientWrapper::AutoMock(
            Arc::new(AutoMockRpcClient::new(config.rpc_url.clone()))
        );

        let mut parsers: HashMap<String, Arc<dyn DexParserTrait>> = HashMap::new();

        // 注册协议解析器
        // Pumpswap
        parsers.insert(
            DexProtocol::PumpSwap.program_id().to_string(),
            Arc::new(PumpswapParser) as Arc<dyn DexParserTrait>
        );
        // Raydium CLMM
        parsers.insert(
            DexProtocol::RaydiumClmm.program_id().to_string(),
            Arc::new(RaydiumClmmParser) as Arc<dyn DexParserTrait>
        );
        // Raydium V4
        parsers.insert(
            DexProtocol::RaydiumV4.program_id().to_string(),
            Arc::new(RaydiumV4Parser) as Arc<dyn DexParserTrait>
        );
        // Raydium CPMM
        parsers.insert(
            DexProtocol::RaydiumCpmm.program_id().to_string(),
            Arc::new(RaydiumCpmmParser) as Arc<dyn DexParserTrait>
        );

        Self {
            config,
            rpc_client,
            parsers,
        }
    }

    /// 解析交易
    ///
    /// # 参数
    /// - `signature`: 交易签名
    ///
    /// # 返回
    /// 解析结果
    pub async fn parse_transaction(&self, signature: &str) -> ParseResult {
        // 1. 获取并解析交易数据
        match self.fetch_and_parse_transaction(signature).await {
            Ok(trades) => ParseResult {
                success: !trades.is_empty(),
                trades,
                error: None,
            },
            Err(e) => ParseResult {
                success: false,
                trades: vec![],
                error: Some(format!("解析失败: {}", e)),
            },
        }
    }

    /// 获取并解析交易
    async fn fetch_and_parse_transaction(
        &self,
        signature: &str,
    ) -> Result<Vec<super::types::ParsedTradeInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let signature = signature.to_string();

        let sig = Signature::from_str(&signature)
            .map_err(|e| format!("无效签名: {}", e))?;

        // 使用 rpc_client 获取交易（异步调用）
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        let tx: EncodedConfirmedTransactionWithStatusMeta = self.rpc_client.get_transaction_with_config(
            &sig,
            config,
        ).await
        .map_err(|e| format!("获取交易失败: {}", e))?;

        let slot = tx.slot;
        let block_time = tx.block_time;

        // 创建交易适配器并解析
        let adapter = TransactionAdapter::from_encoded_transaction(&tx, slot, block_time)?;

        // 识别协议并分发到对应的解析器
        match self.parse_with_correct_parser(&adapter).await {
            Ok(trades) => Ok(trades),
            Err(ParseError::UnsupportedProtocol(msg)) => {
                // 如果无法识别协议，返回空结果而不是错误
                if self.config.verbose {
                    println!("无法识别协议: {}", msg);
                }
                Ok(vec![])
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    /// 识别协议并分发到对应的解析器
    #[allow(dead_code)]  // 将在后续实现中使用
    async fn parse_with_correct_parser(
        &self,
        adapter: &TransactionAdapter,
    ) -> Result<Vec<super::types::ParsedTradeInfo>, ParseError> {
        // 尝试每个已注册的解析器
        for (program_id, parser) in &self.parsers {
            if parser.can_parse(adapter) {
                if self.config.verbose {
                    println!("识别到程序 {}，开始解析...", program_id);
                }
                return parser.parse(adapter).await;
            }
        }

        Err(ParseError::UnsupportedProtocol(
            "无法识别交易中的 DEX 协议".to_string(),
        ))
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
