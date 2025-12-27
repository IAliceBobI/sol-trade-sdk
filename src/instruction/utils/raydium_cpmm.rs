use crate::{
    common::SolanaRpcClient,
    instruction::utils::raydium_cpmm_types::{PoolState, pool_state_decode, POOL_STATE_SIZE},
    trading::core::params::RaydiumCpmmParams,
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

pub async fn fetch_pool_state(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    let account = rpc.get_account(pool_address).await?;
    if account.owner != accounts::RAYDIUM_CPMM {
        return Err(anyhow!("Account is not owned by Raydium Cpmm program"));
    }
    let pool_state = pool_state_decode(&account.data[8..])
        .ok_or_else(|| anyhow!("Failed to decode pool state"))?;
    Ok(pool_state)
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
    let token0_balance = rpc.get_token_account_balance(&token0_vault).await?;
    let token1_vault = get_vault_pda(pool_state, token1_mint).unwrap();
    let token1_balance = rpc.get_token_account_balance(&token1_vault).await?;

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
    let pool_state = fetch_pool_state(rpc, pool_address).await?;
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

/// Find CPMM pool by mint address using getProgramAccounts
/// This searches for pools containing the given mint as either token0 or token1
pub async fn find_pool_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    use solana_account_decoder::UiAccountEncoding;
    use solana_rpc_client_api::{
        config::RpcProgramAccountsConfig,
        filter::RpcFilterType,
    };
    use solana_client::rpc_filter::Memcmp;

    // Try to find pool by token0_mint (offset 40 in PoolState after discriminator)
    // PoolState structure: discriminator(8) + amm_config(32) + pool_creator(32) + ... + token0_mint(32) at offset 40
    let filters_token0 = vec![
        RpcFilterType::DataSize(POOL_STATE_SIZE as u64),
        RpcFilterType::Memcmp(
            Memcmp::new_base58_encoded(40, &mint.to_bytes()),
        ),
    ];

    let config = RpcProgramAccountsConfig {
        filters: Some(filters_token0),
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
    
    if !accounts.is_empty() {
        for (addr, acc) in accounts {
            let data_bytes = match &acc.data {
                UiAccountData::Binary(base64_str, _) => {
                    match STANDARD.decode(base64_str) {
                        Ok(bytes) => bytes,
                        Err(_) => continue,
                    }
                }
                _ => continue,
            };
            if data_bytes.len() > 8 {
                if let Some(pool_state) = pool_state_decode(&data_bytes[8..]) {
                    // Verify it's the correct mint
                    if pool_state.token0_mint == *mint || pool_state.token1_mint == *mint {
                        return Ok((addr, pool_state));
                    }
                }
            }
        }
    }

    // Try to find pool by token1_mint (offset 72 in PoolState)
    let filters_token1 = vec![
        RpcFilterType::DataSize(POOL_STATE_SIZE as u64),
        RpcFilterType::Memcmp(
            Memcmp::new_base58_encoded(72, &mint.to_bytes()),
        ),
    ];

    let config = RpcProgramAccountsConfig {
        filters: Some(filters_token1),
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
    
    if !accounts.is_empty() {
        for (addr, acc) in accounts {
            let data_bytes = match &acc.data {
                UiAccountData::Binary(base64_str, _) => {
                    match STANDARD.decode(base64_str) {
                        Ok(bytes) => bytes,
                        Err(_) => continue,
                    }
                }
                _ => continue,
            };
            if data_bytes.len() > 8 {
                if let Some(pool_state) = pool_state_decode(&data_bytes[8..]) {
                    // Verify it's the correct mint
                    if pool_state.token0_mint == *mint || pool_state.token1_mint == *mint {
                        return Ok((addr, pool_state));
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!("No CPMM pool found for mint {}", mint))
}

/// List all CPMM pools that contain the given mint as token0 or token1.
///
/// This is a discovery helper for routing/selection layers. It does NOT pick a best pool.

pub async fn list_pools_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    use solana_account_decoder::UiAccountEncoding;
    use solana_rpc_client_api::{config::RpcProgramAccountsConfig, filter::RpcFilterType};
    use solana_client::rpc_filter::Memcmp;
    use std::collections::HashSet;

    let mut out: Vec<(Pubkey, PoolState)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // token0_mint offset 40
    let filters_token0 = vec![
        RpcFilterType::DataSize(POOL_STATE_SIZE as u64),
        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(40, &mint.to_bytes())),
    ];
    let config = RpcProgramAccountsConfig {
        filters: Some(filters_token0),
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
        .await?;
    for (addr, acc) in accounts {
        let data_bytes = match &acc.data {
            UiAccountData::Binary(base64_str, _) => {
                match STANDARD.decode(base64_str) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                }
            }
            _ => continue,
        };
        if data_bytes.len() > 8 {
            if let Some(pool_state) = pool_state_decode(&data_bytes[8..]) {
                if pool_state.token0_mint == *mint || pool_state.token1_mint == *mint {
                    let k = addr.to_string();
                    if seen.insert(k) {
                        out.push((addr, pool_state));
                    }
                }
            }
        }
    }

    // token1_mint offset 72
    let filters_token1 = vec![
        RpcFilterType::DataSize(POOL_STATE_SIZE as u64),
        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(72, &mint.to_bytes())),
    ];
    let config = RpcProgramAccountsConfig {
        filters: Some(filters_token1),
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
        .await?;
    for (addr, acc) in accounts {
        let data_bytes = match &acc.data {
            UiAccountData::Binary(base64_str, _) => {
                match STANDARD.decode(base64_str) {
                    Ok(bytes) => bytes,
                    Err(_) => continue,
                }
            }
            _ => continue,
        };
        if data_bytes.len() > 8 {
            if let Some(pool_state) = pool_state_decode(&data_bytes[8..]) {
                if pool_state.token0_mint == *mint || pool_state.token1_mint == *mint {
                    let k = addr.to_string();
                    if seen.insert(k) {
                        out.push((addr, pool_state));
                    }
                }
            }
        }
    }

    if out.is_empty() {
        return Err(anyhow::anyhow!("No CPMM pool found for mint {}", mint));
    }
    Ok(out)
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