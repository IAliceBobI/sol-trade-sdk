use crate::{
    common::{SolanaRpcClient, auto_mock_rpc::PoolRpcClient},
    constants::{SOL_MINT, USDC_MINT, USDT_MINT},
    instruction::utils::raydium_clmm_types::{
        pool_state_decode, amm_config_decode, tick_array_state_decode,
        PoolState, AmmConfig, TickArrayState,
    },
};
use anyhow::anyhow;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use solana_account_decoder::UiAccountData;
use solana_sdk::{pubkey, pubkey::Pubkey};

/// Raydium CLMM WSOL-USDT 锚定池（用于 USD 价格计算）
/// 如果不传入锚定池参数，默认使用此池
pub const DEFAULT_WSOL_USDT_CLMM_POOL: Pubkey = pubkey!("ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6");

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
pub fn get_tick_array_pda(
    pool_id: &Pubkey,
    start_tick_index: i32,
) -> Result<(Pubkey, u8), anyhow::Error> {
    let tick_index_bytes = start_tick_index.to_be_bytes(); // Use big-endian like reference implementation
    Pubkey::try_find_program_address(
        &[seeds::TICK_ARRAY_SEED, pool_id.as_ref(), &tick_index_bytes],
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

/// 判断是否为 Hot Mint（主流桥接资产）
/// 当前包含：WSOL、USDC、USDT
fn is_hot_mint(mint: &Pubkey) -> bool {
    *mint == SOL_MINT || *mint == USDC_MINT || *mint == USDT_MINT
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
        &[seeds::POOL_TICK_ARRAY_BITMAP_SEED, pool_id.as_ref()],
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

    #[expect(dead_code, reason = "预留用于未来缓存策略优化")]
    pub(crate) fn get_cached_pools_list_by_mint(mint: &Pubkey) -> Option<Vec<(Pubkey, PoolState)>> {
        MINT_TO_POOLS_LIST_CACHE.get(mint).map(|p| p.clone())
    }

    #[expect(dead_code, reason = "预留用于未来缓存策略优化")]
    pub(crate) fn cache_pools_list_by_mint(mint: &Pubkey, pools: &[(Pubkey, PoolState)]) {
        MINT_TO_POOLS_LIST_CACHE.insert(*mint, pools.to_vec());
    }

    pub(crate) fn clear_all() {
        MINT_TO_POOL_CACHE.clear();
        POOL_DATA_CACHE.clear();
        MINT_TO_POOLS_LIST_CACHE.clear();
    }
}

// 常量偏移量
const TOKEN_MINT0_OFFSET: usize = 73;
const TOKEN_MINT1_OFFSET: usize = 105;

/// 使用 PoolRpcClient trait 获取 Pool（支持 Auto Mock）
///
/// 这是一个泛型版本，可以接受任何实现了 PoolRpcClient 的客户端。
/// 支持标准的 RpcClient 和 AutoMockRpcClient。
pub async fn get_pool_by_address<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool) = raydium_clmm_cache::get_cached_pool_by_address(pool_address) {
        return Ok(pool);
    }
    // 2. RPC 查询
    let account = rpc.get_account(pool_address).await
        .map_err(|e| anyhow!("RPC 调用失败: {}", e))?;
    if account.owner != accounts::RAYDIUM_CLMM {
        return Err(anyhow!("Account is not owned by Raydium CLMM program"));
    }
    let pool_state = pool_state_decode(&account.data[8..])
        .ok_or_else(|| anyhow!("Failed to decode pool state"))?;
    // 3. 写入缓存
    raydium_clmm_cache::cache_pool_by_address(pool_address, &pool_state);
    Ok(pool_state)
}

/// 获取 amm_config 配置
pub async fn get_amm_config(
    rpc: &SolanaRpcClient,
    amm_config_address: &Pubkey,
) -> Result<AmmConfig, anyhow::Error> {
    let account = rpc.get_account(amm_config_address).await?;
    if account.owner != accounts::RAYDIUM_CLMM {
        return Err(anyhow!("Account is not owned by Raydium CLMM program"));
    }
    amm_config_decode(&account.data)
        .ok_or_else(|| anyhow!("Failed to decode amm config"))
}

/// 获取多个 tick arrays
pub async fn get_tick_arrays(
    rpc: &SolanaRpcClient,
    pool_id: &Pubkey,
    start_indices: &[i32],
) -> Result<Vec<(i32, TickArrayState)>, anyhow::Error> {
    let mut addresses = Vec::new();
    for &start_index in start_indices {
        let (tick_array_pda, _) = get_tick_array_pda(pool_id, start_index)?;
        addresses.push((start_index, tick_array_pda));
    }

    let mut result = Vec::new();
    for (start_index, address) in addresses {
        match rpc.get_account(&address).await {
            Ok(account) => {
                if account.owner != accounts::RAYDIUM_CLMM {
                    continue;
                }
                if let Some(tick_array) = tick_array_state_decode(&account.data) {
                    result.push((start_index, tick_array));
                }
            }
            Err(_) => {
                // Tick array 可能不存在，跳过
                continue;
            }
        }
    }

    Ok(result)
}

