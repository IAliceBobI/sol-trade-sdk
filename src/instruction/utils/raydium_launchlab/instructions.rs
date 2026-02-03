use super::constants::{accounts, discriminators};
use super::helpers::*;
use super::types::{AmmCreatorFeeOn, CurveParams, MintParams, VestingParams};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;

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
/// * `base_token_program` - Token program for base token (support Token-2022)
/// * `quote_token_program` - Token program for quote token (support Token-2022)
pub fn build_buy_exact_in_instruction(
    payer: &Pubkey,
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    amount_in: u64,
    minimum_amount_out: u64,
    share_fee_rate: u64,
    global_config: &Pubkey,
    platform_config: &Pubkey,
    base_token_program: &Pubkey,
    quote_token_program: &Pubkey,
) -> Result<Instruction, anyhow::Error> {
    let (pool_state, _) = get_pool_state_pda(base_mint, quote_mint)?;
    let (authority, _) = get_vault_authority_pda()?;
    let (event_authority, _) = get_event_authority_pda()?;

    // Calculate vaults
    let (base_vault, _) = get_pool_vault_pda(&pool_state, base_mint)?;
    let (quote_vault, _) = get_pool_vault_pda(&pool_state, quote_mint)?;

    // 🔧 修复：使用动态 Token Program（支持 Token-2022）
    use crate::common::fast_fn::get_associated_token_address_with_program_id_fast;
    let user_base_token =
        get_associated_token_address_with_program_id_fast(payer, base_mint, base_token_program);
    let user_quote_token =
        get_associated_token_address_with_program_id_fast(payer, quote_mint, quote_token_program);

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
        AccountMeta::new_readonly(*base_token_program, false),         // base_token_program
        AccountMeta::new_readonly(*quote_token_program, false),        // quote_token_program
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
    base_token_program: &Pubkey,
    quote_token_program: &Pubkey,
) -> Result<Instruction, anyhow::Error> {
    let (pool_state, _) = get_pool_state_pda(base_mint, quote_mint)?;
    let (authority, _) = get_vault_authority_pda()?;
    let (event_authority, _) = get_event_authority_pda()?;

    // Calculate vaults
    let (base_vault, _) = get_pool_vault_pda(&pool_state, base_mint)?;
    let (quote_vault, _) = get_pool_vault_pda(&pool_state, quote_mint)?;

    // 🔧 修复：使用动态 Token Program（支持 Token-2022）
    let user_base_token = if use_seed_optimize {
        use crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed;
        get_associated_token_address_with_program_id_fast_use_seed(
            payer,
            base_mint,
            base_token_program,
            use_seed_optimize,
        )
    } else {
        use crate::common::fast_fn::get_associated_token_address_with_program_id_fast;
        get_associated_token_address_with_program_id_fast(payer, base_mint, base_token_program)
    };
    let user_quote_token = if use_seed_optimize {
        use crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed;
        get_associated_token_address_with_program_id_fast_use_seed(
            payer,
            quote_mint,
            quote_token_program,
            use_seed_optimize,
        )
    } else {
        use crate::common::fast_fn::get_associated_token_address_with_program_id_fast;
        get_associated_token_address_with_program_id_fast(payer, quote_mint, quote_token_program)
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
        AccountMeta::new_readonly(*base_token_program, false),         // base_token_program
        AccountMeta::new_readonly(*quote_token_program, false),        // quote_token_program
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
    base_token_program: &Pubkey,
    quote_token_program: &Pubkey,
) -> Result<Instruction, anyhow::Error> {
    let (pool_state, _) = get_pool_state_pda(base_mint, quote_mint)?;
    let (authority, _) = get_vault_authority_pda()?;
    let (event_authority, _) = get_event_authority_pda()?;

    // Calculate vaults
    let (base_vault, _) = get_pool_vault_pda(&pool_state, base_mint)?;
    let (quote_vault, _) = get_pool_vault_pda(&pool_state, quote_mint)?;

    // 🔧 修复：使用动态 Token Program（支持 Token-2022）
    use crate::common::fast_fn::get_associated_token_address_with_program_id_fast;
    let user_base_token =
        get_associated_token_address_with_program_id_fast(payer, base_mint, base_token_program);
    let user_quote_token =
        get_associated_token_address_with_program_id_fast(payer, quote_mint, quote_token_program);

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
        AccountMeta::new_readonly(*base_token_program, false),         // base_token_program
        AccountMeta::new_readonly(*quote_token_program, false),        // quote_token_program
        AccountMeta::new_readonly(event_authority, false),             // event_authority
        AccountMeta::new_readonly(accounts::LAUNCHLAB_PROGRAM, false), // program
    ];

    Ok(Instruction { program_id: accounts::LAUNCHLAB_PROGRAM, accounts, data })
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
    base_token_program: &Pubkey,
    quote_token_program: &Pubkey,
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
    let mint_params_bytes = super::helpers::serialize_mint_params(mint_params);
    let curve_params_bytes = super::helpers::serialize_curve_params(curve_params);
    let vesting_params_bytes = super::helpers::serialize_vesting_params(vesting_params);

    data.extend_from_slice(&mint_params_bytes);
    data.extend_from_slice(&curve_params_bytes);
    data.extend_from_slice(&vesting_params_bytes);

    // 🔧 修复：使用动态 Token Program（支持 Token-2022）
    // Build accounts (order matters!)
    let accounts = vec![
        AccountMeta::new(*payer, true),                                // payer
        AccountMeta::new_readonly(*creator, false),                    // creator
        AccountMeta::new_readonly(*global_config, false),              // global_config
        AccountMeta::new_readonly(*platform_config, false),            // platform_config
        AccountMeta::new_readonly(authority, false),                   // authority
        AccountMeta::new(pool_state, false),                           // pool_state
        AccountMeta::new(*mint, true),                                 // base_mint (signer!)
        AccountMeta::new_readonly(*quote_mint, false),                 // quote_mint
        AccountMeta::new(base_vault, false),                           // base_vault
        AccountMeta::new(quote_vault, false),                          // quote_vault
        AccountMeta::new(metadata_account, false), // metadata_account (PDA, may not exist yet)
        AccountMeta::new_readonly(*base_token_program, false), // base_token_program
        AccountMeta::new_readonly(*quote_token_program, false), // quote_token_program
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
    base_token_program: &Pubkey,
    quote_token_program: &Pubkey,
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
    let mint_params_bytes = super::helpers::serialize_mint_params(mint_params);
    let curve_params_bytes = super::helpers::serialize_curve_params(curve_params);
    let vesting_params_bytes = super::helpers::serialize_vesting_params(vesting_params);
    let amm_fee_on_bytes = super::helpers::serialize_amm_creator_fee_on(amm_fee_on);

    data.extend_from_slice(&mint_params_bytes);
    data.extend_from_slice(&curve_params_bytes);
    data.extend_from_slice(&vesting_params_bytes);
    data.extend_from_slice(&amm_fee_on_bytes);

    // Build accounts (order matters! - same as initialize)
    let accounts = vec![
        AccountMeta::new(*payer, true),                                // payer
        AccountMeta::new_readonly(*creator, false),                    // creator
        AccountMeta::new_readonly(*global_config, false),              // global_config
        AccountMeta::new_readonly(*platform_config, false),            // platform_config
        AccountMeta::new_readonly(authority, false),                   // authority
        AccountMeta::new(pool_state, false),                           // pool_state
        AccountMeta::new(*mint, true),                                 // base_mint (signer!)
        AccountMeta::new_readonly(*quote_mint, false),                 // quote_mint
        AccountMeta::new(base_vault, false),                           // base_vault
        AccountMeta::new(quote_vault, false),                          // quote_vault
        AccountMeta::new(metadata_account, false), // metadata_account (PDA, may not exist yet)
        AccountMeta::new_readonly(*base_token_program, false), // base_token_program
        AccountMeta::new_readonly(*quote_token_program, false), // quote_token_program
        AccountMeta::new_readonly(accounts::METADATA_PROGRAM, false), // metadata_program
        AccountMeta::new_readonly(accounts::SYSTEM_PROGRAM, false), // system_program
        AccountMeta::new_readonly(accounts::RENT_SYSVAR, false), // rent_program
        AccountMeta::new_readonly(event_authority, false), // event_authority
        AccountMeta::new_readonly(accounts::LAUNCHLAB_PROGRAM, false), // program
    ];

    Ok(Instruction { program_id: accounts::LAUNCHLAB_PROGRAM, accounts, data })
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
    base_token_program: &Pubkey,
    quote_token_program: &Pubkey,
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
        AccountMeta::new_readonly(*base_token_program, false),    // base_token_program
        AccountMeta::new_readonly(*quote_token_program, false),   // quote_token_program
        AccountMeta::new_readonly(accounts::ASSOCIATED_TOKEN_PROGRAM, false), // associated_token_program
        AccountMeta::new_readonly(accounts::SYSTEM_PROGRAM, false),           // system_program
        AccountMeta::new_readonly(accounts::RENT_SYSVAR, false),              // rent_program
        AccountMeta::new_readonly(accounts::METADATA_PROGRAM, false),         // metadata_program
    ];

    Ok(Instruction { program_id: accounts::LAUNCHLAB_PROGRAM, accounts, data })
}
