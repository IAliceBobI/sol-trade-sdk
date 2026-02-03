use super::constants::accounts;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use crate::common::SolanaRpcClient;
use solana_account_decoder::UiAccountData;
use solana_account_decoder::UiAccountEncoding;
use solana_rpc_client_api::config::RpcProgramAccountsConfig;
use solana_rpc_client_api::filter::RpcFilterType;
use solana_sdk::pubkey::Pubkey;

/// Try to use known config address, trying both mainnet and devnet
async fn try_known_config_address(rpc: &SolanaRpcClient) -> Option<(Pubkey, Pubkey)> {
    // Try mainnet config first
    let mainnet_config = accounts::CPMM_CONFIG_MAINNET;
    if let Ok(account) = rpc.get_account(&mainnet_config).await
        && account.owner == accounts::CPMM_PROGRAM
    {
        println!("   ✅ 使用已知的 CPMM config 地址: {} (主网)", mainnet_config);
        return Some((mainnet_config, accounts::CPMM_CREATE_POOL_FEE));
    }

    // Try devnet config
    let devnet_config = accounts::CPMM_CONFIG_DEVNET;
    if let Ok(account) = rpc.get_account(&devnet_config).await
        && account.owner == accounts::CPMM_PROGRAM_DEVNET
    {
        println!("   ✅ 使用已知的 CPMM config 地址: {} (Devnet)", devnet_config);
        return Some((devnet_config, accounts::CPMM_CREATE_POOL_FEE));
    }

    None
}

/// Query all AmmConfig accounts from CPMM program
/// Returns a list of (config_address, amm_config) tuples
async fn query_all_amm_configs(
    rpc: &SolanaRpcClient,
    cpmm_program: &Pubkey,
) -> Result<Vec<(Pubkey, crate::instruction::utils::raydium_cpmm_types::AmmConfig)>, anyhow::Error> {
    use crate::instruction::utils::raydium_cpmm_types::{AMM_CONFIG_SIZE, amm_config_decode};

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
            && let Some(amm_config) = amm_config_decode(&data_bytes[8..8 + AMM_CONFIG_SIZE])
        {
            // Verify owner is CPMM program (owner is a string in UiAccount, convert to Pubkey for comparison)
            if let Ok(owner_pubkey) = acc.owner.parse::<Pubkey>()
                && owner_pubkey == *cpmm_program
            {
                configs.push((addr, amm_config));
            }
        }
    }

    Ok(configs)
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

    // Try mainnet program first, then devnet
    let cpmm_programs = vec![accounts::CPMM_PROGRAM, accounts::CPMM_PROGRAM_DEVNET];

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
                            && let Some(pool_state) = pool_state_decode(
                                &data_bytes[8..],
                                crate::constants::dex_protocols::DexProtocol::RaydiumCpmm
                                    .program_id_pubkey(),
                            )
                        {
                            let cpswap_config = pool_state.amm_config;
                            let cpswap_create_pool_fee = accounts::CPMM_CREATE_POOL_FEE;

                            println!("   ℹ️  通过程序账户查询找到 CPMM config: {}", cpswap_config);
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
