use crate::{
    common::SolanaRpcClient,
    instruction::utils::raydium_cpmm_types::{PoolState, pool_state_decode},
    trading::core::params::RaydiumCpmmParams,
    constants::{WSOL_TOKEN_ACCOUNT, USDC_MINT, USDT_MINT},
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
}

// 常量偏移量（包含 discriminator）
// 根据实际数据解析结果，mintA 在 offset 160（不包含 discriminator），mintB 在 offset 192（不包含 discriminator）
// RPC 查询时使用包含 discriminator 的偏移量，所以需要加 8
const TOKEN0_MINT_OFFSET: usize = 168;  // mintA offset (160 + 8 discriminator)
const TOKEN1_MINT_OFFSET: usize = 200;  // mintB offset (192 + 8 discriminator)

/// 判断是否为 Hot Mint（主流桥接资产）
/// 当前包含：WSOL、USDC、USDT
fn is_hot_mint(mint: &Pubkey) -> bool {
    *mint == WSOL_TOKEN_ACCOUNT || *mint == USDC_MINT || *mint == USDT_MINT
}

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

/// 按 LP 供应量选择最佳池（CPMM 池没有交易量字段，使用 lp_supply 作为流动性指标）
/// 
/// 策略：
/// - 优先选择已激活且有流动性的池
/// - LP 供应量越大，说明流动性越好
fn select_best_pool_by_liquidity(pools: &[(Pubkey, PoolState)]) -> (Pubkey, PoolState) {
    if pools.is_empty() {
        panic!("Cannot select best pool from empty list");
    }

    if pools.len() == 1 {
        return pools[0].clone();
    }

    // 优先选择已激活且有流动性的池
    let mut active_pools: Vec<_> = pools
        .iter()
        .filter(|(_, pool)| pool.status != 0 && pool.lp_supply > 0)
        .map(|(addr, pool)| (*addr, pool.clone()))
        .collect();

    if active_pools.is_empty() {
        // 如果全部池都不活跃，使用所有池
        active_pools = pools.to_vec();
    }

    // 按 LP 供应量排序
    active_pools.sort_by(|(_, pool_a), (_, pool_b)| {
        // 按 LP 供应量降序排序
        match pool_b.lp_supply.cmp(&pool_a.lp_supply) {
            std::cmp::Ordering::Equal => {
                // LP 供应量相同时，按开池时间排序（更早的池更成熟）
                pool_b.open_time.cmp(&pool_a.open_time)
            }
            other => other,
        }
    });

    // 返回 LP 供应量最高的池
    active_pools.into_iter().next().unwrap()
}

/// 内部实现：查找指定 mint 的所有 Raydium CPMM Pool
///
/// 策略：
/// 1. 并行查询 token0_mint 与 token1_mint 包含该 mint 的所有池
/// 2. 合并并去重
async fn find_all_pools_by_mint_impl(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    use std::collections::HashSet;

    let (token0_result, token1_result) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN0_MINT_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN1_MINT_OFFSET),
    );

    let mut all_pools: Vec<(Pubkey, PoolState)> = Vec::new();

    if let Ok(pools) = token0_result {
        all_pools.extend(pools);
    }

    if let Ok(quote_pools) = token1_result {
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

    Ok(all_pools)
}

/// 内部实现：查找 mint 对应的最优池
async fn find_pool_by_mint_impl(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 获取所有池子
    let all_pools = find_all_pools_by_mint_impl(rpc, mint).await?;

    // 分类：Hot 对（包含 WSOL/USDC/USDT）vs 其他对
    let mut hot_pools: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, PoolState)> = Vec::new();

    for (addr, pool) in all_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if pool.token0_mint == *mint {
            pool.token1_mint
        } else if pool.token1_mint == *mint {
            pool.token0_mint
        } else {
            // 理论上不会出现，但为了稳健性仍加入非 Hot 集合
            other_pools.push((addr, pool));
            continue;
        };

        if is_hot_mint(&other_mint) {
            hot_pools.push((addr, pool));
        } else {
            other_pools.push((addr, pool));
        }
    }

    let best_pool = if !hot_pools.is_empty() {
        // Hot 对优先：通常是 mint/WSOL、mint/USDC、mint/USDT 等主路由
        // 使用 LP 供应量选池
        select_best_pool_by_liquidity(&hot_pools)
    } else if *mint == WSOL_TOKEN_ACCOUNT {
        // 特殊情况：当 mint 本身是 WSOL 时
        // 在所有池中按 LP 供应量选择
        select_best_pool_by_liquidity(&other_pools)
    } else {
        // 没有 Hot 对时，使用 LP 供应量选池
        select_best_pool_by_liquidity(&other_pools)
    };

    Ok(best_pool)
}

/// List all CPMM pools that contain the given mint as token0 or token1.
///
/// This is a discovery helper for routing/selection layers. It does NOT pick a best pool.
/// Results are cached to improve performance on repeated queries.

pub async fn list_pools_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    // 1. 检查缓存
    if let Some(cached_pools) = raydium_cpmm_cache::get_cached_pools_list_by_mint(mint) {
        return Ok(cached_pools);
    }

    // 2. 通过共用函数查询所有池子
    let pools = find_all_pools_by_mint_impl(rpc, mint).await?;

    // 3. 写入缓存
    raydium_cpmm_cache::cache_pools_list_by_mint(mint, &pools);

    Ok(pools)
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