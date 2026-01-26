use crate::common::{SolanaRpcClient, bonding_curve::BondingCurveAccount};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use solana_account_decoder::UiAccountData;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey,
    pubkey::Pubkey,
};
use std::sync::Arc;

/// Constants used as seeds for deriving PDAs (Program Derived Addresses)
pub mod seeds {
    /// Seed for bonding curve PDAs (pool_state)
    pub const POOL_SEED: &[u8] = b"pool";

    /// Seed for vault authority PDAs
    pub const VAULT_AUTH_SEED: &[u8] = b"vault_auth_seed";

    /// Seed for pool vault PDAs
    pub const POOL_VAULT_SEED: &[u8] = b"pool_vault";

    /// Seed for event authority PDAs
    pub const EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";

    /// Seed for platform config PDAs
    pub const PLATFORM_CONFIG_SEED: &[u8] = b"platform_config";
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};

    /// Raydium LaunchLab program ID (mainnet)
    pub const LAUNCHLAB_PROGRAM: Pubkey = pubkey!("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj");

    /// Raydium CPMM program ID (mainnet) - used for external pool after migration
    pub const CPMM_PROGRAM: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");

    /// Raydium CPMM program ID (devnet)
    pub const CPMM_PROGRAM_DEVNET: Pubkey = pubkey!("DRaycpLY18LhpbydsBWbVJtxpNv9oXPgjRSfpF2bWpYb");

    /// Metaplex Token Metadata program
    pub const METADATA_PROGRAM: Pubkey = pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

    /// System Program
    pub const SYSTEM_PROGRAM: Pubkey = pubkey!("11111111111111111111111111111111");

    /// Rent Sysvar
    pub const RENT_SYSVAR: Pubkey = pubkey!("SysvarRent111111111111111111111111111111111");

    /// Associated Token Program
    pub const ASSOCIATED_TOKEN_PROGRAM: Pubkey =
        pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

    /// CPMM Create Pool Fee account (mainnet)
    /// This is the account that receives fees when creating CPMM pools
    pub const CPMM_CREATE_POOL_FEE: Pubkey =
        pubkey!("3oE58BKVt8KuYkGxx8zBojugnymWmBiyafWgMrnb6eYy");

    /// Known platform_config addresses (mainnet)
    /// LetsBonk.fun platform config
    pub const LETSBONK_PLATFORM_CONFIG: Pubkey =
        pubkey!("FfYek5vEz23cMkWsdJwG2oa6EphsvXSHrGpdALN4g6W1");

    /// CPMM Config address (devnet)
    /// From Raydium LaunchLab documentation
    pub const CPMM_CONFIG_DEVNET: Pubkey = pubkey!("EsTevfacYXpuho5VBuzBjDZi8dtWidGnXoSYAr8krTvz");

    /// CPMM Config address (mainnet)
    /// Found from Solscan and GitHub: https://github.com/raydium-io/raydium-cpi-example
    pub const CPMM_CONFIG_MAINNET: Pubkey = pubkey!("D4FPEruKEHrG5TenZ2mpDGEfu1iUvTiqBxvpU8HLBvC2");

    /// CPMM Authority address (mainnet)
    /// Known authority address for CPMM program operations
    /// From: docs/raydium/raydium-addresses-reference.md
    pub const CPMM_AUTHORITY: Pubkey = pubkey!("GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL");

    /// Lock Program address
    /// Used for LP token locking in migrate_to_cpswap
    pub const LOCK_PROGRAM: Pubkey = pubkey!("LockrWmn6K5twhz3y9w1dQERbmgSaRkfnTeTKbpofwE");

    /// Raydium Launchpad Authority (mainnet)
    /// Known authority address for LaunchLab vault operations
    /// From actual transaction analysis
    pub const LAUNCHPAD_AUTHORITY: Pubkey = pubkey!("WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh");

    /// Event Authority (mainnet)
    /// Known event authority PDA for LaunchLab events
    /// From actual transaction analysis
    pub const EVENT_AUTHORITY: Pubkey = pubkey!("2DPAtwB8L12vrMRExbLuyGnC7n2J5LNoZQSejeQGpwkr");

    /// Global Config for USD1 quote token (mainnet)
    /// Known global config address when using USD1 as quote token
    /// From actual transaction analysis: EPiZbnrThjyLnoQ6QQzkxeFqyL5uyg9RzNHHAudUPxBz
    pub const USD1_GLOBAL_CONFIG: Pubkey = pubkey!("EPiZbnrThjyLnoQ6QQzkxeFqyL5uyg9RzNHHAudUPxBz");

    /// Known migrate_to_cpswap_wallet address (mainnet)
    /// This is the wallet that must be used as payer for migrate_to_cpswap instruction
    /// From transaction: 4NkRLPVhpr2EB9mxVtf2sP7Ftn1BfxBTPw6HgK1pkPeLNbnGtSVZdVtecVJwozEgKdM6C9TAT1S1LBRmQWaovJ1a
    pub const MIGRATE_TO_CPSWAP_WALLET: Pubkey =
        pubkey!("RAYpQbFNq9i3mu6cKpTKKRwwHFDeK5AuZz8xvxUrCgw");

    /// Known lock_lp_vault address (mainnet)
    /// Used for locking LP tokens during migration
    /// From transaction: 4NkRLPVhpr2EB9mxVtf2sP7Ftn1BfxBTPw6HgK1pkPeLNbnGtSVZdVtecVJwozEgKdM6C9TAT1S1LBRmQWaovJ1a
    /// Note: This might be a PDA or fixed address. If it's a PDA, we may need to calculate it dynamically.
    pub const LOCK_LP_VAULT: Pubkey = pubkey!("B26Asj7NX4pKnx7s3jrW6CuxaRYWq8HroceTRyoxTE7b");
}

/// Calculate the bonding curve PDA for a given mint
/// Note: In Raydium LaunchLab, the pool_state PDA uses seeds: ["pool", base_mint, quote_mint]
/// This function is kept for compatibility but should use get_pool_state_pda instead
pub fn get_bonding_curve_pda(mint: &Pubkey) -> Result<(Pubkey, u8), anyhow::Error> {
    // For Raydium LaunchLab, we need both base_mint and quote_mint to get pool_state
    // This is a simplified version that assumes quote_mint is WSOL
    use crate::constants::WSOL_TOKEN_ACCOUNT;
    get_pool_state_pda(mint, &WSOL_TOKEN_ACCOUNT)
}

