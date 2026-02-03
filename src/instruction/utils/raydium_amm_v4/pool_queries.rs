// Pool 查询函数

use super::constants::{
    COIN_MINT_OFFSET, MAX_CACHE_SIZE, PC_MINT_OFFSET, accounts::RAYDIUM_AMM_V4,
};
use super::helpers::{calculate_effective_volume, is_hot_mint, select_best_pool_by_volume};
use crate::common::{SolanaRpcClient, auto_mock_rpc::PoolRpcClient};
use crate::constants::{SOL_MINT, USDC_MINT, USDT_MINT};
use crate::instruction::utils::raydium_amm_v4_types::{AMM_INFO_SIZE, AmmInfo, amm_info_decode};
use anyhow::anyhow;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;

// ==================== 缓存模块 ====================

/// pool_address → AmmInfo 数据缓存
static POOL_DATA_CACHE: Lazy<DashMap<Pubkey, AmmInfo>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

/// mint → pool_address 映射缓存
static MINT_TO_POOL_CACHE: Lazy<DashMap<Pubkey, Pubkey>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

/// mint → Vec<(pool_address, AmmInfo)> 列表缓存（用于 list_pools_by_mint）
static MINT_TO_POOLS_LIST_CACHE: Lazy<DashMap<Pubkey, Vec<(Pubkey, AmmInfo)>>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

/// 从缓存中获取 Pool 信息
#[allow(dead_code)]
pub(crate) fn get_cached_pool_by_address(pool_address: &Pubkey) -> Option<AmmInfo> {
    POOL_DATA_CACHE.get(pool_address).map(|p| p.clone())
}

/// 将 Pool 信息写入缓存
#[allow(dead_code)]
pub(crate) fn cache_pool_by_address(pool_address: &Pubkey, amm_info: &AmmInfo) {
    POOL_DATA_CACHE.insert(*pool_address, amm_info.clone());
}

/// 从缓存中根据 mint 获取 Pool 地址
#[expect(dead_code, reason = "预留用于未来缓存策略优化")]
pub(crate) fn get_cached_pool_address_by_mint(mint: &Pubkey) -> Option<Pubkey> {
    MINT_TO_POOL_CACHE.get(mint).map(|p| *p)
}

/// 将 mint → pool_address 映射写入缓存
#[expect(dead_code, reason = "预留用于未来缓存策略优化")]
pub(crate) fn cache_pool_address_by_mint(mint: &Pubkey, pool_address: &Pubkey) {
    MINT_TO_POOL_CACHE.insert(*mint, *pool_address);
}

/// 从缓存中获取 mint 对应的池子列表
#[expect(dead_code, reason = "预留用于未来缓存策略优化")]
pub(crate) fn get_cached_pools_list_by_mint(mint: &Pubkey) -> Option<Vec<(Pubkey, AmmInfo)>> {
    MINT_TO_POOLS_LIST_CACHE.get(mint).map(|p| p.clone())
}

/// 将 mint → Vec<(pool_address, AmmInfo)> 列表写入缓存
#[expect(dead_code, reason = "预留用于未来缓存策略优化")]
pub(crate) fn cache_pools_list_by_mint(mint: &Pubkey, pools: &[(Pubkey, AmmInfo)]) {
    MINT_TO_POOLS_LIST_CACHE.insert(*mint, pools.to_vec());
}

/// 清除所有缓存
pub(crate) fn clear_pool_cache_internal() {
    POOL_DATA_CACHE.clear();
    MINT_TO_POOL_CACHE.clear();
    MINT_TO_POOLS_LIST_CACHE.clear();
}

/// 清除所有 Pool 缓存
///
/// 清除所有缓存中的 Pool 数据。
pub fn clear_pool_cache() {
    clear_pool_cache_internal();
}

// ==================== Pool 查询函数 ====================

/// 根据地址获取 AMM Pool 信息（使用 PoolRpcClient trait，支持 Auto Mock）
///
/// 这是一个泛型版本，可以接受任何实现了 PoolRpcClient 的客户端。
/// 支持标准的 RpcClient 和 AutoMockRpcClient。
pub async fn get_pool_by_address<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<AmmInfo, anyhow::Error> {
    // RPC 查询（不缓存，每次获取最新数据）
    let account = rpc
        .get_account(pool_address)
        .await
        .map_err(|e| anyhow!("RPC 调用失败: {}", e))?;
    if account.owner != RAYDIUM_AMM_V4 {
        return Err(anyhow!("Account is not owned by Raydium AMM V4 program"));
    }
    // 使用修改后的 amm_info_decode（传入 program_id）
    let amm_info = amm_info_decode(&account.data, account.owner)
        .ok_or_else(|| anyhow!("Failed to decode amm info"))?;

    // 不写入缓存
    Ok(amm_info)
}

