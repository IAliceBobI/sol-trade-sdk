use crate::{
    common::auto_mock_rpc::PoolRpcClient,
    constants::{USDC_MINT, USDT_MINT, WSOL_TOKEN_ACCOUNT},
    instruction::utils::pumpswap_types::Pool,
};
use anyhow::anyhow;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use solana_account_decoder::{UiAccountData, UiAccountEncoding};
use solana_sdk::pubkey::Pubkey;

use super::cache;
use super::constants::{accounts, BASE_MINT_OFFSET, QUOTE_MINT_OFFSET};
use super::helpers::{calculate_canonical_pool_pda, is_hot_mint, select_best_pool_by_liquidity};

/// Find a pool for a specific mint
/// 查找指定 mint 的 Pool（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
pub async fn find_pool<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<Pubkey, anyhow::Error> {
    let (pool_address, _) = get_pool_by_mint(rpc, mint).await?;
    Ok(pool_address)
}

/// 获取指定地址的 Pool（不缓存，每次从链上获取最新数据）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
pub async fn get_pool_by_address<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<Pool, anyhow::Error> {
    use crate::instruction::utils::pumpswap_types::pool_decode;

    // RPC 查询
    let account = rpc
        .get_account(pool_address)
        .await
        .map_err(|e| anyhow!("RPC 调用失败: {}", e))?;
    if account.owner != accounts::AMM_PROGRAM {
        return Err(anyhow!("Account is not owned by PumpSwap program"));
    }
    // 使用修改后的 pool_decode（传入 program_id）
    let pool = pool_decode(&account.data[8..], account.owner)
        .ok_or_else(|| anyhow!("Failed to decode pool"))?;
    // 不写入缓存
    Ok(pool)
}

/// 带缓存的 mint 查询（返回最优池）
/// 查询指定 mint 的 Pool（带缓存，支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
pub async fn get_pool_by_mint<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<(Pubkey, Pool), anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool_address) = cache::get_cached_pool_address_by_mint(mint)
        && let Some(pool) = cache::get_cached_pool_by_address(&pool_address)
    {
        return Ok((pool_address, pool));
    }
    // 2. RPC 查询
    let (pool_address, pool) = find_pool_by_mint_impl(rpc, mint).await?;
    // 3. 写入缓存
    cache::cache_pool_address_by_mint(mint, &pool_address);
    cache::cache_pool_by_address(&pool_address, &pool);
    Ok((pool_address, pool))
}

/// 强制刷新并重新查询指定 Pool（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
pub async fn get_pool_by_address_force<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<Pool, anyhow::Error> {
    cache::POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

/// 强制刷新并重新查询 mint 对应的 Pool（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
pub async fn get_pool_by_mint_force<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<(Pubkey, Pool), anyhow::Error> {
    cache::MINT_TO_POOL_CACHE.remove(mint);
    get_pool_by_mint(rpc, mint).await
}

/// 清除所有 Pool 缓存
pub fn clear_pool_cache() {
    cache::clear_all();
}

/// 通用内部实现：通过 offset 查找所有 Pool（返回 Vec）
#[allow(dead_code)]
/// 通过 offset 查找所有 Pool（支持 Auto Mock）
async fn find_pools_by_mint_offset_collect<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
    offset: usize,
) -> Result<Vec<(Pubkey, Pool)>, anyhow::Error> {
    use crate::instruction::utils::pumpswap_types::pool_decode;

    let filters = vec![solana_rpc_client_api::filter::RpcFilterType::Memcmp(
        solana_client::rpc_filter::Memcmp::new_base58_encoded(offset, &mint.to_bytes()),
    )];
    let config = solana_rpc_client_api::config::RpcProgramAccountsConfig {
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
    let program_id = accounts::AMM_PROGRAM;
    let accounts = rpc
        .get_program_ui_accounts_with_config(&program_id, config)
        .await
        .map_err(|e| anyhow!("RPC 调用失败: {}", e))?;

    let pools: Vec<(Pubkey, Pool)> = accounts
        .into_iter()
        .filter_map(|(addr, acc)| {
            let addr_pubkey = addr.parse::<Pubkey>().ok()?;
            let data_bytes = match &acc.data {
                UiAccountData::Binary(base64_str, _) => STANDARD.decode(base64_str).ok()?,
                _ => return None,
            };
            if data_bytes.len() > 8 {
                // 使用 program_id (所有账户都属于 AMM_PROGRAM)
                pool_decode(&data_bytes[8..], accounts::AMM_PROGRAM).map(|pool| (addr_pubkey, pool))
            } else {
                None
            }
        })
        .collect();

    Ok(pools)
}

/// 内部实现：查找指定 mint 的所有 PumpSwap Pool（支持 Auto Mock）
///
/// 策略：
/// 1. 并行查询 base_mint 与 quote_mint 包含该 mint 的所有池
/// 2. 合并并去重
async fn find_all_pools_by_mint_impl<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, Pool)>, anyhow::Error> {
    use std::collections::HashSet;

    let (base_result, quote_result) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, BASE_MINT_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, QUOTE_MINT_OFFSET)
    );

    // 检测是否都失败，如果都失败则返回第一个错误（通常包含 RPC 限制信息）
    match (&base_result, &quote_result) {
        (Err(e), Err(_)) => return Err(anyhow::anyhow!("{}", e)),
        _ => {},
    }

    let mut all_pools: Vec<(Pubkey, Pool)> = Vec::new();

    if let Ok(pools) = base_result {
        all_pools.extend(pools);
    }

    if let Ok(quote_pools) = quote_result {
        let mut seen: HashSet<Pubkey> = all_pools.iter().map(|(addr, _)| *addr).collect();
        for (addr, pool) in quote_pools {
            if seen.insert(addr) {
                all_pools.push((addr, pool));
            }
        }
    }

    if all_pools.is_empty() {
        return Err(anyhow!("No pool found for mint {}", mint));
    }

    Ok(all_pools)
}

