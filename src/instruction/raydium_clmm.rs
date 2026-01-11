use crate::{
    common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed,
    constants::{accounts::TOKEN_PROGRAM,   trade::trade::DEFAULT_SLIPPAGE},
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
  
    signer::Signer,
};

/// Instruction discriminator for CLMM swap
/// 
/// Based on Jupiter aggregator usage and production observations:
/// - swap (standard): [248, 198, 158, 145, 225, 117, 135, 200] - SwapSingle, widely used
/// - swap_v2 (extended): [43, 4, 237, 11, 26, 201, 30, 98] - SwapSingleV2, includes token_program_2022 & memo
/// 
/// We use the standard swap instruction for maximum compatibility
const SWAP_DISCRIMINATOR: &[u8] = &[248, 198, 158, 145, 225, 117, 135, 200];
const _SWAP_V2_DISCRIMINATOR: &[u8] = &[43, 4, 237, 11, 26, 201, 30, 98];

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
        // For buy: user input can be SOL/WSOL/USDC, params.output_mint is the token we're buying
        let user_input_mint = params.input_mint;
        let output_mint = params.output_mint;

        // Verify output_mint matches one of the pool tokens
        if output_mint != protocol_params.token0_mint && output_mint != protocol_params.token1_mint {
            return Err(anyhow!("Output mint {} does not match pool tokens", output_mint));
        }

        // Verify input mint is one of SOL/WSOL/USDC
        let is_supported_input = user_input_mint == crate::constants::SOL_TOKEN_ACCOUNT
            || user_input_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || user_input_mint == crate::constants::USDC_TOKEN_ACCOUNT;
        if !is_supported_input {
            return Err(anyhow!("Input mint must be SOL, WSOL or USDC for buy"));
        }

        // Determine the stable mint (WSOL or USDC) actually used by this pool
        let wsol_mint = crate::constants::WSOL_TOKEN_ACCOUNT;
        let usdc_mint = crate::constants::USDC_TOKEN_ACCOUNT;
        let stable_mint_in_pool = if protocol_params.token0_mint == wsol_mint
            || protocol_params.token0_mint == usdc_mint
        {
            protocol_params.token0_mint
        } else if protocol_params.token1_mint == wsol_mint
            || protocol_params.token1_mint == usdc_mint
        {
            protocol_params.token1_mint
        } else {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        };

        // Map SOL input to the actual stable mint used by the pool
        let input_mint = if user_input_mint == crate::constants::SOL_TOKEN_ACCOUNT {
            stable_mint_in_pool
        } else {
            user_input_mint
        };

        // Ensure the effective input mint matches the pool's stable mint
        if input_mint != stable_mint_in_pool {
            return Err(anyhow!(
                "Input mint {} does not match pool stable mint {}",
                input_mint, stable_mint_in_pool
            ));
        }

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
        
        eprintln!("[CLMM Debug] Price calculation:");
        eprintln!("  price: {}", price);
        eprintln!("  input_amount: {} ({} in decimals)", amount_in, input_amount_f64);
        eprintln!("  expected_output_f64: {}", output_amount_f64);
        eprintln!("  expected_output: {}", expected_output);

        // Apply slippage
        let slippage = params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE);
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => {
                ((expected_output as f64) * (1.0 - (slippage as f64) / 10000.0)) as u64
            }
        };
        
        eprintln!("  slippage: {}bp", slippage);
        eprintln!("  minimum_amount_out: {}", minimum_amount_out);

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
            // 不使用 seed 优化，使用标准 ATA 创建方式
            instructions.extend(
                crate::common::fast_fn::create_associated_token_account_idempotent_fast(
                    &params.payer.pubkey(),
                    &params.payer.pubkey(),
                    &output_mint,
                    &output_token_program,
                ),
            );
        }

        // Calculate tick arrays - CLMM requires multiple tick arrays for swap
        // 根据官方 client 实现，需要获取多个 tick arrays（最多 5 个）
        let zero_for_one = is_token0_in;
        let mut tick_array_start_index = crate::instruction::utils::raydium_clmm::get_first_initialized_tick_array_start_index(
            &pool_state,
            zero_for_one,
        );
        
        let mut tick_array_pdas = Vec::new();
        let (first_tick_array_pda, _) = get_tick_array_pda(&protocol_params.pool_state, tick_array_start_index)?;
        tick_array_pdas.push(first_tick_array_pda);
        
        // 获取后续的 tick arrays（最多 5 个）
        let tick_spacing = pool_state.tick_spacing as i32;
        const TICK_ARRAY_SIZE: i32 = 60; // raydium_amm_v3::states::TICK_ARRAY_SIZE
        let ticks_per_array = tick_spacing * TICK_ARRAY_SIZE;
        
        for _ in 0..4 {
            tick_array_start_index = if zero_for_one {
                tick_array_start_index - ticks_per_array
            } else {
                tick_array_start_index + ticks_per_array
            };
            
            // 检查是否超出范围
            const MIN_TICK: i32 = -443636;
            const MAX_TICK: i32 = 443636;
            if (zero_for_one && tick_array_start_index < MIN_TICK) || 
               (!zero_for_one && tick_array_start_index > MAX_TICK) {
                break;
            }
            
            if let Ok((tick_array_pda, _)) = get_tick_array_pda(&protocol_params.pool_state, tick_array_start_index) {
                tick_array_pdas.push(tick_array_pda);
            }
        }
        
        eprintln!("   [CLMM Debug] Generated {} tick array PDAs", tick_array_pdas.len());
        
        // Get tick array bitmap extension PDA
        let (tick_array_bitmap_extension_pda, _) = crate::instruction::utils::raydium_clmm::get_tick_array_bitmap_extension_pda(&protocol_params.pool_state);

        // Create swap instruction
        // Account order for standard Swap instruction (not SwapV2):
        // 0. payer (signer)
        // 1. ammConfig (readonly)
        // 2. poolId (writable)
        // 3. inputTokenAccount (writable)
        // 4. outputTokenAccount (writable)
        // 5. inputVault (writable)
        // 6. outputVault (writable)
        // 7. observationId (writable)
        // 8. TOKEN_PROGRAM_ID (readonly)
        // 9+. remainingAccounts (tick arrays)
        
        // Debug: Print account information
        eprintln!("   [CLMM Debug] Building standard swap instruction with accounts:");
        eprintln!("     0. Payer: {}", params.payer.pubkey());
        eprintln!("     1. Amm Config: {}", protocol_params.amm_config);
        eprintln!("     2. Pool State: {}", protocol_params.pool_state);
        eprintln!("     3. Input Token Account: {}", input_token_account);
        eprintln!("     4. Output Token Account: {}", output_token_account);
        eprintln!("     5. Input Vault: {}", input_vault);
        eprintln!("     6. Output Vault: {}", output_vault);
        eprintln!("     7. Observation State: {}", protocol_params.observation_state);
        eprintln!("     8. Token Program: {}", TOKEN_PROGRAM);
        eprintln!("     9. First Tick Array PDA: {}", tick_array_pdas[0]);
        
        // 根据 Raydium CLMM 官方源码，标准 Swap 指令需要 10 个账户：
        // https://github.com/raydium-io/raydium-clmm/blob/master/programs/amm/src/instructions/swap.rs#L16-L53
        // SwapSingle 结构体包含第一个 tick_array，额外的 tick_arrays 通过 remaining_accounts 添加
        let mut account_metas = vec![
            AccountMeta::new(params.payer.pubkey(), true), // 0. Payer (signer)
            AccountMeta::new_readonly(protocol_params.amm_config, false), // 1. Amm Config (readonly)
            AccountMeta::new(protocol_params.pool_state, false), // 2. Pool State (writable)
            AccountMeta::new(input_token_account, false), // 3. Input Token Account (writable)
            AccountMeta::new(output_token_account, false), // 4. Output Token Account (writable)
            AccountMeta::new(input_vault, false), // 5. Input Vault (writable)
            AccountMeta::new(output_vault, false), // 6. Output Vault (writable)
            AccountMeta::new(protocol_params.observation_state, false), // 7. Observation State (writable)
            AccountMeta::new_readonly(crate::constants::TOKEN_PROGRAM, false), // 8. Token Program (readonly)
            AccountMeta::new(tick_array_pdas[0], false), // 9. 第一个 Tick Array (writable)
        ];
        
        // 根据官方 client 实现（line 1812-1834）：
        // remaining_accounts 第一个是 tickarray_bitmap_extension (readonly)
        // 然后是额外的 tick_arrays
        account_metas.push(AccountMeta::new_readonly(tick_array_bitmap_extension_pda, false)); // 10. TickArray Bitmap Extension (readonly)
        
        // 添加额外的 tick arrays（从第 2 个开始）
        for i in 1..tick_array_pdas.len() {
            account_metas.push(AccountMeta::new(tick_array_pdas[i], false));
        }
        
        eprintln!("   [CLMM Debug] Total accounts: {} (10 main + {} remaining)", account_metas.len(), account_metas.len() - 10);

        // 如果 input 是 WSOL，在 swap 之前再次调用 SyncNative 以确保 token amount 正确同步
        // 这对 mainnet-fork 环境尤其重要，因为 Token Program 可能缓存了旧数据
        if input_mint == crate::constants::WSOL_TOKEN_ACCOUNT {
            instructions.push(Instruction {
                program_id: crate::constants::TOKEN_PROGRAM,
                accounts: vec![AccountMeta::new(input_token_account, false)],
                data: vec![17], // SyncNative discriminator
            });
            eprintln!("   [CLMM Debug] Added SyncNative instruction before swap for WSOL input");
        }

        // Create instruction data: discriminator (8 bytes) + amount (u64) + other_amount_threshold (u64) + sqrt_price_limit_x64 (u128) + is_base_input (bool)
        // 根据官方源码，标准 Swap 指令使用 SWAP_DISCRIMINATOR [248, 198, 158, 145, 225, 117, 135, 200]
        // SwapV2 指令（支持 Token2022）使用 [43, 4, 237, 11, 26, 201, 30, 98]
        let mut data = vec![0u8; 41];
        data[0..8].copy_from_slice(SWAP_DISCRIMINATOR);
        data[8..16].copy_from_slice(&amount_in.to_le_bytes());
        data[16..24].copy_from_slice(&minimum_amount_out.to_le_bytes());
        data[24..40].copy_from_slice(&sqrt_price_limit_x64.to_le_bytes());
        data[40] = if is_token0_in { 1 } else { 0 }; // is_base_input

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
        // For sell: input_mint is the token we're selling, user output can be SOL/WSOL/USDC
        let input_mint = params.input_mint;
        let user_output_mint = params.output_mint;

        // Verify input_mint matches one of the pool tokens
        if input_mint != protocol_params.token0_mint && input_mint != protocol_params.token1_mint {
            return Err(anyhow!("Input mint {} does not match pool tokens", input_mint));
        }

        // Verify output mint is one of SOL/WSOL/USDC
        let is_supported_output = user_output_mint == crate::constants::SOL_TOKEN_ACCOUNT
            || user_output_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || user_output_mint == crate::constants::USDC_TOKEN_ACCOUNT;
        if !is_supported_output {
            return Err(anyhow!("Output mint must be SOL, WSOL or USDC for sell"));
        }

        // Determine the stable mint (WSOL or USDC) actually used by this pool
        let wsol_mint = crate::constants::WSOL_TOKEN_ACCOUNT;
        let usdc_mint = crate::constants::USDC_TOKEN_ACCOUNT;
        let stable_mint_in_pool = if protocol_params.token0_mint == wsol_mint
            || protocol_params.token0_mint == usdc_mint
        {
            protocol_params.token0_mint
        } else if protocol_params.token1_mint == wsol_mint
            || protocol_params.token1_mint == usdc_mint
        {
            protocol_params.token1_mint
        } else {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        };

        // Map SOL output to the actual stable mint used by the pool
        let output_mint = if user_output_mint == crate::constants::SOL_TOKEN_ACCOUNT {
            stable_mint_in_pool
        } else {
            user_output_mint
        };

        // Ensure the effective output mint matches the pool's stable mint
        if output_mint != stable_mint_in_pool {
            return Err(anyhow!(
                "Output mint {} does not match pool stable mint {}",
                output_mint, stable_mint_in_pool
            ));
        }

        // ========================================
        // Trade calculation and account address preparation
        // ========================================
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
        
        // 验证 Tick Array 账户是否存在
        if let Some(rpc) = &params.rpc {
            match rpc.get_account(&tick_array_pda).await {
                Ok(account) => {
                    eprintln!("   [CLMM Debug] Tick Array account exists, owner: {}", account.owner);
                    eprintln!("   [CLMM Debug] Tick Array data length: {}", account.data.len());
                },
                Err(e) => {
                    eprintln!("   [CLMM Debug] WARNING: Tick Array account NOT FOUND: {:?}", e);
                    eprintln!("   [CLMM Debug] This will cause 'AccountOwnedByWrongProgram' error");
                    eprintln!("   [CLMM Debug] Try querying mainnet to verify the account exists");
                }
            }
        }
        
        // Get tick array bitmap extension PDA (may not exist)
        let (_tick_array_bitmap_extension_pda, _) = crate::instruction::utils::raydium_clmm::get_tick_array_bitmap_extension_pda(&protocol_params.pool_state);

        // Create swap instruction
        // 根据 Raydium CLMM 官方源码，标准 Swap 指令需要 10 个账户：
        // https://github.com/raydium-io/raydium-clmm/blob/master/programs/amm/src/instructions/swap.rs#L16-L53
        // SwapSingle 结构体包含第一个 tick_array，额外的 tick_arrays 通过 remaining_accounts 添加
        
        // Debug: Print account information
        eprintln!("   [CLMM Debug] Building standard swap instruction with accounts:");
        eprintln!("     0. Payer: {}", params.payer.pubkey());
        eprintln!("     1. Amm Config: {}", protocol_params.amm_config);
        eprintln!("     2. Pool State: {}", protocol_params.pool_state);
        eprintln!("     3. Input Token Account: {}", input_token_account);
        eprintln!("     4. Output Token Account: {}", output_token_account);
        eprintln!("     5. Input Vault: {}", input_vault);
        eprintln!("     6. Output Vault: {}", output_vault);
        eprintln!("     7. Observation State: {}", protocol_params.observation_state);
        eprintln!("     8. Token Program: {}", TOKEN_PROGRAM);
        eprintln!("     9. Tick Array PDA: {}", tick_array_pda);
        
        let account_metas = vec![
            AccountMeta::new(params.payer.pubkey(), true), // 0. Payer (signer)
            AccountMeta::new_readonly(protocol_params.amm_config, false), // 1. Amm Config (readonly)
            AccountMeta::new(protocol_params.pool_state, false), // 2. Pool State (writable)
            AccountMeta::new(input_token_account, false), // 3. Input Token Account (writable)
            AccountMeta::new(output_token_account, false), // 4. Output Token Account (writable)
            AccountMeta::new(input_vault, false), // 5. Input Vault (writable)
            AccountMeta::new(output_vault, false), // 6. Output Vault (writable)
            AccountMeta::new(protocol_params.observation_state, false), // 7. Observation State (writable)
            AccountMeta::new_readonly(crate::constants::TOKEN_PROGRAM, false), // 8. Token Program (readonly)
            AccountMeta::new(tick_array_pda, false), // 9. Tick Array (writable) - 第一个 tick_array 在主账户中！
        ];

        // Create instruction data: discriminator (8 bytes) + amount (u64) + other_amount_threshold (u64) + sqrt_price_limit_x64 (u128) + is_base_input (bool)
        // 根据官方源码，标准 Swap 指令使用 SWAP_DISCRIMINATOR [248, 198, 158, 145, 225, 117, 135, 200]
        let mut data = vec![0u8; 41];
        data[0..8].copy_from_slice(SWAP_DISCRIMINATOR);
        data[8..16].copy_from_slice(&amount_in.to_le_bytes());
        data[16..24].copy_from_slice(&minimum_amount_out.to_le_bytes());
        data[24..40].copy_from_slice(&sqrt_price_limit_x64.to_le_bytes());
        data[40] = if is_token0_in { 1 } else { 0 }; // is_base_input

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
