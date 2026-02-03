// Raydium CPMM Pool 查询函数

use crate::{
    common::auto_mock_rpc::PoolRpcClient,
    constants::{USDC_MINT, USDT_MINT, WSOL_TOKEN_ACCOUNT},
    instruction::utils::{
        raydium_cpmm_types::PoolState,
        raydium_cpmm::{cache, constants::accounts, constants::TOKEN0_MINT_OFFSET, constants::TOKEN1_MINT_OFFSET, helpers::is_hot_mint, helpers::select_best_pool_by_liquidity},
    },
};
use anyhow::anyhow;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use solana_account_decoder::{UiAccountData, UiAccountEncoding};
use solana_client::rpc_filter::Memcmp;
use solana_rpc_client_api::{config::RpcProgramAccountsConfig, filter::RpcFilterType};
use solana_sdk::{pubkey::Pubkey};
use std::str::FromStr;

/// 获取指定地址的 CPMM 池（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient
/// 包含缓存功能以提高性能。
///
/// # Arguments
/// * `rpc`: 实现了 PoolRpcClient 的 RPC 客户端（支持 AutoMockRpcClient 或标准 RpcClient）
/// * `pool_address`: Pool 地址
///
/// # Returns
/// 返回 Pool 状态
pub async fn get_pool_by_address<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    // RPC 查询（不缓存，每次获取最新数据）
    let account = rpc
        .get_account(pool_address)
        .await
        .map_err(|e| anyhow!("RPC 调用失败: {}", e))?;
    if account.owner != accounts::RAYDIUM_CPMM {
        return Err(anyhow!("Account is not owned by Raydium Cpmm program"));
    }
    // 使用修改后的 pool_state_decode（传入 program_id）
    let pool_state = crate::instruction::utils::raydium_cpmm_types::pool_state_decode(
        &account.data[8..],
        account.owner,
    )
    .ok_or_else(|| anyhow!("Failed to decode pool state"))?;
    // 不写入缓存
    Ok(pool_state)
}

/// 获取指定 mint 对应的最优 CPMM 池（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient
/// 包含缓存功能以提高性能。
///
/// # Arguments
/// * `rpc`: 实现了 PoolRpcClient 的 RPC 客户端（支持 AutoMockRpcClient 或标准 RpcClient）
/// * `mint`: Token mint 地址
///
/// # Returns
/// 返回最优池的地址和状态
pub async fn get_pool_by_mint<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool_address) = cache::get_cached_pool_address_by_mint(mint)
        && let Some(pool) = cache::get_cached_pool_by_address(&pool_address)
    {
        return Ok((pool_address, pool));
    }
    // 2. RPC 查询 - 使用 find_all_pools_by_mint_impl 获取所有池
    let all_pools = find_all_pools_by_mint_impl(rpc, mint).await?;

    if all_pools.is_empty() {
        return Err(anyhow!("No CPMM pool found for mint: {}", mint));
    }

    // 分类：稳定币对 > WSOL 对 > 其他对
    let mut stable_pools: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut wsol_pools: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, PoolState)> = Vec::new();

    for (addr, pool) in all_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if pool.token0_mint == *mint {
            pool.token1_mint
        } else if pool.token1_mint == *mint {
            pool.token0_mint
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

    // 按优先级选择最佳池
    let best_pool = if !stable_pools.is_empty() {
        select_best_pool_by_liquidity(&stable_pools)
    } else if !wsol_pools.is_empty() {
        select_best_pool_by_liquidity(&wsol_pools)
    } else if *mint == WSOL_TOKEN_ACCOUNT {
        select_best_pool_by_liquidity(&other_pools)
    } else {
        select_best_pool_by_liquidity(&other_pools)
    };

    // 3. 写入缓存
    if let Some((pool_addr, pool_state)) = best_pool.as_ref() {
        cache::cache_pool_address_by_mint(mint, pool_addr);
        cache::cache_pool_by_address(pool_addr, pool_state);
    }

    best_pool.ok_or_else(|| anyhow::anyhow!("未找到 {} 的可用 Raydium CPMM 池", mint))
}

/// 强制刷新并获取指定地址的 CPMM 池（支持 Auto Mock）
///
/// 清除缓存后重新获取池信息。
///
/// # Arguments
/// * `rpc`: 实现了 PoolRpcClient 的 RPC 客户端
/// * `pool_address`: Pool 地址
pub async fn get_pool_by_address_force<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    cache::POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

