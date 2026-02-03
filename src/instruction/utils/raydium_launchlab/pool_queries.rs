use super::constants::accounts;
use super::helpers::{get_global_config_pda, get_platform_config_pda};
use super::parsing::{parse_global_config, parse_platform_config, parse_pool_state};
use super::types::{GlobalConfig, LaunchLabPoolState, PlatformConfig};
use crate::common::{SolanaRpcClient, bonding_curve::BondingCurveAccount};
use solana_sdk::{pubkey, pubkey::Pubkey};
use std::sync::Arc;

/// Fetch and parse PlatformConfig from RPC
pub async fn fetch_platform_config(
    rpc: &SolanaRpcClient,
    platform_config_address: &Pubkey,
) -> Result<PlatformConfig, anyhow::Error> {
    let account = rpc.get_account(platform_config_address).await?;
    parse_platform_config(&account.data)
}

/// Fetch and parse GlobalConfig from RPC
pub async fn fetch_global_config(
    rpc: &SolanaRpcClient,
    global_config_address: &Pubkey,
) -> Result<GlobalConfig, anyhow::Error> {
    let account = rpc.get_account(global_config_address).await?;
    parse_global_config(&account.data)
}

/// Fetch and parse the bonding curve account for Raydium LaunchLab
pub async fn fetch_bonding_curve_account(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Arc<BondingCurveAccount>, Pubkey), anyhow::Error> {
    let (bonding_curve_pda, _bump) = super::helpers::get_bonding_curve_pda(mint)?;

    let account = rpc.get_account(&bonding_curve_pda).await?;

    // Parse using Borsh deserialization for Raydium LaunchLab PoolState
    // Skip the 8-byte discriminator
    if account.data.len() < 8 {
        return Err(anyhow::anyhow!("Invalid account data: too short"));
    }

    let pool_state: LaunchLabPoolState = parse_pool_state(&account.data)?;

    // Map Raydium LaunchLab PoolState to BondingCurveAccount
    // status: 0 = active, 1 = migrate (complete)
    let complete = pool_state.status == 1;

    let bonding_curve = Arc::new(BondingCurveAccount {
        discriminator: 0,
        account: bonding_curve_pda,
        virtual_token_reserves: pool_state.virtual_base,
        virtual_sol_reserves: pool_state.virtual_quote,
        real_sol_reserves: pool_state.real_quote,
        real_token_reserves: pool_state.real_base,
        token_total_supply: pool_state.supply,
        complete,
        creator: pool_state.creator,
        is_mayhem_mode: false, // Raydium LaunchLab doesn't use mayhem mode
    });

    Ok((bonding_curve, bonding_curve_pda))
}

/// Try to find global_config by querying common configurations
/// This is a helper function that tries common curve_type and index values
pub async fn find_global_config(
    rpc: &SolanaRpcClient,
    quote_mint: &Pubkey,
) -> Result<Pubkey, anyhow::Error> {
    use crate::constants::WSOL_TOKEN_ACCOUNT;

    // Try common configurations: curve_type=0 (ConstantProduct), index=0
    let (config_pda, _) = get_global_config_pda(quote_mint, 0, 0)?;

    // Try to fetch the account to verify it exists
    match rpc.get_account(&config_pda).await {
        Ok(_) => Ok(config_pda),
        Err(_) => {
            // Try index=1
            let (config_pda, _) = get_global_config_pda(quote_mint, 0, 1)?;
            match rpc.get_account(&config_pda).await {
                Ok(_) => Ok(config_pda),
                Err(_) => {
                    // If quote_mint is USD1, try known USD1 global config
                    // USD1 mint: USD1ttGY1N17NEEHLmELoaybftRBUSErhqYiQzvEmuB
                    let usd1_mint = pubkey!("USD1ttGY1N17NEEHLmELoaybftRBUSErhqYiQzvEmuB");
                    if quote_mint == &usd1_mint
                        && let Ok(_) = rpc.get_account(&accounts::USD1_GLOBAL_CONFIG).await
                    {
                        println!(
                            "   ℹ️  使用已知的 USD1 global_config: {}",
                            accounts::USD1_GLOBAL_CONFIG
                        );
                        return Ok(accounts::USD1_GLOBAL_CONFIG);
                    }

                    // If quote_mint is WSOL, we could add known WSOL global config here if available
                    if quote_mint == &WSOL_TOKEN_ACCOUNT {
                        // TODO: Add known WSOL global config if available
                    }

                    Err(anyhow::anyhow!(
                        "Could not find global_config. Please provide it explicitly. Tried PDA derivation and known addresses."
                    ))
                },
            }
        },
    }
}

/// Try to find platform_config by querying with payer as platform_admin
/// In many cases, the payer is also the platform_admin
/// This function tries multiple approaches:
/// 1. Derive PDA from platform_admin
/// 2. Try known platform_config addresses (e.g., LetsBonk.fun)
pub async fn find_platform_config(
    rpc: &SolanaRpcClient,
    platform_admin: &Pubkey,
) -> Result<Pubkey, anyhow::Error> {
    // First, try to derive PDA from platform_admin
    let (config_pda, _) = get_platform_config_pda(platform_admin)?;

    // Try to fetch the account to verify it exists
    if rpc.get_account(&config_pda).await.is_ok() {
        return Ok(config_pda);
    }

    // If not found, try known platform_config addresses
    let known_configs = vec![accounts::LETSBONK_PLATFORM_CONFIG];

    for known_config in known_configs {
        if rpc.get_account(&known_config).await.is_ok() {
            println!("   ℹ️  使用已知的 platform_config: {}", known_config);
            return Ok(known_config);
        }
    }

    Err(anyhow::anyhow!(
        "Could not find platform_config. Please provide it explicitly. Tried: {} and known addresses",
        config_pda
    ))
}
