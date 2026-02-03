//! Raydium CPMM 费率查询模块
//!
//! 从 AmmConfig 账户读取实际费率，支持缓存以避免重复 RPC 调用。
//!
//! # 实现状态
//!
//! ✅ 动态费率读取已实现并正常工作
//! ✅ 缓存机制已实现（TTL: 30分钟）
//! ⚠️  **已知问题**: 使用动态费率后，CPMM quote 计算仍有 ~0.54% 的误差
//!
//! # 误差分析
//!
//! 经过测试验证：
//! - 费率读取正确: trade=2500, protocol=120000, fund=40000
//! - 与硬编码常量完全相同
//! - **结论**: 误差不是由费率引起，可能来自其他因素：
//!   1. CPMM 计算公式的精度问题
//!   2. Reserve 数据获取时机
//!   3. 链上其他费用/调整机制
//!   4. 浮点数精度损失
//!
//! # 后续改进方向
//!
//! - [ ] 深入分析链上计算逻辑
//! - [ ] 对比 Raydium 官方 SDK 的实现
//! - [ ] 检查是否有其他隐藏的费用或调整
//! - [ ] 或者临时放宽误差容忍度到 1%

use crate::{
    common::{amm_config_cache::AmmConfigCache, SolanaRpcClient},
    instruction::utils::raydium_cpmm_types::{amm_config_decode, AmmConfig},
};
use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;

/// 费率配置
#[derive(Debug, Clone, Copy)]
pub struct FeeRates {
    /// 交易费率 (除以 1_000_000 得到百分比)
    pub trade_fee_rate: u64,
    /// 协议费率 (除以 1_000_000 得到百分比)
    pub protocol_fee_rate: u64,
    /// 基金费率 (除以 1_000_000 得到百分比)
    pub fund_fee_rate: u64,
    /// 创建者费率 (除以 1_000_000 得到百分比)
    pub creator_fee_rate: u64,
}

impl FeeRates {
    /// 从 AmmConfig 创建费率配置
    fn from_amm_config(config: &AmmConfig) -> Self {
        Self {
            trade_fee_rate: config.trade_fee_rate,
            protocol_fee_rate: config.protocol_fee_rate,
            fund_fee_rate: config.fund_fee_rate,
            creator_fee_rate: 0, // CPMM 目前没有 creator_fee
        }
    }

    /// 获取总费率（用于计算）
    ///
    /// 总费率 = 交易费率 + 协议费率 + 基金费率
    #[inline]
    pub fn total_fee_rate(&self) -> u64 {
        self.trade_fee_rate + self.protocol_fee_rate + self.fund_fee_rate
    }
}

/// 获取 AmmConfig 费率（带缓存）
///
/// # 流程
///
/// 1. 检查缓存
/// 2. 缓存未命中 → RPC 调用获取账户
/// 3. 跳过 8 字节 discriminator 后解码账户数据
/// 4. 存入缓存并返回
///
/// # Arguments
///
/// * `rpc` - RPC 客户端
/// * `amm_config` - AmmConfig 账户地址
///
/// # Returns
///
/// * `Ok(FeeRates)` - 费率配置
/// * `Err(anyhow::Error)` - 获取失败
///
/// # Example
///
/// ```ignore
/// use sol_trade_sdk::instruction::utils::raydium_cpmm::fee_queries::get_amm_config_fees;
///
/// let fees = get_amm_config_fees(&rpc, &amm_config_pubkey).await?;
/// println!("Trade fee rate: {}", fees.trade_fee_rate);
/// ```
pub async fn get_amm_config_fees(
    rpc: &SolanaRpcClient,
    amm_config: &Pubkey,
) -> Result<FeeRates, anyhow::Error> {
    // 1. 检查缓存
    if let Some(config) = AmmConfigCache::get(amm_config) {
        return Ok(FeeRates::from_amm_config(&config));
    }

    // 2. RPC 调用
    let account = rpc.get_account(amm_config).await?;

    // 3. 跳过 8 字节 discriminator 后解码
    // Solana Anchor 账户数据格式: [discriminator(8 bytes)] [account data]
    if account.data.len() < 8 {
        return Err(anyhow!("AmmConfig account data too short: {}", account.data.len()));
    }

    let config =
        amm_config_decode(&account.data[8..]).ok_or_else(|| anyhow!("Failed to decode AmmConfig"))?;

    // 4. 存入缓存
    AmmConfigCache::insert(*amm_config, config.clone());

    Ok(FeeRates::from_amm_config(&config))
}

/// 清除 AmmConfig 缓存
///
/// 通常用于测试或强制刷新配置。
///
/// # Example
///
/// ```ignore
/// use sol_trade_sdk::instruction::utils::raydium_cpmm::fee_queries::clear_amm_config_cache;
///
/// clear_amm_config_cache();
/// ```
pub fn clear_amm_config_cache() {
    AmmConfigCache::clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_rates_from_amm_config() {
        let config = AmmConfig {
            trade_fee_rate: 2500,
            protocol_fee_rate: 120000,
            fund_fee_rate: 40000,
            ..Default::default()
        };

        let fees = FeeRates::from_amm_config(&config);

        assert_eq!(fees.trade_fee_rate, 2500);
        assert_eq!(fees.protocol_fee_rate, 120000);
        assert_eq!(fees.fund_fee_rate, 40000);
        assert_eq!(fees.total_fee_rate(), 162500); // 2500 + 120000 + 40000
    }

    #[test]
    fn test_clear_cache() {
        clear_amm_config_cache();
        // 确保不 panic
        clear_amm_config_cache();
        clear_amm_config_cache();
    }
}
