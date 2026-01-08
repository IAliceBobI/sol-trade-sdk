use crate::{
    common::SolanaRpcClient,
    instruction::utils::raydium_cpmm_types::{PoolState, pool_state_decode},
    trading::core::params::RaydiumCpmmParams,
};
use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;
use solana_account_decoder::UiAccountData;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// Constants used as seeds for deriving PDAs (Program Derived Addresses)
pub mod seeds {
    pub const POOL_SEED: &[u8] = b"pool";
    pub const POOL_VAULT_SEED: &[u8] = b"pool_vault";
    pub const OBSERVATION_STATE_SEED: &[u8] = b"observation";
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};
    pub const AUTHORITY: Pubkey = pubkey!("GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL");
    pub const RAYDIUM_CPMM: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
    pub const FEE_RATE_DENOMINATOR_VALUE: u128 = 1_000_000;
    pub const TRADE_FEE_RATE: u64 = 2500;
    pub const CREATOR_FEE_RATE: u64 = 0;
    pub const PROTOCOL_FEE_RATE: u64 = 120000;
    pub const FUND_FEE_RATE: u64 = 40000;
    // META
    pub const AUTHORITY_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: AUTHORITY,
            is_signer: false,
            is_writable: false,
        };
}

pub const SWAP_BASE_IN_DISCRIMINATOR: &[u8] = &[143, 190, 90, 218, 196, 30, 51, 222];
pub const SWAP_BASE_OUT_DISCRIMINATOR: &[u8] = &[55, 217, 98, 86, 163, 74, 180, 173];

// ==================== 缓存模块 ====================

const MAX_CACHE_SIZE: usize = 50_000;

pub(crate) mod raydium_cpmm_cache {
    use super::*;
    use dashmap::DashMap;
    use once_cell::sync::Lazy;

    /// mint → pool_address 缓存
    pub(crate) static MINT_TO_POOL_CACHE: Lazy<DashMap<Pubkey, Pubkey>> =
        Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

    /// pool_address → PoolState 数据缓存
    pub(crate) static POOL_DATA_CACHE: Lazy<DashMap<Pubkey, PoolState>> =
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

    pub(crate) fn clear_all() {
        MINT_TO_POOL_CACHE.clear();
        POOL_DATA_CACHE.clear();
    }
}

// 常量偏移量（包含 discriminator）
// 根据实际数据解析结果，mintA 在 offset 160（不包含 discriminator），mintB 在 offset 192（不包含 discriminator）
// RPC 查询时使用包含 discriminator 的偏移量，所以需要加 8
const TOKEN0_MINT_OFFSET: usize = 168;  // mintA offset (160 + 8 discriminator)
const TOKEN1_MINT_OFFSET: usize = 200;  // mintB offset (192 + 8 discriminator)

pub async fn get_pool_by_address(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool) = raydium_cpmm_cache::get_cached_pool_by_address(pool_address) {
        return Ok(pool);
    }
    // 2. RPC 查询
    let account = rpc.get_account(pool_address).await?;
    if account.owner != accounts::RAYDIUM_CPMM {
        return Err(anyhow!("Account is not owned by Raydium Cpmm program"));
    }
    let pool_state = pool_state_decode(&account.data[8..])
        .ok_or_else(|| anyhow!("Failed to decode pool state"))?;
    // 3. 写入缓存
    raydium_cpmm_cache::cache_pool_by_address(pool_address, &pool_state);
    Ok(pool_state)
}

pub async fn get_pool_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool_address) = raydium_cpmm_cache::get_cached_pool_address_by_mint(mint) {
        if let Some(pool) = raydium_cpmm_cache::get_cached_pool_by_address(&pool_address) {
            return Ok((pool_address, pool));
        }
    }
    // 2. RPC 查询
    let (pool_address, pool) = find_pool_by_mint_impl(rpc, mint).await?;
    // 3. 写入缓存
    raydium_cpmm_cache::cache_pool_address_by_mint(mint, &pool_address);
    raydium_cpmm_cache::cache_pool_by_address(&pool_address, &pool);
    Ok((pool_address, pool))
}