// Raydium LaunchLab PoolState structure (matching solana-streamer)
#[derive(Clone, Debug, Default, borsh::BorshDeserialize)]
struct LaunchLabPoolState {
    #[allow(dead_code)]
    pub epoch: u64,
    #[allow(dead_code)]
    pub auth_bump: u8,
    pub status: u8,
    #[allow(dead_code)]
    pub base_decimals: u8,
    #[allow(dead_code)]
    pub quote_decimals: u8,
    #[allow(dead_code)]
    pub migrate_type: u8,
    pub supply: u64,
    #[allow(dead_code)]
    pub total_base_sell: u64,
    pub virtual_base: u64,  // virtual_token_reserves
    pub virtual_quote: u64, // virtual_sol_reserves
    pub real_base: u64,     // real_token_reserves
    pub real_quote: u64,    // real_sol_reserves
    #[allow(dead_code)]
    pub total_quote_fund_raising: u64,
    #[allow(dead_code)]
    pub quote_protocol_fee: u64,
    #[allow(dead_code)]
    pub platform_fee: u64,
    #[allow(dead_code)]
    pub migrate_fee: u64,
    #[allow(dead_code)]
    pub vesting_schedule: LaunchLabVestingSchedule,
    #[allow(dead_code)]
    pub global_config: Pubkey,
    #[allow(dead_code)]
    pub platform_config: Pubkey,
    #[allow(dead_code)]
    pub base_mint: Pubkey,
    #[allow(dead_code)]
    pub quote_mint: Pubkey,
    #[allow(dead_code)]
    pub base_vault: Pubkey,
    #[allow(dead_code)]
    pub quote_vault: Pubkey,
    pub creator: Pubkey,
    #[allow(dead_code)]
    pub padding: [u64; 8],
}

#[derive(Clone, Debug, Default, borsh::BorshDeserialize)]
struct LaunchLabVestingSchedule {
    #[allow(dead_code)]
    pub total_locked_amount: u64,
    #[allow(dead_code)]
    pub cliff_period: u64,
    #[allow(dead_code)]
    pub unlock_period: u64,
    #[allow(dead_code)]
    pub start_time: u64,
    #[allow(dead_code)]
    pub allocated_share_amount: u64,
}

/// MigrateNftInfo structure for PlatformConfig
#[derive(Clone, Debug, Default, borsh::BorshDeserialize)]
pub struct MigrateNftInfo {
    #[allow(dead_code)]
    pub platform_scale: u64,
    #[allow(dead_code)]
    pub creator_scale: u64,
    #[allow(dead_code)]
    pub burn_scale: u64,
}

/// PlatformConfig structure for Raydium LaunchLab
/// Based on SDK layout.ts:
#[derive(Clone, Debug, Default)]
pub struct PlatformConfig {
    pub epoch: u64,
    pub fee_wallet: Pubkey, // platformClaimFeeWallet
    pub nft_wallet: Pubkey, // platformLockNftWallet
    pub migrate_nft_info: MigrateNftInfo,
    pub fee_rate: u64,
    pub name: String,
    pub web: String,
    pub img: String,
    pub cp_config_id: Pubkey,
    pub creator_fee_rate: u64,
    pub transfer_fee_extension_auth: Pubkey,
}

/// Parse PlatformConfig from account data
/// Structure based on SDK layout.ts - uses fixed-size byte arrays for name/web/img
pub fn parse_platform_config(account_data: &[u8]) -> Result<PlatformConfig, anyhow::Error> {
    // Minimum size: 8 (discriminator) + 8 (epoch) + 32 (fee_wallet) + 32 (nft_wallet) +
    //              24 (migrate_nft_info) + 8 (fee_rate) + 64 (name) + 256 (web) + 256 (img) +
    //              32 (cpConfigId) + 8 (creator_fee_rate) + 32 (transfer_fee_extension_auth) = 760 bytes
    const MIN_SIZE: usize = 8 + 8 + 32 + 32 + 24 + 8 + 64 + 256 + 256 + 32 + 8 + 32;

    if account_data.len() < MIN_SIZE {
        return Err(anyhow::anyhow!(
            "Account data too short: expected at least {} bytes, got {}",
            MIN_SIZE,
            account_data.len()
        ));
    }

    let mut offset = 8; // Skip discriminator

    // Read epoch (8 bytes)
    let epoch = u64::from_le_bytes(
        account_data[offset..offset + 8]
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to parse epoch: {}", e))?,
    );
    offset += 8;

    // Read fee_wallet (32 bytes)
    let fee_wallet = Pubkey::try_from(&account_data[offset..offset + 32])
        .map_err(|e| anyhow::anyhow!("Failed to parse fee_wallet: {}", e))?;
    offset += 32;

    // Read nft_wallet (32 bytes)
    let nft_wallet = Pubkey::try_from(&account_data[offset..offset + 32])
        .map_err(|e| anyhow::anyhow!("Failed to parse nft_wallet: {}", e))?;
    offset += 32;

    // Read migrate_nft_info (24 bytes: 3 * u64)
    let platform_scale = u64::from_le_bytes(
        account_data[offset..offset + 8]
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to parse platform_scale: {}", e))?,
    );
    offset += 8;
    let creator_scale = u64::from_le_bytes(
        account_data[offset..offset + 8]
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to parse creator_scale: {}", e))?,
    );
    offset += 8;
    let burn_scale = u64::from_le_bytes(
        account_data[offset..offset + 8]
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to parse burn_scale: {}", e))?,
    );
    offset += 8;

    // Read fee_rate (8 bytes)
    let fee_rate = u64::from_le_bytes(
        account_data[offset..offset + 8]
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to parse fee_rate: {}", e))?,
    );
    offset += 8;

    // Read name (fixed 64 bytes) - trim null bytes
    let name_bytes = &account_data[offset..offset + 64];
    let name = String::from_utf8_lossy(name_bytes).trim_end_matches('\0').to_string();
    offset += 64;

    // Read web (fixed 256 bytes) - trim null bytes
    let web_bytes = &account_data[offset..offset + 256];
    let web = String::from_utf8_lossy(web_bytes).trim_end_matches('\0').to_string();
    offset += 256;

    // Read img (fixed 256 bytes) - trim null bytes
    let img_bytes = &account_data[offset..offset + 256];
    let img = String::from_utf8_lossy(img_bytes).trim_end_matches('\0').to_string();
    offset += 256;

    // Read cpConfigId (32 bytes)
    let cp_config_id = Pubkey::try_from(&account_data[offset..offset + 32])
        .map_err(|e| anyhow::anyhow!("Failed to parse cp_config_id: {}", e))?;
    offset += 32;

    // Read creator_fee_rate (8 bytes)
    let creator_fee_rate = u64::from_le_bytes(
        account_data[offset..offset + 8]
            .try_into()
            .map_err(|e| anyhow::anyhow!("Failed to parse creator_fee_rate: {}", e))?,
    );
    offset += 8;

    // Read transfer_fee_extension_auth (32 bytes)
    let transfer_fee_extension_auth = Pubkey::try_from(&account_data[offset..offset + 32])
        .map_err(|e| anyhow::anyhow!("Failed to parse transfer_fee_extension_auth: {}", e))?;

    Ok(PlatformConfig {
        epoch,
        fee_wallet,
        nft_wallet,
        migrate_nft_info: MigrateNftInfo { platform_scale, creator_scale, burn_scale },
        fee_rate,
        name,
        web,
        img,
        cp_config_id,
        creator_fee_rate,
        transfer_fee_extension_auth,
    })
}

/// Fetch and parse PlatformConfig from RPC
pub async fn fetch_platform_config(
    rpc: &SolanaRpcClient,
    platform_config_address: &Pubkey,
) -> Result<PlatformConfig, anyhow::Error> {
    let account = rpc.get_account(platform_config_address).await?;
    parse_platform_config(&account.data)
}