/// 强制刷新：强制重新查询指定 Pool
///
/// 先从缓存中删除该 Pool，然后重新查询并写入缓存。
pub async fn get_pool_by_address_force(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<AmmInfo, anyhow::Error> {
    POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

// ==================== Mint 查询相关内部函数 ====================

/// 使用 PoolRpcClient 通过 offset 查找所有包含指定 mint 的 Raydium AMM V4 Pool
async fn find_pools_by_mint_offset_collect<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
    offset: usize,
) -> Result<Vec<(Pubkey, AmmInfo)>, anyhow::Error> {
    use solana_account_decoder::UiAccountEncoding;
    use solana_client::rpc_filter::Memcmp;
    use solana_rpc_client_api::{config::RpcProgramAccountsConfig, filter::RpcFilterType};

    let filters = vec![
        RpcFilterType::DataSize(AMM_INFO_SIZE as u64),
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

    let accounts =
        rpc.get_program_ui_accounts_with_config(&RAYDIUM_AMM_V4, config)
            .await
            .map_err(|e| {
                // 检测公共 RPC 限制错误
                if e.contains("excluded from account secondary indexes") {
                    anyhow!(
                        "Public RPC does not support getProgramAccounts for Raydium AMM V4. \
                    Please use: (1) paid RPC service (Helius, QuickNode, Triton), \
                    (2) local full node, or (3) known pool addresses directly."
                    )
                } else {
                    anyhow!("RPC error: {}", e)
                }
            })?;

    let pools: Vec<(Pubkey, AmmInfo)> = accounts
        .into_iter()
        .filter_map(|(addr, acc)| {
            let addr_pubkey = addr.parse::<Pubkey>().ok()?;
            let data_bytes = match &acc.data {
                solana_account_decoder::UiAccountData::Binary(base64_str, _) => {
                    STANDARD.decode(base64_str).ok()?
                },
                _ => return None,
            };
            // 使用 program_id (所有账户都属于 RAYDIUM_AMM_V4)
            amm_info_decode(&data_bytes, RAYDIUM_AMM_V4).map(|amm| (addr_pubkey, amm))
        })
        .collect();

    Ok(pools)
}

/// 使用 PoolRpcClient 查找指定 mint 的所有 Raydium AMM V4 Pool
async fn find_all_pools_by_mint_impl<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
    filter_active: bool,
) -> Result<Vec<(Pubkey, AmmInfo)>, anyhow::Error> {
    use std::collections::HashSet;

    let (coin_result, pc_result) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, COIN_MINT_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, PC_MINT_OFFSET),
    );

    // 检测是否都失败，如果都失败则返回第一个错误（通常包含 RPC 限制信息）
    match (&coin_result, &pc_result) {
        (Err(e), Err(_)) => return Err(anyhow::anyhow!("{}", e)),
        _ => {},
    }

    let mut all_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();

    if let Ok(pools) = coin_result {
        all_pools.extend(pools);
    }

    if let Ok(pools) = pc_result {
        let mut seen: HashSet<Pubkey> = all_pools.iter().map(|(addr, _)| *addr).collect();
        for (addr, amm) in pools {
            if seen.insert(addr) {
                all_pools.push((addr, amm));
            }
        }
    }

    if all_pools.is_empty() {
        return Err(anyhow!("No Raydium AMM V4 pool found for mint {}", mint));
    }

    // 如果需要过滤活跃状态的 pool
    if filter_active {
        all_pools.retain(|(_, amm)| super::helpers::is_pool_tradeable(amm));
        if all_pools.is_empty() {
            return Err(anyhow!(
                "No active Raydium AMM V4 pool found for mint {} (all pools are disabled or not tradeable)",
                mint
            ));
        }
    }

    Ok(all_pools)
}

