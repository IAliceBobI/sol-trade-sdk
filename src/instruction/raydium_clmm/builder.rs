use crate::{
    common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed,
    constants::trade_consts::DEFAULT_SLIPPAGE,
    instruction::{
        raydium_clmm::{
            builder_helpers::{
                build_swap_account_metas, build_swap_instruction_data, calculate_slippage_amount,
                get_swap_tick_arrays,
            },
            helpers::{amount_with_slippage, fallback_price_calculation},
        },
        utils::raydium_clmm::{accounts, get_pool_by_address},
    },
    trading::core::{
        params::{RaydiumClmmParams, SwapParams},
        traits::InstructionBuilder,
    },
    utils::calc::raydium_clmm as clmm_math,
};
use anyhow::{Result, anyhow};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signer::Signer,
};

/// Instruction builder for RaydiumClmm protocol
pub struct RaydiumClmmInstructionBuilder;

#[async_trait::async_trait]
impl InstructionBuilder for RaydiumClmmInstructionBuilder {
    async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // ========================================
        // 参数验证和基本数据准备
        // ========================================
        let has_fixed_output = params.fixed_output_amount.is_some();

        let input_amount = if !has_fixed_output {
            let amount = params.input_amount.ok_or_else(|| anyhow!("Input amount is required"))?;
            if amount == 0 {
                return Err(anyhow!("Amount cannot be zero"));
            }
            amount
        } else {
            0
        };

        let protocol_params = params
            .protocol_params
            .as_any()
            .downcast_ref::<RaydiumClmmParams>()
            .ok_or_else(|| anyhow!("Invalid protocol params for RaydiumClmm"))?;

        // 获取 Pool 状态
        let pool_state = get_pool_by_address(
            params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?,
            &protocol_params.pool_state,
        )
        .await?;

        // 验证 Pool 包含 WSOL 或 USDC
        let is_wsol = protocol_params.token0_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.token1_mint == crate::constants::WSOL_TOKEN_ACCOUNT;
        let is_usdc = protocol_params.token0_mint == crate::constants::USDC_TOKEN_ACCOUNT
            || protocol_params.token1_mint == crate::constants::USDC_TOKEN_ACCOUNT;

        if !is_wsol && !is_usdc {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        }

        // ========================================
        // 交易计算和账户地址准备
        // ========================================
        let user_input_mint = params.input_mint;
        let output_mint = params.output_mint;

        // 验证输出 mint
        if output_mint != protocol_params.token0_mint && output_mint != protocol_params.token1_mint
        {
            return Err(anyhow!("Output mint {} does not match pool tokens", output_mint));
        }

        // 验证输入 mint
        let is_supported_input = user_input_mint == crate::constants::SOL_TOKEN_ACCOUNT
            || user_input_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || user_input_mint == crate::constants::USDC_TOKEN_ACCOUNT;
        if !is_supported_input {
            return Err(anyhow!("Input mint must be SOL, WSOL or USDC for buy"));
        }

        // 确定稳定币 mint
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

        // 映射 SOL 输入到 Pool 使用的实际稳定币 mint
        let input_mint = if user_input_mint == crate::constants::SOL_TOKEN_ACCOUNT {
            stable_mint_in_pool
        } else {
            user_input_mint
        };

        if input_mint != stable_mint_in_pool {
            return Err(anyhow!(
                "Input mint {} does not match pool stable mint {}",
                input_mint,
                stable_mint_in_pool
            ));
        }

        // 确定 token0 是否为输入
        let is_token0_in = protocol_params.token0_mint == input_mint;

        // 获取 vaults 和 token programs
        let (_input_vault, input_token_program) = if is_token0_in {
            (protocol_params.token0_vault, protocol_params.token0_program)
        } else {
            (protocol_params.token1_vault, protocol_params.token1_program)
        };

        let (_output_vault, output_token_program) = if output_mint == protocol_params.token0_mint {
            (protocol_params.token0_vault, protocol_params.token0_program)
        } else {
            (protocol_params.token1_vault, protocol_params.token1_program)
        };