/// 获取指定 mint 对应的最优 CLMM 池（带选项）
///
/// # Arguments
/// * `use_vault_balance` - 是否使用金库余额选池策略（需要RPC调用，但更准确）
///   - `true`: 并发读取候选池的USDC/USDT/WSOL金库余额，按余额从大到小选择（推荐用于生产环境）
///   - `false`: 使用PoolState中的现有字段（liquidity等）选池，零网络开销（推荐用于测试/快速查询）
pub async fn get_pool_by_mint_with_options(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
    use_vault_balance: bool,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool_address) = raydium_clmm_cache::get_cached_pool_address_by_mint(mint) {
        if let Some(pool) = raydium_clmm_cache::get_cached_pool_by_address(&pool_address) {
            return Ok((pool_address, pool));
        }
    }

    // 2. RPC 查询 - 复用 get_pool_by_mint 的逻辑
    // 注意：当 use_vault_balance=true 时，仍使用旧的 find_pool_by_mint_impl
    // 当 use_vault_balance=false 时，使用共享的 get_pool_by_mint 逻辑
    let (pool_address, pool) = if use_vault_balance {
        // 使用金库余额策略（需要额外 RPC 调用）
        find_pool_by_mint_impl(rpc, mint, true).await?
    } else {
        // 使用共享逻辑（零额外网络开销）
        get_pool_by_mint(rpc, mint).await?
    };

    // 3. 写入缓存
    raydium_clmm_cache::cache_pool_address_by_mint(mint, &pool_address);
    raydium_clmm_cache::cache_pool_by_address(&pool_address, &pool);
    Ok((pool_address, pool))
}

/// Force 刷新：强制重新查询指定 Pool（泛型版本，支持 Auto Mock）
pub async fn get_pool_by_address_force<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    raydium_clmm_cache::POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

/// 强制刷新缓存并获取指定 mint 对应的最优 CLMM 池（带选项）
pub async fn get_pool_by_mint_force_with_options(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
    use_vault_balance: bool,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    raydium_clmm_cache::MINT_TO_POOL_CACHE.remove(mint);
    get_pool_by_mint_with_options(rpc, mint, use_vault_balance).await
}

/// 使用 PoolRpcClient 获取指定 mint 对应的最优 CLMM 池（支持 Auto Mock）
///
/// 这是一个简化版本，不支持缓存和 use_vault_balance 选项，
/// 主要用于测试环境加速。
///
/// # Arguments
/// * `rpc`: 实现了 PoolRpcClient 的 RPC 客户端（支持 AutoMockRpcClient）
/// * `mint`: Token mint 地址
///
/// # Returns
/// 返回最优池的地址和状态
pub async fn get_pool_by_mint<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 使用 list_pools_by_mint 获取所有包含该 mint 的池
    let all_pools = list_pools_by_mint(rpc, mint).await?;

    if all_pools.is_empty() {
        return Err(anyhow::anyhow!("No CLMM pool found for mint: {}", mint));
    }

    // 简单选择策略：优先选择与 Hot Mint（WSOL/USDC/USDT）配对的池
    let mut hot_pools: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, PoolState)> = Vec::new();

    for (addr, pool) in all_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if pool.token_mint0 == *mint {
            pool.token_mint1
        } else if pool.token_mint1 == *mint {
            pool.token_mint0
        } else {
            other_pools.push((addr, pool));
            continue;
        };

        if is_hot_mint(&other_mint) {
            hot_pools.push((addr, pool));
        } else {
            other_pools.push((addr, pool));
        }
    }

    // 使用累计交易量选池（零网络开销）
    let best_pool = if !hot_pools.is_empty() {
        select_best_pool_by_volume(&hot_pools)
    } else if *mint == SOL_MINT {
        select_best_pool_by_volume(&other_pools)
    } else {
        select_best_pool_by_volume(&other_pools)
    };

    best_pool.ok_or_else(|| anyhow::anyhow!("未找到 {} 的可用 Raydium CLMM 池", mint))
}

pub fn clear_pool_cache() {
    raydium_clmm_cache::clear_all();
}

