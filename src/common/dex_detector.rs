//! DEX 协议检测模块
//!
//! 提供通过 Pool 地址识别 DEX 协议的便捷工具函数

use crate::common::types::SolanaRpcClient;
use crate::constants::DexProtocol;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// DEX 检测结果
#[derive(Debug, Clone)]
pub struct DexInfo {
    /// DEX 协议枚举
    pub protocol: DexProtocol,
    /// Pool 地址
    pub pool_address: String,
    /// Program ID (owner)
    pub program_id: String,
}

impl DexInfo {
    /// 创建新的 DEX 信息
    pub fn new(pool_address: String, program_id: String) -> Option<Self> {
        let pubkey = Pubkey::from_str(&program_id).ok()?;
        let protocol = DexProtocol::from_program_id(&pubkey)?;

        Some(Self { protocol, pool_address, program_id })
    }

    /// 获取 DEX 代码名称（用于代码/数据库）
    pub fn dex_name(&self) -> &str {
        self.protocol.name()
    }

    /// 获取 DEX 显示名称（用于 UI 显示）
    pub fn display_name(&self) -> &str {
        self.protocol.display_name()
    }
}

/// 通过 Pool 地址检测 DEX 协议
///
/// # 参数
/// - `rpc`: RPC 客户端
/// - `pool_address`: Pool 地址（字符串格式）
///
/// # 返回
/// 成功返回 `DexInfo`，失败返回 `anyhow::Error`
pub async fn detect_dex_from_pool(
    rpc: &SolanaRpcClient,
    pool_address: &str,
) -> anyhow::Result<DexInfo> {
    // 解析 Pool 地址
    let pool_pubkey =
        Pubkey::from_str(pool_address).map_err(|e| anyhow::anyhow!("无效的 Pool 地址: {}", e))?;

    // 获取账户信息
    let account = rpc
        .get_account(&pool_pubkey)
        .await
        .map_err(|e| anyhow::anyhow!("获取账户失败: {}", e))?;

    // 提取 owner（program_id）
    let program_id = account.owner.to_string();

    // 识别 DEX 协议
    let protocol = DexProtocol::from_program_id(&account.owner)
        .ok_or_else(|| anyhow::anyhow!("未知的 DEX 协议，Program ID: {}", program_id))?;

    Ok(DexInfo { protocol, pool_address: pool_address.to_string(), program_id })
}

/// 批量检测多个 Pool 地址的 DEX
///
/// # 参数
/// - `rpc`: RPC 客户端
/// - `pool_addresses`: Pool 地址列表
///
/// # 返回
/// 成功的检测结果列表（忽略失败的 Pool）
pub async fn detect_dex_from_pools_batch(
    rpc: &SolanaRpcClient,
    pool_addresses: &[&str],
) -> Vec<DexInfo> {
    let futures: Vec<_> =
        pool_addresses.iter().map(|&addr| detect_dex_from_pool(rpc, addr)).collect();

    // 并发执行所有请求
    let results = futures::future::join_all(futures).await;

    // 过滤掉失败的结果
    results.into_iter().filter_map(|result| result.ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dex_info_creation() {
        let info = DexInfo::new(
            "58L7CzkRV5qD1owUPurbAC7gUNV1kSzwj2TMkvgEpbjZ".to_string(),
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(),
        )
        .expect("应该成功创建 DexInfo");

        assert_eq!(info.dex_name(), "raydium_amm_v4");
        assert_eq!(info.display_name(), "Raydium AMM V4");
        assert_eq!(info.program_id, "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
    }

    #[test]
    fn test_dex_info_unknown_program() {
        let info = DexInfo::new(
            "somepooladdress".to_string(),
            "Unknown1111111111111111111111111111111111".to_string(),
        );

        assert!(info.is_none(), "未知的 Program ID 应该返回 None");
    }
}
