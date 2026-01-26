use crate::{
    common::{
        SolanaRpcClient, auto_mock_rpc::PoolRpcClient,
        spl_associated_token_account::get_associated_token_address_with_program_id,
    },
    constants::{TOKEN_PROGRAM, USDC_MINT, USDT_MINT, WSOL_TOKEN_ACCOUNT},
    instruction::utils::pumpswap_types::{Pool, pool_decode},
};
use anyhow::anyhow;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use solana_account_decoder::{UiAccountData, UiAccountEncoding};
use solana_sdk::{pubkey, pubkey::Pubkey};

/// Raydium CLMM WSOL-USDT 锚定池（用于 USD 价格计算）
/// 如果不传入锚定池参数，默认使用此池
pub const DEFAULT_WSOL_USDT_CLMM_POOL: Pubkey =
    pubkey!("ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6");

/// Constants used as seeds for deriving PDAs (Program Derived Addresses)
pub mod seeds {
    /// Seed for the global state PDA
    pub const GLOBAL_SEED: &[u8] = b"global";

    /// Seed for the mint authority PDA
    pub const MINT_AUTHORITY_SEED: &[u8] = b"mint-authority";

    /// Seed for bonding curve PDAs
    pub const BONDING_CURVE_SEED: &[u8] = b"bonding-curve";

    /// Seed for metadata PDAs
    pub const METADATA_SEED: &[u8] = b"metadata";

    pub const USER_VOLUME_ACCUMULATOR_SEED: &[u8] = b"user_volume_accumulator";
    pub const GLOBAL_VOLUME_ACCUMULATOR_SEED: &[u8] = b"global_volume_accumulator";
    pub const FEE_CONFIG_SEED: &[u8] = b"fee_config";
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};

    /// Public key for the fee recipient
    pub const FEE_RECIPIENT: Pubkey = pubkey!("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV");

    /// Public key for the global PDA
    pub const GLOBAL_ACCOUNT: Pubkey = pubkey!("ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw");

    /// Authority for program events
    pub const EVENT_AUTHORITY: Pubkey = pubkey!("GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR");

    /// Associated Token Program ID
    pub const ASSOCIATED_TOKEN_PROGRAM: Pubkey =
        pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

    // PumpSwap protocol fee recipient
    pub const PROTOCOL_FEE_RECIPIENT: Pubkey =
        pubkey!("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV");

    pub const AMM_PROGRAM: Pubkey = pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");

    pub const LP_FEE_BASIS_POINTS: u64 = 25;
    pub const PROTOCOL_FEE_BASIS_POINTS: u64 = 5;
    pub const COIN_CREATOR_FEE_BASIS_POINTS: u64 = 5;

    pub const FEE_PROGRAM: Pubkey = pubkey!("pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ");

    pub const GLOBAL_VOLUME_ACCUMULATOR: Pubkey =
        pubkey!("C2aFPdENg4A2HQsmrd5rTw5TaYBX5Ku887cWjbFKtZpw"); // get_global_volume_accumulator_pda().unwrap();

    pub const FEE_CONFIG: Pubkey = pubkey!("5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx"); // get_fee_config_pda().unwrap();

    pub const DEFAULT_COIN_CREATOR_VAULT_AUTHORITY: Pubkey =
        pubkey!("8N3GDaZ2iwN65oxVatKTLPNooAVUJTbfiVJ1ahyqwjSk");

    /// Mayhem fee recipient (for mayhem mode coins)
    pub const MAYHEM_FEE_RECIPIENT: Pubkey =
        pubkey!("GesfTA3X2arioaHp8bbKdjG9vJtskViWACZoYvxp4twS");

    // META

    pub const GLOBAL_ACCOUNT_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: GLOBAL_ACCOUNT,
            is_signer: false,
            is_writable: false,
        };

    pub const FEE_RECIPIENT_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: FEE_RECIPIENT,
            is_signer: false,
            is_writable: false,
        };

    pub const MAYHEM_FEE_RECIPIENT_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: MAYHEM_FEE_RECIPIENT,
            is_signer: false,
            is_writable: false,
        };

    pub const ASSOCIATED_TOKEN_PROGRAM_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: ASSOCIATED_TOKEN_PROGRAM,
            is_signer: false,
            is_writable: false,
        };

    pub const EVENT_AUTHORITY_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: EVENT_AUTHORITY,
            is_signer: false,
            is_writable: false,
        };

    pub const AMM_PROGRAM_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: AMM_PROGRAM,
            is_signer: false,
            is_writable: false,
        };

    pub const GLOBAL_VOLUME_ACCUMULATOR_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: GLOBAL_VOLUME_ACCUMULATOR,
            is_signer: false,
            is_writable: true,
        };

    pub const FEE_CONFIG_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: FEE_CONFIG,
            is_signer: false,
            is_writable: false,
        };

    pub const FEE_PROGRAM_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: FEE_PROGRAM,
            is_signer: false,
            is_writable: false,
        };
}