/// 使用 PoolRpcClient 通过 offset 查找所有 Pool
async fn find_pools_by_mint_offset_collect<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
    offset: usize,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    use solana_account_decoder::UiAccountEncoding;
    use solana_client::rpc_filter::Memcmp;
    use solana_rpc_client_api::{config::RpcProgramAccountsConfig, filter::RpcFilterType};

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

    let accounts = rpc.get_program_ui_accounts_with_config(&accounts::RAYDIUM_CLMM, config).await
        .map_err(|e| anyhow!("RPC 调用失败: {}", e))?;

    // 检查是否需要限制返回数量（测试环境优化）
    // 生产环境通过环境变量 CLMM_POOL_SCAN_LIMIT 控制，默认不限制
    let pools: Vec<(Pubkey, PoolState)> = if let Ok(limit_str) = std::env::var("CLMM_POOL_SCAN_LIMIT") {
        let limit = match limit_str.parse::<usize>() {
            Ok(n) => n,
            Err(_) => {
                eprintln!("警告: CLMM_POOL_SCAN_LIMIT 环境变量值无效 '{}'，将不限制返回数量", limit_str);
                usize::MAX
            }
        };
        // 测试环境：限制返回数量，避免超时
        accounts
            .into_iter()
            .filter_map(|(addr, acc)| {
                let addr_pubkey = addr.parse::<Pubkey>().ok()?;
                let data_bytes = match &acc.data {
                    UiAccountData::Binary(base64_str, _) => STANDARD.decode(base64_str).ok()?,
                    _ => return None,
                };
                if data_bytes.len() > 8 {
                    pool_state_decode(&data_bytes[8..]).map(|pool| (addr_pubkey, pool))
                } else {
                    None
                }
            })
            .take(limit) // 限制返回数量
            .collect()
    } else {
        // 生产环境：读取所有 Pool
        accounts
            .into_iter()
            .filter_map(|(addr, acc)| {
                let addr_pubkey = addr.parse::<Pubkey>().ok()?;
                let data_bytes = match &acc.data {
                    UiAccountData::Binary(base64_str, _) => STANDARD.decode(base64_str).ok()?,
                    _ => return None,
                };
                if data_bytes.len() > 8 {
                    pool_state_decode(&data_bytes[8..]).map(|pool| (addr_pubkey, pool))
                } else {
                    None
                }
            })
            .collect()
    };

    Ok(pools)
}

/// 通用选池逻辑（降级使用）
/// 基于 status、liquidity、open_time、tick_spacing 选择
fn select_best_pool(pools: &[(Pubkey, PoolState)]) -> Option<(Pubkey, PoolState)> {
    if pools.is_empty() {
        return None;
    }

    if pools.len() == 1 {
        return pools.first().cloned();
    }

    // 1. 优先选择「已激活且有流动性」的池
    let tradeable_pools: Vec<_> =
        pools.iter().filter(|(_, pool)| pool.status != 0 && pool.liquidity > 0).collect();

    let fallback_liquid_pools: Vec<_> =
        pools.iter().filter(|(_, pool)| pool.liquidity > 0).collect();

    let candidates = if !tradeable_pools.is_empty() {
        &tradeable_pools.iter().map(|&p| p.clone()).collect::<Vec<_>>()[..]
    } else if !fallback_liquid_pools.is_empty() {
        // 所有池的 status 都为 0 时，至少保证有流动性
        &fallback_liquid_pools.iter().map(|&p| p.clone()).collect::<Vec<_>>()[..]
    } else {
        // 极端情况：全部池流动性为 0，退化为任意池
        pools
    };

    // 2. 在候选集中按「流动性 -> open_time -> tick_spacing」排序
    let best = candidates
        .iter()
        .max_by(|(_, pool_a), (_, pool_b)| {
            use std::cmp::Ordering;

            match pool_a.liquidity.cmp(&pool_b.liquidity) {
                Ordering::Equal => match pool_a.open_time.cmp(&pool_b.open_time) {
                    Ordering::Equal => {
                        // Tick spacing 越小，价格粒度越细，优先级越高
                        pool_b.tick_spacing.cmp(&pool_a.tick_spacing)
                    }
                    other => other,
                },
                other => other,
            }
        });

    best.cloned()
}