/// 强制刷新并获取指定 mint 的最优 CPMM 池（支持 Auto Mock）
///
/// 清除缓存后重新获取池信息。
///
/// # Arguments
/// * `rpc`: 实现了 PoolRpcClient 的 RPC 客户端
/// * `mint`: Token mint 地址
pub async fn get_pool_by_mint_force<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    cache::MINT_TO_POOL_CACHE.remove(mint);
    get_pool_by_mint(rpc, mint).await
}

/// 列出所有包含指定 mint 的 Raydium CPMM Pool（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
/// 结果已缓存以提高重复查询的性能。
///
/// # Arguments
/// * `rpc`: 实现了 PoolRpcClient 的 RPC 客户端（支持 AutoMockRpcClient 或标准 RpcClient）
/// * `mint`: Token mint 地址
///
/// # Returns
/// 返回按 Hot Token 优先策略排序后的池子列表：
/// 1. 稳定币对（USDC/USDT）优先
/// 2. WSOL 对次之
/// 3. 其他对最后
pub async fn list_pools_by_mint<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    // 1. 检查缓存
    if let Some(cached_pools) = cache::get_cached_pools_list_by_mint(mint) {
        return Ok(cached_pools);
    }

    // 2. 通过共用函数查询所有池子
    let all_pools = find_all_pools_by_mint_impl(rpc, mint).await?;

    // 分类：稳定币对 > WSOL 对 > 其他对
    let mut stable_pools: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut wsol_pools: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, PoolState)> = Vec::new();

    for (addr, pool) in all_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if pool.token0_mint == *mint {
            pool.token1_mint
        } else if pool.token1_mint == *mint {
            pool.token0_mint
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
            // Hot mint 但不在上述分类中（理论上不会发生，但为了完整性）
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

/// 清除 Pool 缓存
pub fn clear_pool_cache() {
    cache::clear_all();
}

/// 获取 Pool 的两个 token 余额（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
///
/// # Returns
/// 返回 token0_balance, token1_balance
pub async fn get_pool_token_balances<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_state: &Pubkey,
    token0_mint: &Pubkey,
    token1_mint: &Pubkey,
) -> Result<(u64, u64), anyhow::Error> {
    let token0_vault = super::helpers::get_vault_pda(pool_state, token0_mint).unwrap();
    let token1_vault = super::helpers::get_vault_pda(pool_state, token1_mint).unwrap();

    let (token0_balance_result, token1_balance_result) = tokio::join!(
        rpc.get_token_account_balance(&token0_vault),
        rpc.get_token_account_balance(&token1_vault),
    );

    let token0_balance = token0_balance_result.map_err(|e| anyhow!("RPC 调用失败: {}", e))?;
    let token1_balance = token1_balance_result.map_err(|e| anyhow!("RPC 调用失败: {}", e))?;

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

/// 内部实现：通过 offset 查找所有 Pool
/// 通过 offset 查找所有 Pool（支持 Auto Mock）
async fn find_pools_by_mint_offset_collect<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
    offset: usize,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    // 暂时移除 DataSize 过滤，只使用 Memcmp 过滤
    let filters = vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        offset,
        &mint.to_bytes(),
    ))];
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
        .get_program_ui_accounts_with_config(&accounts::RAYDIUM_CPMM, config)
        .await
        .map_err(|e| anyhow!("RPC 调用失败: {}", e))?;

    let pools: Vec<(Pubkey, PoolState)> = accounts
        .into_iter()
        .filter_map(|(addr, acc)| {
            let pubkey = Pubkey::from_str(&addr).ok()?;
            let data_bytes = match &acc.data {
                UiAccountData::Binary(base64_str, _) => STANDARD.decode(base64_str).ok()?,
                _ => return None,
            };
            if data_bytes.len() > 8 {
                // 使用 program_id (所有账户都属于 RAYDIUM_CPMM)
                crate::instruction::utils::raydium_cpmm_types::pool_state_decode(
                    &data_bytes[8..],
                    accounts::RAYDIUM_CPMM,
                )
                .map(|pool| (pubkey, pool))
            } else {
                None
            }
        })
        .collect();

    Ok(pools)
}

/// 内部实现：查找指定 mint 的所有 Raydium CPMM Pool
///
/// 策略：
/// 1. 并行查询 token0_mint 与 token1_mint 包含该 mint 的所有池
/// 2. 合并并去重
async fn find_all_pools_by_mint_impl<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    use std::collections::HashSet;

    let (token0_result, token1_result) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN0_MINT_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN1_MINT_OFFSET),
    );

    // 检测是否都失败，如果都失败则返回第一个错误（通常包含 RPC 限制信息）
    match (&token0_result, &token1_result) {
        (Err(e), Err(_)) => return Err(anyhow::anyhow!("{}", e)),
        _ => {}
    }

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
