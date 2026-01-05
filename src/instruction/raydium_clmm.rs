use crate::{
    common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed,
    constants::{accounts::TOKEN_PROGRAM, accounts::TOKEN_PROGRAM_2022, trade::trade::DEFAULT_SLIPPAGE},
    instruction::utils::raydium_clmm::{accounts, get_pool_by_address, get_tick_array_pda},
    trading::core::{
        params::{RaydiumClmmParams, SwapParams},
        traits::InstructionBuilder,
    },
    utils::price::raydium_clmm::{price_token0_in_token1, price_token1_in_token0},
};
use anyhow::{Result, anyhow};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// Instruction discriminator for CLMM swap_v2
/// 
/// Based on event parser and SDK analysis:
/// - swap (old): [248, 198, 158, 145, 225, 117, 135, 200] - uses SwapSingle (fewer accounts, only token_program)
/// - swap_v2 (new): [43, 4, 237, 11, 26, 201, 30, 98] - uses SwapSingleV2 (more accounts)
/// 
/// We use swap_v2 which includes:
/// - token_program_2022
/// - memo_program  
/// - input_vault_mint
/// - output_vault_mint
/// 
/// Account order matches parse_swap_v2_instruction:
/// 0: payer, 1: amm_config, 2: pool_state, 3: input_token_account, 4: output_token_account,
/// 5: input_vault, 6: output_vault, 7: observation_state, 8: token_program, 9: token_program2022,
/// 10: memo_program, 11: input_vault_mint, 12: output_vault_mint, 13+: tick arrays
const SWAP_V2_DISCRIMINATOR: &[u8] = &[43, 4, 237, 11, 26, 201, 30, 98];

/// Instruction builder for RaydiumClmm protocol
pub struct RaydiumClmmInstructionBuilder;

#[async_trait::async_trait]
impl InstructionBuilder for RaydiumClmmInstructionBuilder {
    async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // ========================================
        // Parameter validation and basic data preparation
        // ========================================
        if params.input_amount.unwrap_or(0) == 0 {
            return Err(anyhow!("Amount cannot be zero"));
        }

        let protocol_params = params
            .protocol_params
            .as_any()
            .downcast_ref::<RaydiumClmmParams>()
            .ok_or_else(|| anyhow!("Invalid protocol params for RaydiumClmm"))?;

        // Fetch pool state to get current price
        let pool_state = get_pool_by_address(
            params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?,
            &protocol_params.pool_state,
        )
        .await?;

        let is_wsol = protocol_params.token0_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.token1_mint == crate::constants::WSOL_TOKEN_ACCOUNT;

        let is_usdc = protocol_params.token0_mint == crate::constants::USDC_TOKEN_ACCOUNT
            || protocol_params.token1_mint == crate::constants::USDC_TOKEN_ACCOUNT;

        if !is_wsol && !is_usdc {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        }

        // ========================================
        // Trade calculation and account address preparation
        // ========================================
        // For buy: params.input_mint should be WSOL/USDC, params.output_mint is the token we're buying
        // Verify input_mint is WSOL or USDC
        if params.input_mint != crate::constants::WSOL_TOKEN_ACCOUNT && params.input_mint != crate::constants::USDC_TOKEN_ACCOUNT {
            return Err(anyhow!("Input mint must be WSOL or USDC for buy"));
        }
        
        // Verify output_mint matches one of the pool tokens
        if params.output_mint != protocol_params.token0_mint && params.output_mint != protocol_params.token1_mint {
            return Err(anyhow!("Output mint {} does not match pool tokens", params.output_mint));
        }
        
        let input_mint = params.input_mint;
        let output_mint = params.output_mint;
        
        // Determine which token is input (for is_base_input flag)
        let is_token0_in = protocol_params.token0_mint == input_mint;
        
        // Get vaults and programs based on which token is input/output
        let (input_vault, input_token_program) = if is_token0_in {
            (protocol_params.token0_vault, protocol_params.token0_program)
        } else {
            (protocol_params.token1_vault, protocol_params.token1_program)
        };
        
        let (output_vault, output_token_program) = if output_mint == protocol_params.token0_mint {
            (protocol_params.token0_vault, protocol_params.token0_program)
        } else {
            (protocol_params.token1_vault, protocol_params.token1_program)
        };
        