/// 内部实现：查找指定 mint 对应的最优 Raydium AMM V4 Pool
///
/// 策略（参考 CLMM 的 Hot Token 优先策略）：
/// 1. 获取所有活跃的池子
/// 2. 优先选择包含 Hot Mint (WSOL/USDC/USDT) 的交易对
/// 3. 在 Hot 对中优先选择稳定币对（USDC/USDT），再考虑 WSOL 对
/// 4. 在同类池子中，按累计交易量从大到小排序，选择流动性最好的池
#[allow(dead_code)]
async fn find_pool_by_mint_impl(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, AmmInfo), anyhow::Error> {
    // 获取所有活跃的池子
    let active_pools = find_all_pools_by_mint_impl(rpc, mint, true).await?;

    // 分类：稳定币对 > WSOL 对 > 其他对
    let mut stable_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();
    let mut wsol_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();

    for (addr, amm) in active_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if amm.coin_mint == *mint {
            amm.pc_mint
        } else if amm.pc_mint == *mint {
            amm.coin_mint
        } else {
            // 理论上不会出现，但为了稳健性仍加入非 Hot 集合
            other_pools.push((addr, amm));
            continue;
        };

        // 按 Hot Token 优先级分类
        if other_mint == USDC_MINT || other_mint == USDT_MINT {
            // 最优：稳定币对
            stable_pools.push((addr, amm));
        } else if other_mint == SOL_MINT {
            // 次优：WSOL 对
            wsol_pools.push((addr, amm));
        } else if is_hot_mint(&other_mint) {
            // Hot mint 但不在上述分类中（理论上不会发生，但为了完整性）
            wsol_pools.push((addr, amm));
        } else {
            other_pools.push((addr, amm));
        }
    }

    // 按优先级选择最佳池
    let best_pool = if !stable_pools.is_empty() {
        // 优先级 1: 稳定币对（USDC/USDT）
        select_best_pool_by_volume(&stable_pools)
    } else if !wsol_pools.is_empty() {
        // 优先级 2: WSOL 对
        select_best_pool_by_volume(&wsol_pools)
    } else if *mint == SOL_MINT {
        // 特殊情况：当 mint 本身是 WSOL 时
        // 在所有池中按交易量选择
        select_best_pool_by_volume(&other_pools)
    } else {
        // 优先级 3: 其他对
        select_best_pool_by_volume(&other_pools)
    };

    best_pool.ok_or_else(|| anyhow::anyhow!("未找到 {} 的可用 Raydium AMM V4 池", mint))
}

// ==================== 基于 Mint 的公共查询 API ====================

/// 使用 PoolRpcClient 获取指定 mint 对应的最优 Raydium AMM V4 池（支持 Auto Mock）
///
/// 这是一个简化版本，不支持缓存，主要用于测试环境加速。
///
/// # Arguments
/// * `rpc`: 实现了 PoolRpcClient 的 RPC 客户端（支持 AutoMockRpcClient）
/// * `mint`: Token mint 地址
///
/// # Returns
/// 返回最优池的地址和 AMM 信息
pub async fn get_pool_by_mint<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<(Pubkey, AmmInfo), anyhow::Error> {
    // 使用 find_all_pools_by_mint_impl 获取所有活跃池子
    let active_pools = find_all_pools_by_mint_impl(rpc, mint, true).await?;

    if active_pools.is_empty() {
        return Err(anyhow::anyhow!("No active AMM V4 pool found for mint: {}", mint));
    }

    // 分类：稳定币对 > WSOL 对 > 其他对
    let mut stable_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();
    let mut wsol_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();

    for (addr, amm) in active_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if amm.coin_mint == *mint {
            amm.pc_mint
        } else if amm.pc_mint == *mint {
            amm.coin_mint
        } else {
            other_pools.push((addr, amm));
            continue;
        };

        // 按 Hot Token 优先级分类
        if other_mint == USDC_MINT || other_mint == USDT_MINT {
            stable_pools.push((addr, amm));
        } else if other_mint == SOL_MINT {
            wsol_pools.push((addr, amm));
        } else if is_hot_mint(&other_mint) {
            wsol_pools.push((addr, amm));
        } else {
            other_pools.push((addr, amm));
        }
    }

    // 按优先级选择最佳池
    let best_pool = if !stable_pools.is_empty() {
        select_best_pool_by_volume(&stable_pools)
    } else if !wsol_pools.is_empty() {
        select_best_pool_by_volume(&wsol_pools)
    } else if *mint == SOL_MINT {
        select_best_pool_by_volume(&other_pools)
    } else {
        select_best_pool_by_volume(&other_pools)
    };

    best_pool.ok_or_else(|| anyhow::anyhow!("未找到 {} 的可用 Raydium AMM V4 池", mint))
}

