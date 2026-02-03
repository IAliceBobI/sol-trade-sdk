// Raydium CPMM 缓存管理

use crate::instruction::utils::raydium_cpmm_types::PoolState;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;

use super::constants::MAX_CACHE_SIZE;

/// mint → pool_address 缓存
pub(crate) static MINT_TO_POOL_CACHE: Lazy<DashMap<Pubkey, Pubkey>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

/// pool_address → PoolState 数据缓存
pub(crate) static POOL_DATA_CACHE: Lazy<DashMap<Pubkey, PoolState>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

/// mint → Vec<(pool_address, PoolState)> 列表缓存（用于 list_pools_by_mint）
pub(crate) static MINT_TO_POOLS_LIST_CACHE: Lazy<DashMap<Pubkey, Vec<(Pubkey, PoolState)>>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

pub(crate) fn get_cached_pool_by_address(pool_address: &Pubkey) -> Option<PoolState> {
    POOL_DATA_CACHE.get(pool_address).map(|p| p.clone())
}

pub(crate) fn cache_pool_by_address(pool_address: &Pubkey, pool: &PoolState) {
    POOL_DATA_CACHE.insert(*pool_address, pool.clone());
}

pub(crate) fn get_cached_pool_address_by_mint(mint: &Pubkey) -> Option<Pubkey> {
    MINT_TO_POOL_CACHE.get(mint).map(|p| *p)
}

pub(crate) fn cache_pool_address_by_mint(mint: &Pubkey, pool_address: &Pubkey) {
    MINT_TO_POOL_CACHE.insert(*mint, *pool_address);
}

pub(crate) fn get_cached_pools_list_by_mint(mint: &Pubkey) -> Option<Vec<(Pubkey, PoolState)>> {
    MINT_TO_POOLS_LIST_CACHE.get(mint).map(|p| p.clone())
}

pub(crate) fn cache_pools_list_by_mint(mint: &Pubkey, pools: &[(Pubkey, PoolState)]) {
    MINT_TO_POOLS_LIST_CACHE.insert(*mint, pools.to_vec());
}

pub(crate) fn clear_all() {
    MINT_TO_POOL_CACHE.clear();
    POOL_DATA_CACHE.clear();
    MINT_TO_POOLS_LIST_CACHE.clear();
}