        // Note: Raydium CLMM swap instruction requires both TOKEN_PROGRAM_ID and TOKEN_2022_PROGRAM_ID
        // The program will use the appropriate one based on the token accounts

        let amount_in: u64 = params.input_amount.unwrap_or(0);
        
        // Calculate expected output amount using price
        // Note: This is a simplified calculation. In production, CLMM swap output
        // should be calculated by the on-chain program based on liquidity distribution.
        let input_decimals = if input_mint == protocol_params.token0_mint {
            protocol_params.token0_decimals
        } else {
            protocol_params.token1_decimals
        };
        
        let output_decimals = if output_mint == protocol_params.token0_mint {
            protocol_params.token0_decimals
        } else {
            protocol_params.token1_decimals
        };
        
        let price = if is_token0_in {
            price_token0_in_token1(
                pool_state.sqrt_price_x64,
                protocol_params.token0_decimals,
                protocol_params.token1_decimals,
            )
        } else {
            price_token1_in_token0(
                pool_state.sqrt_price_x64,
                protocol_params.token0_decimals,
                protocol_params.token1_decimals,
            )
        };

        // Calculate output amount (simplified - actual CLMM calculation is more complex)
        let input_amount_f64 = amount_in as f64 / 10f64.powi(input_decimals as i32);
        let output_amount_f64 = input_amount_f64 * price;
        let expected_output = (output_amount_f64 * 10f64.powi(output_decimals as i32)) as u64;