pub const BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
pub const BUY_EXACT_QUOTE_IN_DISCRIMINATOR: [u8; 8] = [198, 46, 21, 82, 180, 217, 232, 112];
pub const SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

/// 判断是否为 Hot Mint（主流桥接资产）
/// 当前包含：WSOL、USDC、USDT
fn is_hot_mint(mint: &Pubkey) -> bool {
    *mint == WSOL_TOKEN_ACCOUNT || *mint == USDC_MINT || *mint == USDT_MINT
}

/// 按 LP 供应量选择最佳池（PumpSwap 池没有交易量字段，使用 lp_supply 作为流动性指标）
///
/// 策略：
/// - LP 供应量越大，说明流动性越好
fn select_best_pool_by_liquidity(pools: &[(Pubkey, Pool)]) -> Option<(Pubkey, Pool)> {
    if pools.is_empty() {
        return None;
    }

    if pools.len() == 1 {
        return pools.first().cloned();
    }

    // 按 LP 供应量排序
    let mut sorted_pools = pools.to_vec();
    sorted_pools.sort_by(|(_, pool_a), (_, pool_b)| {
        // 按 LP 供应量降序排序
        pool_b.lp_supply.cmp(&pool_a.lp_supply)
    });

    // 返回 LP 供应量最高的池
    sorted_pools.into_iter().next()
}

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

pub(crate) fn coin_creator_vault_authority(coin_creator: Pubkey) -> Pubkey {
    let (pump_pool_authority, _) = Pubkey::find_program_address(
        &[b"creator_vault", &coin_creator.to_bytes()],
        &accounts::AMM_PROGRAM,
    );
    pump_pool_authority
}

pub(crate) fn coin_creator_vault_ata(coin_creator: Pubkey, quote_mint: Pubkey) -> Pubkey {
    let creator_vault_authority = coin_creator_vault_authority(coin_creator);

    get_associated_token_address_with_program_id(
        &creator_vault_authority,
        &quote_mint,
        &TOKEN_PROGRAM,
    )
}

pub(crate) fn fee_recipient_ata(fee_recipient: Pubkey, quote_mint: Pubkey) -> Pubkey {
    crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
        &fee_recipient,
        &quote_mint,
        &TOKEN_PROGRAM,
    )
}

pub fn get_user_volume_accumulator_pda(user: &Pubkey) -> Option<Pubkey> {
    crate::common::fast_fn::get_cached_pda(
        crate::common::fast_fn::PdaCacheKey::PumpSwapUserVolume(*user),
        || {
            let seeds: &[&[u8]; 2] = &[seeds::USER_VOLUME_ACCUMULATOR_SEED, user.as_ref()];
            let program_id: &Pubkey = &accounts::AMM_PROGRAM;
            let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
            pda.map(|pubkey| pubkey.0)
        },
    )
}