/// GlobalConfig structure for Raydium LaunchLab
/// This matches the structure used in solana-streamer
#[derive(Clone, Debug, Default, borsh::BorshDeserialize)]
pub struct GlobalConfig {
    pub epoch: u64,
    pub curve_type: u8,
    pub index: u16,
    pub migrate_fee: u64,
    pub trade_fee_rate: u64,
    pub max_share_fee_rate: u64,
    pub min_base_supply: u64,
    pub max_lock_rate: u64,
    pub min_base_sell_rate: u64,
    pub min_base_migrate_rate: u64,
    pub min_quote_fund_raising: u64,
    pub quote_mint: Pubkey,
    pub protocol_fee_owner: Pubkey,
    pub migrate_fee_owner: Pubkey,
    pub migrate_to_amm_wallet: Pubkey,
    pub migrate_to_cpswap_wallet: Pubkey,
    pub padding: [u64; 16],
}

/// Size of GlobalConfig account data (excluding discriminator)
pub const GLOBAL_CONFIG_SIZE: usize = 8 + 1 + 2 + 8 * 8 + 32 * 5 + 8 * 16;

/// Parse GlobalConfig from account data
/// The account data should start with the discriminator (8 bytes), followed by the GlobalConfig data
pub fn parse_global_config(account_data: &[u8]) -> Result<GlobalConfig, anyhow::Error> {
    if account_data.len() < 8 + GLOBAL_CONFIG_SIZE {
        return Err(anyhow::anyhow!(
            "Account data too short: expected at least {} bytes, got {}",
            8 + GLOBAL_CONFIG_SIZE,
            account_data.len()
        ));
    }

    // Skip discriminator (first 8 bytes) and parse the rest
    let config_data = &account_data[8..8 + GLOBAL_CONFIG_SIZE];
    borsh::from_slice::<GlobalConfig>(config_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse GlobalConfig: {}", e))
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
    let (bonding_curve_pda, _bump) = get_bonding_curve_pda(mint)?;

    let account = rpc.get_account(&bonding_curve_pda).await?;

    // Parse using Borsh deserialization for Raydium LaunchLab PoolState
    // Skip the 8-byte discriminator
    if account.data.len() < 8 {
        return Err(anyhow::anyhow!("Invalid account data: too short"));
    }

    let pool_state_data = &account.data[8..];
    let pool_state: LaunchLabPoolState =
        borsh::BorshDeserialize::try_from_slice(pool_state_data)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize PoolState: {}", e))?;

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

/// Instruction discriminators from IDL
pub mod discriminators {
    /// buy_exact_in discriminator: [250, 234, 13, 123, 213, 156, 19, 236]
    pub const BUY_EXACT_IN: [u8; 8] = [250, 234, 13, 123, 213, 156, 19, 236];

    /// sell_exact_in discriminator: [149, 39, 222, 155, 211, 124, 152, 26]
    pub const SELL_EXACT_IN: [u8; 8] = [149, 39, 222, 155, 211, 124, 152, 26];

    /// initialize discriminator: [175, 175, 109, 31, 13, 251, 127, 237]
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];

    /// initialize_v2 discriminator: [67, 153, 175, 39, 218, 16, 38, 32]
    pub const INITIALIZE_V2: [u8; 8] = [67, 153, 175, 39, 218, 16, 38, 32];

    /// migrate_to_cpswap discriminator: [136, 92, 200, 103, 28, 218, 144, 140]
    pub const MIGRATE_TO_CPSWAP: [u8; 8] = [136, 92, 200, 103, 28, 218, 144, 140];
}

/// Calculate pool state PDA (seeds: ["pool", base_mint, quote_mint])
pub fn get_pool_state_pda(
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
) -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(
        &[seeds::POOL_SEED, base_mint.as_ref(), quote_mint.as_ref()],
        &accounts::LAUNCHLAB_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find pool state PDA"))
}

/// Calculate vault authority PDA (seeds: ["vault_auth_seed"])
pub fn get_vault_authority_pda() -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(&[seeds::VAULT_AUTH_SEED], &accounts::LAUNCHLAB_PROGRAM)
        .ok_or_else(|| anyhow::anyhow!("Failed to find vault authority PDA"))
}

/// Calculate pool vault PDA (seeds: ["pool_vault", pool_state, mint])
pub fn get_pool_vault_pda(
    pool_state: &Pubkey,
    mint: &Pubkey,
) -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(
        &[seeds::POOL_VAULT_SEED, pool_state.as_ref(), mint.as_ref()],
        &accounts::LAUNCHLAB_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find pool vault PDA"))
}

/// Calculate event authority PDA (seeds: ["__event_authority"])
pub fn get_event_authority_pda() -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(&[seeds::EVENT_AUTHORITY_SEED], &accounts::LAUNCHLAB_PROGRAM)
        .ok_or_else(|| anyhow::anyhow!("Failed to find event authority PDA"))
}

/// Calculate platform config PDA (seeds: ["platform_config", platform_admin])
pub fn get_platform_config_pda(platform_admin: &Pubkey) -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(
        &[seeds::PLATFORM_CONFIG_SEED, platform_admin.as_ref()],
        &accounts::LAUNCHLAB_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find platform config PDA"))
}

/// Calculate platform fee vault PDA (seeds: [platform_id, mint_b])
/// This is the vault where platform fees are collected
pub fn get_platform_fee_vault_pda(
    platform_id: &Pubkey,
    mint_b: &Pubkey,
) -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(
        &[platform_id.as_ref(), mint_b.as_ref()],
        &accounts::LAUNCHLAB_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find platform fee vault PDA"))
}

/// Calculate creator fee vault PDA (seeds: [creator, mint_b])
/// This is the vault where creator fees are collected
pub fn get_creator_fee_vault_pda(
    creator: &Pubkey,
    mint_b: &Pubkey,
) -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(
        &[creator.as_ref(), mint_b.as_ref()],
        &accounts::LAUNCHLAB_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find creator fee vault PDA"))
}