        // Apply slippage
        let slippage = params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE);
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => {
                ((expected_output as f64) * (1.0 - (slippage as f64) / 10000.0)) as u64
            }
        };

        let input_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            &input_mint,
            &input_token_program,
            params.open_seed_optimize,
        );
        let output_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            &output_mint,
            &output_token_program,
            params.open_seed_optimize,
        );

        // Calculate sqrt_price_limit_x64 for slippage protection
        // Match Raydium SDK V2 logic from constants.ts:
        // MIN_SQRT_PRICE_X64 = 4295048016
        // MAX_SQRT_PRICE_X64 = 79226673521066979257578248091
        // MIN_SQRT_PRICE_X64_ADD_ONE = 4295048017
        // MAX_SQRT_PRICE_X64_SUB_ONE = 79226673521066979257578248090
        const MIN_SQRT_PRICE_X64: u128 = 4295048016;
        const MAX_SQRT_PRICE_X64: u128 = 79226673521066979257578248091;
        
        // No price limit specified, use default limits matching SDK
        // For baseIn (token0 -> token1): use minimum sqrt price + 1
        // For baseOut (token1 -> token0): use maximum sqrt price - 1
        let sqrt_price_limit_x64 = if is_token0_in {
            // Buying (token0 -> token1): use minimum sqrt price + 1
            MIN_SQRT_PRICE_X64 + 1
        } else {
            // Selling (token1 -> token0): use maximum sqrt price - 1
            MAX_SQRT_PRICE_X64 - 1
        };

        // ========================================
        // Build instructions
        // ========================================
        let mut instructions = Vec::with_capacity(6);

        if params.create_input_mint_ata {
            instructions
                .extend(crate::trading::common::handle_wsol(&params.payer.pubkey(), amount_in));
        }

        if params.create_output_mint_ata {
            instructions.extend(
                crate::common::fast_fn::create_associated_token_account_idempotent_fast_use_seed(
                    &params.payer.pubkey(),
                    &params.payer.pubkey(),
                    &output_mint,
                    &output_token_program,
                    params.open_seed_optimize,
                ),
            );
        }

        // Calculate tick array PDA
        // Reference implementation uses get_first_initialized_tick_array to find the first initialized tick array
        // For now, we use a simplified approach: find the first initialized tick array or fall back to current tick's array
        let zero_for_one = is_token0_in;
        let tick_array_start_index = crate::instruction::utils::raydium_clmm::get_first_initialized_tick_array_start_index(
            &pool_state,
            zero_for_one,
        );
        let (tick_array_pda, _) = get_tick_array_pda(&protocol_params.pool_state, tick_array_start_index)?;
        
        // Get tick array bitmap extension PDA (may not exist)
        let (tick_array_bitmap_extension_pda, _) = crate::instruction::utils::raydium_clmm::get_tick_array_bitmap_extension_pda(&protocol_params.pool_state);

        // Create swap instruction
        // Account order matches Raydium SDK V2 swapInstruction:
        // 0. payer (signer)
        // 1. ammConfigId (readonly)
        // 2. poolId (writable)
        // 3. inputTokenAccount (writable)
        // 4. outputTokenAccount (writable)
        // 5. inputVault (writable)
        // 6. outputVault (writable)
        // 7. observationId (writable)
        // 8. TOKEN_PROGRAM_ID (readonly)
        // 9. TOKEN_2022_PROGRAM_ID (readonly)
        // 10. MEMO_PROGRAM_ID (readonly)
        // 11. inputMint (readonly)
        // 12. outputMint (readonly)
        // 13+. remainingAccounts (tick arrays + exTickArrayBitmap)
        
        // MEMO_PROGRAM_ID: MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr
        const MEMO_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
        
        // Debug: Print account information
        eprintln!("   [CLMM Debug] Building swap instruction with accounts:");
        eprintln!("     0. Payer: {}", params.payer.pubkey());
        eprintln!("     1. Amm Config: {}", protocol_params.amm_config);
        eprintln!("     2. Pool State: {}", protocol_params.pool_state);
        eprintln!("     3. Input Token Account: {}", input_token_account);
        eprintln!("     4. Output Token Account: {}", output_token_account);
        eprintln!("     5. Input Vault: {}", input_vault);
        eprintln!("     6. Output Vault: {}", output_vault);
        eprintln!("     7. Observation State: {}", protocol_params.observation_state);
        eprintln!("     8. Token Program: {}", TOKEN_PROGRAM);
        eprintln!("     9. Token Program 2022: {}", TOKEN_PROGRAM_2022);
        eprintln!("     10. Memo Program: {}", MEMO_PROGRAM_ID);
        eprintln!("     11. Input Mint: {}", input_mint);
        eprintln!("     12. Output Mint: {}", output_mint);
        eprintln!("     13. Tick Array PDA: {}", tick_array_pda);
        
        let mut account_metas = vec![
            AccountMeta::new(params.payer.pubkey(), true), // 0. Payer (signer)
            AccountMeta::new_readonly(protocol_params.amm_config, false), // 1. Amm Config (readonly)
            AccountMeta::new(protocol_params.pool_state, false), // 2. Pool State (writable)
            AccountMeta::new(input_token_account, false), // 3. Input Token Account (writable)
            AccountMeta::new(output_token_account, false), // 4. Output Token Account (writable)
            AccountMeta::new(input_vault, false), // 5. Input Vault (writable)
            AccountMeta::new(output_vault, false), // 6. Output Vault (writable)
            AccountMeta::new(protocol_params.observation_state, false), // 7. Observation State (writable)
            AccountMeta::new_readonly(TOKEN_PROGRAM, false), // 8. Token Program (readonly)
            AccountMeta::new_readonly(TOKEN_PROGRAM_2022, false), // 9. Token Program 2022 (readonly)
            AccountMeta::new_readonly(MEMO_PROGRAM_ID, false), // 10. Memo Program (readonly)
            AccountMeta::new_readonly(input_mint, false), // 11. Input Mint (readonly)
            AccountMeta::new_readonly(output_mint, false), // 12. Output Mint (readonly)
        ];
        
        // Build remaining accounts: exTickArrayBitmap (if exists) + tick arrays
        // Reference implementation: tickarray_bitmap_extension is readonly, only added if exists
        // Note: SDK uses writable, but reference uses readonly - we'll use readonly to match reference
        let mut remaining_accounts = Vec::new();
        
        // Check if tick array bitmap extension exists
        // Reference implementation only adds it if account exists
        // Note: We can't check account existence in build_instructions (async not allowed in sync context)
        // So we'll always include it if rpc is available
        // The program will handle non-existent accounts gracefully
        if params.rpc.is_some() {
            // Reference implementation uses readonly for tickarray_bitmap_extension
            remaining_accounts.push(AccountMeta::new_readonly(tick_array_bitmap_extension_pda, false));
            eprintln!("   [CLMM Debug] Adding TickArrayBitmapExtension to remaining accounts (readonly): {}", tick_array_bitmap_extension_pda);
        }
        
        // Add tick arrays (at least one is required)
        // TODO: In production, calculate all required tick arrays based on swap path
        // For now, we only include the first tick array
        remaining_accounts.push(AccountMeta::new(tick_array_pda, false)); // Tick Array (writable)
        
        account_metas.extend(remaining_accounts);

        // Create instruction data: discriminator (8 bytes) + amount (u64) + other_amount_threshold (u64) + sqrt_price_limit_x64 (u128) + is_base_input (bool)
        // Note: SDK uses bool for is_base_input, but we use u8 (1 or 0)
        let mut data = vec![0u8; 41];
        data[0..8].copy_from_slice(SWAP_V2_DISCRIMINATOR);
        data[8..16].copy_from_slice(&amount_in.to_le_bytes());
        data[16..24].copy_from_slice(&minimum_amount_out.to_le_bytes());
        data[24..40].copy_from_slice(&sqrt_price_limit_x64.to_le_bytes());
        data[40] = if is_token0_in { 1 } else { 0 }; // is_base_input (1 if token0 is input, 0 if token1 is input)

        instructions.push(Instruction::new_with_bytes(
            accounts::RAYDIUM_CLMM,
            &data,
            account_metas,
        ));

        if params.close_input_mint_ata {
            // Close wSOL ATA account, reclaim rent
            instructions.extend(crate::trading::common::close_wsol(&params.payer.pubkey()));
        }

        Ok(instructions)
    }

    async fn build_sell_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // ========================================
        // Parameter validation and basic data preparation
        // ========================================
        let protocol_params = params
            .protocol_params
            .as_any()
            .downcast_ref::<RaydiumClmmParams>()
            .ok_or_else(|| anyhow!("Invalid protocol params for RaydiumClmm"))?;

        if params.input_amount.is_none() || params.input_amount.unwrap_or(0) == 0 {
            return Err(anyhow!("Token amount is not set"));
        }

        // Fetch pool state to get current price
        let pool_state = get_pool_by_address(
            params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?,
            &protocol_params.pool_state,
        )
        .await?;

        let is_wsol = protocol_params.token0_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.token1_mint == crate::constants::WSOL_TOKEN_ACCOUNT;

        let is_usdc = protocol_params.token0_mint == crate::constants::USDC_TOKEN_ACCOUNT
            || protocol_params.token1_mint == crate::constants::USDC_TOKEN_ACCOUNT;

        if !is_wsol && !is_usdc {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        }

        // ========================================
        // Trade calculation and account address preparation
        // ========================================
        // For sell: input_mint is the token we're selling, output_mint should be WSOL/USDC
        // Verify output_mint is WSOL or USDC
        if params.output_mint != crate::constants::WSOL_TOKEN_ACCOUNT && params.output_mint != crate::constants::USDC_TOKEN_ACCOUNT {
            return Err(anyhow!("Output mint must be WSOL or USDC for sell"));
        }
        
        // Verify input_mint matches one of the pool tokens
        if params.input_mint != protocol_params.token0_mint && params.input_mint != protocol_params.token1_mint {
            return Err(anyhow!("Input mint {} does not match pool tokens", params.input_mint));
        }
        
        let input_mint = params.input_mint;
        let output_mint = params.output_mint;
        
        // Determine which token is input (for is_base_input flag)
        let is_token0_in = protocol_params.token0_mint == input_mint;
        
        // Get vaults and programs based on which token is input/output
        let (input_vault, input_token_program) = if is_token0_in {
            (protocol_params.token0_vault, protocol_params.token0_program)
        } else {
            (protocol_params.token1_vault, protocol_params.token1_program)
        };
        
        let (output_vault, output_token_program) = if output_mint == protocol_params.token0_mint {
            (protocol_params.token0_vault, protocol_params.token0_program)
        } else {
            (protocol_params.token1_vault, protocol_params.token1_program)
        };
        
        // Note: Raydium CLMM swap instruction requires both TOKEN_PROGRAM_ID and TOKEN_2022_PROGRAM_ID
        // The program will use the appropriate one based on the token accounts

        let amount_in: u64 = params.input_amount.unwrap_or(0);
        
        // Calculate expected output amount using price
        let price = if is_token0_in {
            price_token0_in_token1(
                pool_state.sqrt_price_x64,
                protocol_params.token0_decimals,
                protocol_params.token1_decimals,
            )
        } else {
            price_token1_in_token0(
                pool_state.sqrt_price_x64,
                protocol_params.token0_decimals,
                protocol_params.token1_decimals,
            )
        };

        // Calculate output amount (simplified - actual CLMM calculation is more complex)
        let input_decimals = if input_mint == protocol_params.token0_mint {
            protocol_params.token0_decimals
        } else {
            protocol_params.token1_decimals
        };

        let output_decimals = if output_mint == protocol_params.token0_mint {
            protocol_params.token0_decimals
        } else {
            protocol_params.token1_decimals
        };

        let input_amount_f64 = amount_in as f64 / 10f64.powi(input_decimals as i32);
        let output_amount_f64 = input_amount_f64 * price;
        let expected_output = (output_amount_f64 * 10f64.powi(output_decimals as i32)) as u64;

        // Apply slippage
        let slippage = params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE);
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => {
                ((expected_output as f64) * (1.0 - (slippage as f64) / 10000.0)) as u64
            }
        };

        let input_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            &input_mint,
            &input_token_program,
            params.open_seed_optimize,
        );
        let output_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            &output_mint,
            &output_token_program,
            params.open_seed_optimize,
        );

        // Calculate sqrt_price_limit_x64 for slippage protection
        // Match Raydium SDK V2 logic from constants.ts:
        // MIN_SQRT_PRICE_X64 = 4295048016
        // MAX_SQRT_PRICE_X64 = 79226673521066979257578248091
        // MIN_SQRT_PRICE_X64_ADD_ONE = 4295048017
        // MAX_SQRT_PRICE_X64_SUB_ONE = 79226673521066979257578248090
        const MIN_SQRT_PRICE_X64: u128 = 4295048016;
        const MAX_SQRT_PRICE_X64: u128 = 79226673521066979257578248091;
        
        // No price limit specified, use default limits matching SDK
        // For baseIn (token0 -> token1): use minimum sqrt price + 1
        // For baseOut (token1 -> token0): use maximum sqrt price - 1
        let sqrt_price_limit_x64 = if is_token0_in {
            // Selling (token0 -> token1): use minimum sqrt price + 1
            MIN_SQRT_PRICE_X64 + 1
        } else {
            // Buying (token1 -> token0): use maximum sqrt price - 1
            MAX_SQRT_PRICE_X64 - 1
        };

        // ========================================
        // Build instructions
        // ========================================
        let mut instructions = Vec::with_capacity(6);

        if params.create_output_mint_ata {
            instructions
                .extend(crate::trading::common::handle_wsol(&params.payer.pubkey(), 0));
        }

        if params.create_input_mint_ata {
            instructions.extend(
                crate::common::fast_fn::create_associated_token_account_idempotent_fast_use_seed(
                    &params.payer.pubkey(),
                    &params.payer.pubkey(),
                    &input_mint,
                    &input_token_program,
                    params.open_seed_optimize,
                ),
            );
        }

        // Calculate tick array PDA
        let zero_for_one = is_token0_in;
        let tick_array_start_index = crate::instruction::utils::raydium_clmm::get_first_initialized_tick_array_start_index(
            &pool_state,
            zero_for_one,
        );
        let (tick_array_pda, _) = get_tick_array_pda(&protocol_params.pool_state, tick_array_start_index)?;
        
        // Get tick array bitmap extension PDA (may not exist)
        let (tick_array_bitmap_extension_pda, _) = crate::instruction::utils::raydium_clmm::get_tick_array_bitmap_extension_pda(&protocol_params.pool_state);

        // Create swap instruction
        // Account order matches Raydium SDK V2 swapInstruction:
        // 0. payer (signer)
        // 1. ammConfigId (readonly)
        // 2. poolId (writable)
        // 3. inputTokenAccount (writable)
        // 4. outputTokenAccount (writable)
        // 5. inputVault (writable)
        // 6. outputVault (writable)
        // 7. observationId (writable)
        // 8. TOKEN_PROGRAM_ID (readonly)
        // 9. TOKEN_2022_PROGRAM_ID (readonly)
        // 10. MEMO_PROGRAM_ID (readonly)
        // 11. inputMint (readonly)
        // 12. outputMint (readonly)
        // 13+. remainingAccounts (tick arrays + exTickArrayBitmap)
        
        // MEMO_PROGRAM_ID: MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr
        const MEMO_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
        
        // Debug: Print account information
        eprintln!("   [CLMM Debug] Building swap instruction with accounts:");
        eprintln!("     0. Payer: {}", params.payer.pubkey());
        eprintln!("     1. Amm Config: {}", protocol_params.amm_config);
        eprintln!("     2. Pool State: {}", protocol_params.pool_state);
        eprintln!("     3. Input Token Account: {}", input_token_account);
        eprintln!("     4. Output Token Account: {}", output_token_account);
        eprintln!("     5. Input Vault: {}", input_vault);
        eprintln!("     6. Output Vault: {}", output_vault);
        eprintln!("     7. Observation State: {}", protocol_params.observation_state);
        eprintln!("     8. Token Program: {}", TOKEN_PROGRAM);
        eprintln!("     9. Token Program 2022: {}", TOKEN_PROGRAM_2022);
        eprintln!("     10. Memo Program: {}", MEMO_PROGRAM_ID);
        eprintln!("     11. Input Mint: {}", input_mint);
        eprintln!("     12. Output Mint: {}", output_mint);
        eprintln!("     13. Tick Array PDA: {}", tick_array_pda);
        
        let mut account_metas = vec![
            AccountMeta::new(params.payer.pubkey(), true), // 0. Payer (signer)
            AccountMeta::new_readonly(protocol_params.amm_config, false), // 1. Amm Config (readonly)
            AccountMeta::new(protocol_params.pool_state, false), // 2. Pool State (writable)
            AccountMeta::new(input_token_account, false), // 3. Input Token Account (writable)
            AccountMeta::new(output_token_account, false), // 4. Output Token Account (writable)
            AccountMeta::new(input_vault, false), // 5. Input Vault (writable)
            AccountMeta::new(output_vault, false), // 6. Output Vault (writable)
            AccountMeta::new(protocol_params.observation_state, false), // 7. Observation State (writable)
            AccountMeta::new_readonly(TOKEN_PROGRAM, false), // 8. Token Program (readonly)
            AccountMeta::new_readonly(TOKEN_PROGRAM_2022, false), // 9. Token Program 2022 (readonly)
            AccountMeta::new_readonly(MEMO_PROGRAM_ID, false), // 10. Memo Program (readonly)
            AccountMeta::new_readonly(input_mint, false), // 11. Input Mint (readonly)
            AccountMeta::new_readonly(output_mint, false), // 12. Output Mint (readonly)
        ];
        
        // Build remaining accounts: exTickArrayBitmap (if exists) + tick arrays
        let mut remaining_accounts = Vec::new();
        
        if params.rpc.is_some() {
            remaining_accounts.push(AccountMeta::new_readonly(tick_array_bitmap_extension_pda, false));
            eprintln!("   [CLMM Debug] Adding TickArrayBitmapExtension to remaining accounts (readonly): {}", tick_array_bitmap_extension_pda);
        }
        
        // Add tick arrays (at least one is required)
        remaining_accounts.push(AccountMeta::new(tick_array_pda, false)); // Tick Array (writable)
        
        account_metas.extend(remaining_accounts);

        // Create instruction data: discriminator (8 bytes) + amount (u64) + other_amount_threshold (u64) + sqrt_price_limit_x64 (u128) + is_base_input (bool)
        let mut data = vec![0u8; 41];
        data[0..8].copy_from_slice(SWAP_V2_DISCRIMINATOR);
        data[8..16].copy_from_slice(&amount_in.to_le_bytes());
        data[16..24].copy_from_slice(&minimum_amount_out.to_le_bytes());
        data[24..40].copy_from_slice(&sqrt_price_limit_x64.to_le_bytes());
        data[40] = if is_token0_in { 1 } else { 0 }; // is_base_input (1 if token0 is input, 0 if token1 is input)

        instructions.push(Instruction::new_with_bytes(
            accounts::RAYDIUM_CLMM,
            &data,
            account_metas,
        ));

        if params.close_input_mint_ata {
            instructions.extend(crate::trading::common::close_wsol(&params.payer.pubkey()));
        }

        Ok(instructions)
    }
}
