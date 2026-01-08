use crate::{
    common::SolanaRpcClient,
    instruction::utils::raydium_amm_v4_types::{amm_info_decode, AmmInfo, AMM_INFO_SIZE},
    constants::SOL_MINT,
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

/// 清除所有缓存
pub(crate) fn clear_pool_cache_internal() {
    POOL_DATA_CACHE.clear();
    MINT_TO_POOL_CACHE.clear();
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
        .await?;

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

/// 内部实现：查找指定 mint 对应的最优 Raydium AMM V4 Pool
///
/// 策略：
/// 1. 并行查询 coin_mint 与 pc_mint 包含该 mint 的所有池
/// 2. 合并并去重
/// 3. 优先选择包含 WSOL (SOL_MINT) 的交易对
/// 4. 若没有 WSOL 交易对，则按 lp_amount 从大到小排序，选择流动性最好的池
async fn find_pool_by_mint_impl(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, AmmInfo), anyhow::Error> {
    use std::collections::HashSet;

    let (coin_result, pc_result) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, COIN_MINT_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, PC_MINT_OFFSET),
    );

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

    // 优先选择包含 WSOL 的交易对
    let mut wsol_pools: Vec<&(Pubkey, AmmInfo)> = all_pools
        .iter()
        .filter(|(_, amm)| amm.coin_mint == SOL_MINT || amm.pc_mint == SOL_MINT)
        .collect();

    if !wsol_pools.is_empty() {
        // 按 LP 供应量从大到小排序
        wsol_pools.sort_by(|a, b| b.1.lp_amount.cmp(&a.1.lp_amount));
        let (address, amm) = wsol_pools[0];
        return Ok((*address, amm.clone()));
    }

    // 若没有 WSOL 交易对，则按 LP 供应量从大到小排序，选择流动性最好的池
    all_pools.sort_by(|a, b| b.1.lp_amount.cmp(&a.1.lp_amount));
    let (address, amm) = all_pools[0].clone();
    Ok((address, amm))
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

/// 列出所有包含指定 mint 的 Raydium AMM V4 Pool（不做最优选择）
///
/// 该接口主要用于上层路由与策略模块进行池路由与选择，不做任何排序或过滤。
pub async fn list_pools_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, AmmInfo)>, anyhow::Error> {
    use std::collections::HashSet;

    let mut out: Vec<(Pubkey, AmmInfo)> = Vec::new();
    let mut seen: HashSet<Pubkey> = HashSet::new();

    // 扫描 coin_mint = mint 的池
    if let Ok(coin_pools) = find_pools_by_mint_offset_collect(rpc, mint, COIN_MINT_OFFSET).await {
        for (addr, amm) in coin_pools {
            if seen.insert(addr) {
                out.push((addr, amm));
            }
        }
    }

    // 扫描 pc_mint = mint 的池并合并
    if let Ok(pc_pools) = find_pools_by_mint_offset_collect(rpc, mint, PC_MINT_OFFSET).await {
        for (addr, amm) in pc_pools {
            if seen.insert(addr) {
                out.push((addr, amm));
            }
        }
    }

    if out.is_empty() {
        return Err(anyhow!("No Raydium AMM V4 pool found for mint {}", mint));
    }

    Ok(out)
}