/// Build buy_exact_in instruction
///
/// # Arguments
/// * `payer` - The user performing the swap (signer)
/// * `base_mint` - The mint of the base token (token being bought)
/// * `quote_mint` - The mint of the quote token (token being sold, usually WSOL)
/// * `amount_in` - Amount of quote token to purchase
/// * `minimum_amount_out` - Minimum amount of base token to receive (slippage protection)
/// * `share_fee_rate` - Fee rate for the share (in basis points, typically 0)
/// * `global_config` - Global configuration account (can be found using find_global_config)
/// * `platform_config` - Platform configuration account (can be found using find_platform_config)
pub fn build_buy_exact_in_instruction(
    payer: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    share_fee_rate: u64,
    global_config: &Pubkey,
    platform_config: &Pubkey,
) -> Result<Instruction, anyhow::Error> {
    let (pool_state, _) = get_pool_state_pda(base_mint, quote_mint)?;
    let (authority, _) = get_vault_authority_pda()?;
    let (event_authority, _) = get_event_authority_pda()?;

    // Calculate vaults
    let (base_vault, _) = get_pool_vault_pda(&pool_state, base_mint)?;
    let (quote_vault, _) = get_pool_vault_pda(&pool_state, quote_mint)?;

    // Calculate user token accounts
    use crate::common::fast_fn::get_associated_token_address_with_program_id_fast;
    use crate::constants::TOKEN_PROGRAM;
    let user_base_token =
        get_associated_token_address_with_program_id_fast(payer, base_mint, &TOKEN_PROGRAM);
    let user_quote_token =
        get_associated_token_address_with_program_id_fast(payer, quote_mint, &TOKEN_PROGRAM);

    // Build instruction data
    let mut data = Vec::with_capacity(32);
    data.extend_from_slice(&discriminators::BUY_EXACT_IN);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.extend_from_slice(&share_fee_rate.to_le_bytes());

    // Build accounts
    let accounts = vec![
        AccountMeta::new(*payer, true),                                // payer
        AccountMeta::new_readonly(authority, false),                   // authority
        AccountMeta::new_readonly(*global_config, false),              // global_config
        AccountMeta::new_readonly(*platform_config, false),            // platform_config
        AccountMeta::new(pool_state, false),                           // pool_state
        AccountMeta::new(user_base_token, false),                      // user_base_token
        AccountMeta::new(user_quote_token, false),                     // user_quote_token
        AccountMeta::new(base_vault, false),                           // base_vault
        AccountMeta::new(quote_vault, false),                          // quote_vault
        AccountMeta::new_readonly(*base_mint, false),                  // base_token_mint
        AccountMeta::new_readonly(*quote_mint, false),                 // quote_token_mint
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),               // base_token_program
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),               // quote_token_program
        AccountMeta::new_readonly(event_authority, false),             // event_authority
        AccountMeta::new_readonly(accounts::LAUNCHLAB_PROGRAM, false), // program
    ];

    Ok(Instruction { program_id: accounts::LAUNCHLAB_PROGRAM, accounts, data })
}

/// Build buy_exact_in instruction for Raydium LaunchLab (with seed optimization support)
/// This version allows you to specify whether to use seed optimization for address calculation
/// The address calculation must match the one used when creating the token account
pub fn build_buy_exact_in_instruction_with_seed(
    payer: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    share_fee_rate: u64,
    global_config: &Pubkey,
    platform_config: &Pubkey,
    use_seed_optimize: bool,
    creator: &Pubkey,
) -> Result<Instruction, anyhow::Error> {
    let (pool_state, _) = get_pool_state_pda(base_mint, quote_mint)?;
    let (authority, _) = get_vault_authority_pda()?;
    let (event_authority, _) = get_event_authority_pda()?;

    // Calculate vaults
    let (base_vault, _) = get_pool_vault_pda(&pool_state, base_mint)?;
    let (quote_vault, _) = get_pool_vault_pda(&pool_state, quote_mint)?;

    // Calculate user token accounts (must match the address used when creating the account)
    use crate::constants::TOKEN_PROGRAM;
    let user_base_token = if use_seed_optimize {
        use crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed;
        get_associated_token_address_with_program_id_fast_use_seed(
            payer,
            base_mint,
            &TOKEN_PROGRAM,
            use_seed_optimize,
        )
    } else {
        use crate::common::fast_fn::get_associated_token_address_with_program_id_fast;
        get_associated_token_address_with_program_id_fast(payer, base_mint, &TOKEN_PROGRAM)
    };
    let user_quote_token = if use_seed_optimize {
        use crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed;
        get_associated_token_address_with_program_id_fast_use_seed(
            payer,
            quote_mint,
            &TOKEN_PROGRAM,
            use_seed_optimize,
        )
    } else {
        use crate::common::fast_fn::get_associated_token_address_with_program_id_fast;
        get_associated_token_address_with_program_id_fast(payer, quote_mint, &TOKEN_PROGRAM)
    };

    // Build instruction data
    let mut data = Vec::with_capacity(32);
    data.extend_from_slice(&discriminators::BUY_EXACT_IN);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.extend_from_slice(&share_fee_rate.to_le_bytes());

    // Calculate fee vaults - these are PDA accounts, not ATA accounts
    // platformClaimFeeVault: PDA with seeds [platform_id, mint_b]
    // creatorClaimFeeVault: PDA with seeds [creator, mint_b]
    let (platform_claim_fee_vault, _) = get_platform_fee_vault_pda(platform_config, quote_mint)?;
    let (creator_claim_fee_vault, _) = get_creator_fee_vault_pda(creator, quote_mint)?;

    // Build accounts
    let mut accounts = vec![
        AccountMeta::new(*payer, true),                                // payer
        AccountMeta::new_readonly(authority, false),                   // authority
        AccountMeta::new_readonly(*global_config, false),              // global_config
        AccountMeta::new_readonly(*platform_config, false),            // platform_config
        AccountMeta::new(pool_state, false),                           // pool_state
        AccountMeta::new(user_base_token, false),                      // user_base_token
        AccountMeta::new(user_quote_token, false),                     // user_quote_token
        AccountMeta::new(base_vault, false),                           // base_vault
        AccountMeta::new(quote_vault, false),                          // quote_vault
        AccountMeta::new_readonly(*base_mint, false),                  // base_token_mint
        AccountMeta::new_readonly(*quote_mint, false),                 // quote_token_mint
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),               // base_token_program
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),               // quote_token_program
        AccountMeta::new_readonly(event_authority, false),             // event_authority
        AccountMeta::new_readonly(accounts::LAUNCHLAB_PROGRAM, false), // program
    ];

    // Add shareFeeReceiver if share_fee_rate > 0 (optional)
    // For now, we'll skip it since share_fee_rate is typically 0

    // Add required accounts from TypeScript SDK
    accounts.push(AccountMeta::new_readonly(accounts::SYSTEM_PROGRAM, false)); // system_program
    accounts.push(AccountMeta::new(platform_claim_fee_vault, false)); // platformClaimFeeVault
    accounts.push(AccountMeta::new(creator_claim_fee_vault, false)); // creatorClaimFeeVault

    Ok(Instruction { program_id: accounts::LAUNCHLAB_PROGRAM, accounts, data })
}

/// Build sell_exact_in instruction
pub fn build_sell_exact_in_instruction(
    payer: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    share_fee_rate: u64,
    global_config: &Pubkey,
    platform_config: &Pubkey,
) -> Result<Instruction, anyhow::Error> {
    let (pool_state, _) = get_pool_state_pda(base_mint, quote_mint)?;
    let (authority, _) = get_vault_authority_pda()?;
    let (event_authority, _) = get_event_authority_pda()?;

    // Calculate vaults
    let (base_vault, _) = get_pool_vault_pda(&pool_state, base_mint)?;
    let (quote_vault, _) = get_pool_vault_pda(&pool_state, quote_mint)?;

    // Calculate user token accounts
    use crate::common::fast_fn::get_associated_token_address_with_program_id_fast;
    use crate::constants::TOKEN_PROGRAM;
    let user_base_token =
        get_associated_token_address_with_program_id_fast(payer, base_mint, &TOKEN_PROGRAM);
    let user_quote_token =
        get_associated_token_address_with_program_id_fast(payer, quote_mint, &TOKEN_PROGRAM);

    // Build instruction data
    let mut data = Vec::with_capacity(32);
    data.extend_from_slice(&discriminators::SELL_EXACT_IN);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&minimum_amount_out.to_le_bytes());
    data.extend_from_slice(&share_fee_rate.to_le_bytes());

    // Build accounts
    let accounts = vec![
        AccountMeta::new(*payer, true),                                // payer
        AccountMeta::new_readonly(authority, false),                   // authority
        AccountMeta::new_readonly(*global_config, false),              // global_config
        AccountMeta::new_readonly(*platform_config, false),            // platform_config
        AccountMeta::new(pool_state, false),                           // pool_state
        AccountMeta::new(user_base_token, false),                      // user_base_token
        AccountMeta::new(user_quote_token, false),                     // user_quote_token
        AccountMeta::new(base_vault, false),                           // base_vault
        AccountMeta::new(quote_vault, false),                          // quote_vault
        AccountMeta::new_readonly(*base_mint, false),                  // base_token_mint
        AccountMeta::new_readonly(*quote_mint, false),                 // quote_token_mint
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),               // base_token_program
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),               // quote_token_program
        AccountMeta::new_readonly(event_authority, false),             // event_authority
        AccountMeta::new_readonly(accounts::LAUNCHLAB_PROGRAM, false), // program
    ];

    Ok(Instruction { program_id: accounts::LAUNCHLAB_PROGRAM, accounts, data })
}