pub async fn get_pool_by_address_force(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    raydium_cpmm_cache::POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

pub async fn get_pool_by_mint_force(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    raydium_cpmm_cache::MINT_TO_POOL_CACHE.remove(mint);
    get_pool_by_mint(rpc, mint).await
}

pub fn clear_pool_cache() {
    raydium_cpmm_cache::clear_all();
}

pub fn get_pool_pda(amm_config: &Pubkey, mint1: &Pubkey, mint2: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 4] =
        &[seeds::POOL_SEED, amm_config.as_ref(), mint1.as_ref(), mint2.as_ref()];
    let program_id: &Pubkey = &accounts::RAYDIUM_CPMM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

pub fn get_vault_pda(pool_state: &Pubkey, mint: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 3] = &[seeds::POOL_VAULT_SEED, pool_state.as_ref(), mint.as_ref()];
    let program_id: &Pubkey = &accounts::RAYDIUM_CPMM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

pub fn get_observation_state_pda(pool_state: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 2] = &[seeds::OBSERVATION_STATE_SEED, pool_state.as_ref()];
    let program_id: &Pubkey = &accounts::RAYDIUM_CPMM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

/// Get the balances of two tokens in the pool
///
/// # Returns
/// Returns token0_balance, token1_balance
pub async fn get_pool_token_balances(
    rpc: &SolanaRpcClient,
    pool_state: &Pubkey,
    token0_mint: &Pubkey,
    token1_mint: &Pubkey,
) -> Result<(u64, u64), anyhow::Error> {
    let token0_vault = get_vault_pda(pool_state, token0_mint).unwrap();
    let token1_vault = get_vault_pda(pool_state, token1_mint).unwrap();

    let (token0_balance_result, token1_balance_result) = tokio::join!(
        rpc.get_token_account_balance(&token0_vault),
        rpc.get_token_account_balance(&token1_vault),
    );

    let token0_balance = token0_balance_result?;
    let token1_balance = token1_balance_result?;

    // Parse balance string to u64
    let token0_amount = token0_balance
        .amount
        .parse::<u64>()
        .map_err(|e| anyhow!("Failed to parse token0 balance: {}", e))?;

    let token1_amount = token1_balance
        .amount
        .parse::<u64>()
        .map_err(|e| anyhow!("Failed to parse token1 balance: {}", e))?;

    Ok((token0_amount, token1_amount))
}

/// Quote an exact-in swap against a Raydium CPMM pool.
///
/// - If `is_token0_in=true`: token0 -> token1
/// - If `is_token0_in=false`: token1 -> token0
pub async fn quote_exact_in(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_in: u64,
    is_token0_in: bool,
) -> Result<crate::utils::quote::QuoteExactInResult, anyhow::Error> {
    let pool_state = get_pool_by_address(rpc, pool_address).await?;
    let (token0_reserve, token1_reserve) =
        get_pool_token_balances(rpc, pool_address, &pool_state.token0_mint, &pool_state.token1_mint)
            .await?;

    let q = crate::utils::calc::raydium_cpmm::compute_swap_amount(
        token0_reserve,
        token1_reserve,
        is_token0_in,
        amount_in,
        0,
    );
    Ok(crate::utils::quote::QuoteExactInResult {
        amount_out: q.amount_out,
        fee_amount: q.fee,
        price_impact_bps: None,
        extra_accounts_read: 2,
    })
}

/// 内部实现：通过 offset 查找所有 Pool
async fn find_pools_by_mint_offset_collect(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
    offset: usize,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    use solana_account_decoder::UiAccountEncoding;
    use solana_rpc_client_api::{config::RpcProgramAccountsConfig, filter::RpcFilterType};
    use solana_client::rpc_filter::Memcmp;

    // 暂时移除 DataSize 过滤，只使用 Memcmp 过滤
    // let filters = vec![
    //     RpcFilterType::DataSize((POOL_STATE_SIZE + 8) as u64),
    //     RpcFilterType::Memcmp(Memcmp::new_base58_encoded(offset, &mint.to_bytes())),
    // ];
    let filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(offset, &mint.to_bytes())),
    ];
    let config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: solana_rpc_client_api::config::RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: None,
            min_context_slot: None,
        },
        with_context: None,
        sort_results: None,
    };

    let accounts = rpc.get_program_ui_accounts_with_config(&accounts::RAYDIUM_CPMM, config).await?;

    let pools: Vec<(Pubkey, PoolState)> = accounts
        .into_iter()
        .filter_map(|(addr, acc)| {
            let data_bytes = match &acc.data {
                UiAccountData::Binary(base64_str, _) => STANDARD.decode(base64_str).ok()?,
                _ => return None,
            };
            if data_bytes.len() > 8 {
                pool_state_decode(&data_bytes[8..]).map(|pool| (addr, pool))
            } else {
                None
            }
        })
        .collect();

    Ok(pools)
}

