use super::constants::{accounts, seeds};
use super::types::{AmmCreatorFeeOn, CurveParams, MintParams, VestingParams};
use solana_sdk::pubkey::Pubkey;

/// Serialize AmmCreatorFeeOn to bytes
pub(crate) fn serialize_amm_creator_fee_on(fee_on: &AmmCreatorFeeOn) -> Vec<u8> {
    match fee_on {
        AmmCreatorFeeOn::QuoteToken => vec![0], // Variant discriminator: 0
        AmmCreatorFeeOn::BothToken => vec![1],  // Variant discriminator: 1
    }
}

/// Serialize MintParams to bytes
pub(crate) fn serialize_mint_params(params: &MintParams) -> Vec<u8> {
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
pub(crate) fn serialize_curve_params(params: &CurveParams) -> Vec<u8> {
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
pub(crate) fn serialize_vesting_params(params: &VestingParams) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&params.total_locked_amount.to_le_bytes());
    data.extend_from_slice(&params.cliff_period.to_le_bytes());
    data.extend_from_slice(&params.unlock_period.to_le_bytes());
    data
}

/// Calculate pool state PDA (seeds: ["pool", base_mint, quote_mint])
pub(crate) fn get_pool_state_pda(
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
pub(crate) fn get_vault_authority_pda() -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(&[seeds::VAULT_AUTH_SEED], &accounts::LAUNCHLAB_PROGRAM)
        .ok_or_else(|| anyhow::anyhow!("Failed to find vault authority PDA"))
}

/// Calculate pool vault PDA (seeds: ["pool_vault", pool_state, mint])
pub(crate) fn get_pool_vault_pda(
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
pub(crate) fn get_event_authority_pda() -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(&[seeds::EVENT_AUTHORITY_SEED], &accounts::LAUNCHLAB_PROGRAM)
        .ok_or_else(|| anyhow::anyhow!("Failed to find event authority PDA"))
}

/// Calculate platform config PDA (seeds: ["platform_config", platform_admin])
pub(crate) fn get_platform_config_pda(
    platform_admin: &Pubkey,
) -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(
        &[seeds::PLATFORM_CONFIG_SEED, platform_admin.as_ref()],
        &accounts::LAUNCHLAB_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find platform config PDA"))
}

/// Calculate platform fee vault PDA (seeds: [platform_id, mint_b])
/// This is the vault where platform fees are collected
pub(crate) fn get_platform_fee_vault_pda(
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
pub(crate) fn get_creator_fee_vault_pda(
    creator: &Pubkey,
    mint_b: &Pubkey,
) -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(
        &[creator.as_ref(), mint_b.as_ref()],
        &accounts::LAUNCHLAB_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find creator fee vault PDA"))
}

/// Get global config PDA
/// Seeds: ["global_config", quote_token_mint, curve_type, index]
pub(crate) fn get_global_config_pda(
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

/// Calculate Metaplex metadata PDA
/// Seeds: ["metadata", METADATA_PROGRAM_ID, mint]
pub(crate) fn get_metadata_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"metadata", accounts::METADATA_PROGRAM.as_ref(), mint.as_ref()],
        &accounts::METADATA_PROGRAM,
    )
    .0
}

/// Calculate the bonding curve PDA for a given mint
/// Note: In Raydium LaunchLab, the pool_state PDA uses seeds: ["pool", base_mint, quote_mint]
/// This function is kept for compatibility but should use get_pool_state_pda instead
pub(crate) fn get_bonding_curve_pda(mint: &Pubkey) -> Result<(Pubkey, u8), anyhow::Error> {
    // For Raydium LaunchLab, we need both base_mint and quote_mint to get pool_state
    // This is a simplified version that assumes quote_mint is WSOL
    use crate::constants::WSOL_TOKEN_ACCOUNT;
    get_pool_state_pda(mint, &WSOL_TOKEN_ACCOUNT)
}

/// Calculate CPMM pool PDA
/// Seeds: ["pool", cpswap_config, token_0_mint, token_1_mint]
pub(crate) fn get_cpswap_pool_pda(
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
pub(crate) fn get_cpswap_authority_pda() -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(&[b"vault_and_lp_mint_auth_seed"], &accounts::CPMM_PROGRAM)
        .ok_or_else(|| anyhow::anyhow!("Failed to find CPMM authority PDA"))
}

/// Calculate CPMM LP mint PDA
/// Seeds: ["pool_lp_mint", cpswap_pool]
pub(crate) fn get_cpswap_lp_mint_pda(cpswap_pool: &Pubkey) -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(
        &[b"pool_lp_mint", cpswap_pool.as_ref()],
        &accounts::CPMM_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find CPMM LP mint PDA"))
}

/// Calculate CPMM vault PDA
/// Seeds: ["pool_vault", cpswap_pool, mint]
pub(crate) fn get_cpswap_vault_pda(
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
pub(crate) fn get_cpswap_observation_pda(
    cpswap_pool: &Pubkey,
) -> Result<(Pubkey, u8), anyhow::Error> {
    use crate::instruction::utils::raydium_cpmm::seeds as cpmm_seeds;
    Pubkey::try_find_program_address(
        &[cpmm_seeds::OBSERVATION_STATE_SEED, cpswap_pool.as_ref()],
        &accounts::CPMM_PROGRAM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find CPMM observation PDA"))
}

/// Calculate lock authority PDA
/// Seeds: ["lock_cp_authority_seed"]
pub(crate) fn get_lock_authority_pda() -> Result<(Pubkey, u8), anyhow::Error> {
    Pubkey::try_find_program_address(&[b"lock_cp_authority_seed"], &accounts::LOCK_PROGRAM)
        .ok_or_else(|| anyhow::anyhow!("Failed to find lock authority PDA"))
}
