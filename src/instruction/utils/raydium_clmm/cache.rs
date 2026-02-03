// Raydium CLMM 缓存模块

use dashmap::DashMap;
use once_cell::sync::Lazy;

use crate::instruction::utils::raydium_clmm_types::PoolState;

use super::constants::MAX_CACHE_SIZE;

/// mint → pool_address 缓存
pub(crate) static MINT_TO_POOL_CACHE: Lazy<DashMap<solana_sdk::pubkey::Pubkey, solana_sdk::pubkey::Pubkey>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

/// pool_address → PoolState 数据缓存
pub(crate) static POOL_DATA_CACHE: Lazy<DashMap<solana_sdk::pubkey::Pubkey, PoolState>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

/// mint → Vec<(pool_address, PoolState)> 列表缓存（用于 list_pools_by_mint）
pub(crate) static MINT_TO_POOLS_LIST_CACHE: Lazy<DashMap<solana_sdk::pubkey::Pubkey, Vec<(solana_sdk::pubkey::Pubkey, PoolState)>>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

pub(crate) fn get_cached_pool_by_address(pool_address: &solana_sdk::pubkey::Pubkey) -> Option<PoolState> {
    POOL_DATA_CACHE.get(pool_address).map(|p| p.clone())
}

pub(crate) fn cache_pool_by_address(pool_address: &solana_sdk::pubkey::Pubkey, pool: &PoolState) {
    POOL_DATA_CACHE.insert(*pool_address, pool.clone());
}

pub(crate) fn get_cached_pool_address_by_mint(mint: &solana_sdk::pubkey::Pubkey) -> Option<solana_sdk::pubkey::Pubkey> {
    MINT_TO_POOL_CACHE.get(mint).map(|p| *p)
}

pub(crate) fn cache_pool_address_by_mint(mint: &solana_sdk::pubkey::Pubkey, pool_address: &solana_sdk::pubkey::Pubkey) {
    MINT_TO_POOL_CACHE.insert(*mint, *pool_address);
}

#[expect(dead_code, reason = "预留用于未来缓存策略优化")]
pub(crate) fn get_cached_pools_list_by_mint(mint: &solana_sdk::pubkey::Pubkey) -> Option<Vec<(solana_sdk::pubkey::Pubkey, PoolState)>> {
    MINT_TO_POOLS_LIST_CACHE.get(mint).map(|p| p.clone())
}

#[expect(dead_code, reason = "预留用于未来缓存策略优化")]
pub(crate) fn cache_pools_list_by_mint(mint: &solana_sdk::pubkey::Pubkey, pools: &[(solana_sdk::pubkey::Pubkey, PoolState)]) {
    MINT_TO_POOLS_LIST_CACHE.insert(*mint, pools.to_vec());
}

pub(crate) fn clear_all() {
    MINT_TO_POOL_CACHE.clear();
    POOL_DATA_CACHE.clear();
    MINT_TO_POOLS_LIST_CACHE.clear();
}