/// 内部实现：查找 mint 对应的最优池（支持 Auto Mock）
///
/// 策略（参考 CLMM 的 Hot Token 优先策略）：
/// 1. 优先尝试 canonical pool (PumpFun 迁移的 mint/WSOL 对)
/// 2. 在所有池中优先选择稳定币对（USDC/USDT），再考虑 WSOL 对
/// 3. 在同类池子中，按 LP 供应量从大到小排序
#[allow(dead_code)]
async fn find_pool_by_mint_impl<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<(Pubkey, Pool), anyhow::Error> {
    // Priority 1: Try to find canonical pool (mint/WSOL pair) first
    // This is the most common case for PumpFun migrated tokens
    if let Some((pool_address, _)) = calculate_canonical_pool_pda(mint)
        && let Ok(pool) = get_pool_by_address(rpc, &pool_address).await
    {
        // Verify it's actually a mint/WSOL pool
        if (pool.base_mint == *mint && pool.quote_mint == WSOL_TOKEN_ACCOUNT)
            || (pool.base_mint == WSOL_TOKEN_ACCOUNT && pool.quote_mint == *mint)
        {
            return Ok((pool_address, pool));
        }
    }

    // Priority 2 & 3: 获取所有池子
    let all_pools = find_all_pools_by_mint_impl(rpc, mint).await?;

    // 分类：稳定币对 > WSOL 对 > 其他对
    let mut stable_pools: Vec<(Pubkey, Pool)> = Vec::new();
    let mut wsol_pools: Vec<(Pubkey, Pool)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, Pool)> = Vec::new();

    for (addr, pool) in all_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if pool.base_mint == *mint {
            pool.quote_mint
        } else if pool.quote_mint == *mint {
            pool.base_mint
        } else {
            // 理论上不会出现，但为了稳健性仍加入非 Hot 集合
            other_pools.push((addr, pool));
            continue;
        };

        // 按 Hot Token 优先级分类
        if other_mint == USDC_MINT || other_mint == USDT_MINT {
            // 最优：稳定币对
            stable_pools.push((addr, pool));
        } else if other_mint == WSOL_TOKEN_ACCOUNT {
            // 次优：WSOL 对
            wsol_pools.push((addr, pool));
        } else if is_hot_mint(&other_mint) {
            // Hot mint 但不在上述分类中（理论上不会发生，但为了完整性）
            wsol_pools.push((addr, pool));
        } else {
            other_pools.push((addr, pool));
        }
    }

    // 按优先级选择最佳池
    let best_pool = if !stable_pools.is_empty() {
        // 优先级 1: 稳定币对（USDC/USDT）
        select_best_pool_by_liquidity(&stable_pools)
    } else if !wsol_pools.is_empty() {
        // 优先级 2: WSOL 对
        select_best_pool_by_liquidity(&wsol_pools)
    } else if *mint == WSOL_TOKEN_ACCOUNT {
        // 特殊情况：当 mint 本身是 WSOL 时
        // 在所有池中按 LP 供应量选择
        select_best_pool_by_liquidity(&other_pools)
    } else {
        // 优先级 3: 其他对
        select_best_pool_by_liquidity(&other_pools)
    };

    best_pool.ok_or_else(|| anyhow::anyhow!("未找到 {} 的可用池", mint))
}