pub fn get_global_volume_accumulator_pda() -> Option<Pubkey> {
    let seeds: &[&[u8]; 1] = &[seeds::GLOBAL_VOLUME_ACCUMULATOR_SEED];
    let program_id: &Pubkey = &accounts::AMM_PROGRAM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

/// 获取指定地址的 Pool（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
pub async fn get_pool_by_address<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<Pool, anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool) = pump_swap_cache::get_cached_pool_by_address(pool_address) {
        return Ok(pool);
    }
    // 2. RPC 查询
    let account = rpc
        .get_account(pool_address)
        .await
        .map_err(|e| anyhow!("RPC 调用失败: {}", e))?;
    if account.owner != accounts::AMM_PROGRAM {
        return Err(anyhow!("Account is not owned by PumpSwap program"));
    }
    let pool = pool_decode(&account.data[8..]).ok_or_else(|| anyhow!("Failed to decode pool"))?;
    // 3. 写入缓存
    pump_swap_cache::cache_pool_by_address(pool_address, &pool);
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
    if let Some(pool_address) = pump_swap_cache::get_cached_pool_address_by_mint(mint)
        && let Some(pool) = pump_swap_cache::get_cached_pool_by_address(&pool_address)
    {
        return Ok((pool_address, pool));
    }
    // 2. RPC 查询
    let (pool_address, pool) = find_pool_by_mint_impl(rpc, mint).await?;
    // 3. 写入缓存
    pump_swap_cache::cache_pool_address_by_mint(mint, &pool_address);
    pump_swap_cache::cache_pool_by_address(&pool_address, &pool);
    Ok((pool_address, pool))
}

/// 强制刷新并重新查询指定 Pool（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
pub async fn get_pool_by_address_force<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<Pool, anyhow::Error> {
    pump_swap_cache::POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

/// 强制刷新并重新查询 mint 对应的 Pool（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
pub async fn get_pool_by_mint_force<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<(Pubkey, Pool), anyhow::Error> {
    pump_swap_cache::MINT_TO_POOL_CACHE.remove(mint);
    get_pool_by_mint(rpc, mint).await
}

/// 清除所有 Pool 缓存
pub fn clear_pool_cache() {
    pump_swap_cache::clear_all();
}

// PumpSwap Pool 缓存（Step 2）
pub(crate) mod pump_swap_cache {
    use super::*;
    use dashmap::DashMap;
    use once_cell::sync::Lazy;

    const MAX_CACHE_SIZE: usize = 50_000;

    /// mint → pool_address 缓存
    pub(crate) static MINT_TO_POOL_CACHE: Lazy<DashMap<Pubkey, Pubkey>> =
        Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

    /// pool_address → Pool 数据缓存
    pub(crate) static POOL_DATA_CACHE: Lazy<DashMap<Pubkey, Pool>> =
        Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

    /// mint → Vec<(pool_address, Pool)> 列表缓存（用于 list_pools_by_mint）
    pub(crate) static MINT_TO_POOLS_LIST_CACHE: Lazy<DashMap<Pubkey, Vec<(Pubkey, Pool)>>> =
        Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

    pub(crate) fn get_cached_pool_by_address(pool_address: &Pubkey) -> Option<Pool> {
        POOL_DATA_CACHE.get(pool_address).map(|p| p.clone())
    }

    pub(crate) fn cache_pool_by_address(pool_address: &Pubkey, pool: &Pool) {
        POOL_DATA_CACHE.insert(*pool_address, pool.clone());
    }

    pub(crate) fn get_cached_pool_address_by_mint(mint: &Pubkey) -> Option<Pubkey> {
        MINT_TO_POOL_CACHE.get(mint).map(|p| *p)
    }

    pub(crate) fn cache_pool_address_by_mint(mint: &Pubkey, pool_address: &Pubkey) {
        MINT_TO_POOL_CACHE.insert(*mint, *pool_address);
    }

    pub(crate) fn get_cached_pools_list_by_mint(mint: &Pubkey) -> Option<Vec<(Pubkey, Pool)>> {
        MINT_TO_POOLS_LIST_CACHE.get(mint).map(|p| p.clone())
    }

    pub(crate) fn cache_pools_list_by_mint(mint: &Pubkey, pools: &[(Pubkey, Pool)]) {
        MINT_TO_POOLS_LIST_CACHE.insert(*mint, pools.to_vec());
    }

    pub(crate) fn clear_all() {
        MINT_TO_POOL_CACHE.clear();
        POOL_DATA_CACHE.clear();
        MINT_TO_POOLS_LIST_CACHE.clear();
    }
}

// 常量偏移量
const BASE_MINT_OFFSET: usize = 43;
const QUOTE_MINT_OFFSET: usize = 75;

/// 通用内部实现：通过 offset 查找所有 Pool（返回 Vec）
#[allow(dead_code)]
/// 通过 offset 查找所有 Pool（支持 Auto Mock）
async fn find_pools_by_mint_offset_collect<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
    offset: usize,
) -> Result<Vec<(Pubkey, Pool)>, anyhow::Error> {
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
                pool_decode(&data_bytes[8..]).map(|pool| (addr_pubkey, pool))
            } else {
                None
            }
        })
        .collect();

    Ok(pools)
}

