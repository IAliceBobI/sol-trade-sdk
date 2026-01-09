use crate::{
    common::SolanaRpcClient,
    instruction::utils::raydium_clmm_types::{PoolState, pool_state_decode},
};
use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;
use solana_account_decoder::UiAccountData;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

/// Seeds for PDA derivation
pub mod seeds {
    pub const TICK_ARRAY_SEED: &[u8] = b"tick_array";
    pub const POOL_TICK_ARRAY_BITMAP_SEED: &[u8] = b"pool_tick_array_bitmap_extension";
}

/// Calculate tick array PDA
///
/// # Arguments
/// * `pool_id` - Pool state account address
/// * `start_tick_index` - Starting tick index for the tick array
///
/// # Returns
/// (tick_array_pda, bump)
///
/// Note: Reference implementation uses to_be_bytes() for tick index
pub fn get_tick_array_pda(pool_id: &Pubkey, start_tick_index: i32) -> Result<(Pubkey, u8), anyhow::Error> {
    let tick_index_bytes = start_tick_index.to_be_bytes(); // Use big-endian like reference implementation
    Pubkey::try_find_program_address(
        &[
            seeds::TICK_ARRAY_SEED,
            pool_id.as_ref(),
            &tick_index_bytes,
        ],
        &accounts::RAYDIUM_CLMM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find tick array PDA"))
}

/// Calculate tick array start index from current tick and tick spacing
///
/// # Arguments
/// * `tick_current` - Current tick
/// * `tick_spacing` - Tick spacing
///
/// # Returns
/// Starting tick index for the tick array containing the current tick
///
/// Each tick array contains 60 ticks (TICKS_PER_ARRAY = 60)
/// Implementation matches Raydium SDK V2: TickUtils.getTickArrayStartIndexByTick
///
/// Formula: getTickArrayBitIndex(tickIndex, tickSpacing) * tickCount(tickSpacing)
/// where tickCount = TICK_ARRAY_SIZE * tickSpacing
pub fn get_tick_array_start_index(tick_current: i32, tick_spacing: u16) -> i32 {
    const TICKS_PER_ARRAY: i32 = 60;
    let tick_spacing_i32 = tick_spacing as i32;

    // Calculate ticks per array (tickCount)
    let ticks_in_array = TICKS_PER_ARRAY * tick_spacing_i32;

    // Calculate tick array bit index (getTickArrayBitIndex)
    // This is the array index, not the tick index
    let mut start_index: i32 = tick_current / ticks_in_array;

    // Handle negative ticks: round down towards negative infinity
    if tick_current < 0 && tick_current % ticks_in_array != 0 {
        start_index = ((start_index as f64).ceil() as i32) - 1;
    } else {
        start_index = (start_index as f64).floor() as i32;
    }

    // Convert bit index to tick index
    start_index * ticks_in_array
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};
    pub const RAYDIUM_CLMM: Pubkey = pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
}

/// Calculate tick array bitmap extension PDA
///
/// # Arguments
/// * `pool_id` - Pool state account address
///
/// # Returns
/// (tick_array_bitmap_extension_pda, bump)
pub fn get_tick_array_bitmap_extension_pda(pool_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            seeds::POOL_TICK_ARRAY_BITMAP_SEED,
            pool_id.as_ref(),
        ],
        &accounts::RAYDIUM_CLMM,
    )
}

/// Find first initialized tick array from bitmap
///
/// This is a simplified version. In production, you should use the full bitmap logic
/// from the pool state's tick_array_bitmap field.
///
/// # Arguments
/// * `pool_state` - Pool state
/// * `zero_for_one` - Swap direction (true = token0 -> token1)
///
/// # Returns
/// First initialized tick array start index, or falls back to current tick's array
pub fn get_first_initialized_tick_array_start_index(
    pool_state: &PoolState,
    _zero_for_one: bool,
) -> i32 {
    // TODO: Implement full bitmap search logic
    // For now, fall back to current tick's array
    get_tick_array_start_index(pool_state.tick_current, pool_state.tick_spacing)
}