/// List all PumpSwap pools for a mint (as base or quote).
///
/// 返回按 Hot Token 优先策略排序后的池子列表：
/// 1. 稳定币对（USDC/USDT）优先
/// 2. WSOL 对次之
/// 3. 其他对最后
/// 4. 同类池子按 LP 供应量从大到小排序
///
/// Results are cached to improve performance on repeated queries.
/// 列出所有包含指定 mint 的 PumpSwap Pool（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
/// 结果已缓存以提高重复查询的性能。
///
/// # 参数
/// - `rpc`: 实现了 PoolRpcClient 的 RPC 客户端（支持 AutoMockRpcClient 或标准 RpcClient）
/// - `mint`: 要查询的代币 mint 地址
///
/// # 返回
/// - 返回排序后的包含指定 mint 的 pool 列表
pub async fn list_pools_by_mint<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, Pool)>, anyhow::Error> {
    // 1. 检查缓存
    if let Some(cached_pools) = cache::get_cached_pools_list_by_mint(mint) {
        return Ok(cached_pools);
    }

    // 2. 获取所有池子并分类排序
    let all_pools = find_all_pools_by_mint_impl(rpc, mint).await?;

    // 分类：稳定币对 > WSOL 对 > 其他对
    let mut stable_pools: Vec<(Pubkey, Pool)> = Vec::new();
    let mut wsol_pools: Vec<(Pubkey, Pool)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, Pool)> = Vec::new();

    for (addr, pool) in all_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if pool.base_mint == *mint {
            pool.quote_mint
        } else if pool.quote_mint == *mint {
            pool.base_mint
        } else {
            other_pools.push((addr, pool));
            continue;
        };

        // 按 Hot Token 优先级分类
        if other_mint == USDC_MINT || other_mint == USDT_MINT {
            stable_pools.push((addr, pool));
        } else if other_mint == WSOL_TOKEN_ACCOUNT {
            wsol_pools.push((addr, pool));
        } else if is_hot_mint(&other_mint) {
            wsol_pools.push((addr, pool));
        } else {
            other_pools.push((addr, pool));
        }
    }

    // 在各分类内按 LP 供应量排序
    stable_pools.sort_by(|(_, a), (_, b)| b.lp_supply.cmp(&a.lp_supply));
    wsol_pools.sort_by(|(_, a), (_, b)| b.lp_supply.cmp(&a.lp_supply));
    other_pools.sort_by(|(_, a), (_, b)| b.lp_supply.cmp(&a.lp_supply));

    // 合并：稳定币对 > WSOL 对 > 其他对
    let mut sorted_pools = Vec::new();
    sorted_pools.extend(stable_pools);
    sorted_pools.extend(wsol_pools);
    sorted_pools.extend(other_pools);

    // 3. 写入缓存
    cache::cache_pools_list_by_mint(mint, &sorted_pools);

    Ok(sorted_pools)
}

/// 获取 Pool 的 base 和 quote token 余额（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
pub async fn get_token_balances<T: PoolRpcClient + ?Sized>(
    pool: &Pool,
    rpc: &T,
) -> Result<(u64, u64), anyhow::Error> {
    let (base_balance_result, quote_balance_result) = tokio::join!(
        rpc.get_token_account_balance(&pool.pool_base_token_account),
        rpc.get_token_account_balance(&pool.pool_quote_token_account),
    );

    let base_balance =
        base_balance_result.map_err(|e| anyhow::anyhow!("获取 base token 余额失败: {}", e))?;
    let quote_balance =
        quote_balance_result.map_err(|e| anyhow::anyhow!("获取 quote token 余额失败: {}", e))?;

    // UiTokenAmount 的 amount 字段是字符串形式
    let base_amount = base_balance
        .amount
        .parse::<u64>()
        .map_err(|e| anyhow!("解析 base token 余额失败: {}", e))?;
    let quote_amount = quote_balance
        .amount
        .parse::<u64>()
        .map_err(|e| anyhow!("解析 quote token 余额失败: {}", e))?;

    Ok((base_amount, quote_amount))
}
