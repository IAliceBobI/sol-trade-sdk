use crate::{
    common::SolanaRpcClient,
    instruction::utils::meteora_damm_v2_types::{Pool, pool_decode},
};
use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;

/// Constants used as seeds for deriving PDAs (Program Derived Addresses)
pub mod seeds {
    pub const EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};

    pub const AUTHORITY: Pubkey = pubkey!("HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC");
    pub const METEORA_DAMM_V2: Pubkey = pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");

    // META

    pub const METEORA_DAMM_V2_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: METEORA_DAMM_V2,
            is_signer: false,
            is_writable: false,
        };

    pub const AUTHORITY_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: AUTHORITY,
            is_signer: false,
            is_writable: false,
        };
}

pub const SWAP_DISCRIMINATOR: &[u8] = &[248, 198, 158, 145, 225, 117, 135, 200];

// ==================== 缓存模块 ====================

const MAX_CACHE_SIZE: usize = 50_000;

pub(crate) mod meteora_cache {
    use super::*;
    use dashmap::DashMap;
    use once_cell::sync::Lazy;

    /// pool_address → Pool 数据缓存
    pub(crate) static POOL_DATA_CACHE: Lazy<DashMap<Pubkey, Pool>> =
        Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

    #[allow(dead_code)]
    pub(crate) fn get_cached_pool_by_address(pool_address: &Pubkey) -> Option<Pool> {
        POOL_DATA_CACHE.get(pool_address).map(|p| p.clone())
    }

    #[allow(dead_code)]
    pub(crate) fn cache_pool_by_address(pool_address: &Pubkey, pool: &Pool) {
        POOL_DATA_CACHE.insert(*pool_address, pool.clone());
    }

    pub(crate) fn clear_all() {
        POOL_DATA_CACHE.clear();
    }
}

pub async fn get_pool_by_address(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<Pool, anyhow::Error> {
    // RPC 查询（不缓存，每次获取最新数据）
    let account = rpc.get_account(pool_address).await?;
    if account.owner != accounts::METEORA_DAMM_V2 {
        return Err(anyhow!("Account is not owned by Meteora Damm V2 program"));
    }
    // 使用修改后的 pool_decode（传入 program_id）
    let pool = pool_decode(&account.data[8..], account.owner)
        .ok_or_else(|| anyhow!("Failed to decode pool"))?;
    // 不写入缓存
    Ok(pool)
}

pub async fn get_pool_by_address_force(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<Pool, anyhow::Error> {
    meteora_cache::POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

pub fn clear_pool_cache() {
    meteora_cache::clear_all();
}

#[inline]
pub fn get_event_authority_pda() -> Pubkey {
    Pubkey::find_program_address(&[seeds::EVENT_AUTHORITY_SEED], &accounts::METEORA_DAMM_V2).0
}