/// 按累计交易量选择最佳池（零网络开销）
///
/// 策略：
/// - 如果池子包含 WSOL/USDC/USDT，只计算这些稳定资产侧的累计交易量
/// - 否则计算两侧的总交易量
/// 交易量越大，说明池子被实际使用越多，深度越可靠
fn select_best_pool_by_volume(pools: &[(Pubkey, PoolState)]) -> Option<(Pubkey, PoolState)> {
    if pools.is_empty() {
        return None;
    }

    if pools.len() == 1 {
        return pools.first().cloned();
    }

    // 过滤掉流动性为0的池
    let mut valid_pools: Vec<_> = pools
        .iter()
        .filter(|(_, pool)| pool.liquidity > 0)
        .map(|(addr, pool)| (*addr, pool.clone()))
        .collect();

    if valid_pools.is_empty() {
        // 如果全部池流动性为0，降级为通用选池逻辑
        return select_best_pool(pools);
    }

    // 按累计交易量排序
    valid_pools.sort_by(|(_, pool_a), (_, pool_b)| {
        // 计算有效交易量（优先只看WSOL/USDC/USDT侧）
        let volume_a = calculate_effective_volume(pool_a);
        let volume_b = calculate_effective_volume(pool_b);

        // 按交易量降序排序
        match volume_b.cmp(&volume_a) {
            std::cmp::Ordering::Equal => {
                // 交易量相同时，按流动性排序
                match pool_b.liquidity.cmp(&pool_a.liquidity) {
                    std::cmp::Ordering::Equal => {
                        // 流动性也相同时，按开池时间排序（更早的池更成熟）
                        pool_b.open_time.cmp(&pool_a.open_time)
                    }
                    other => other,
                }
            }
            other => other,
        }
    });

    // 返回交易量最高的池
    valid_pools.into_iter().next()
}

/// 计算池子的有效交易量
/// - 如果包含 WSOL/USDC/USDT，只计算这些资产侧的交易量
/// - 否则计算两侧的总交易量
fn calculate_effective_volume(pool: &PoolState) -> u128 {
    // 检查 token0 是否为 WSOL/USDC/USDT
    let token0_is_stable = pool.token_mint0 == SOL_MINT 
        || pool.token_mint0 == USDC_MINT 
        || pool.token_mint0 == USDT_MINT;
    
    // 检查 token1 是否为 WSOL/USDC/USDT
    let token1_is_stable = pool.token_mint1 == SOL_MINT 
        || pool.token_mint1 == USDC_MINT 
        || pool.token_mint1 == USDT_MINT;
    
    if token0_is_stable && !token1_is_stable {
        // 只计算 token0 侧（WSOL/USDC/USDT）的交易量
        pool.swap_in_amount_token0.saturating_add(pool.swap_out_amount_token0)
    } else if token1_is_stable && !token0_is_stable {
        // 只计算 token1 侧（WSOL/USDC/USDT）的交易量
        pool.swap_in_amount_token1.saturating_add(pool.swap_out_amount_token1)
    } else {
        // 两侧都是稳定资产或都不是，计算总交易量
        pool.swap_in_amount_token0
            .saturating_add(pool.swap_out_amount_token0)
            .saturating_add(pool.swap_in_amount_token1)
            .saturating_add(pool.swap_out_amount_token1)
    }
}

/// 内部使用的候选结构：包含池地址、池状态、优先金库地址
struct PoolCandidate {
    addr: Pubkey,
    pool: PoolState,
    priority_vault: Pubkey,
}

/// 在一组候选池中，按金库余额并发读取并选择最佳池
/// 
/// 策略：并发读取所有候选池的金库余额（控制并发数为100），按余额从大到小选择
async fn pick_best_by_vault_balance_concurrent(
    rpc: &SolanaRpcClient,
    candidates: Vec<PoolCandidate>,
) -> Option<(Pubkey, PoolState)> {
    if candidates.is_empty() {
        return None;
    }

    use futures::stream::{self, StreamExt};

    // 并发读取所有金库余额，控制并发数为100
    let results: Vec<_> = stream::iter(candidates)
        .map(|cand| async move {
            let balance_res = rpc.get_token_account_balance(&cand.priority_vault).await;
            let amount: u64 = match balance_res {
                Ok(bal) => bal.amount.parse::<u64>().unwrap_or(0),
                Err(_) => 0,
            };
            (cand.addr, cand.pool, amount)
        })
        .buffer_unordered(1000) // 控制并发数为100
        .collect()
        .await;

    // 过滤掉余额为0的池，并按余额从大到小排序
    let mut valid_pools: Vec<_> = results
        .into_iter()
        .filter(|(_, _, amount)| *amount > 0)
        .collect();

    if valid_pools.is_empty() {
        return None;
    }

    // 按余额降序排序
    valid_pools.sort_by(|(_, _, amount_a), (_, _, amount_b)| amount_b.cmp(amount_a));

    // 返回余额最高的池
    valid_pools.into_iter().next().map(|(addr, pool, _)| (addr, pool))
}

