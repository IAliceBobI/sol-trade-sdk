use super::types::{GLOBAL_CONFIG_SIZE, GlobalConfig, LaunchLabPoolState, PlatformConfig};
use solana_sdk::pubkey::Pubkey;

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
        migrate_nft_info: super::types::MigrateNftInfo {
            platform_scale,
            creator_scale,
            burn_scale,
        },
        fee_rate,
        name,
        web,
        img,
        cp_config_id,
        creator_fee_rate,
        transfer_fee_extension_auth,
    })
}

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

/// Parse LaunchLab pool state from account data
pub fn parse_pool_state(account_data: &[u8]) -> Result<LaunchLabPoolState, anyhow::Error> {
    // Skip the 8-byte discriminator
    if account_data.len() < 8 {
        return Err(anyhow::anyhow!("Invalid account data: too short"));
    }

    let pool_state_data = &account_data[8..];
    borsh::BorshDeserialize::try_from_slice(pool_state_data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize PoolState: {}", e))
}
