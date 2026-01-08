use crate::{
    common::{
        SolanaRpcClient, spl_associated_token_account::get_associated_token_address_with_program_id,
    },
    constants::TOKEN_PROGRAM,
    instruction::utils::pumpswap_types::{Pool, pool_decode},
};
use anyhow::anyhow;
use solana_account_decoder::{UiAccountEncoding, UiAccountData};
use solana_sdk::pubkey::Pubkey;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;

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
pub const SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];

// Find a pool for a specific mint
pub async fn find_pool(rpc: &SolanaRpcClient, mint: &Pubkey) -> Result<Pubkey, anyhow::Error> {
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

pub async fn get_pool_by_address(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<Pool, anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool) = pump_swap_cache::get_cached_pool_by_address(pool_address) {
        return Ok(pool);
    }
    // 2. RPC 查询
    let account = rpc.get_account(pool_address).await?;
    if account.owner != accounts::AMM_PROGRAM {
        return Err(anyhow!("Account is not owned by PumpSwap program"));
    }
    let pool = pool_decode(&account.data[8..]).ok_or_else(|| anyhow!("Failed to decode pool"))?;
    // 3. 写入缓存
    pump_swap_cache::cache_pool_by_address(pool_address, &pool);
    Ok(pool)
}

/// 带缓存的 mint 查询（返回最优池）
pub async fn get_pool_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, Pool), anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool_address) = pump_swap_cache::get_cached_pool_address_by_mint(mint) {
        if let Some(pool) = pump_swap_cache::get_cached_pool_by_address(&pool_address) {
            return Ok((pool_address, pool));
        }
    }
    // 2. RPC 查询
    let (pool_address, pool) = find_pool_by_mint_impl(rpc, mint).await?;
    // 3. 写入缓存
    pump_swap_cache::cache_pool_address_by_mint(mint, &pool_address);
    pump_swap_cache::cache_pool_by_address(&pool_address, &pool);
    Ok((pool_address, pool))
}