/// 对 Hot Mint 对（WSOL/USDC/USDT 相关）进一步按金库余额择优（并发读取）
///
/// 策略：
/// - 如果存在稳定币对（USDC/USDT），优先在这些池中按稳定币金库余额从大到小选择
/// - 否则如果存在 WSOL 对，在这些池中按 WSOL 金库余额从大到小选择
/// - 如果都无法区分，则退化为 select_best_pool 的通用逻辑
/// - 并发读取余额，控制并发数为100
async fn select_best_hot_pool_by_vault_balance(
    rpc: &SolanaRpcClient,
    pools: &[(Pubkey, PoolState)],
) -> Option<(Pubkey, PoolState)> {
    if pools.is_empty() {
        return None;
    }

    if pools.len() == 1 {
        return pools.first().cloned();
    }

    let mut stable_candidates: Vec<PoolCandidate> = Vec::new();
    let mut wsol_candidates: Vec<PoolCandidate> = Vec::new();

    for (addr, pool) in pools.iter() {
        // 先找稳定币侧
        if pool.token_mint0 == USDC_MINT || pool.token_mint0 == USDT_MINT {
            stable_candidates.push(PoolCandidate {
                addr: *addr,
                pool: pool.clone(),
                priority_vault: pool.token_vault0,
            });
            continue;
        }
        if pool.token_mint1 == USDC_MINT || pool.token_mint1 == USDT_MINT {
            stable_candidates.push(PoolCandidate {
                addr: *addr,
                pool: pool.clone(),
                priority_vault: pool.token_vault1,
            });
            continue;
        }

        // 其次考虑 WSOL 侧
        if pool.token_mint0 == SOL_MINT {
            wsol_candidates.push(PoolCandidate {
                addr: *addr,
                pool: pool.clone(),
                priority_vault: pool.token_vault0,
            });
            continue;
        }
        if pool.token_mint1 == SOL_MINT {
            wsol_candidates.push(PoolCandidate {
                addr: *addr,
                pool: pool.clone(),
                priority_vault: pool.token_vault1,
            });
            continue;
        }
    }

    // 1. 优先在稳定币相关池中按金库余额择优（并发读取）
    if !stable_candidates.is_empty() {
        if let Some(best) = pick_best_by_vault_balance_concurrent(rpc, stable_candidates).await {
            return Some(best);
        }
    }

    // 2. 否则在 WSOL 相关池中按 WSOL 金库余额择优（并发读取）
    if !wsol_candidates.is_empty() {
        if let Some(best) = pick_best_by_vault_balance_concurrent(rpc, wsol_candidates).await {
            return Some(best);
        }
    }

    // 3. 都无法区分时退化为原有通用规则
    select_best_pool(pools)
}

/// 在所有包含 WSOL 的池中，按 WSOL 金库余额择优（并发读取）
async fn select_best_wsol_pool_by_vault_balance(
    rpc: &SolanaRpcClient,
    pools: &[(Pubkey, PoolState)],
) -> Option<(Pubkey, PoolState)> {
    let wsol_candidates: Vec<PoolCandidate> = pools
        .iter()
        .filter_map(|(addr, pool)| {
            if pool.token_mint0 == SOL_MINT {
                Some(PoolCandidate {
                    addr: *addr,
                    pool: pool.clone(),
                    priority_vault: pool.token_vault0,
                })
            } else if pool.token_mint1 == SOL_MINT {
                Some(PoolCandidate {
                    addr: *addr,
                    pool: pool.clone(),
                    priority_vault: pool.token_vault1,
                })
            } else {
                None
            }
        })
        .collect();

    if wsol_candidates.is_empty() {
        return None;
    }

    pick_best_by_vault_balance_concurrent(rpc, wsol_candidates).await
}

/// 内部实现：查找 mint 对应的最优池
async fn find_pool_by_mint_impl(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
    use_vault_balance: bool,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 复用 list_pools_by_mint 获取所有包含该 mint 的池（带缓存）
    let all_pools = list_pools_by_mint(rpc, mint).await?;

    // 优先选择与 Hot Mint（如 WSOL/USDC/USDT）配对的池，参考 Pool 选择算法分析文档的 Hot/Cold 策略
    let mut hot_pools: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, PoolState)> = Vec::new();

    for (addr, pool) in all_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if pool.token_mint0 == *mint {
            pool.token_mint1
        } else if pool.token_mint1 == *mint {
            pool.token_mint0
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
        if use_vault_balance {
            // 对 Hot 对额外按金库余额（USDC/USDT/WSOL）择优（并发读取，控制并发数1000）
            select_best_hot_pool_by_vault_balance(rpc, &hot_pools).await
        } else {
            // 使用累计交易量选池（零网络开销，反映真实使用深度）
            select_best_pool_by_volume(&hot_pools)
        }
    } else if *mint == SOL_MINT {
        // 特殊情况：当 mint 本身是 WSOL 时
        if use_vault_balance {
            // 在所有包含 WSOL 的池中按 WSOL 金库余额择优
            if let Some(best) = select_best_wsol_pool_by_vault_balance(rpc, &other_pools).await {
                Some(best)
            } else {
                select_best_pool_by_volume(&other_pools)
            }
        } else {
            // 使用累计交易量选池（零网络开销）
            select_best_pool_by_volume(&other_pools)
        }
    } else {
        // 没有 Hot 对时，使用累计交易量选池
        select_best_pool_by_volume(&other_pools)
    };

    best_pool.ok_or_else(|| anyhow::anyhow!("未找到 {} 的可用 Raydium CLMM 池", mint))
}