/// Get global config PDA
/// Seeds: ["global_config", quote_token_mint, curve_type, index]
pub fn get_global_config_pda(
    quote_mint: &Pubkey,
    curve_type: u8,
    index: u16,
) -> Result<(Pubkey, u8), anyhow::Error> {
    let curve_type_bytes = curve_type.to_le_bytes();
    let index_bytes = index.to_le_bytes();
    Pubkey::try_find_program_address(
        &[b"global_config", quote_mint.as_ref(), &curve_type_bytes, &index_bytes],
        &accounts::LAUNCHLAB_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find global config PDA"))
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
                        && let Ok(_) = rpc.get_account(&accounts::USD1_GLOBAL_CONFIG).await {
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

/// Parameters for creating a token mint
#[derive(Clone, Debug)]
pub struct MintParams {
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

/// Curve parameters for bonding curve
#[derive(Clone, Debug)]
pub enum CurveParams {
    Constant {
        supply: u64,
        total_base_sell: u64,
        total_quote_fund_raising: u64,
        migrate_type: u8, // 0: amm, 1: cpswap
    },
    Fixed {
        supply: u64,
        total_quote_fund_raising: u64,
        migrate_type: u8,
    },
    Linear {
        supply: u64,
        total_quote_fund_raising: u64,
        migrate_type: u8,
    },
}

/// Vesting parameters
#[derive(Clone, Debug)]
pub struct VestingParams {
    pub total_locked_amount: u64,
    pub cliff_period: u64,
    pub unlock_period: u64,
}

/// AMM creator fee configuration
#[derive(Clone, Debug)]
pub enum AmmCreatorFeeOn {
    QuoteToken, // Creator fee only on quote token
    BothToken,  // Creator fee on both tokens
}

/// Serialize AmmCreatorFeeOn to bytes
fn serialize_amm_creator_fee_on(fee_on: &AmmCreatorFeeOn) -> Vec<u8> {
    match fee_on {
        AmmCreatorFeeOn::QuoteToken => vec![0], // Variant discriminator: 0
        AmmCreatorFeeOn::BothToken => vec![1],  // Variant discriminator: 1
    }
}

/// Serialize MintParams to bytes
fn serialize_mint_params(params: &MintParams) -> Vec<u8> {
    let mut data = Vec::new();

    // decimals: u8
    data.push(params.decimals);

    // name: String (4 bytes length + bytes)
    let name_bytes = params.name.as_bytes();
    data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(name_bytes);

    // symbol: String (4 bytes length + bytes)
    let symbol_bytes = params.symbol.as_bytes();
    data.extend_from_slice(&(symbol_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(symbol_bytes);

    // uri: String (4 bytes length + bytes)
    let uri_bytes = params.uri.as_bytes();
    data.extend_from_slice(&(uri_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(uri_bytes);

    data
}

/// Serialize CurveParams to bytes
fn serialize_curve_params(params: &CurveParams) -> Vec<u8> {
    let mut data = Vec::new();

    match params {
        CurveParams::Constant {
            supply,
            total_base_sell,
            total_quote_fund_raising,
            migrate_type,
        } => {
            // Variant discriminator: 0 for Constant
            data.push(0);
            // ConstantCurve data
            data.extend_from_slice(&supply.to_le_bytes());
            data.extend_from_slice(&total_base_sell.to_le_bytes());
            data.extend_from_slice(&total_quote_fund_raising.to_le_bytes());
            data.push(*migrate_type);
        },
        CurveParams::Fixed { supply, total_quote_fund_raising, migrate_type } => {
            // Variant discriminator: 1 for Fixed
            data.push(1);
            // FixedCurve data
            data.extend_from_slice(&supply.to_le_bytes());
            data.extend_from_slice(&total_quote_fund_raising.to_le_bytes());
            data.push(*migrate_type);
        },
        CurveParams::Linear { supply, total_quote_fund_raising, migrate_type } => {
            // Variant discriminator: 2 for Linear
            data.push(2);
            // LinearCurve data
            data.extend_from_slice(&supply.to_le_bytes());
            data.extend_from_slice(&total_quote_fund_raising.to_le_bytes());
            data.push(*migrate_type);
        },
    }

    data
}

/// Serialize VestingParams to bytes
fn serialize_vesting_params(params: &VestingParams) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&params.total_locked_amount.to_le_bytes());
    data.extend_from_slice(&params.cliff_period.to_le_bytes());
    data.extend_from_slice(&params.unlock_period.to_le_bytes());
    data
}

/// Calculate Metaplex metadata PDA
/// Seeds: ["metadata", METADATA_PROGRAM_ID, mint]
pub fn get_metadata_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"metadata", accounts::METADATA_PROGRAM.as_ref(), mint.as_ref()],
        &accounts::METADATA_PROGRAM,
    )
    .0
}

/// Build initialize instruction for creating a token on Raydium LaunchLab
pub fn build_initialize_instruction(
    payer: &Pubkey,
    creator: &Pubkey,
    mint: &Pubkey, // mint pubkey (must be signer in transaction)
    quote_mint: &Pubkey,
    global_config: &Pubkey,
    platform_config: &Pubkey,
    mint_params: &MintParams,
    curve_params: &CurveParams,
    vesting_params: &VestingParams,
) -> Result<Instruction, anyhow::Error> {
    // Calculate PDAs
    let (pool_state, _) = get_pool_state_pda(mint, quote_mint)?;
    let (authority, _) = get_vault_authority_pda()?;
    let (event_authority, _) = get_event_authority_pda()?;
    let (base_vault, _) = get_pool_vault_pda(&pool_state, mint)?;
    let (quote_vault, _) = get_pool_vault_pda(&pool_state, quote_mint)?;

    // Calculate metadata PDA (even though we're not creating it, we need the address)
    let metadata_account = get_metadata_pda(mint);

    // Build instruction data
    let mut data = Vec::new();
    data.extend_from_slice(&discriminators::INITIALIZE);

    // Serialize arguments
    let mint_params_bytes = serialize_mint_params(mint_params);
    let curve_params_bytes = serialize_curve_params(curve_params);
    let vesting_params_bytes = serialize_vesting_params(vesting_params);

    data.extend_from_slice(&mint_params_bytes);
    data.extend_from_slice(&curve_params_bytes);
    data.extend_from_slice(&vesting_params_bytes);

    // Build accounts (order matters!)
    let accounts = vec![
        AccountMeta::new(*payer, true),                     // payer
        AccountMeta::new_readonly(*creator, false),         // creator
        AccountMeta::new_readonly(*global_config, false),   // global_config
        AccountMeta::new_readonly(*platform_config, false), // platform_config
        AccountMeta::new_readonly(authority, false),        // authority
        AccountMeta::new(pool_state, false),                // pool_state
        AccountMeta::new(*mint, true),                      // base_mint (signer!)
        AccountMeta::new_readonly(*quote_mint, false),      // quote_mint
        AccountMeta::new(base_vault, false),                // base_vault
        AccountMeta::new(quote_vault, false),               // quote_vault
        AccountMeta::new(metadata_account, false), // metadata_account (PDA, may not exist yet)
        AccountMeta::new_readonly(crate::constants::TOKEN_PROGRAM, false), // base_token_program
        AccountMeta::new_readonly(crate::constants::TOKEN_PROGRAM, false), // quote_token_program
        AccountMeta::new_readonly(accounts::METADATA_PROGRAM, false), // metadata_program
        AccountMeta::new_readonly(accounts::SYSTEM_PROGRAM, false), // system_program
        AccountMeta::new_readonly(accounts::RENT_SYSVAR, false), // rent_program
        AccountMeta::new_readonly(event_authority, false), // event_authority
        AccountMeta::new_readonly(accounts::LAUNCHLAB_PROGRAM, false), // program
    ];

    Ok(Instruction { program_id: accounts::LAUNCHLAB_PROGRAM, accounts, data })
}

/// Build initialize_v2 instruction for creating a token on Raydium LaunchLab
/// This is the recommended instruction (initialize is deprecated)
pub fn build_initialize_v2_instruction(
    payer: &Pubkey,
    creator: &Pubkey,
    mint: &Pubkey, // mint pubkey (must be signer in transaction)
    quote_mint: &Pubkey,
    global_config: &Pubkey,
    platform_config: &Pubkey,
    mint_params: &MintParams,
    curve_params: &CurveParams,
    vesting_params: &VestingParams,
    amm_fee_on: &AmmCreatorFeeOn,
) -> Result<Instruction, anyhow::Error> {
    // Calculate PDAs
    let (pool_state, _) = get_pool_state_pda(mint, quote_mint)?;
    let (authority, _) = get_vault_authority_pda()?;
    let (event_authority, _) = get_event_authority_pda()?;
    let (base_vault, _) = get_pool_vault_pda(&pool_state, mint)?;
    let (quote_vault, _) = get_pool_vault_pda(&pool_state, quote_mint)?;

    // Calculate metadata PDA (even though we're not creating it, we need the address)
    let metadata_account = get_metadata_pda(mint);

    // Build instruction data
    let mut data = Vec::new();
    data.extend_from_slice(&discriminators::INITIALIZE_V2);

    // Serialize arguments
    let mint_params_bytes = serialize_mint_params(mint_params);
    let curve_params_bytes = serialize_curve_params(curve_params);
    let vesting_params_bytes = serialize_vesting_params(vesting_params);
    let amm_fee_on_bytes = serialize_amm_creator_fee_on(amm_fee_on);

    data.extend_from_slice(&mint_params_bytes);
    data.extend_from_slice(&curve_params_bytes);
    data.extend_from_slice(&vesting_params_bytes);
    data.extend_from_slice(&amm_fee_on_bytes);

    // Build accounts (order matters! - same as initialize)
    let accounts = vec![
        AccountMeta::new(*payer, true),                     // payer
        AccountMeta::new_readonly(*creator, false),         // creator
        AccountMeta::new_readonly(*global_config, false),   // global_config
        AccountMeta::new_readonly(*platform_config, false), // platform_config
        AccountMeta::new_readonly(authority, false),        // authority
        AccountMeta::new(pool_state, false),                // pool_state
        AccountMeta::new(*mint, true),                      // base_mint (signer!)
        AccountMeta::new_readonly(*quote_mint, false),      // quote_mint
        AccountMeta::new(base_vault, false),                // base_vault
        AccountMeta::new(quote_vault, false),               // quote_vault
        AccountMeta::new(metadata_account, false), // metadata_account (PDA, may not exist yet)
        AccountMeta::new_readonly(crate::constants::TOKEN_PROGRAM, false), // base_token_program
        AccountMeta::new_readonly(crate::constants::TOKEN_PROGRAM, false), // quote_token_program
        AccountMeta::new_readonly(accounts::METADATA_PROGRAM, false), // metadata_program
        AccountMeta::new_readonly(accounts::SYSTEM_PROGRAM, false), // system_program
        AccountMeta::new_readonly(accounts::RENT_SYSVAR, false), // rent_program
        AccountMeta::new_readonly(event_authority, false), // event_authority
        AccountMeta::new_readonly(accounts::LAUNCHLAB_PROGRAM, false), // program
    ];

    Ok(Instruction { program_id: accounts::LAUNCHLAB_PROGRAM, accounts, data })
}

/// Query all AmmConfig accounts from CPMM program
/// Returns a list of (config_address, amm_config) tuples
async fn query_all_amm_configs(
    rpc: &SolanaRpcClient,
    cpmm_program: &Pubkey,
) -> Result<Vec<(Pubkey, crate::instruction::utils::raydium_cpmm_types::AmmConfig)>, anyhow::Error>
{
    use crate::instruction::utils::raydium_cpmm_types::{AMM_CONFIG_SIZE, amm_config_decode};
    use solana_account_decoder::UiAccountEncoding;
    use solana_rpc_client_api::config::RpcProgramAccountsConfig;
    use solana_rpc_client_api::filter::RpcFilterType;

    // AmmConfig account size: 228 bytes (data) + 8 bytes (discriminator) = 236 bytes
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::DataSize(236), // AmmConfig size
        ]),
        account_config: solana_rpc_client_api::config::RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: None,
            min_context_slot: None,
        },
        with_context: None,
        sort_results: None,
    };

    let accounts = rpc.get_program_ui_accounts_with_config(cpmm_program, config).await?;

    let mut configs = Vec::new();
    for (addr, acc) in accounts {
        // Skip discriminator (first 8 bytes) and decode AmmConfig
        let data_bytes = match &acc.data {
            UiAccountData::Binary(base64_str, _) => match STANDARD.decode(base64_str) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            },
            _ => continue,
        };
        if data_bytes.len() >= 8 + AMM_CONFIG_SIZE
            && let Some(amm_config) = amm_config_decode(&data_bytes[8..8 + AMM_CONFIG_SIZE]) {
                // Verify owner is CPMM program (owner is a string in UiAccount, convert to Pubkey for comparison)
                if let Ok(owner_pubkey) = acc.owner.parse::<Pubkey>()
                    && owner_pubkey == *cpmm_program {
                        configs.push((addr, amm_config));
                    }
            }
    }

    Ok(configs)
}