/// Calculate the canonical PumpSwap pool PDA for a mint that was migrated from PumpFun
///
/// Canonical pools are created by the PumpFun migrate instruction and use:
/// - pool_index = [0, 0] (CANONICAL_POOL_INDEX)
/// - pool_authority = PDA("pool-authority", mint) under PumpFun program
/// - pool = PDA("pool", [0, 0], pool_authority, mint, wsol_mint) under PumpSwap AMM program
pub fn calculate_canonical_pool_pda(mint: &Pubkey) -> Option<(Pubkey, Pubkey)> {
    use crate::constants::WSOL_TOKEN_ACCOUNT;
    use crate::instruction::utils::pumpfun::accounts::PUMPFUN;

    // Calculate pool_authority PDA (seeds: "pool-authority" + mint, program: PumpFun)
    let (pool_authority, _) =
        Pubkey::try_find_program_address(&[b"pool-authority", mint.as_ref()], &PUMPFUN)?;

    // Calculate pool PDA (seeds: "pool" + [0, 0] + pool_authority + mint + wsol_mint, program: PumpSwap AMM)
    let pool_index = [0u8, 0u8];
    let wsol_mint = WSOL_TOKEN_ACCOUNT; // WSOL mint address
    let (pool, _) = Pubkey::try_find_program_address(
        &[b"pool", &pool_index, pool_authority.as_ref(), mint.as_ref(), wsol_mint.as_ref()],
        &accounts::AMM_PROGRAM,
    )?;

    Some((pool, pool_authority))
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
    if base_result.is_err() && quote_result.is_err() {
        // 返回 base_result 的错误，它包含我们的自定义错误消息
        return Err(base_result.unwrap_err());
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
    if let Some(cached_pools) = pump_swap_cache::get_cached_pools_list_by_mint(mint) {
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
    pump_swap_cache::cache_pools_list_by_mint(mint, &sorted_pools);

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

/// Quote an exact-in swap against a PumpSwap pool.
///
/// - If `is_base_in=true`: base -> quote
/// - If `is_base_in=false`: quote -> base
pub async fn quote_exact_in(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_in: u64,
    is_base_in: bool,
) -> Result<crate::utils::quote::QuoteExactInResult, anyhow::Error> {
    let pool = get_pool_by_address(rpc, pool_address).await?;
    let (base_reserve, quote_reserve) = get_token_balances(&pool, rpc).await?;

    if is_base_in {
        // base -> quote
        let r = crate::utils::calc::pumpswap::sell_base_input_internal(
            amount_in,
            0,
            base_reserve,
            quote_reserve,
            &pool.coin_creator,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        // fee in output token space is less helpful; we expose fee in input token units when possible.
        // For base->quote we don't have an input-fee field; return 0 here for now.
        Ok(crate::utils::quote::QuoteExactInResult {
            amount_out: r.ui_quote,
            fee_amount: 0,
            price_impact_bps: None,
            extra_accounts_read: 2, // two token accounts
        })
    } else {
        // quote -> base
        let r = crate::utils::calc::pumpswap::buy_quote_input_internal(
            amount_in,
            0,
            base_reserve,
            quote_reserve,
            &pool.coin_creator,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        // fee in input token units: amount_in - effective_quote (without fees)
        let fee_amount = amount_in.saturating_sub(r.internal_quote_without_fees);
        Ok(crate::utils::quote::QuoteExactInResult {
            amount_out: r.base,
            fee_amount,
            price_impact_bps: None,
            extra_accounts_read: 2,
        })
    }
}

/// 获取任意 Token 在 PumpSwap 上的 USD 价格（通过 X-WSOL 池 + Raydium CLMM WSOL-USD 锚定池）
///
/// 价格计算路径：Token X -> WSOL -> USD
/// - 要求：存在一个 X-WSOL 的 PumpSwap 池，以及一个 Raydium CLMM 上的 WSOL-USDT/USDC 锚定池
pub async fn get_token_price_in_usd(
    rpc: &SolanaRpcClient,
    token_mint: &Pubkey,
    wsol_usd_clmm_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool = wsol_usd_clmm_pool_address.unwrap_or(&DEFAULT_WSOL_USDT_CLMM_POOL);
    use crate::constants::{SOL_MINT, USDC_MINT, USDT_MINT, WSOL_TOKEN_ACCOUNT};
    use crate::utils::price::pumpswap::{price_base_in_quote, price_quote_in_base};

    // 稳定币自身的价格直接认为是 1 USD
    if *token_mint == USDC_MINT || *token_mint == USDT_MINT {
        return Ok(1.0);
    }

    // WSOL/SOL 的价格通过 Raydium CLMM 锚定池获取
    if *token_mint == SOL_MINT || *token_mint == WSOL_TOKEN_ACCOUNT {
        return crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await;
    }

    // 1. 在 PumpSwap 中找到 Token X 的最优池（优先 X-WSOL/USDC/USDT 对）
    let (pool_address, pool_best) = get_pool_by_mint(rpc, token_mint).await?;

    // 2. 为了价格实时性，对选中的池地址强制刷新一次 Pool
    let pool = get_pool_by_address_force(rpc, &pool_address).await.unwrap_or(pool_best);

    // 3. 只处理 X-WSOL 对（X 是任意 token，另一侧必须是 WSOL_TOKEN_ACCOUNT）
    let is_base_x = pool.base_mint == *token_mint && pool.quote_mint == WSOL_TOKEN_ACCOUNT;
    let is_quote_x = pool.quote_mint == *token_mint && pool.base_mint == WSOL_TOKEN_ACCOUNT;

    if !is_base_x && !is_quote_x {
        return Err(anyhow!(
            "Best PumpSwap pool for mint {} is not paired with WSOL; USD pricing via WSOL is not supported yet",
            token_mint
        ));
    }

    // 4. 获取池子实时余额
    let (base_reserve, quote_reserve) = get_token_balances(&pool, rpc).await?;

    // 5. 获取两侧代币精度
    let base_decimals = crate::utils::token::get_token_decimals(rpc, &pool.base_mint).await?;
    let quote_decimals = crate::utils::token::get_token_decimals(rpc, &pool.quote_mint).await?;

    // 6. 计算 X 相对 WSOL 的价格
    let price_x_in_wsol = if is_base_x {
        // base = X, quote = WSOL
        price_base_in_quote(base_reserve, quote_reserve, base_decimals, quote_decimals)
    } else {
        // quote = X, base = WSOL
        price_quote_in_base(base_reserve, quote_reserve, base_decimals, quote_decimals)
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price on PumpSwap is invalid (<= 0)"));
    }

    // 7. 获取 WSOL 的 USD 价格（通过 Raydium CLMM 锚定池）
    let price_wsol_in_usd =
        crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}

/// 获取任意 Token 在 PumpSwap 上的 USD 价格（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
/// 此函数要求调用者已知 X-WSOL 池地址，直接传入，避免 `get_pool_by_mint` 的查找开销。
/// 适用于高频调用、已缓存池地址的场景。
///
/// # Arguments
/// * `rpc` - 实现了 PoolRpcClient 的 RPC 客户端（支持 AutoMockRpcClient 或标准 RpcClient）
/// * `token_mint` - Token X 的 mint 地址
/// * `x_wsol_pool_address` - Token X 与 WSOL 配对的 PumpSwap 池地址
/// * `wsol_usd_clmm_pool_address` - Raydium CLMM 上的 WSOL-USDT/USDC 锚定池地址
pub async fn get_token_price_in_usd_with_pool<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    token_mint: &Pubkey,
    x_wsol_pool_address: &Pubkey,
    wsol_usd_clmm_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool = wsol_usd_clmm_pool_address.unwrap_or(&DEFAULT_WSOL_USDT_CLMM_POOL);
    use crate::constants::{SOL_MINT, USDC_MINT, USDT_MINT, WSOL_TOKEN_ACCOUNT};
    use crate::utils::price::pumpswap::{price_base_in_quote, price_quote_in_base};

    // 稳定币自身的价格直接认为是 1 USD
    if *token_mint == USDC_MINT || *token_mint == USDT_MINT {
        return Ok(1.0);
    }

    // WSOL/SOL 的价格通过 Raydium CLMM 锚定池获取
    if *token_mint == SOL_MINT || *token_mint == WSOL_TOKEN_ACCOUNT {
        return crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await;
    }

    // 1. 直接强制刷新指定的 X-WSOL 池（跳过查找步骤）
    let pool = get_pool_by_address_force(rpc, x_wsol_pool_address).await?;

    // 2. 只处理 X-WSOL 对（X 是任意 token，另一侧必须是 WSOL_TOKEN_ACCOUNT）
    let is_base_x = pool.base_mint == *token_mint && pool.quote_mint == WSOL_TOKEN_ACCOUNT;
    let is_quote_x = pool.quote_mint == *token_mint && pool.base_mint == WSOL_TOKEN_ACCOUNT;

    if !is_base_x && !is_quote_x {
        return Err(anyhow!(
            "Provided PumpSwap pool {} is not paired with WSOL; USD pricing via WSOL is not supported yet",
            x_wsol_pool_address
        ));
    }

    // 3. 获取池子实时余额
    let (base_reserve, quote_reserve) = get_token_balances(&pool, rpc).await?;

    // 4. 获取两侧代币精度
    let base_decimals =
        crate::utils::token::get_token_decimals_with_client(rpc, &pool.base_mint).await?;
    let quote_decimals =
        crate::utils::token::get_token_decimals_with_client(rpc, &pool.quote_mint).await?;

    // 5. 计算 X 相对 WSOL 的价格
    let price_x_in_wsol = if is_base_x {
        // base = X, quote = WSOL
        price_base_in_quote(base_reserve, quote_reserve, base_decimals, quote_decimals)
    } else {
        // quote = X, base = WSOL
        price_quote_in_base(base_reserve, quote_reserve, base_decimals, quote_decimals)
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price on PumpSwap is invalid (<= 0)"));
    }

    // 6. 获取 WSOL 的 USD 价格（通过 Raydium CLMM 锚定池）
    let price_wsol_in_usd =
        crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：USDC/USDT 价格分支（不依赖真实 RPC，只要函数能返回 1.0 即可）
    #[tokio::test]
    async fn test_get_token_price_in_usd_stable_tokens() {
        use crate::constants::{USDC_MINT, USDT_MINT};
        use solana_client::nonblocking::rpc_client::RpcClient;

        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        let dummy_anchor_pool = Pubkey::new_unique();

        let usdc_price = get_token_price_in_usd(&rpc, &USDC_MINT, Some(&dummy_anchor_pool))
            .await
            .unwrap();
        let usdt_price = get_token_price_in_usd(&rpc, &USDT_MINT, Some(&dummy_anchor_pool))
            .await
            .unwrap();

        assert_eq!(usdc_price, 1.0);
        assert_eq!(usdt_price, 1.0);
    }
}

#[inline]
pub fn get_fee_config_pda() -> Option<Pubkey> {
    let seeds: &[&[u8]; 2] = &[seeds::FEE_CONFIG_SEED, accounts::AMM_PROGRAM.as_ref()];
    let program_id: &Pubkey = &accounts::FEE_PROGRAM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}