// ==================== 缓存模块 ====================

const MAX_CACHE_SIZE: usize = 50_000;

pub(crate) mod raydium_clmm_cache {
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

// 常量偏移量
const TOKEN_MINT0_OFFSET: usize = 73;
const TOKEN_MINT1_OFFSET: usize = 105;

pub async fn get_pool_by_address(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool) = raydium_clmm_cache::get_cached_pool_by_address(pool_address) {
        return Ok(pool);
    }
    // 2. RPC 查询
    let account = rpc.get_account(pool_address).await?;
    if account.owner != accounts::RAYDIUM_CLMM {
        return Err(anyhow!("Account is not owned by Raydium CLMM program"));
    }
    let pool_state = pool_state_decode(&account.data[8..])
        .ok_or_else(|| anyhow!("Failed to decode pool state"))?;
    // 3. 写入缓存
    raydium_clmm_cache::cache_pool_by_address(pool_address, &pool_state);
    Ok(pool_state)
}

pub async fn get_pool_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool_address) = raydium_clmm_cache::get_cached_pool_address_by_mint(mint) {
        if let Some(pool) = raydium_clmm_cache::get_cached_pool_by_address(&pool_address) {
            return Ok((pool_address, pool));
        }
    }
    // 2. RPC 查询
    let (pool_address, pool) = find_pool_by_mint_impl(rpc, mint).await?;
    // 3. 写入缓存
    raydium_clmm_cache::cache_pool_address_by_mint(mint, &pool_address);
    raydium_clmm_cache::cache_pool_by_address(&pool_address, &pool);
    Ok((pool_address, pool))
}

pub async fn get_pool_by_address_force(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    raydium_clmm_cache::POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

pub async fn get_pool_by_mint_force(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    raydium_clmm_cache::MINT_TO_POOL_CACHE.remove(mint);
    get_pool_by_mint(rpc, mint).await
}

pub fn clear_pool_cache() {
    raydium_clmm_cache::clear_all();
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

    let filters = vec![
        // CLMM 账户总大小 = 1536 (数据) + 8 (discriminator) = 1544
        RpcFilterType::DataSize(1544),
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

    let accounts = rpc.get_program_ui_accounts_with_config(&accounts::RAYDIUM_CLMM, config).await?;

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
    // Parallel search: try both token_mint0 and token_mint1 offsets simultaneously
    let (result0, result1) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN_MINT0_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN_MINT1_OFFSET)
    );

    let mut all_pools: Vec<(Pubkey, PoolState)> = result0.unwrap_or_default();

    // Merge token_mint1 results
    if let Ok(quote_pools) = result1 {
        use std::collections::HashSet;
        let mut seen: HashSet<Pubkey> = all_pools.iter().map(|(addr, _)| *addr).collect();
        for (addr, pool) in quote_pools {
            if seen.insert(addr) {
                all_pools.push((addr, pool));
            }
        }
    }

    if all_pools.is_empty() {
        return Err(anyhow!("No CLMM pool found for mint {}", mint));
    }

    // Return first pool (could be improved with liquidity sorting)
    let (address, pool) = all_pools[0].clone();
    Ok((address, pool))
}

/// List all Raydium CLMM pools that contain the given mint as token0 or token1.
///
/// This is a discovery helper for routing/selection layers. It does NOT pick a best pool.

pub async fn list_pools_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    use std::collections::HashSet;

    // Parallel search: scan both token_mint0 and token_mint1 simultaneously
    let (result0, result1) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN_MINT0_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN_MINT1_OFFSET)
    );

    let mut out: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut seen: HashSet<Pubkey> = HashSet::new();

    // Merge token_mint0 results
    if let Ok(token0_pools) = result0 {
        for (addr, pool) in token0_pools {
            if seen.insert(addr) {
                out.push((addr, pool));
            }
        }
    }

    // Merge token_mint1 results
    if let Ok(token1_pools) = result1 {
        for (addr, pool) in token1_pools {
            if seen.insert(addr) {
                out.push((addr, pool));
            }
        }
    }

    if out.is_empty() {
        return Err(anyhow!("No CLMM pool found for mint {}", mint));
    }
    Ok(out)
}

