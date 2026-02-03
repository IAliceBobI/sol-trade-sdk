// Raydium CLMM Pool 查询函数

use anyhow::anyhow;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use futures::stream::{self, StreamExt};
use solana_account_decoder::UiAccountData;
use solana_sdk::pubkey::Pubkey;

use crate::{
    common::{SolanaRpcClient, auto_mock_rpc::PoolRpcClient},
    constants::{SOL_MINT, USDC_MINT, USDT_MINT},
    instruction::utils::raydium_clmm_types::{
        AmmConfig, PoolState, TickArrayState, amm_config_decode, pool_state_decode,
        tick_array_state_decode,
    },
};

use super::{
    cache,
    constants::TOKEN_MINT0_OFFSET,
    constants::TOKEN_MINT1_OFFSET,
    helpers::{accounts, get_tick_array_pda, is_hot_mint},
};

/// 使用 PoolRpcClient trait 获取 Pool（支持 Auto Mock）
///
/// 这是一个泛型版本，可以接受任何实现了 PoolRpcClient 的客户端。
/// 支持标准的 RpcClient 和 AutoMockRpcClient。
pub async fn get_pool_by_address<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    // RPC 查询（不缓存，每次获取最新数据）
    let account = rpc
        .get_account(pool_address)
        .await
        .map_err(|e| anyhow!("RPC 调用失败: {}", e))?;
    if account.owner != accounts::RAYDIUM_CLMM {
        return Err(anyhow!("Account is not owned by Raydium CLMM program"));
    }
    // 使用修改后的 pool_state_decode（传入 program_id）
    let pool_state = pool_state_decode(&account.data[8..], account.owner)
        .ok_or_else(|| anyhow!("Failed to decode pool state"))?;
    // 不写入缓存
    Ok(pool_state)
}

/// Force 刷新：强制重新查询指定 Pool（泛型版本，支持 Auto Mock）
pub async fn get_pool_by_address_force<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    cache::POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
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
    amm_config_decode(&account.data).ok_or_else(|| anyhow!("Failed to decode amm config"))
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
            },
            Err(_) => {
                // Tick array 可能不存在，跳过
                continue;
            },
        }
    }

    Ok(result)
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

    let accounts = rpc
        .get_program_ui_accounts_with_config(&accounts::RAYDIUM_CLMM, config)
        .await
        .map_err(|e| anyhow!("RPC 调用失败: {}", e))?;

    // 检查是否需要限制返回数量（测试环境优化）
    // 生产环境通过环境变量 CLMM_POOL_SCAN_LIMIT 控制，默认不限制
    let pools: Vec<(Pubkey, PoolState)> =
        if let Ok(limit_str) = std::env::var("CLMM_POOL_SCAN_LIMIT") {
            let limit = match limit_str.parse::<usize>() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!(
                        "警告: CLMM_POOL_SCAN_LIMIT 环境变量值无效 '{}'，将不限制返回数量",
                        limit_str
                    );
                    usize::MAX
                },
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
                        // 使用 program_id (所有账户都属于 RAYDIUM_CLMM)
                        pool_state_decode(&data_bytes[8..], accounts::RAYDIUM_CLMM)
                            .map(|pool| (addr_pubkey, pool))
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
                        // 使用 program_id (所有账户都属于 RAYDIUM_CLMM)
                        pool_state_decode(&data_bytes[8..], accounts::RAYDIUM_CLMM)
                            .map(|pool| (addr_pubkey, pool))
                    } else {
                        None
                    }
                })
                .collect()
        };

    Ok(pools)
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

    // 并发读取所有金库余额，控制并发数为1000
    let results: Vec<_> = stream::iter(candidates)
        .map(|cand| async move {
            let balance_res = rpc.get_token_account_balance(&cand.priority_vault).await;
            let amount: u64 = match balance_res {
                Ok(bal) => bal.amount.parse::<u64>().unwrap_or(0),
                Err(_) => 0,
            };
            (cand.addr, cand.pool, amount)
        })
        .buffer_unordered(1000) // 控制并发数为1000
        .collect()
        .await;

    // 过滤掉余额为0的池，并按余额从大到小排序
    let mut valid_pools: Vec<_> =
        results.into_iter().filter(|(_, _, amount)| *amount > 0).collect();

    if valid_pools.is_empty() {
        return None;
    }

    // 按余额降序排序
    valid_pools.sort_by(|(_, _, amount_a), (_, _, amount_b)| amount_b.cmp(amount_a));

    // 返回余额最高的池
    valid_pools.into_iter().next().map(|(addr, pool, _)| (addr, pool))
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
    let tradeable_pools: Vec<_> = pools
        .iter()
        .filter(|(_, pool)| pool.status != 0 && pool.liquidity > 0)
        .collect();

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
    let best = candidates.iter().max_by(|(_, pool_a), (_, pool_b)| {
        use std::cmp::Ordering;

        match pool_a.liquidity.cmp(&pool_b.liquidity) {
            Ordering::Equal => match pool_a.open_time.cmp(&pool_b.open_time) {
                Ordering::Equal => {
                    // Tick spacing 越小，价格粒度越细，优先级越高
                    pool_b.tick_spacing.cmp(&pool_a.tick_spacing)
                },
                other => other,
            },
            other => other,
        }
    });

    best.cloned()
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

/// 按累计交易量选择最佳池（零网络开销）
///
/// 策略：
/// - 如果池子包含 WSOL/USDC/USDT，只计算这些稳定资产侧的累计交易量
/// - 否则计算两侧的总交易量
///   交易量越大，说明池子被实际使用越多，深度越可靠
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
                    },
                    other => other,
                }
            },
            other => other,
        }
    });

    // 返回交易量最高的池
    valid_pools.into_iter().next()
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

/// 对 Hot Mint 对（WSOL/USDC/USDT 相关）进一步按金库余额择优（并发读取）
///
/// 策略：
/// - 如果存在稳定币对（USDC/USDT），优先在这些池中按稳定币金库余额从大到小选择
/// - 否则如果存在 WSOL 对，在这些池中按 WSOL 金库余额从大到小选择
/// - 如果都无法区分，则退化为 select_best_pool 的通用逻辑
/// - 并发读取余额，控制并发数为1000
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
    if !stable_candidates.is_empty()
        && let Some(best) = pick_best_by_vault_balance_concurrent(rpc, stable_candidates).await
    {
        return Some(best);
    }

    // 2. 否则在 WSOL 相关池中按 WSOL 金库余额择优（并发读取）
    if !wsol_candidates.is_empty()
        && let Some(best) = pick_best_by_vault_balance_concurrent(rpc, wsol_candidates).await
    {
        return Some(best);
    }

    // 3. 都无法区分时退化为原有通用规则
    select_best_pool(pools)
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
    if let Some(pool_address) = cache::get_cached_pool_address_by_mint(mint)
        && let Some(pool) = cache::get_cached_pool_by_address(&pool_address)
    {
        return Ok((pool_address, pool));
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
    cache::cache_pool_address_by_mint(mint, &pool_address);
    cache::cache_pool_by_address(&pool_address, &pool);
    Ok((pool_address, pool))
}

/// 强制刷新缓存并获取指定 mint 对应的最优 CLMM 池（带选项）
pub async fn get_pool_by_mint_force_with_options(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
    use_vault_balance: bool,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    cache::MINT_TO_POOL_CACHE.remove(mint);
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
    match (&result0, &result1) {
        (Err(e), Err(_)) => return Err(anyhow::anyhow!("{}", e)),
        _ => {},
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

/// 清理 Pool 缓存
pub fn clear_pool_cache() {
    cache::clear_all();
}