/// Try to use known config address, trying both mainnet and devnet
async fn try_known_config_address(rpc: &SolanaRpcClient) -> Option<(Pubkey, Pubkey)> {
    // Try mainnet config first
    let mainnet_config = accounts::CPMM_CONFIG_MAINNET;
    if let Ok(account) = rpc.get_account(&mainnet_config).await
        && account.owner == accounts::CPMM_PROGRAM {
            println!("   ✅ 使用已知的 CPMM config 地址: {} (主网)", mainnet_config);
            return Some((mainnet_config, accounts::CPMM_CREATE_POOL_FEE));
        }

    // Try devnet config
    let devnet_config = accounts::CPMM_CONFIG_DEVNET;
    if let Ok(account) = rpc.get_account(&devnet_config).await
        && account.owner == accounts::CPMM_PROGRAM_DEVNET {
            println!("   ✅ 使用已知的 CPMM config 地址: {} (Devnet)", devnet_config);
            return Some((devnet_config, accounts::CPMM_CREATE_POOL_FEE));
        }

    None
}

/// Find CPMM config by querying an existing pool
/// Returns (cpswap_config, cpswap_create_pool_fee_account)
/// Note: cpswap_create_pool_fee_account might be the same as cpswap_config or a separate account
pub async fn find_cpswap_config(rpc: &SolanaRpcClient) -> Result<(Pubkey, Pubkey), anyhow::Error> {
    use crate::constants::WSOL_TOKEN_ACCOUNT;
    use crate::instruction::utils::raydium_cpmm::get_pool_by_mint;

    // Method 1: Try known config addresses first (simplest and most reliable)
    if let Some((config, fee)) = try_known_config_address(rpc).await {
        return Ok((config, fee));
    }

    // Method 2: Try to find an existing CPMM pool (using WSOL as a common token)
    match get_pool_by_mint(rpc, &WSOL_TOKEN_ACCOUNT).await {
        Ok((_pool_address, pool_state)) => {
            let cpswap_config = pool_state.amm_config;

            // Use the known CPMM Create Pool Fee address
            let cpswap_create_pool_fee = accounts::CPMM_CREATE_POOL_FEE;

            println!("   ℹ️  通过 WSOL pool 找到 CPMM config: {}", cpswap_config);
            println!("   ℹ️  使用 CPMM Create Pool Fee: {}", cpswap_create_pool_fee);

            return Ok((cpswap_config, cpswap_create_pool_fee));
        },
        Err(e) => {
            println!("   ⚠️  通过 WSOL 查找 CPMM pool 失败: {}", e);
        },
    }

    // Method 3: Try to query program accounts directly to find any CPMM pool
    // This is a fallback for fork mainnet environments
    use solana_account_decoder::UiAccountEncoding;
    use solana_rpc_client_api::config::RpcProgramAccountsConfig;
    use solana_rpc_client_api::filter::RpcFilterType;

    // Try mainnet program first, then devnet
    let cpmm_programs = vec![accounts::CPMM_PROGRAM, accounts::CPMM_PROGRAM_DEVNET];

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::DataSize(629), // CPMM PoolState size (8 discriminator + 621 data)
        ]),
        account_config: solana_rpc_client_api::config::RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: None,
            min_context_slot: None,
        },
        with_context: None,
        sort_results: None,
    };

    for cpmm_program in &cpmm_programs {
        match rpc.get_program_ui_accounts_with_config(cpmm_program, config.clone()).await {
            Ok(accounts) => {
                if !accounts.is_empty() {
                    // Try to decode the first pool to get amm_config
                    use crate::instruction::utils::raydium_cpmm_types::pool_state_decode;
                    for (_addr, acc) in accounts.iter().take(5) {
                        // Try first 5 pools
                        let data_bytes = match &acc.data {
                            UiAccountData::Binary(base64_str, _) => {
                                match STANDARD.decode(base64_str) {
                                    Ok(bytes) => bytes,
                                    Err(_) => continue,
                                }
                            },
                            _ => continue,
                        };
                        if data_bytes.len() > 8
                            && let Some(pool_state) = pool_state_decode(&data_bytes[8..]) {
                                let cpswap_config = pool_state.amm_config;
                                let cpswap_create_pool_fee = accounts::CPMM_CREATE_POOL_FEE;

                                println!(
                                    "   ℹ️  通过程序账户查询找到 CPMM config: {}",
                                    cpswap_config
                                );
                                println!(
                                    "   ℹ️  使用 CPMM Create Pool Fee: {}",
                                    cpswap_create_pool_fee
                                );

                                return Ok((cpswap_config, cpswap_create_pool_fee));
                            }
                    }
                }
            },
            Err(e) => {
                println!("   ⚠️  通过程序账户查询失败 (程序: {}): {}", cpmm_program, e);
            },
        }
    }

    // Method 4: Query all AmmConfig accounts directly
    for cpmm_program in &cpmm_programs {
        match query_all_amm_configs(rpc, cpmm_program).await {
            Ok(configs) => {
                if !configs.is_empty() {
                    // Use the first config found
                    let (config_address, _amm_config) = &configs[0];
                    let cpswap_create_pool_fee = accounts::CPMM_CREATE_POOL_FEE;

                    println!(
                        "   ℹ️  通过查询所有 AmmConfig 账户找到 CPMM config: {}",
                        config_address
                    );
                    println!("   ℹ️  找到 {} 个 config 账户，使用第一个", configs.len());
                    println!("   ℹ️  使用 CPMM Create Pool Fee: {}", cpswap_create_pool_fee);

                    return Ok((*config_address, cpswap_create_pool_fee));
                }
            },
            Err(e) => {
                println!("   ⚠️  查询所有 AmmConfig 账户失败 (程序: {}): {}", cpmm_program, e);
            },
        }
    }

    // If all approaches fail, return error with helpful message
    Err(anyhow::anyhow!(
        "Could not find CPMM config. Tried:\n\
        - Known config addresses (mainnet: {}, devnet: {})\n\
        - Querying pools by WSOL mint\n\
        - Querying program accounts for pools\n\
        - Querying all AmmConfig accounts\n\
        Please provide cpswap_config explicitly.\n\
        Note: CPMM Create Pool Fee is: {}",
        accounts::CPMM_CONFIG_MAINNET,
        accounts::CPMM_CONFIG_DEVNET,
        accounts::CPMM_CREATE_POOL_FEE
    ))
}

