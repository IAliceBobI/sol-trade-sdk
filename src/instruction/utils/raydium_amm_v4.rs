use crate::{
    common::SolanaRpcClient,
    instruction::utils::raydium_amm_v4_types::{amm_info_decode, AmmInfo, AMM_INFO_SIZE},
    constants::{SOL_MINT, USDC_MINT, USDT_MINT},
};
use anyhow::anyhow;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;

/// Constants used as seeds for deriving PDAs (Program Derived Addresses)
pub mod seeds {
    pub const POOL_SEED: &[u8] = b"pool";
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};
    pub const AUTHORITY: Pubkey = pubkey!("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    pub const RAYDIUM_AMM_V4: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");

    pub const TRADE_FEE_NUMERATOR: u64 = 25;
    pub const TRADE_FEE_DENOMINATOR: u64 = 10000;
    pub const SWAP_FEE_NUMERATOR: u64 = 25;
    pub const SWAP_FEE_DENOMINATOR: u64 = 10000;

    // META

    pub const AUTHORITY_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: AUTHORITY,
            is_signer: false,
            is_writable: false,
        };
}

pub const SWAP_BASE_IN_DISCRIMINATOR: &[u8] = &[9];
pub const SWAP_BASE_OUT_DISCRIMINATOR: &[u8] = &[11];

// ==================== 缓存模块 ====================

const MAX_CACHE_SIZE: usize = 50_000;

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
pub(crate) fn get_cached_pool_by_address(pool_address: &Pubkey) -> Option<AmmInfo> {
    POOL_DATA_CACHE.get(pool_address).map(|p| p.clone())
}

/// 将 Pool 信息写入缓存
pub(crate) fn cache_pool_by_address(pool_address: &Pubkey, amm_info: &AmmInfo) {
    POOL_DATA_CACHE.insert(*pool_address, amm_info.clone());
}

/// 从缓存中根据 mint 获取 Pool 地址
pub(crate) fn get_cached_pool_address_by_mint(mint: &Pubkey) -> Option<Pubkey> {
    MINT_TO_POOL_CACHE.get(mint).map(|p| *p)
}

/// 将 mint → pool_address 映射写入缓存
pub(crate) fn cache_pool_address_by_mint(mint: &Pubkey, pool_address: &Pubkey) {
    MINT_TO_POOL_CACHE.insert(*mint, *pool_address);
}

/// 从缓存中获取 mint 对应的池子列表
pub(crate) fn get_cached_pools_list_by_mint(mint: &Pubkey) -> Option<Vec<(Pubkey, AmmInfo)>> {
    MINT_TO_POOLS_LIST_CACHE.get(mint).map(|p| p.clone())
}

/// 将 mint → Vec<(pool_address, AmmInfo)> 列表写入缓存
pub(crate) fn cache_pools_list_by_mint(mint: &Pubkey, pools: &[(Pubkey, AmmInfo)]) {
    MINT_TO_POOLS_LIST_CACHE.insert(*mint, pools.to_vec());
}

/// 清除所有缓存
pub(crate) fn clear_pool_cache_internal() {
    POOL_DATA_CACHE.clear();
    MINT_TO_POOL_CACHE.clear();
    MINT_TO_POOLS_LIST_CACHE.clear();
}

// ==================== 公共函数 ====================

