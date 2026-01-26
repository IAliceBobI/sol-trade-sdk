use crate::{
    common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed,
    constants::trade::trade::DEFAULT_SLIPPAGE,
    instruction::utils::raydium_clmm::{accounts, get_pool_by_address, get_tick_array_pda},
    trading::core::{
        params::{RaydiumClmmParams, SwapParams},
        traits::InstructionBuilder,
    },
    utils::{
        calc::raydium_clmm as clmm_math,
        price::raydium_clmm::{price_token0_in_token1, price_token1_in_token0},
    },
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
/// - swap_v2 (extended): [43, 4, 237, 11, 26, 201, 30, 98] - SwapV2, includes token_program_2022 & memo
///
/// IMPORTANT: Raydium SDK V2 uses SwapV2 instruction for better compatibility
const SWAP_V2_DISCRIMINATOR: &[u8] = &[43, 4, 237, 11, 26, 201, 30, 98];
const _SWAP_DISCRIMINATOR: &[u8] = &[248, 198, 158, 145, 225, 117, 135, 200];

/// Instruction builder for RaydiumClmm protocol
pub struct RaydiumClmmInstructionBuilder;

/// 滑点计算辅助函数
/// 根据官方 client 实现移植
fn amount_with_slippage(amount: u64, slippage_bps: u16, round_up: bool) -> u64 {
    let slippage_f64 = (slippage_bps as f64) / 10000.0; // 将 BP 转换为小数
    if round_up {
        // max in: amount * (1 + slippage), 向上取整
        ((amount as f64) * (1.0 + slippage_f64)).ceil() as u64
    } else {
        // min out: amount * (1 - slippage), 向下取整
        ((amount as f64) * (1.0 - slippage_f64)).floor() as u64
    }
}

/// 简化算法降级方案
fn fallback_simple_calculation(
    amount_in: u64,
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current: i32,
    fee_rate: u32,
    zero_for_one: bool,
    is_token0_in: bool,
    input_decimals: u8,
    output_decimals: u8,
    protocol_params: &RaydiumClmmParams,
) -> u64 {
    // 尝试使用简单的 compute_swap_step
    match clmm_math::calculate_swap_amount_simple(
        amount_in,
        sqrt_price_x64,
        liquidity,
        tick_current,
        fee_rate,
        zero_for_one,
    ) {
        Ok(amount) => amount,
        Err(_e) => {
            // 最后的降级：使用价格计算
            let price = if is_token0_in {
                price_token0_in_token1(
                    sqrt_price_x64,
                    protocol_params.token0_decimals,
                    protocol_params.token1_decimals,
                )
            } else {
                price_token1_in_token0(
                    sqrt_price_x64,
                    protocol_params.token0_decimals,
                    protocol_params.token1_decimals,
                )
            };

            let input_amount_f64 = amount_in as f64 / 10f64.powi(input_decimals as i32);
            let output_amount_f64 = input_amount_f64 * price;
            (output_amount_f64 * 10f64.powi(output_decimals as i32)) as u64
        },
    }
}

#[async_trait::async_trait]
impl InstructionBuilder for RaydiumClmmInstructionBuilder {
    async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // ========================================
        // Parameter validation and basic data preparation
        // ========================================
        // 🔧 修复：显式检查 Option 以提高代码清晰度
        let input_amount =
            params.input_amount.ok_or_else(|| anyhow!("Input amount is required"))?;
        if input_amount == 0 {
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
        if output_mint != protocol_params.token0_mint && output_mint != protocol_params.token1_mint
        {
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
                input_mint,
                stable_mint_in_pool
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

        // 🔧 修复：使用已经解包的 input_amount
        let amount_in: u64 = input_amount;

        // ========================================
        // 使用官方 CLMM 算法计算精确输出量
        // ========================================

        // 获取 decimals
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

        let zero_for_one = is_token0_in;

        // 从 RPC 获取 amm_config 以获取精确的 fee_rate
        let rpc = params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?;
        let amm_config =
            crate::instruction::utils::raydium_clmm::get_amm_config(rpc, &pool_state.amm_config)
                .await?;

        let fee_rate = amm_config.trade_fee_rate;

        // 尝试使用完整的 tick-by-tick 算法
        let expected_output = if pool_state.liquidity > 0 {
            // 计算需要的 tick array start indices
            let current_tick_array_start =
                crate::instruction::utils::raydium_clmm::get_tick_array_start_index(
                    pool_state.tick_current,
                    pool_state.tick_spacing,
                );

            // 获取附近的 3 个 tick arrays（当前 + 前后各1个）
            let tick_spacing_i32 = pool_state.tick_spacing as i32;
            let ticks_per_array = 60 * tick_spacing_i32;

            let mut tick_array_indices = vec![current_tick_array_start];

            // 添加前一个和后一个 tick array
            let prev_index = current_tick_array_start - ticks_per_array;
            let next_index = current_tick_array_start + ticks_per_array;

            if prev_index >= clmm_math::MIN_TICK {
                tick_array_indices.push(prev_index);
            }
            if next_index <= clmm_math::MAX_TICK {
                tick_array_indices.push(next_index);
            }

            // 从 RPC 获取 tick arrays
            match crate::instruction::utils::raydium_clmm::get_tick_arrays(
                params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?,
                &protocol_params.pool_state,
                &tick_array_indices,
            )
            .await
            {
                Ok(tick_arrays) if !tick_arrays.is_empty() => {
                    // 转换为算法需要的格式
                    let tick_data: Vec<(i32, Vec<(i32, i128, u128)>)> = tick_arrays
                        .iter()
                        .map(|(start_index, tick_array)| {
                            let ticks = tick_array
                                .ticks
                                .iter()
                                .filter(|t| t.liquidity_gross > 0)
                                .map(|t| (t.tick, t.liquidity_net, t.liquidity_gross))
                                .collect();
                            (*start_index, ticks)
                        })
                        .collect();

                    // 使用完整算法计算
                    match clmm_math::calculate_swap_amount_with_tick_arrays(
                        amount_in,
                        pool_state.sqrt_price_x64,
                        pool_state.liquidity,
                        pool_state.tick_current,
                        pool_state.tick_spacing,
                        fee_rate,
                        zero_for_one,
                        &tick_data,
                    ) {
                        Ok(amount) => {
                            // 现在使用官方的 uint 库实现，精度与链上完全一致
                            amount
                        },
                        Err(_e) => {
                            // 降级到简化算法
                            fallback_simple_calculation(
                                amount_in,
                                pool_state.sqrt_price_x64,
                                pool_state.liquidity,
                                pool_state.tick_current,
                                fee_rate,
                                zero_for_one,
                                is_token0_in,
                                input_decimals,
                                output_decimals,
                                protocol_params,
                            )
                        },
                    }
                },
                _ => {
                    // 降级到简化算法
                    fallback_simple_calculation(
                        amount_in,
                        pool_state.sqrt_price_x64,
                        pool_state.liquidity,
                        pool_state.tick_current,
                        fee_rate,
                        zero_for_one,
                        is_token0_in,
                        input_decimals,
                        output_decimals,
                        protocol_params,
                    )
                },
            }
        } else {
            // 降级到价格计算
            fallback_simple_calculation(
                amount_in,
                pool_state.sqrt_price_x64,
                pool_state.liquidity,
                pool_state.tick_current,
                fee_rate,
                zero_for_one,
                is_token0_in,
                input_decimals,
                output_decimals,
                protocol_params,
            )
        };

        // Apply slippage using official client logic
        // For buy (base_in=true): minimum_amount_out = expected_output * (1 - slippage)
        let slippage = params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE);
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => {
                // 使用官方的 amount_with_slippage 函数
                // is_base_input=true: 计算 min out，round_up=false
                amount_with_slippage(expected_output, slippage as u16, false)
            },
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
        let mut tick_array_start_index =
            crate::instruction::utils::raydium_clmm::get_first_initialized_tick_array_start_index(
                &pool_state,
                zero_for_one,
            );

        let mut tick_array_pdas = Vec::new();
        let (first_tick_array_pda, _) =
            get_tick_array_pda(&protocol_params.pool_state, tick_array_start_index)?;
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
            if (zero_for_one && tick_array_start_index < MIN_TICK)
                || (!zero_for_one && tick_array_start_index > MAX_TICK)
            {
                break;
            }

            if let Ok((tick_array_pda, _)) =
                get_tick_array_pda(&protocol_params.pool_state, tick_array_start_index)
            {
                tick_array_pdas.push(tick_array_pda);
            }
        }

        // Get tick array bitmap extension PDA
        let (tick_array_bitmap_extension_pda, _) =
            crate::instruction::utils::raydium_clmm::get_tick_array_bitmap_extension_pda(
                &protocol_params.pool_state,
            );

        // Create swap instruction
        // Account order for SwapV2 instruction (Raydium SDK V2):
        // 0. payer (signer, readonly)  // ❗ Note: readonly, not writable
        // 1. ammConfig (readonly)
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
        // remainingAccounts:
        // 13. exTickArrayBitmap (readonly for SwapV2)
        // 14+. tickArrays (writable)

        // SwapV2 指令的主账户列表（13 个账户）
        let mut account_metas = vec![
            AccountMeta::new_readonly(params.payer.pubkey(), true), // 0. Payer (signer, readonly)
            AccountMeta::new_readonly(protocol_params.amm_config, false), // 1. Amm Config (readonly)
            AccountMeta::new(protocol_params.pool_state, false), // 2. Pool State (writable)
            AccountMeta::new(input_token_account, false), // 3. Input Token Account (writable)
            AccountMeta::new(output_token_account, false), // 4. Output Token Account (writable)
            AccountMeta::new(input_vault, false),         // 5. Input Vault (writable)
            AccountMeta::new(output_vault, false),        // 6. Output Vault (writable)
            AccountMeta::new(protocol_params.observation_state, false), // 7. Observation State (writable)
            AccountMeta::new_readonly(crate::constants::TOKEN_PROGRAM, false), // 8. Token Program (readonly)
            AccountMeta::new_readonly(crate::constants::TOKEN_2022_PROGRAM, false), // 9. Token 2022 Program (readonly)
            AccountMeta::new_readonly(crate::constants::MEMO_PROGRAM, false), // 10. Memo Program (readonly)
            AccountMeta::new_readonly(input_mint, false), // 11. Input Mint (readonly)
            AccountMeta::new_readonly(output_mint, false), // 12. Output Mint (readonly)
        ];

        // remainingAccounts: exTickArrayBitmap (readonly for SwapV2) + tickArrays (writable)
        account_metas.push(AccountMeta::new_readonly(tick_array_bitmap_extension_pda, false)); // 13. TickArray Bitmap Extension (readonly)

        // 添加额外的 tick arrays（全部 writable）
        for i in 0..tick_array_pdas.len() {
            account_metas.push(AccountMeta::new(tick_array_pdas[i], false));
        }

        if input_mint == crate::constants::WSOL_TOKEN_ACCOUNT && params.create_input_mint_ata {
            instructions.push(Instruction {
                program_id: crate::constants::TOKEN_PROGRAM,
                accounts: vec![AccountMeta::new(input_token_account, false)],
                data: vec![17], // SyncNative discriminator
            });
        }

        // Create instruction data: discriminator (8 bytes) + amount (u64) + other_amount_threshold (u64) + sqrt_price_limit_x64 (u128) + is_base_input (bool)
        // 使用 SwapV2 指令 discriminator
        //
        // IMPORTANT: is_base_input 的含义：
        // - true: 指定输入金额，计算输出金额 (amount = input, other_amount_threshold = min output)
        // - false: 指定输出金额，计算输入金额 (amount = output, other_amount_threshold = max input)
        // 买入场景：输入固定，输出浮动，所以 is_base_input = true
        let mut data = vec![0u8; 41];
        data[0..8].copy_from_slice(SWAP_V2_DISCRIMINATOR);
        data[8..16].copy_from_slice(&amount_in.to_le_bytes());
        data[16..24].copy_from_slice(&minimum_amount_out.to_le_bytes());
        data[24..40].copy_from_slice(&sqrt_price_limit_x64.to_le_bytes());
        data[40] = 1; // is_base_input = true (买入场景：输入固定)

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

        // 🔧 修复：改进 Option 检查的清晰度
        if params.input_amount.map_or(true, |a| a == 0) {
            return Err(anyhow!("Token amount is not set"));
        }

        // 🔧 修复：提前解包 input_amount
        let input_amount = params.input_amount.unwrap();

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
                output_mint,
                stable_mint_in_pool
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

        // 🔧 修复：使用已经解包的 input_amount
        let amount_in: u64 = input_amount;

        // 获取 decimals（用于简化计算降级）
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

        // 获取 AMM config 以获取费率
        let amm_config = crate::instruction::utils::raydium_clmm::get_amm_config(
            params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?,
            &pool_state.amm_config,
        )
        .await?;

        let fee_rate = amm_config.trade_fee_rate;

        // zero_for_one: token0 -> token1 为 true, token1 -> token0 为 false
        let zero_for_one = is_token0_in;

        // 尝试使用完整的 tick-by-tick 算法（与买入指令相同）
        let expected_output = if pool_state.liquidity > 0 {
            // 计算需要的 tick array start indices
            let current_tick_array_start =
                crate::instruction::utils::raydium_clmm::get_tick_array_start_index(
                    pool_state.tick_current,
                    pool_state.tick_spacing,
                );

            // 获取附近的 3 个 tick arrays（当前 + 前后各1个）
            let tick_spacing_i32 = pool_state.tick_spacing as i32;
            let ticks_per_array = 60 * tick_spacing_i32;

            let mut tick_array_indices = vec![current_tick_array_start];

            // 添加前一个和后一个 tick array
            let prev_index = current_tick_array_start - ticks_per_array;
            let next_index = current_tick_array_start + ticks_per_array;

            if prev_index >= clmm_math::MIN_TICK {
                tick_array_indices.push(prev_index);
            }
            if next_index <= clmm_math::MAX_TICK {
                tick_array_indices.push(next_index);
            }

            // 从 RPC 获取 tick arrays
            match crate::instruction::utils::raydium_clmm::get_tick_arrays(
                params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?,
                &protocol_params.pool_state,
                &tick_array_indices,
            )
            .await
            {
                Ok(tick_arrays) if !tick_arrays.is_empty() => {
                    // 转换为算法需要的格式
                    let tick_data: Vec<(i32, Vec<(i32, i128, u128)>)> = tick_arrays
                        .iter()
                        .map(|(start_index, tick_array)| {
                            let ticks = tick_array
                                .ticks
                                .iter()
                                .filter(|t| t.liquidity_gross > 0)
                                .map(|t| (t.tick, t.liquidity_net, t.liquidity_gross))
                                .collect();
                            (*start_index, ticks)
                        })
                        .collect();

                    // 使用完整算法计算
                    match clmm_math::calculate_swap_amount_with_tick_arrays(
                        amount_in,
                        pool_state.sqrt_price_x64,
                        pool_state.liquidity,
                        pool_state.tick_current,
                        pool_state.tick_spacing,
                        fee_rate,
                        zero_for_one,
                        &tick_data,
                    ) {
                        Ok(amount) => {
                            // 使用官方的精确计算
                            amount
                        },
                        Err(_e) => {
                            // 降级到简化算法
                            fallback_simple_calculation(
                                amount_in,
                                pool_state.sqrt_price_x64,
                                pool_state.liquidity,
                                pool_state.tick_current,
                                fee_rate,
                                zero_for_one,
                                is_token0_in,
                                input_decimals,
                                output_decimals,
                                protocol_params,
                            )
                        },
                    }
                },
                _ => {
                    // 降级到简化算法
                    fallback_simple_calculation(
                        amount_in,
                        pool_state.sqrt_price_x64,
                        pool_state.liquidity,
                        pool_state.tick_current,
                        fee_rate,
                        zero_for_one,
                        is_token0_in,
                        input_decimals,
                        output_decimals,
                        protocol_params,
                    )
                },
            }
        } else {
            // 降级到简化算法
            fallback_simple_calculation(
                amount_in,
                pool_state.sqrt_price_x64,
                pool_state.liquidity,
                pool_state.tick_current,
                fee_rate,
                zero_for_one,
                is_token0_in,
                input_decimals,
                output_decimals,
                protocol_params,
            )
        };

        // Apply slippage
        let slippage = params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE);
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => ((expected_output as f64) * (1.0 - (slippage as f64) / 10000.0)) as u64,
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
            instructions.extend(crate::trading::common::handle_wsol(&params.payer.pubkey(), 0));
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

        // Calculate tick arrays - CLMM requires multiple tick arrays for swap
        // 根据官方 client 实现，需要获取多个 tick arrays（最多 5 个）
        let zero_for_one = is_token0_in;
        let mut tick_array_start_index =
            crate::instruction::utils::raydium_clmm::get_first_initialized_tick_array_start_index(
                &pool_state,
                zero_for_one,
            );

        let mut tick_array_pdas = Vec::new();
        let (first_tick_array_pda, _) =
            get_tick_array_pda(&protocol_params.pool_state, tick_array_start_index)?;
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
            if (zero_for_one && tick_array_start_index < MIN_TICK)
                || (!zero_for_one && tick_array_start_index > MAX_TICK)
            {
                break;
            }

            if let Ok((tick_array_pda, _)) =
                get_tick_array_pda(&protocol_params.pool_state, tick_array_start_index)
            {
                tick_array_pdas.push(tick_array_pda);
            }
        }

        // Get tick array bitmap extension PDA (may not exist)
        let (tick_array_bitmap_extension_pda, _) =
            crate::instruction::utils::raydium_clmm::get_tick_array_bitmap_extension_pda(
                &protocol_params.pool_state,
            );

        // Create swap instruction
        // SwapV2 指令账户顺序（与 buy 相同）

        let mut account_metas = vec![
            AccountMeta::new_readonly(params.payer.pubkey(), true), // 0. Payer (signer, readonly)
            AccountMeta::new_readonly(protocol_params.amm_config, false), // 1. Amm Config (readonly)
            AccountMeta::new(protocol_params.pool_state, false), // 2. Pool State (writable)
            AccountMeta::new(input_token_account, false), // 3. Input Token Account (writable)
            AccountMeta::new(output_token_account, false), // 4. Output Token Account (writable)
            AccountMeta::new(input_vault, false),         // 5. Input Vault (writable)
            AccountMeta::new(output_vault, false),        // 6. Output Vault (writable)
            AccountMeta::new(protocol_params.observation_state, false), // 7. Observation State (writable)
            AccountMeta::new_readonly(crate::constants::TOKEN_PROGRAM, false), // 8. Token Program (readonly)
            AccountMeta::new_readonly(crate::constants::TOKEN_2022_PROGRAM, false), // 9. Token 2022 Program (readonly)
            AccountMeta::new_readonly(crate::constants::MEMO_PROGRAM, false), // 10. Memo Program (readonly)
            AccountMeta::new_readonly(input_mint, false), // 11. Input Mint (readonly)
            AccountMeta::new_readonly(output_mint, false), // 12. Output Mint (readonly)
        ];

        // remainingAccounts: exTickArrayBitmap (readonly for SwapV2) + tickArrays (writable)
        account_metas.push(AccountMeta::new_readonly(tick_array_bitmap_extension_pda, false)); // 13. TickArray Bitmap Extension (readonly)

        // 添加额外的 tick arrays（全部 writable）
        for i in 0..tick_array_pdas.len() {
            account_metas.push(AccountMeta::new(tick_array_pdas[i], false));
        }

        // Create instruction data: discriminator (8 bytes) + amount (u64) + other_amount_threshold (u64) + sqrt_price_limit_x64 (u128) + is_base_input (bool)
        // 使用 SwapV2 指令 discriminator
        let mut data = vec![0u8; 41];
        data[0..8].copy_from_slice(SWAP_V2_DISCRIMINATOR);
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