/// Force 刷新：强制重新查询指定 Pool
pub async fn get_pool_by_address_force(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<Pool, anyhow::Error> {
    pump_swap_cache::POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

/// Force 刷新：强制重新查询 mint 对应的 Pool
pub async fn get_pool_by_mint_force(
    rpc: &SolanaRpcClient,
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

    pub(crate) fn clear_all() {
        MINT_TO_POOL_CACHE.clear();
        POOL_DATA_CACHE.clear();
    }
}

// 常量偏移量
const BASE_MINT_OFFSET: usize = 43;
const QUOTE_MINT_OFFSET: usize = 75;

/// 通用内部实现：通过 offset 查找所有 Pool（返回 Vec）
async fn find_pools_by_mint_offset_collect(
    rpc: &SolanaRpcClient,
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
    let accounts = rpc.get_program_ui_accounts_with_config(&program_id, config).await?;

    let pools: Vec<(Pubkey, Pool)> = accounts
        .into_iter()
        .filter_map(|(addr, acc)| {
            let data_bytes = match &acc.data {
                UiAccountData::Binary(base64_str, _) => STANDARD.decode(base64_str).ok()?,
                _ => return None,
            };
            if data_bytes.len() > 8 {
                pool_decode(&data_bytes[8..]).map(|pool| (addr, pool))
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

/// 内部实现：查找 mint 对应的最优池
async fn find_pool_by_mint_impl(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, Pool), anyhow::Error> {
    use crate::constants::WSOL_TOKEN_ACCOUNT;

    // Priority 1: Try to find canonical pool (mint/WSOL pair) first
    // This is the most common case for PumpFun migrated tokens
    if let Some((pool_address, _)) = calculate_canonical_pool_pda(mint) {
        if let Ok(pool) = get_pool_by_address(rpc, &pool_address).await {
            // Verify it's actually a mint/WSOL pool
            if (pool.base_mint == *mint && pool.quote_mint == WSOL_TOKEN_ACCOUNT) ||
               (pool.base_mint == WSOL_TOKEN_ACCOUNT && pool.quote_mint == *mint) {
                return Ok((pool_address, pool));
            }
        }
    }

    // Priority 2 & 3: 并行扫描 base_mint 和 quote_mint pools
    let (base_result, quote_result) = tokio::join!(
        find_pools_by_mint_offset_collect(rpc, mint, BASE_MINT_OFFSET),
        find_pools_by_mint_offset_collect(rpc, mint, QUOTE_MINT_OFFSET)
    );

    let mut all_pools: Vec<(Pubkey, Pool)> = Vec::new();

    match base_result {
        Ok(pools) => all_pools.extend(pools),
        Err(_) => {}
    }

    match quote_result {
        Ok(quote_pools) => {
            // Merge and deduplicate
            use std::collections::HashSet;
            let mut seen: HashSet<Pubkey> = all_pools.iter().map(|(addr, _)| *addr).collect();
            for (addr, pool) in quote_pools {
                if seen.insert(addr) {
                    all_pools.push((addr, pool));
                }
            }
        }
        Err(_) => {}
    }

    if all_pools.is_empty() {
        return Err(anyhow!("No pool found for mint {}", mint));
    }

    // Priority: Prefer WSOL pairs
    let mut wsol_pools: Vec<_> = all_pools
        .iter()
        .filter(|(_, pool)| {
            pool.base_mint == WSOL_TOKEN_ACCOUNT || pool.quote_mint == WSOL_TOKEN_ACCOUNT
        })
        .collect();

    if !wsol_pools.is_empty() {
        // Sort by LP supply (highest first) and return the first one
        wsol_pools.sort_by(|a, b| b.1.lp_supply.cmp(&a.1.lp_supply));
        let (address, pool) = wsol_pools[0];
        return Ok((*address, pool.clone()));
    }

    // If no WSOL pair found, return the pool with highest LP supply
    all_pools.sort_by(|a, b| b.1.lp_supply.cmp(&a.1.lp_supply));
    let (address, pool) = all_pools[0].clone();
    Ok((address, pool))
}

/// List all PumpSwap pools for a mint (as base or quote).
///
/// This is a discovery helper for routing/selection layers. It does NOT pick a best pool.

pub async fn list_pools_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, Pool)>, anyhow::Error> {
    use std::collections::HashSet;

    let mut out: Vec<(Pubkey, Pool)> = Vec::new();
    let mut seen: HashSet<Pubkey> = HashSet::new();

    // Scan base_mint pools
    if let Ok(base_pools) = find_pools_by_mint_offset_collect(rpc, mint, BASE_MINT_OFFSET).await {
        for (addr, pool) in base_pools {
            if seen.insert(addr) {
                out.push((addr, pool));
            }
        }
    }

    // Scan quote_mint pools and merge
    if let Ok(quote_pools) = find_pools_by_mint_offset_collect(rpc, mint, QUOTE_MINT_OFFSET).await {
        for (addr, pool) in quote_pools {
            if seen.insert(addr) {
                out.push((addr, pool));
            }
        }
    }

    if out.is_empty() {
        return Err(anyhow!("No pool found for mint {}", mint));
    }
    Ok(out)
}

pub async fn get_token_balances(
    pool: &Pool,
    rpc: &SolanaRpcClient,
) -> Result<(u64, u64), anyhow::Error> {
    let (base_balance_result, quote_balance_result) = tokio::join!(
        rpc.get_token_account_balance(&pool.pool_base_token_account),
        rpc.get_token_account_balance(&pool.pool_quote_token_account),
    );

    let base_balance = base_balance_result?;
    let quote_balance = quote_balance_result?;

    let base_amount = base_balance.amount.parse::<u64>().map_err(|e| anyhow!(e))?;
    let quote_amount = quote_balance.amount.parse::<u64>().map_err(|e| anyhow!(e))?;

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

#[inline]
pub fn get_fee_config_pda() -> Option<Pubkey> {
    let seeds: &[&[u8]; 2] = &[seeds::FEE_CONFIG_SEED, accounts::AMM_PROGRAM.as_ref()];
    let program_id: &Pubkey = &accounts::FEE_PROGRAM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}