/// 使用 PoolRpcClient 列出所有包含指定 mint 的 Raydium CLMM Pool（支持 Auto Mock）
///
/// 此函数与 `list_pools_by_mint` 功能相同，但接受 `PoolRpcClient` trait，
/// 因此可以使用 `AutoMockRpcClient` 来加速测试。
///
/// # 参数
/// - `rpc`: 实现了 PoolRpcClient 的 RPC 客户端（支持 AutoMockRpcClient）
/// - `mint`: 要查询的代币 mint 地址
///
/// # 返回
/// - 返回排序后的包含指定 mint 的 pool 列表
pub async fn list_pools_by_mint<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    use std::collections::HashSet;

    // 注意：这里不使用内存缓存，直接查询
    // Auto Mock 会在文件层面缓存

    // Parallel search: scan both token_mint0 and token_mint1 simultaneously
    let (result0, result1) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN_MINT0_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN_MINT1_OFFSET)
    );

    // 检测是否都失败，如果都失败则返回第一个错误（通常包含 RPC 限制信息）
    if result0.is_err() && result1.is_err() {
        // 返回 result0 的错误，它包含我们的自定义错误消息
        return Err(result0.unwrap_err());
    }

    let mut all_pools: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut seen: HashSet<Pubkey> = HashSet::new();

    // Merge token_mint0 results
    if let Ok(token0_pools) = result0 {
        for (addr, pool) in token0_pools {
            if seen.insert(addr) {
                all_pools.push((addr, pool));
            }
        }
    }

    // Merge token_mint1 results
    if let Ok(token1_pools) = result1 {
        for (addr, pool) in token1_pools {
            if seen.insert(addr) {
                all_pools.push((addr, pool));
            }
        }
    }

    if all_pools.is_empty() {
        return Err(anyhow!("No CLMM pool found for mint {}", mint));
    }

    // 分类：稳定币对 > WSOL 对 > 其他对
    let mut stable_pools: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut wsol_pools: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, PoolState)> = Vec::new();

    for (addr, pool) in all_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if pool.token_mint0 == *mint {
            pool.token_mint1
        } else if pool.token_mint1 == *mint {
            pool.token_mint0
        } else {
            other_pools.push((addr, pool));
            continue;
        };

        // 按 Hot Token 优先级分类
        if other_mint == USDC_MINT || other_mint == USDT_MINT {
            stable_pools.push((addr, pool));
        } else if other_mint == SOL_MINT {
            wsol_pools.push((addr, pool));
        } else if is_hot_mint(&other_mint) {
            // Hot mint 但不在上述分类中（理论上不会发生，但为了完整性）
            wsol_pools.push((addr, pool));
        } else {
            other_pools.push((addr, pool));
        }
    }

    // 在各分类内按累计交易量排序
    stable_pools.sort_by(|(_, a), (_, b)| {
        let volume_a = calculate_effective_volume(a);
        let volume_b = calculate_effective_volume(b);
        volume_b.cmp(&volume_a)
    });
    wsol_pools.sort_by(|(_, a), (_, b)| {
        let volume_a = calculate_effective_volume(a);
        let volume_b = calculate_effective_volume(b);
        volume_b.cmp(&volume_a)
    });
    other_pools.sort_by(|(_, a), (_, b)| {
        let volume_a = calculate_effective_volume(a);
        let volume_b = calculate_effective_volume(b);
        volume_b.cmp(&volume_a)
    });

    // 合并：稳定币对 > WSOL 对 > 其他对
    let mut sorted_pools = Vec::new();
    sorted_pools.extend(stable_pools);
    sorted_pools.extend(wsol_pools);
    sorted_pools.extend(other_pools);

    Ok(sorted_pools)
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