/// 内部实现：查找 mint 对应的最优池
async fn find_pool_by_mint_impl(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 并行扫描 token0_mint 和 token1_mint 对应的池
    let (token0_result, token1_result) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN0_MINT_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN1_MINT_OFFSET),
    );

    let mut all_pools: Vec<(Pubkey, PoolState)> = Vec::new();

    if let Ok(pools) = token0_result {
        all_pools.extend(pools);
    }

    if let Ok(quote_pools) = token1_result {
        use std::collections::HashSet;
        let mut seen: HashSet<Pubkey> = all_pools.iter().map(|(addr, _)| *addr).collect();
        for (addr, pool) in quote_pools {
            if seen.insert(addr) {
                all_pools.push((addr, pool));
            }
        }
    }

    if all_pools.is_empty() {
        return Err(anyhow!("No CPMM pool found for mint {}", mint));
    }

    // Return first pool (could be improved with liquidity sorting)
    let (address, pool) = all_pools[0].clone();
    Ok((address, pool))
}

/// List all CPMM pools that contain the given mint as token0 or token1.
///
/// This is a discovery helper for routing/selection layers. It does NOT pick a best pool.

pub async fn list_pools_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    use std::collections::HashSet;

    let mut out: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut seen: HashSet<Pubkey> = HashSet::new();

    // 并行扫描 token0_mint 和 token1_mint 对应的池
    let (token0_result, token1_result) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN0_MINT_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN1_MINT_OFFSET),
    );

    if let Ok(token0_pools) = token0_result {
        for (addr, pool) in token0_pools {
            if seen.insert(addr) {
                out.push((addr, pool));
            }
        }
    }

    if let Ok(token1_pools) = token1_result {
        for (addr, pool) in token1_pools {
            if seen.insert(addr) {
                out.push((addr, pool));
            }
        }
    }

    if out.is_empty() {
        return Err(anyhow!("No CPMM pool found for mint {}", mint));
    }
    Ok(out)
}

/// Helper function to get token vault account address
///
/// # Parameters
/// - `pool_state`: Pool state account address
/// - `token_mint`: Token mint address
/// - `protocol_params`: Protocol parameters
///
/// # Returns
/// Returns the corresponding token vault account address
pub fn get_vault_account(
    pool_state: &Pubkey,
    token_mint: &Pubkey,
    protocol_params: &RaydiumCpmmParams,
) -> Pubkey {
    if protocol_params.base_mint == *token_mint && protocol_params.base_vault != Pubkey::default() {
        protocol_params.base_vault
    } else if protocol_params.quote_mint == *token_mint && protocol_params.quote_vault != Pubkey::default() {
        protocol_params.quote_vault
    } else {
        get_vault_pda(pool_state, token_mint).unwrap()
    }
}