/// Calculate CPMM pool PDA
/// Seeds: ["pool", cpswap_config, token_0_mint, token_1_mint]
fn get_cpswap_pool_pda(
    cpswap_config: &Pubkey,
    token_0_mint: &Pubkey,
    token_1_mint: &Pubkey,
) -> Result<(Pubkey, u8), anyhow::Error> {
    use crate::instruction::utils::raydium_cpmm::seeds as cpmm_seeds;
    Pubkey::try_find_program_address(
        &[
            cpmm_seeds::POOL_SEED,
            cpswap_config.as_ref(),
            token_0_mint.as_ref(),
            token_1_mint.as_ref(),
        ],
        &accounts::CPMM_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find CPMM pool PDA"))
}

/// Calculate CPMM authority PDA
/// Seeds: ["vault_and_lp_mint_auth_seed"]
/// Note: We use the known CPMM_AUTHORITY address instead, but this function is kept as a fallback
#[allow(dead_code)]
fn get_cpswap_authority_pda() -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(&[b"vault_and_lp_mint_auth_seed"], &accounts::CPMM_PROGRAM)
        .ok_or_else(|| anyhow::anyhow!("Failed to find CPMM authority PDA"))
}

/// Calculate CPMM LP mint PDA
/// Seeds: ["pool_lp_mint", cpswap_pool]
fn get_cpswap_lp_mint_pda(cpswap_pool: &Pubkey) -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(
        &[b"pool_lp_mint", cpswap_pool.as_ref()],
        &accounts::CPMM_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find CPMM LP mint PDA"))
}