/// Quote an exact-in swap against a Raydium CLMM pool.
///
/// IMPORTANT: This implementation currently assumes the swap does **not** cross initialized ticks
/// (i.e. stays within the current tick). It still reads the current tick array account to
/// validate availability and for future extension, but does not yet decode tick liquidity nets.
///
/// - `zero_for_one=true`: token0 -> token1
/// - `zero_for_one=false`: token1 -> token0
pub async fn quote_exact_in(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_in: u64,
    zero_for_one: bool,
) -> Result<crate::utils::quote::QuoteExactInResult, anyhow::Error> {
    let pool_state = get_pool_by_address(rpc, pool_address).await?;

    // Read the current tick array account (best-effort) so higher layers can account for IO cost.
    let start_index = get_tick_array_start_index(pool_state.tick_current, pool_state.tick_spacing);
    if let Ok((tick_array_pda, _)) = get_tick_array_pda(pool_address, start_index) {
        // ignore errors; quote can still be approximated from pool_state
        let _ = rpc.get_account(&tick_array_pda).await;
    }

    // Swap math (Uniswap v3 style) in Q64.64 sqrt price space.
    // We approximate: L constant, no tick crossing.
    let l = pool_state.liquidity;
    if l == 0 || amount_in == 0 {
        return Ok(crate::utils::quote::QuoteExactInResult {
            amount_out: 0,
            fee_amount: 0,
            price_impact_bps: None,
            extra_accounts_read: 1,
        });
    }

    // sqrt_price_x64 is Q64.64. We'll operate in u128.
    let sqrt_p = pool_state.sqrt_price_x64;
    // avoid division by zero
    if sqrt_p == 0 {
        return Ok(crate::utils::quote::QuoteExactInResult {
            amount_out: 0,
            fee_amount: 0,
            price_impact_bps: None,
            extra_accounts_read: 1,
        });
    }

    // Helpers for fixed-point math: represent 1.0 as Q64.64 = 1<<64
    const Q64: u128 = 1u128 << 64;

    let amount_in_u128 = amount_in as u128;
    let amount_out_u128: u128;

    if zero_for_one {
        // token0 in, token1 out
        // sqrtP_next = 1 / (1/sqrtP + amount0_in/L)
        // 1/sqrtP in Q64.64: inv_sqrt = Q64^2 / sqrtP
        let inv_sqrt = (Q64 * Q64) / sqrt_p;
        // amount0_in / L in Q64.64: (amount0_in * Q64) / L
        let delta = (amount_in_u128 * Q64) / l;
        let inv_sqrt_next = inv_sqrt + delta;
        let sqrt_p_next = (Q64 * Q64) / inv_sqrt_next;
        // amount1_out = L * (sqrtP - sqrtP_next) / Q64
        amount_out_u128 = (l * (sqrt_p.saturating_sub(sqrt_p_next))) / Q64;
    } else {
        // token1 in, token0 out
        // sqrtP_next = sqrtP + amount1_in / L
        // amount1_in / L in Q64.64: (amount1_in * Q64) / L
        let delta = (amount_in_u128 * Q64) / l;
        let sqrt_p_next = sqrt_p + delta;
        // amount0_out = L * (1/sqrtP - 1/sqrtP_next)
        let inv_sqrt = (Q64 * Q64) / sqrt_p;
        let inv_sqrt_next = (Q64 * Q64) / sqrt_p_next;
        // result in token0 units: L * (inv_sqrt - inv_sqrt_next) / Q64
        amount_out_u128 = (l * inv_sqrt.saturating_sub(inv_sqrt_next)) / Q64;
    }

    let amount_out = u64::try_from(amount_out_u128).unwrap_or(u64::MAX);
    Ok(crate::utils::quote::QuoteExactInResult {
        amount_out,
        fee_amount: 0,          // TODO: integrate fee tier from config once available
        price_impact_bps: None, // TODO: compute using execution price vs spot
        extra_accounts_read: 1,
    })
}