        let amount_in: u64 = input_amount;

        // ========================================
        // 使用官方 CLMM 算法计算精确输出量
        // ========================================
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

        // 获取 AMM config 以获取费率
        let rpc = params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?;
        let amm_config =
            crate::instruction::utils::raydium_clmm::get_amm_config(rpc, &pool_state.amm_config)
                .await?;

        let fee_rate = amm_config.trade_fee_rate;

        // 计算预期输出
        let expected_output = if pool_state.liquidity > 0 {
            // 计算需要的 tick array start indices
            let current_tick_array_start =
                crate::instruction::utils::raydium_clmm::get_tick_array_start_index(
                    pool_state.tick_current,
                    pool_state.tick_spacing,
                );

            let tick_spacing_i32 = pool_state.tick_spacing as i32;
            let ticks_per_array = 60 * tick_spacing_i32;

            let mut tick_array_indices = vec![current_tick_array_start];

            let prev_index = current_tick_array_start - ticks_per_array;
            let next_index = current_tick_array_start + ticks_per_array;

            if prev_index >= clmm_math::MIN_TICK {
                tick_array_indices.push(prev_index);
            }
            if next_index <= clmm_math::MAX_TICK {
                tick_array_indices.push(next_index);
            }

            match crate::instruction::utils::raydium_clmm::get_tick_arrays(
                params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?,
                &protocol_params.pool_state,
                &tick_array_indices,
            )
            .await
            {
                Ok(tick_arrays) if !tick_arrays.is_empty() => {
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
                        Ok(result) => result.amount_out,
                        Err(_e) => fallback_price_calculation(
                            amount_in,
                            pool_state.sqrt_price_x64,
                            is_token0_in,
                            input_decimals,
                            output_decimals,
                            protocol_params,
                        ),
                    }
                },
                _ => fallback_price_calculation(
                    amount_in,
                    pool_state.sqrt_price_x64,
                    is_token0_in,
                    input_decimals,
                    output_decimals,
                    protocol_params,
                ),
            }
        } else {
            fallback_price_calculation(
                amount_in,
                pool_state.sqrt_price_x64,
                is_token0_in,
                input_decimals,
                output_decimals,
                protocol_params,
            )
        };

        // 应用滑点
        let slippage = params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE);
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => amount_with_slippage(expected_output, slippage as u16, false),
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

        // 计算 sqrt_price_limit_x64
        let sqrt_price_limit_x64 = if is_token0_in {
            crate::instruction::raydium_clmm::builder_helpers::MIN_SQRT_PRICE_X64 + 1
        } else {
            crate::instruction::raydium_clmm::builder_helpers::MAX_SQRT_PRICE_X64 - 1
        };

        // ========================================
        // 构建指令
        // ========================================
        let mut instructions = Vec::with_capacity(6);

        if params.create_input_mint_ata {
            instructions
                .extend(crate::trading::common::handle_wsol(&params.payer.pubkey(), amount_in));
        }

        if params.create_output_mint_ata {
            instructions.extend(
                crate::common::fast_fn::create_associated_token_account_idempotent_fast(
                    &params.payer.pubkey(),
                    &params.payer.pubkey(),
                    &output_mint,
                    &output_token_program,
                ),
            );
        }

        // 获取 tick arrays
        let zero_for_one = is_token0_in;
        let tick_arrays_info = get_swap_tick_arrays(
            &protocol_params.pool_state,
            pool_state.tick_current,
            pool_state.tick_spacing,
            pool_state.tick_array_bitmap,
            zero_for_one,
        )?;

        // 构建 swap 数据
        let (swap_amount, other_threshold, is_base_input) =
            if let Some(fixed_out) = params.fixed_output_amount {
                let max_in = params.input_amount.unwrap_or(0);
                let max_in_with_slippage = calculate_slippage_amount(max_in, slippage, true);
                (fixed_out, max_in_with_slippage, 0)
            } else {
                (amount_in, minimum_amount_out, 1)
            };

        let data = build_swap_instruction_data(
            swap_amount,
            other_threshold,
            sqrt_price_limit_x64,
            is_base_input,
        );

        // 构建账户列表
        let account_metas = build_swap_account_metas(
            params.payer.pubkey(),
            protocol_params,
            input_token_account,
            output_token_account,
            input_mint,
            output_mint,
            &tick_arrays_info,
            is_token0_in,
        );

        if input_mint == crate::constants::WSOL_TOKEN_ACCOUNT && params.create_input_mint_ata {
            instructions.push(Instruction {
                program_id: crate::constants::TOKEN_PROGRAM,
                accounts: vec![AccountMeta::new(input_token_account, false)],
                data: vec![17], // SyncNative discriminator
            });
        }

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

    async fn build_sell_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // 检查是否为 exact_out 模式
        let has_fixed_output = params.fixed_output_amount.is_some();

        if !has_fixed_output {
            if params.input_amount.is_none_or(|a| a == 0) {
                return Err(anyhow!("Token amount is not set"));
            }
        }

        // ========================================
        // 参数验证和基本数据准备
        // ========================================
        let protocol_params = params
            .protocol_params
            .as_any()
            .downcast_ref::<RaydiumClmmParams>()
            .ok_or_else(|| anyhow!("Invalid protocol params for RaydiumClmm"))?;

        let input_amount = params.input_amount.ok_or_else(|| anyhow!("Token amount is not set"))?;
        if input_amount == 0 {
            return Err(anyhow!("Token amount cannot be zero"));
        }

        // 获取 Pool 状态
        let pool_state = get_pool_by_address(
            params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?,
            &protocol_params.pool_state,
        )
        .await?;

        // 验证 Pool 包含 WSOL 或 USDC
        let is_wsol = protocol_params.token0_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.token1_mint == crate::constants::WSOL_TOKEN_ACCOUNT;
        let is_usdc = protocol_params.token0_mint == crate::constants::USDC_TOKEN_ACCOUNT
            || protocol_params.token1_mint == crate::constants::USDC_TOKEN_ACCOUNT;

        if !is_wsol && !is_usdc {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        }

        // ========================================
        // 交易计算和账户地址准备
        // ========================================
        let input_mint = params.input_mint;
        let user_output_mint = params.output_mint;

        // 验证输入 mint
        if input_mint != protocol_params.token0_mint && input_mint != protocol_params.token1_mint {
            return Err(anyhow!("Input mint {} does not match pool tokens", input_mint));
        }

        // 验证输出 mint
        let is_supported_output = user_output_mint == crate::constants::SOL_TOKEN_ACCOUNT
            || user_output_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || user_output_mint == crate::constants::USDC_TOKEN_ACCOUNT;
        if !is_supported_output {
            return Err(anyhow!("Output mint must be SOL, WSOL or USDC for sell"));
        }

        // 确定稳定币 mint
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

        // 映射 SOL 输出到 Pool 使用的实际稳定币 mint
        let output_mint = if user_output_mint == crate::constants::SOL_TOKEN_ACCOUNT {
            stable_mint_in_pool
        } else {
            user_output_mint
        };

        if output_mint != stable_mint_in_pool {
            return Err(anyhow!(
                "Output mint {} does not match pool stable mint {}",
                output_mint,
                stable_mint_in_pool
            ));
        }

        // 确定 token0 是否为输入
        let is_token0_in = protocol_params.token0_mint == input_mint;

        // 获取 vaults 和 token programs
        let (_input_vault, input_token_program) = if is_token0_in {
            (protocol_params.token0_vault, protocol_params.token0_program)
        } else {
            (protocol_params.token1_vault, protocol_params.token1_program)
        };

        let (_output_vault, output_token_program) = if output_mint == protocol_params.token0_mint {
            (protocol_params.token0_vault, protocol_params.token0_program)
        } else {
            (protocol_params.token1_vault, protocol_params.token1_program)
        };

        let amount_in: u64 = input_amount;

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

        // 获取 AMM config 以获取费率
        let amm_config = crate::instruction::utils::raydium_clmm::get_amm_config(
            params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?,
            &pool_state.amm_config,
        )
        .await?;

        let fee_rate = amm_config.trade_fee_rate;

        let zero_for_one = is_token0_in;

        // 计算预期输出（与买入指令相同）
        let expected_output = if pool_state.liquidity > 0 {
            let current_tick_array_start =
                crate::instruction::utils::raydium_clmm::get_tick_array_start_index(
                    pool_state.tick_current,
                    pool_state.tick_spacing,
                );

            let tick_spacing_i32 = pool_state.tick_spacing as i32;
            let ticks_per_array = 60 * tick_spacing_i32;

            let mut tick_array_indices = vec![current_tick_array_start];

            let prev_index = current_tick_array_start - ticks_per_array;
            let next_index = current_tick_array_start + ticks_per_array;

            if prev_index >= clmm_math::MIN_TICK {
                tick_array_indices.push(prev_index);
            }
            if next_index <= clmm_math::MAX_TICK {
                tick_array_indices.push(next_index);
            }

            match crate::instruction::utils::raydium_clmm::get_tick_arrays(
                params.rpc.as_ref().ok_or_else(|| anyhow!("RPC client required"))?,
                &protocol_params.pool_state,
                &tick_array_indices,
            )
            .await
            {
                Ok(tick_arrays) if !tick_arrays.is_empty() => {
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
                        Ok(result) => result.amount_out,
                        Err(_e) => fallback_price_calculation(
                            amount_in,
                            pool_state.sqrt_price_x64,
                            is_token0_in,
                            input_decimals,
                            output_decimals,
                            protocol_params,
                        ),
                    }
                },
                _ => fallback_price_calculation(
                    amount_in,
                    pool_state.sqrt_price_x64,
                    is_token0_in,
                    input_decimals,
                    output_decimals,
                    protocol_params,
                ),
            }
        } else {
            fallback_price_calculation(
                amount_in,
                pool_state.sqrt_price_x64,
                is_token0_in,
                input_decimals,
                output_decimals,
                protocol_params,
            )
        };

        // 应用滑点
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

        // 计算 sqrt_price_limit_x64
        let sqrt_price_limit_x64 = if is_token0_in {
            crate::instruction::raydium_clmm::builder_helpers::MIN_SQRT_PRICE_X64 + 1
        } else {
            crate::instruction::raydium_clmm::builder_helpers::MAX_SQRT_PRICE_X64 - 1
        };

        // ========================================
        // 构建指令
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

        // 获取 tick arrays
        let zero_for_one = is_token0_in;
        let tick_arrays_info = get_swap_tick_arrays(
            &protocol_params.pool_state,
            pool_state.tick_current,
            pool_state.tick_spacing,
            pool_state.tick_array_bitmap,
            zero_for_one,
        )?;

        // 构建 swap 数据
        let (swap_amount, other_threshold, is_base_input_val) =
            if let Some(fixed_out) = params.fixed_output_amount {
                let max_in = params.input_amount.unwrap_or(0);
                let max_in_with_slippage = if max_in > 0 {
                    ((max_in as f64) * (1.0 + (slippage as f64) / 10000.0)) as u64
                } else {
                    0
                };
                (fixed_out, max_in_with_slippage, 0)
            } else {
                (amount_in, minimum_amount_out, if is_token0_in { 1 } else { 0 })
            };

        let data = build_swap_instruction_data(
            swap_amount,
            other_threshold,
            sqrt_price_limit_x64,
            is_base_input_val,
        );

        // 构建账户列表
        let account_metas = build_swap_account_metas(
            params.payer.pubkey(),
            protocol_params,
            input_token_account,
            output_token_account,
            input_mint,
            output_mint,
            &tick_arrays_info,
            is_token0_in,
        );

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