/// 根据地址获取 AMM Pool 信息（带缓存）
///
/// 如果缓存中有该 Pool 的信息，直接从缓存返回；
/// 否则通过 RPC 查询，并将结果写入缓存。
pub async fn get_pool_by_address(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<AmmInfo, anyhow::Error> {
    // 1. 检查缓存
    if let Some(amm_info) = get_cached_pool_by_address(pool_address) {
        return Ok(amm_info);
    }

    // 2. RPC 查询
    let account = rpc.get_account(pool_address).await?;
    if account.owner != accounts::RAYDIUM_AMM_V4 {
        return Err(anyhow!("Account is not owned by Raydium AMM V4 program"));
    }
    let amm_info = amm_info_decode(&account.data)
        .ok_or_else(|| anyhow!("Failed to decode amm info"))?;

    // 3. 写入缓存
    cache_pool_by_address(pool_address, &amm_info);

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

/// 清除所有 Pool 缓存
///
/// 清除所有缓存中的 Pool 数据。
pub fn clear_pool_cache() {
    clear_pool_cache_internal();
}

/// 判断是否为 Hot Mint（主流桥接资产）
/// 当前包含：WSOL、USDC、USDT
fn is_hot_mint(mint: &Pubkey) -> bool {
    *mint == SOL_MINT || *mint == USDC_MINT || *mint == USDT_MINT
}

/// 计算池子的有效交易量（基于 swap 数据）
/// - 如果包含 WSOL/USDC/USDT，只计算这些资产侧的交易量
/// - 否则计算两侧的总交易量
fn calculate_effective_volume(amm: &AmmInfo) -> u128 {
    // 检查 coin_mint 是否为 WSOL/USDC/USDT
    let coin_is_stable = amm.coin_mint == SOL_MINT 
        || amm.coin_mint == USDC_MINT 
        || amm.coin_mint == USDT_MINT;
    
    // 检查 pc_mint 是否为 WSOL/USDC/USDT
    let pc_is_stable = amm.pc_mint == SOL_MINT 
        || amm.pc_mint == USDC_MINT 
        || amm.pc_mint == USDT_MINT;
    
    if coin_is_stable && !pc_is_stable {
        // 只计算 coin 侧（WSOL/USDC/USDT）的交易量
        amm.out_put.swap_coin_in_amount.saturating_add(amm.out_put.swap_pc_out_amount)
    } else if pc_is_stable && !coin_is_stable {
        // 只计算 pc 侧（WSOL/USDC/USDT）的交易量
        amm.out_put.swap_pc_in_amount.saturating_add(amm.out_put.swap_coin_out_amount)
    } else {
        // 两侧都是稳定资产或都不是，计算总交易量
        amm.out_put.swap_coin_in_amount
            .saturating_add(amm.out_put.swap_pc_out_amount)
            .saturating_add(amm.out_put.swap_pc_in_amount)
            .saturating_add(amm.out_put.swap_coin_out_amount)
    }
}

/// 按累计交易量选择最佳池（零网络开销）
/// 
/// 策略：
/// - 优先选择活跃状态的池
/// - 如果池子包含 WSOL/USDC/USDT，只计算这些稳定资产侧的累计交易量
/// - 否则计算两侧的总交易量
/// - 交易量越大，说明池子被实际使用越多，深度越可靠
fn select_best_pool_by_volume(pools: &[(Pubkey, AmmInfo)]) -> (Pubkey, AmmInfo) {
    if pools.is_empty() {
        panic!("Cannot select best pool from empty list");
    }

    if pools.len() == 1 {
        return pools[0].clone();
    }

    // 优先选择活跃状态的池
    let mut active_pools: Vec<_> = pools
        .iter()
        .filter(|(_, amm)| is_pool_tradeable(amm))
        .map(|(addr, amm)| (*addr, amm.clone()))
        .collect();

    if active_pools.is_empty() {
        // 如果全部池都不活跃，使用所有池
        active_pools = pools.to_vec();
    }

    // 按累计交易量排序
    active_pools.sort_by(|(_, amm_a), (_, amm_b)| {
        // 计算有效交易量（优先只看WSOL/USDC/USDT侧）
        let volume_a = calculate_effective_volume(amm_a);
        let volume_b = calculate_effective_volume(amm_b);
        
        // 按交易量降序排序
        match volume_b.cmp(&volume_a) {
            std::cmp::Ordering::Equal => {
                // 交易量相同时，按流动性排序
                amm_b.lp_amount.cmp(&amm_a.lp_amount)
            }
            other => other,
        }
    });

    // 返回交易量最高的池
    active_pools.into_iter().next().unwrap()
}

// ==================== Pool 状态检查函数 ====================

/// Pool 状态常量
pub mod pool_status {
    /// 未初始化
    pub const UNINITIALIZED: u64 = 0;
    /// 已初始化
    pub const INITIALIZED: u64 = 1;
    /// 已禁用
    pub const DISABLED: u64 = 2;
    /// 只能提现
    pub const WITHDRAW_ONLY: u64 = 3;
    /// 只能订单簿
    pub const ORDER_BOOK_ONLY: u64 = 4;
    /// 只能交易
    pub const SWAP_ONLY: u64 = 5;
    /// 活跃状态
    pub const ACTIVE: u64 = 6;
}

/// 检查 pool 是否处于活跃状态
///
/// 只有活跃状态的 pool 才适合进行交易。
pub fn is_pool_active(amm_info: &AmmInfo) -> bool {
    amm_info.status == pool_status::ACTIVE
}

/// 检查 pool 是否已禁用
///
/// 已禁用的 pool 不能进行交易。
pub fn is_pool_disabled(amm_info: &AmmInfo) -> bool {
    amm_info.status == pool_status::DISABLED
}

/// 检查 pool 是否只能提现
///
/// 只能提现的 pool 不能进行交易，只能提取流动性。
pub fn is_pool_withdraw_only(amm_info: &AmmInfo) -> bool {
    amm_info.status == pool_status::WITHDRAW_ONLY
}

/// 检查 pool 是否适合交易
///
/// 适合交易的 pool 必须是活跃状态。
pub fn is_pool_tradeable(amm_info: &AmmInfo) -> bool {
    is_pool_active(amm_info)
}

// ==================== Mint 查询相关常量与内部函数 ====================

/// coin_mint 在 AmmInfo 结构中的偏移量
///
/// 根据 AmmInfo 字段顺序与 Borsh 编码规则计算：
/// - 16 个 u64 字段 (16 * 8 = 128 字节)
/// - Fees (8 个 u64, 8 * 8 = 64 字节)
/// - OutPutData (10 个 u64 与 4 个 u128, 共 144 字节)
/// - token_coin (Pubkey, 32 字节)
/// - token_pc (Pubkey, 32 字节)
/// 因此 coin_mint 起始偏移量为 128 + 64 + 144 + 32 + 32 = 400 字节。
const COIN_MINT_OFFSET: usize = 400;

/// pc_mint 在 AmmInfo 结构中的偏移量
/// 即 coin_mint 之后再偏移一个 Pubkey (32 字节)
const PC_MINT_OFFSET: usize = 432;

/// 内部实现：通过 offset 查找所有包含指定 mint 的 Raydium AMM V4 Pool
async fn find_pools_by_mint_offset_collect(
    rpc: &SolanaRpcClient,
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

    let accounts = rpc
        .get_program_ui_accounts_with_config(&accounts::RAYDIUM_AMM_V4, config)
        .await
        .map_err(|e| {
            // 检测公共 RPC 限制错误
            if e.to_string().contains("excluded from account secondary indexes") {
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
            let data_bytes = match &acc.data {
                solana_account_decoder::UiAccountData::Binary(base64_str, _) => {
                    STANDARD.decode(base64_str).ok()?
                }
                _ => return None,
            };
            amm_info_decode(&data_bytes).map(|amm| (addr, amm))
        })
        .collect();

    Ok(pools)
}

/// 内部实现：查找指定 mint 的所有 Raydium AMM V4 Pool
///
/// 策略：
/// 1. 并行查询 coin_mint 与 pc_mint 包含该 mint 的所有池
/// 2. 合并并去重
/// 3. 可选：过滤掉非活跃状态的 pool（只保留适合交易的 pool）
async fn find_all_pools_by_mint_impl(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
    filter_active: bool,
) -> Result<Vec<(Pubkey, AmmInfo)>, anyhow::Error> {
    use std::collections::HashSet;

    let (coin_result, pc_result) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, COIN_MINT_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, PC_MINT_OFFSET),
    );

    // 检测是否都失败，如果都失败则返回第一个错误（通常包含 RPC 限制信息）
    if coin_result.is_err() && pc_result.is_err() {
        // 返回 coin_result 的错误，它包含我们的自定义错误消息
        return Err(coin_result.unwrap_err());
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
        all_pools.retain(|(_, amm)| is_pool_tradeable(amm));
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
/// 策略：
/// 1. 获取所有活跃的池子
/// 2. 优先选择包含 Hot Mint (WSOL/USDC/USDT) 的交易对
/// 3. 在优先级相同的池子中，按累计交易量从大到小排序，选择流动性最好的池
async fn find_pool_by_mint_impl(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, AmmInfo), anyhow::Error> {
    // 获取所有活跃的池子
    let active_pools = find_all_pools_by_mint_impl(rpc, mint, true).await?;

    // 分类：Hot 对（包含 WSOL/USDC/USDT）vs 其他对
    let mut hot_pools: Vec<(Pubkey, AmmInfo)> = Vec::new();
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

        if is_hot_mint(&other_mint) {
            hot_pools.push((addr, amm));
        } else {
            other_pools.push((addr, amm));
        }
    }

    let best_pool = if !hot_pools.is_empty() {
        // Hot 对优先：通常是 mint/WSOL、mint/USDC、mint/USDT 等主路由
        // 使用累计交易量选池（零网络开销，反映真实使用深度）
        select_best_pool_by_volume(&hot_pools)
    } else if *mint == SOL_MINT {
        // 特殊情况：当 mint 本身是 WSOL 时
        // 在所有池中按交易量选择
        select_best_pool_by_volume(&other_pools)
    } else {
        // 没有 Hot 对时，使用累计交易量选池
        select_best_pool_by_volume(&other_pools)
    };

    Ok(best_pool)
}

// ==================== 基于 Mint 的公共查询 API ====================

/// 根据 mint 获取 Raydium AMM V4 中的最优 Pool（带缓存）
///
/// - 优先从 `MINT_TO_POOL_CACHE` 命中
/// - 未命中时，通过 `find_pool_by_mint_impl` 扫描链上所有 Pool 并选择最优池
/// - 命中后会同时缓存 mint → pool_address 以及 pool_address → AmmInfo
pub async fn get_pool_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, AmmInfo), anyhow::Error> {
    // 1. 先尝试命中缓存
    if let Some(pool_address) = get_cached_pool_address_by_mint(mint) {
        if let Some(amm) = get_cached_pool_by_address(&pool_address) {
            return Ok((pool_address, amm));
        }
    }

    // 2. 未命中缓存时，查询链上数据
    let (pool_address, amm) = find_pool_by_mint_impl(rpc, mint).await?;

    // 3. 写入缓存
    cache_pool_address_by_mint(mint, &pool_address);
    cache_pool_by_address(&pool_address, &amm);

    Ok((pool_address, amm))
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
/// 该接口主要用于上层路由与策略模块进行池路由与选择。
/// Results are cached to improve performance on repeated queries.
///
/// # 参数
/// - `rpc`: RPC 客户端
/// - `mint`: 要查询的代币 mint 地址
/// - `filter_active`: 是否只返回活跃状态的 pool（适合交易的 pool）
///
/// # 返回
/// - 返回所有包含指定 mint 的 pool 列表
/// - 如果 `filter_active` 为 true，则只返回活跃状态的 pool
pub async fn list_pools_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
    filter_active: bool,
) -> Result<Vec<(Pubkey, AmmInfo)>, anyhow::Error> {
    // 1. 检查缓存（注意：缓存的是未过滤的完整列表）
    if let Some(cached_pools) = get_cached_pools_list_by_mint(mint) {
        // 如果需要过滤活跃状态，在返回前过滤
        if filter_active {
            let filtered: Vec<_> = cached_pools
                .into_iter()
                .filter(|(_, amm)| is_pool_tradeable(amm))
                .collect();
            if filtered.is_empty() {
                return Err(anyhow!(
                    "No active Raydium AMM V4 pool found for mint {} (all pools are disabled or not tradeable)",
                    mint
                ));
            }
            return Ok(filtered);
        }
        return Ok(cached_pools);
    }

    // 2. 通过共用函数查询所有池子（不过滤）
    let pools = find_all_pools_by_mint_impl(rpc, mint, false).await?;

    // 3. 写入缓存（缓存未过滤的完整列表）
    cache_pools_list_by_mint(mint, &pools);

    // 如果需要过滤活跃状态的 pool
    if filter_active {
        let filtered: Vec<_> = pools
            .into_iter()
            .filter(|(_, amm)| is_pool_tradeable(amm))
            .collect();
        if filtered.is_empty() {
            return Err(anyhow!(
                "No active Raydium AMM V4 pool found for mint {} (all pools are disabled or not tradeable)",
                mint
            ));
        }
        return Ok(filtered);
    }

    Ok(pools)
}