/// 强制刷新：强制重新查询指定 mint 对应的最优 Pool
///
/// 先从 mint → pool_address 缓存中删除该 mint，然后重新查询并写入缓存。
pub async fn get_pool_by_mint_force(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, AmmInfo), anyhow::Error> {
    MINT_TO_POOL_CACHE.remove(mint);
    get_pool_by_mint(rpc, mint).await
}

/// 列出所有包含指定 mint 的 Raydium AMM V4 Pool
///
/// 返回按 Hot Token 优先策略排序后的池子列表：
/// 1. 稳定币对（USDC/USDT）优先
/// 2. WSOL 对次之
/// 3. 其他对最后
/// 4. 同类池子按累计交易量从大到小排序
///
/// Results are cached to improve performance on repeated queries.
///
/// # 参数
/// - `rpc`: RPC 客户端
/// - `mint`: 要查询的代币 mint 地址
/// - `filter_active`: 是否只返回活跃状态的 pool（适合交易的 pool）
///
/// # 返回
/// - 返回排序后的包含指定 mint 的 pool 列表
/// - 如果 `filter_active` 为 true，则只返回活跃状态的 pool
pub async fn list_pools_by_mint<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
    filter_active: bool,
) -> Result<Vec<(Pubkey, AmmInfo)>, anyhow::Error> {
    use super::helpers::is_pool_tradeable;

    // 注意：这里不使用内存缓存，直接查询
    // Auto Mock 会在文件层面缓存

    // 通过共用函数查询所有池子（不过滤）
    let all_pools = find_all_pools_by_mint_impl(rpc, mint, false).await?;

    // 分类：稳定币对 > WSOL 对 > 其他对
    let mut stable_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();
    let mut wsol_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();
    let mut other_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();

    for (addr, amm) in all_pools.into_iter() {
        // 找到与目标 mint 对应的另一侧 mint
        let other_mint = if amm.coin_mint == *mint {
            amm.pc_mint
        } else if amm.pc_mint == *mint {
            amm.coin_mint
        } else {
            other_pools.push((addr, amm));
            continue;
        };

        // 按 Hot Token 优先级分类
        if other_mint == USDC_MINT || other_mint == USDT_MINT {
            stable_pools.push((addr, amm));
        } else if other_mint == SOL_MINT {
            wsol_pools.push((addr, amm));
        } else if is_hot_mint(&other_mint) {
            // Hot mint 但不在上述分类中（理论上不会发生，但为了完整性）
            wsol_pools.push((addr, amm));
        } else {
            other_pools.push((addr, amm));
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

    // 如果需要过滤活跃状态的 pool
    if filter_active {
        let filtered: Vec<_> =
            sorted_pools.into_iter().filter(|(_, amm)| is_pool_tradeable(amm)).collect();
        if filtered.is_empty() {
            return Err(anyhow!(
                "No active Raydium AMM V4 pool found for mint {} (all pools are disabled or not tradeable)",
                mint
            ));
        }
        return Ok(filtered);
    }

    Ok(sorted_pools)
}

/// 获取任意 Token 在 Raydium AMM V4 上的 USD 价格（通过 X-WSOL 池 + Raydium CLMM WSOL-USD 锚定池）
///
/// 价格计算路径：Token X -> WSOL -> USD
/// - 要求：存在一个 X-WSOL 的 AMM V4 池，以及一个 Raydium CLMM 上的 WSOL-USDT/USDC 锚定池
pub async fn get_token_price_in_usd(
    rpc: &SolanaRpcClient,
    token_mint: &Pubkey,
    wsol_usd_clmm_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool =
        wsol_usd_clmm_pool_address.unwrap_or(&super::constants::DEFAULT_WSOL_USDT_CLMM_POOL);
    use crate::utils::price::raydium_amm_v4::{price_base_in_quote, price_quote_in_base};

    // 稳定币自身的价格直接认为是 1 USD
    if *token_mint == USDC_MINT || *token_mint == USDT_MINT {
        return Ok(1.0);
    }

    // WSOL/SOL 的价格通过 Raydium CLMM 锚定池获取
    if *token_mint == SOL_MINT {
        return crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await;
    }

    // 1. 在 AMM V4 中找到 Token X 的最优池（优先 X-WSOL/USDC/USDT 对）
    let (pool_address, amm_best) = get_pool_by_mint(rpc, token_mint).await?;

    // 2. 为了价格实时性，对选中的池地址强制刷新一次 AmmInfo
    let amm = get_pool_by_address_force(rpc, &pool_address).await.unwrap_or(amm_best);

    // 3. 判断池子配对类型
    let is_coin_x = amm.coin_mint == *token_mint;
    let is_pc_x = amm.pc_mint == *token_mint;

    let other_mint = if is_coin_x {
        amm.pc_mint
    } else if is_pc_x {
        amm.coin_mint
    } else {
        return Err(anyhow!(
            "AMM V4 Pool {} does not contain the target mint {}",
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
        let coin_decimals = crate::utils::token::get_token_decimals(rpc, &amm.coin_mint).await?;
        let pc_decimals = crate::utils::token::get_token_decimals(rpc, &amm.pc_mint).await?;

        // 获取实时余额
        let coin_balance =
            rpc.get_token_account_balance(&amm.token_coin)
                .await?
                .ui_amount
                .ok_or_else(|| anyhow!("Failed to get coin balance"))? as u64;
        let pc_balance = rpc
            .get_token_account_balance(&amm.token_pc)
            .await?
            .ui_amount
            .ok_or_else(|| anyhow!("Failed to get pc balance"))? as u64;

        let price_x_in_stable = if is_coin_x {
            // coin = X, pc = USDC/USDT
            price_base_in_quote(coin_balance, pc_balance, coin_decimals, pc_decimals)
        } else {
            // pc = X, coin = USDC/USDT
            price_quote_in_base(coin_balance, pc_balance, coin_decimals, pc_decimals)
        };

        if price_x_in_stable <= 0.0 {
            return Err(anyhow!(
                "Invalid price from X-Stable AMM V4 pool (<= 0): mint={}, pool={}",
                token_mint,
                pool_address
            ));
        }

        return Ok(price_x_in_stable); // 稳定币 = 1 USD
    }

    if other_mint != SOL_MINT {
        return Err(anyhow!(
            "Best AMM V4 pool for mint {} is paired with {} (not WSOL/USDC/USDT); multi-hop USD pricing is not supported yet",
            token_mint,
            other_mint
        ));
    }

    // X-WSOL 池：计算 X 相对 WSOL 的价格
    let coin_decimals = crate::utils::token::get_token_decimals(rpc, &amm.coin_mint).await?;
    let pc_decimals = crate::utils::token::get_token_decimals(rpc, &amm.pc_mint).await?;

    // 获取实时余额
    let coin_balance = rpc
        .get_token_account_balance(&amm.token_coin)
        .await?
        .ui_amount
        .ok_or_else(|| anyhow!("Failed to get coin balance"))? as u64;
    let pc_balance = rpc
        .get_token_account_balance(&amm.token_pc)
        .await?
        .ui_amount
        .ok_or_else(|| anyhow!("Failed to get pc balance"))? as u64;

    let price_x_in_wsol = if is_coin_x {
        // coin = X, pc = WSOL
        price_base_in_quote(coin_balance, pc_balance, coin_decimals, pc_decimals)
    } else {
        // pc = X, coin = WSOL
        price_quote_in_base(coin_balance, pc_balance, coin_decimals, pc_decimals)
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price on AMM V4 is invalid (<= 0)"));
    }

    // 4. 计算 WSOL 的 USD 价格
    let price_wsol_in_usd =
        crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}

/// 获取任意 Token 在 Raydium AMM V4 上的 USD 价格（直接传入 X-WSOL 池地址，跳过池查找）
///
/// 与 `get_token_price_in_usd` 的区别：
/// - 此函数要求调用者已知 X-WSOL 池地址，直接传入，避免 `get_pool_by_mint` 的查找开销
/// - 适用于高频调用、已缓存池地址的场景
///
/// # Arguments
/// * `rpc` - Solana RPC 客户端
/// * `token_mint` - Token X 的 mint 地址
/// * `x_wsol_pool_address` - Token X 与 WSOL 配对的 AMM V4 池地址
/// * `wsol_usd_clmm_pool_address` - Raydium CLMM 上的 WSOL-USDT/USDC 锚定池地址
pub async fn get_token_price_in_usd_with_pool(
    rpc: &SolanaRpcClient,
    token_mint: &Pubkey,
    x_wsol_pool_address: &Pubkey,
    wsol_usd_clmm_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool =
        wsol_usd_clmm_pool_address.unwrap_or(&super::constants::DEFAULT_WSOL_USDT_CLMM_POOL);
    use crate::utils::price::raydium_amm_v4::{price_base_in_quote, price_quote_in_base};

    // 稳定币自身的价格直接认为是 1 USD
    if *token_mint == USDC_MINT || *token_mint == USDT_MINT {
        return Ok(1.0);
    }

    // WSOL/SOL 的价格通过 Raydium CLMM 锚定池获取
    if *token_mint == SOL_MINT {
        return crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await;
    }

    // 1. 直接强制刷新指定的 X-WSOL 池（跳过查找步骤）
    let amm = get_pool_by_address_force(rpc, x_wsol_pool_address).await?;

    // 2. 判断池子配对类型
    let is_coin_x = amm.coin_mint == *token_mint;
    let is_pc_x = amm.pc_mint == *token_mint;

    let other_mint = if is_coin_x {
        amm.pc_mint
    } else if is_pc_x {
        amm.coin_mint
    } else {
        return Err(anyhow!(
            "Provided AMM V4 pool {} does not contain the target mint {}",
            x_wsol_pool_address,
            token_mint
        ));
    };

    // 支持三种池子类型：
    // 1. X-WSOL：需要通过 WSOL-USD 锚定池计算
    // 2. X-USDC/USDT：直接认为稳定币价格 = 1 USD
    // 3. 其他：暂不支持（需要多跳路由）
    if other_mint == USDC_MINT || other_mint == USDT_MINT {
        // X-稳定币池：直接计算 X 相对稳定币的价格
        let coin_decimals = crate::utils::token::get_token_decimals(rpc, &amm.coin_mint).await?;
        let pc_decimals = crate::utils::token::get_token_decimals(rpc, &amm.pc_mint).await?;

        // 获取实时余额
        let coin_balance =
            rpc.get_token_account_balance(&amm.token_coin)
                .await?
                .ui_amount
                .ok_or_else(|| anyhow!("Failed to get coin balance"))? as u64;
        let pc_balance = rpc
            .get_token_account_balance(&amm.token_pc)
            .await?
            .ui_amount
            .ok_or_else(|| anyhow!("Failed to get pc balance"))? as u64;

        let price_x_in_stable = if is_coin_x {
            // coin = X, pc = USDC/USDT
            price_base_in_quote(coin_balance, pc_balance, coin_decimals, pc_decimals)
        } else {
            // pc = X, coin = USDC/USDT
            price_quote_in_base(coin_balance, pc_balance, coin_decimals, pc_decimals)
        };

        if price_x_in_stable <= 0.0 {
            return Err(anyhow!(
                "Invalid price from X-Stable AMM V4 pool (<= 0): mint={}, pool={}",
                token_mint,
                x_wsol_pool_address
            ));
        }

        return Ok(price_x_in_stable); // 稳定币 = 1 USD
    }

    if other_mint != SOL_MINT {
        return Err(anyhow!(
            "Provided AMM V4 pool {} is paired with {} (not WSOL/USDC/USDT); multi-hop USD pricing is not supported yet",
            x_wsol_pool_address,
            other_mint
        ));
    }

    // 3. X-WSOL 池：计算 X 相对 WSOL 的价格
    let coin_decimals = crate::utils::token::get_token_decimals(rpc, &amm.coin_mint).await?;
    let pc_decimals = crate::utils::token::get_token_decimals(rpc, &amm.pc_mint).await?;

    // 获取实时余额
    let coin_balance = rpc
        .get_token_account_balance(&amm.token_coin)
        .await?
        .ui_amount
        .ok_or_else(|| anyhow!("Failed to get coin balance"))? as u64;
    let pc_balance = rpc
        .get_token_account_balance(&amm.token_pc)
        .await?
        .ui_amount
        .ok_or_else(|| anyhow!("Failed to get pc balance"))? as u64;

    let price_x_in_wsol = if is_coin_x {
        // coin = X, pc = WSOL
        price_base_in_quote(coin_balance, pc_balance, coin_decimals, pc_decimals)
    } else {
        // pc = X, coin = WSOL
        price_quote_in_base(coin_balance, pc_balance, coin_decimals, pc_decimals)
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price on AMM V4 is invalid (<= 0)"));
    }

    // 4. 计算 WSOL 的 USD 价格
    let price_wsol_in_usd =
        crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}
