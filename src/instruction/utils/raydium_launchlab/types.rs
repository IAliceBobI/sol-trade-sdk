use solana_sdk::pubkey::Pubkey;

/// Raydium LaunchLab PoolState structure (matching solana-streamer)
#[derive(Clone, Debug, Default, borsh::BorshDeserialize)]
pub struct LaunchLabPoolState {
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
pub struct LaunchLabVestingSchedule {
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