/// 获取 WSOL 的 USD 价格（泛型版本，支持 Auto Mock）
///
/// # Arguments
/// * `rpc` - RPC 客户端（支持 AutoMockRpcClient）
/// * `wsol_usd_pool_address` - WSOL-USDT/USDC CLMM 池地址（例如你提供的 USDT-WSOL 池）
pub async fn get_wsol_price_in_usd_with_client<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    wsol_usd_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    use crate::utils::price::raydium_clmm::{price_token0_in_token1, price_token1_in_token0};

    let wsol_usd_pool = wsol_usd_pool_address.unwrap_or(&DEFAULT_WSOL_USDT_CLMM_POOL);

    // 强制刷新：每次调用都重新从链上读取池状态，避免价格缓存
    let pool_state = get_pool_by_address_force(rpc, wsol_usd_pool).await?;

    // 只支持 WSOL <-> USDC/USDT 的稳定币池
    let is_token0_sol = pool_state.token_mint0 == SOL_MINT;
    let is_token1_sol = pool_state.token_mint1 == SOL_MINT;
    let is_token0_stable = pool_state.token_mint0 == USDC_MINT || pool_state.token_mint0 == USDT_MINT;
    let is_token1_stable = pool_state.token_mint1 == USDC_MINT || pool_state.token_mint1 == USDT_MINT;

    let price_wsol_in_stable = if is_token0_sol && is_token1_stable {
        // token0 = WSOL, token1 = USDC/USDT
        price_token0_in_token1(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    } else if is_token1_sol && is_token0_stable {
        // token1 = WSOL, token0 = USDC/USDT
        price_token1_in_token0(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    } else {
        return Err(anyhow!(
            "WSOL-USD anchor pool must be a SOL<->USDC/USDT CLMM pool, got {:?} / {:?}",
            pool_state.token_mint0, pool_state.token_mint1
        ));
    };

    if price_wsol_in_stable <= 0.0 {
        return Err(anyhow!("Invalid WSOL price from anchor pool (<= 0)"));
    }

    // 默认认为 USDC / USDT ~= 1 USD
    Ok(price_wsol_in_stable)
}

/// 获取任意 Token 在 Raydium CLMM 上的 USD 价格（支持 PoolRpcClient）
///
/// 与 `get_token_price_in_usd` 功能相同，但接受 `PoolRpcClient` trait 参数，
/// 支持 `AutoMockRpcClient` 进行测试加速。
///
/// 价格计算路径：Token X -> WSOL -> USD
/// - 要求：存在一个 X-WSOL 的 CLMM 池（Hot 对），以及一个 WSOL-USDT/USDC 锚定池
pub async fn get_token_price_in_usd<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    token_mint: &Pubkey,
    wsol_usd_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool = wsol_usd_pool_address.unwrap_or(&DEFAULT_WSOL_USDT_CLMM_POOL);
    use crate::utils::price::raydium_clmm::{price_token0_in_token1, price_token1_in_token0};

    // 稳定币自身的价格直接认为是 1 USD
    if *token_mint == USDC_MINT || *token_mint == USDT_MINT {
        return Ok(1.0);
    }

    // WSOL/SOL 的价格直接来自锚定池
    if *token_mint == SOL_MINT {
        return get_wsol_price_in_usd_with_client(rpc, Some(wsol_usd_pool)).await;
    }

    // 1. 先在 CLMM 中找到 Token X 的最优池（优先 X-WSOL/USDC/USDT 对）
    let (pool_address, pool_state_best) = get_pool_by_mint(rpc, token_mint).await?;

    // 2. 为了价格实时性，对选中的池地址强制刷新一次 PoolState
    let pool_state = get_pool_by_address_force(rpc, &pool_address).await.unwrap_or(pool_state_best);

    // 3. 判断池子配对类型
    let is_token0_x = pool_state.token_mint0 == *token_mint;
    let is_token1_x = pool_state.token_mint1 == *token_mint;

    let other_mint = if is_token0_x {
        pool_state.token_mint1
    } else if is_token1_x {
        pool_state.token_mint0
    } else {
        return Err(anyhow!(
            "Pool {} does not contain the target mint {}",
            pool_address,
            token_mint
        ));
    };

    // 支持三种池子类型：
    // 1. X-WSOL：需要通过 WSOL-USD 锚定池计算
    // 2. X-USDC/USDT：直接认为稳定币价格 = 1 USD
    // 3. 其他：暂不支持（需要多跳路由）
    if other_mint == USDC_MINT || other_mint == USDT_MINT {
        // X-稳定币池：直接计算 X 相对稳定币的价格
        let price_x_in_stable = if is_token0_x {
            price_token0_in_token1(
                pool_state.sqrt_price_x64,
                pool_state.mint_decimals0,
                pool_state.mint_decimals1,
            )
        } else {
            price_token1_in_token0(
                pool_state.sqrt_price_x64,
                pool_state.mint_decimals0,
                pool_state.mint_decimals1,
            )
        };

        if price_x_in_stable <= 0.0 {
            return Err(anyhow!(
                "Invalid price from X-Stable pool (<= 0): mint={}, pool={}",
                token_mint,
                pool_address
            ));
        }

        return Ok(price_x_in_stable); // 稳定币 = 1 USD
    }

    if other_mint != SOL_MINT {
        return Err(anyhow!(
            "Best CLMM pool for mint {} is paired with {} (not WSOL/USDC/USDT); multi-hop USD pricing is not supported yet",
            token_mint,
            other_mint
        ));
    }

    // 4. X-WSOL 池：计算 X 相对 WSOL 的价格
    let price_x_in_wsol = if is_token0_x {
        // token0 = X, token1 = WSOL
        price_token0_in_token1(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    } else {
        // token1 = X, token0 = WSOL
        price_token1_in_token0(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price is invalid (<= 0)"));
    }

    // 5. 计算 WSOL 的 USD 价格
    let price_wsol_in_usd = get_wsol_price_in_usd_with_client(rpc, Some(wsol_usd_pool)).await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}

/// 获取任意 Token 在 Raydium CLMM 上的 USD 价格（直接传入池地址，支持 PoolRpcClient）
///
/// 与 `get_token_price_in_usd_with_pool` 功能相同，但接受 `PoolRpcClient` trait 参数，
/// 支持 `AutoMockRpcClient` 进行测试加速。
///
/// 与 `get_token_price_in_usd` 的区别：
/// - 此函数要求调用者已知 X-WSOL 池地址，直接传入，避免 `get_pool_by_mint` 的查找开销
/// - 适用于高频调用、已缓存池地址的场景
///
/// # Arguments
/// * `rpc` - 实现 PoolRpcClient trait 的 RPC 客户端（支持 AutoMockRpcClient）
/// * `token_mint` - Token X 的 mint 地址
/// * `x_wsol_pool_address` - Token X 与 WSOL 配对的 CLMM 池地址
/// * `wsol_usd_pool_address` - WSOL-USDT/USDC 锚定池地址（可选，默认使用 DEFAULT_WSOL_USDT_CLMM_POOL）
pub async fn get_token_price_in_usd_with_pool<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    token_mint: &Pubkey,
    x_wsol_pool_address: &Pubkey,
    wsol_usd_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool = wsol_usd_pool_address.unwrap_or(&DEFAULT_WSOL_USDT_CLMM_POOL);
    use crate::utils::price::raydium_clmm::{price_token0_in_token1, price_token1_in_token0};

    // 稳定币自身的价格直接认为是 1 USD
    if *token_mint == USDC_MINT || *token_mint == USDT_MINT {
        return Ok(1.0);
    }

    // WSOL/SOL 的价格直接来自锚定池
    if *token_mint == SOL_MINT {
        return get_wsol_price_in_usd_with_client(rpc, Some(wsol_usd_pool)).await;
    }

    // 1. 直接强制刷新指定的 X-WSOL 池（跳过查找步骤）
    let pool_state = get_pool_by_address_force(rpc, x_wsol_pool_address).await?;

    // 2. 判断池子配对类型
    let is_token0_x = pool_state.token_mint0 == *token_mint;
    let is_token1_x = pool_state.token_mint1 == *token_mint;

    let other_mint = if is_token0_x {
        pool_state.token_mint1
    } else if is_token1_x {
        pool_state.token_mint0
    } else {
        return Err(anyhow!(
            "Provided pool {} does not contain the target mint {}",
            x_wsol_pool_address,
            token_mint
        ));
    };

    // 支持三种池子类型：
    // 1. X-WSOL：需要通过 WSOL-USD 锚定池计算
    // 2. X-USDC/USDT：直接认为稳定币价格 = 1 USD
    // 3. 其他：不支持
    if other_mint == USDC_MINT || other_mint == USDT_MINT {
        // X-稳定币池：直接计算 X 相对稳定币的价格
        let price_x_in_stable = if is_token0_x {
            price_token0_in_token1(
                pool_state.sqrt_price_x64,
                pool_state.mint_decimals0,
                pool_state.mint_decimals1,
            )
        } else {
            price_token1_in_token0(
                pool_state.sqrt_price_x64,
                pool_state.mint_decimals0,
                pool_state.mint_decimals1,
            )
        };

        if price_x_in_stable <= 0.0 {
            return Err(anyhow!(
                "Invalid price from X-Stable pool (<= 0): mint={}, pool={}",
                token_mint,
                x_wsol_pool_address
            ));
        }

        return Ok(price_x_in_stable); // 稳定币 = 1 USD
    }

    if other_mint != SOL_MINT {
        return Err(anyhow!(
            "Provided pool {} is paired with {} (not WSOL/USDC/USDT); multi-hop USD pricing is not supported yet",
            x_wsol_pool_address,
            other_mint
        ));
    }

    // 3. X-WSOL 池：计算 X 相对 WSOL 的价格
    let price_x_in_wsol = if is_token0_x {
        // token0 = X, token1 = WSOL
        price_token0_in_token1(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    } else {
        // token1 = X, token0 = WSOL
        price_token1_in_token0(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price is invalid (<= 0)"));
    }

    // 4. 计算 WSOL 的 USD 价格
    let price_wsol_in_usd = get_wsol_price_in_usd_with_client(rpc, Some(wsol_usd_pool)).await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}
