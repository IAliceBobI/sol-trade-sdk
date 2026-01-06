use crate::{
    common::SolanaRpcClient,
    instruction::utils::raydium_amm_v4_types::{amm_info_decode, AmmInfo},
};
use anyhow::anyhow;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;

/// Constants used as seeds for deriving PDAs (Program Derived Addresses)
pub mod seeds {
    pub const POOL_SEED: &[u8] = b"pool";
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};
    pub const AUTHORITY: Pubkey = pubkey!("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    pub const RAYDIUM_AMM_V4: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");

    pub const TRADE_FEE_NUMERATOR: u64 = 25;
    pub const TRADE_FEE_DENOMINATOR: u64 = 10000;
    pub const SWAP_FEE_NUMERATOR: u64 = 25;
    pub const SWAP_FEE_DENOMINATOR: u64 = 10000;

    // META

    pub const AUTHORITY_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: AUTHORITY,
            is_signer: false,
            is_writable: false,
        };
}

pub const SWAP_BASE_IN_DISCRIMINATOR: &[u8] = &[9];
pub const SWAP_BASE_OUT_DISCRIMINATOR: &[u8] = &[11];

// ==================== 缓存模块 ====================

const MAX_CACHE_SIZE: usize = 50_000;

/// pool_address → AmmInfo 数据缓存
static POOL_DATA_CACHE: Lazy<DashMap<Pubkey, AmmInfo>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

/// 从缓存中获取 Pool 信息
pub(crate) fn get_cached_pool_by_address(pool_address: &Pubkey) -> Option<AmmInfo> {
    POOL_DATA_CACHE.get(pool_address).map(|p| p.clone())
}

/// 将 Pool 信息写入缓存
pub(crate) fn cache_pool_by_address(pool_address: &Pubkey, amm_info: &AmmInfo) {
    POOL_DATA_CACHE.insert(*pool_address, amm_info.clone());
}

/// 清除所有缓存
pub(crate) fn clear_pool_cache_internal() {
    POOL_DATA_CACHE.clear();
}

// ==================== 公共函数 ====================

/// 根据地址获取 AMM Pool 信息（带缓存）
///
/// 如果缓存中有该 Pool 的信息，直接从缓存返回；
/// 否则通过 RPC 查询，并将结果写入缓存。
pub async fn get_pool_by_address(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<AmmInfo, anyhow::Error> {
    // 1. 检查缓存
    if let Some(amm_info) = get_cached_pool_by_address(pool_address) {
        return Ok(amm_info);
    }

    // 2. RPC 查询
    let account = rpc.get_account(pool_address).await?;
    if account.owner != accounts::RAYDIUM_AMM_V4 {
        return Err(anyhow!("Account is not owned by Raydium AMM V4 program"));
    }
    let amm_info = amm_info_decode(&account.data)
        .ok_or_else(|| anyhow!("Failed to decode amm info"))?;

    // 3. 写入缓存
    cache_pool_by_address(pool_address, &amm_info);

    Ok(amm_info)
}

/// 强制刷新：强制重新查询指定 Pool
///
/// 先从缓存中删除该 Pool，然后重新查询并写入缓存。
pub async fn get_pool_by_address_force(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<AmmInfo, anyhow::Error> {
    POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

/// 清除所有 Pool 缓存
///
/// 清除所有缓存中的 Pool 数据。
pub fn clear_pool_cache() {
    clear_pool_cache_internal();
}