/// Calculate CPMM vault PDA
/// Seeds: ["pool_vault", cpswap_pool, mint]
fn get_cpswap_vault_pda(
    cpswap_pool: &Pubkey,
    mint: &Pubkey,
) -> Result<(Pubkey, u8), anyhow::Error> {
    use crate::instruction::utils::raydium_cpmm::seeds as cpmm_seeds;
    Pubkey::try_find_program_address(
        &[cpmm_seeds::POOL_VAULT_SEED, cpswap_pool.as_ref(), mint.as_ref()],
        &accounts::CPMM_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find CPMM vault PDA"))
}

/// Calculate CPMM observation PDA
/// Seeds: ["observation", cpswap_pool]
fn get_cpswap_observation_pda(cpswap_pool: &Pubkey) -> Result<(Pubkey, u8), anyhow::Error> {
    use crate::instruction::utils::raydium_cpmm::seeds as cpmm_seeds;
    Pubkey::try_find_program_address(
        &[cpmm_seeds::OBSERVATION_STATE_SEED, cpswap_pool.as_ref()],
        &accounts::CPMM_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find CPMM observation PDA"))
}

/// Calculate lock authority PDA
/// Seeds: ["lock_cp_authority_seed"]
fn get_lock_authority_pda() -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(&[b"lock_cp_authority_seed"], &accounts::LOCK_PROGRAM)
        .ok_or_else(|| anyhow::anyhow!("Failed to find lock authority PDA"))
}

/// Build migrate_to_cpswap instruction
pub fn build_migrate_to_cpswap_instruction(
    payer: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    platform_config: &Pubkey,
    global_config: &Pubkey,
    cpswap_config: &Pubkey,
    cpswap_create_pool_fee: &Pubkey,
) -> Result<Instruction, anyhow::Error> {
    // Calculate LaunchLab PDAs
    let (pool_state, _) = get_pool_state_pda(base_mint, quote_mint)?;
    let (authority, _) = get_vault_authority_pda()?;
    let (base_vault, _) = get_pool_vault_pda(&pool_state, base_mint)?;
    let (quote_vault, _) = get_pool_vault_pda(&pool_state, quote_mint)?;

    // Calculate CPMM PDAs
    // Note: token order matters for CPMM pool. We'll use base_mint as token0 and quote_mint as token1
    let (cpswap_pool, _) = get_cpswap_pool_pda(cpswap_config, base_mint, quote_mint)?;
    // Use known CPMM Authority address (more reliable than PDA derivation)
    // If PDA derivation is needed, use get_cpswap_authority_pda() instead
    let cpswap_authority = accounts::CPMM_AUTHORITY;
    let (cpswap_lp_mint, _) = get_cpswap_lp_mint_pda(&cpswap_pool)?;
    let (cpswap_base_vault, _) = get_cpswap_vault_pda(&cpswap_pool, base_mint)?;
    let (cpswap_quote_vault, _) = get_cpswap_vault_pda(&cpswap_pool, quote_mint)?;
    let (cpswap_observation, _) = get_cpswap_observation_pda(&cpswap_pool)?;
    let (lock_authority, _) = get_lock_authority_pda()?;

    // Calculate pool_lp_token (user's LP token account for receiving LP tokens)
    use crate::common::fast_fn::get_associated_token_address_with_program_id_fast;
    use crate::constants::TOKEN_PROGRAM;
    let pool_lp_token =
        get_associated_token_address_with_program_id_fast(payer, &cpswap_lp_mint, &TOKEN_PROGRAM);

    // lock_lp_vault - Use known address from mainnet transaction
    // From transaction: 4NkRLPVhpr2EB9mxVtf2sP7Ftn1BfxBTPw6HgK1pkPeLNbnGtSVZdVtecVJwozEgKdM6C9TAT1S1LBRmQWaovJ1a
    // Note: This might be a PDA or fixed address. If it's a PDA, we may need to calculate it dynamically.
    let lock_lp_vault = accounts::LOCK_LP_VAULT;

    // Build instruction data (no args for migrate_to_cpswap)
    let mut data = Vec::new();
    data.extend_from_slice(&discriminators::MIGRATE_TO_CPSWAP);

    // Build accounts (order matters!)
    let accounts = vec![
        AccountMeta::new(*payer, true),                           // payer
        AccountMeta::new(*base_mint, false),                      // base_mint
        AccountMeta::new_readonly(*quote_mint, false),            // quote_mint
        AccountMeta::new_readonly(*platform_config, false),       // platform_config
        AccountMeta::new_readonly(accounts::CPMM_PROGRAM, false), // cpswap_program
        AccountMeta::new(cpswap_pool, false),                     // cpswap_pool
        AccountMeta::new_readonly(cpswap_authority, false),       // cpswap_authority
        AccountMeta::new(cpswap_lp_mint, false),                  // cpswap_lp_mint
        AccountMeta::new(cpswap_base_vault, false),               // cpswap_base_vault
        AccountMeta::new(cpswap_quote_vault, false),              // cpswap_quote_vault
        AccountMeta::new_readonly(*cpswap_config, false),         // cpswap_config
        AccountMeta::new(*cpswap_create_pool_fee, false),         // cpswap_create_pool_fee
        AccountMeta::new(cpswap_observation, false),              // cpswap_observation
        AccountMeta::new_readonly(accounts::LOCK_PROGRAM, false), // lock_program
        AccountMeta::new_readonly(lock_authority, false),         // lock_authority
        AccountMeta::new(lock_lp_vault, false),                   // lock_lp_vault (placeholder)
        AccountMeta::new(authority, false),                       // authority
        AccountMeta::new(pool_state, false),                      // pool_state
        AccountMeta::new_readonly(*global_config, false),         // global_config
        AccountMeta::new(base_vault, false),                      // base_vault
        AccountMeta::new(quote_vault, false),                     // quote_vault
        AccountMeta::new(pool_lp_token, false),                   // pool_lp_token
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),          // base_token_program
        AccountMeta::new_readonly(TOKEN_PROGRAM, false),          // quote_token_program
        AccountMeta::new_readonly(accounts::ASSOCIATED_TOKEN_PROGRAM, false), // associated_token_program
        AccountMeta::new_readonly(accounts::SYSTEM_PROGRAM, false),           // system_program
        AccountMeta::new_readonly(accounts::RENT_SYSVAR, false),              // rent_program
        AccountMeta::new_readonly(accounts::METADATA_PROGRAM, false),         // metadata_program
    ];

    Ok(Instruction { program_id: accounts::LAUNCHLAB_PROGRAM, accounts, data })
}
