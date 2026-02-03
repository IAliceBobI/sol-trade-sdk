// 允许在此模块中使用 unwrap，因为：
// 1. PDA 地址计算对有效输入不应该失败
// 2. 参数在调用前已经过验证
#![allow(clippy::unwrap_used)]

use crate::{
    common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed,
    constants::trade_consts::DEFAULT_SLIPPAGE,
    instruction::utils::raydium_cpmm::{
        SWAP_BASE_IN_DISCRIMINATOR, accounts, get_amm_config_fees, get_observation_state_pda,
        get_pool_pda, get_vault_account,
    },
    trading::core::{
        params::{RaydiumCpmmParams, SwapParams},
        traits::InstructionBuilder,
    },
    utils::calc::raydium_cpmm::compute_swap_amount,
};
use anyhow::{Result, anyhow};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
};

/// 获取输入 Token（WSOL/USDC）的 Token Program
///
/// WSOL 固定使用 TOKEN_PROGRAM
/// USDC 自动检测（支持 Token-2022）
fn get_input_token_program(is_wsol: bool) -> &'static solana_sdk::pubkey::Pubkey {
    if is_wsol {
        &crate::constants::TOKEN_PROGRAM
    } else {
        // USDC: 使用 calculate_ata_sync 的内部逻辑
        // 优先从缓存获取，缓存未命中则使用白名单（TOKEN_PROGRAM）
        crate::utils::token::get_token_program_cached(&crate::constants::USDC_TOKEN_ACCOUNT)
            .map(|program| {
                if program == crate::constants::TOKEN_PROGRAM_2022 {
                    &crate::constants::TOKEN_PROGRAM_2022
                } else {
                    &crate::constants::TOKEN_PROGRAM
                }
            })
            .unwrap_or(&crate::constants::TOKEN_PROGRAM)
    }
}

/// Instruction builder for RaydiumCpmm protocol
pub struct RaydiumCpmmInstructionBuilder;

#[async_trait::async_trait]
impl InstructionBuilder for RaydiumCpmmInstructionBuilder {
    async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // ========================================
        // Parameter validation and basic data preparation
        // ========================================
        // 检查是否为 exact_out 模式
        let has_fixed_output = params.fixed_output_amount.is_some();

        let input_amount = if !has_fixed_output {
            // exact_in 模式：需要 input_amount
            let amount = params.input_amount.ok_or_else(|| anyhow!("Input amount is required"))?;
            if amount == 0 {
                return Err(anyhow!("Amount cannot be zero"));
            }
            amount
        } else {
            // exact_out 模式：稍后计算
            0
        };

        let protocol_params = params
            .protocol_params
            .as_any()
            .downcast_ref::<RaydiumCpmmParams>()
            .ok_or_else(|| anyhow!("Invalid protocol params for RaydiumCpmm"))?;

        let pool_state = if protocol_params.pool_state == Pubkey::default() {
            get_pool_pda(
                &protocol_params.amm_config,
                &protocol_params.base_mint,
                &protocol_params.quote_mint,
            )
            .unwrap()
        } else {
            protocol_params.pool_state
        };

        let is_wsol = protocol_params.base_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.quote_mint == crate::constants::WSOL_TOKEN_ACCOUNT;

        let is_usdc = protocol_params.base_mint == crate::constants::USDC_TOKEN_ACCOUNT
            || protocol_params.quote_mint == crate::constants::USDC_TOKEN_ACCOUNT;

        if !is_wsol && !is_usdc {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        }

        // ========================================
        // Trade calculation and account address preparation
        // ========================================
        let is_base_in = protocol_params.base_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.base_mint == crate::constants::USDC_TOKEN_ACCOUNT;
        let mint_token_program = if is_base_in {
            protocol_params.quote_token_program
        } else {
            protocol_params.base_token_program
        };

        // 🔧 修复：使用已经解包的 input_amount
        let amount_in: u64 = input_amount;

        // 获取实际费率（从 amm_config 账户）
        let fees = match params.rpc.as_ref() {
            Some(rpc) => get_amm_config_fees(rpc, &protocol_params.amm_config).await?,
            None => {
                // 无 RPC 客户端时使用默认费率（向后兼容）
                return Err(anyhow!("RPC client is required for fee calculation"));
            },
        };

        let result = compute_swap_amount(
            protocol_params.base_reserve,
            protocol_params.quote_reserve,
            is_base_in,
            amount_in,
            params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE),
            fees.trade_fee_rate,
            fees.protocol_fee_rate,
            fees.fund_fee_rate,
        );
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => result.min_amount_out,
        };

        let input_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            if is_wsol {
                &crate::constants::WSOL_TOKEN_ACCOUNT
            } else {
                &crate::constants::USDC_TOKEN_ACCOUNT
            },
            get_input_token_program(is_wsol),
            params.open_seed_optimize,
        );
        let output_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            &params.output_mint,
            &mint_token_program,
            params.open_seed_optimize,
        );

        let input_vault_account = get_vault_account(
            &pool_state,
            if is_wsol {
                &crate::constants::WSOL_TOKEN_ACCOUNT
            } else {
                &crate::constants::USDC_TOKEN_ACCOUNT
            },
            protocol_params,
        );
        let output_vault_account =
            get_vault_account(&pool_state, &params.output_mint, protocol_params);

        let observation_state_account = if protocol_params.observation_state == Pubkey::default() {
            get_observation_state_pda(&pool_state).unwrap()
        } else {
            protocol_params.observation_state
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
                    &params.output_mint,
                    &mint_token_program,
                    params.open_seed_optimize,
                ),
            );
        }

        // Create buy instruction
        let input_token_program_meta =
            AccountMeta::new_readonly(*get_input_token_program(is_wsol), false);
        let accounts: [AccountMeta; 13] = [
            AccountMeta::new(params.payer.pubkey(), true), // Payer (signer)
            accounts::AUTHORITY_META,                      // Authority (readonly)
            AccountMeta::new(protocol_params.amm_config, false), // Amm Config (readonly)
            AccountMeta::new(pool_state, false),           // Pool State
            AccountMeta::new(input_token_account, false),  // Input Token Account
            AccountMeta::new(output_token_account, false), // Output Token Account
            AccountMeta::new(input_vault_account, false),  // Input Vault Account
            AccountMeta::new(output_vault_account, false), // Output Vault Account
            input_token_program_meta,                      // Input Token Program (readonly)
            AccountMeta::new_readonly(mint_token_program, false), // Output Token Program (readonly)
            if is_wsol {
                crate::constants::WSOL_TOKEN_ACCOUNT_META
            } else {
                crate::constants::USDC_TOKEN_ACCOUNT_META
            }, // Input token mint (readonly)
            AccountMeta::new_readonly(params.output_mint, false), // Output token mint (readonly)
            AccountMeta::new(observation_state_account, false), // Observation State Account
        ];
        // Create instruction data
        let mut data = [0u8; 24];
        data[..8].copy_from_slice(SWAP_BASE_IN_DISCRIMINATOR);
        data[8..16].copy_from_slice(&amount_in.to_le_bytes());
        data[16..24].copy_from_slice(&minimum_amount_out.to_le_bytes());

        instructions.push(Instruction::new_with_bytes(
            accounts::RAYDIUM_CPMM,
            &data,
            accounts.to_vec(),
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
            .downcast_ref::<RaydiumCpmmParams>()
            .ok_or_else(|| anyhow!("Invalid protocol params for RaydiumCpmm"))?;

        // 检查是否为 exact_out 模式
        let has_fixed_output = params.fixed_output_amount.is_some();

        if !has_fixed_output {
            // exact_in 模式：需要 input_amount
            if params.input_amount.is_none_or(|a| a == 0) {
                return Err(anyhow!("Token amount is not set"));
            }
        }

        let pool_state = if protocol_params.pool_state == Pubkey::default() {
            get_pool_pda(
                &protocol_params.amm_config,
                &protocol_params.base_mint,
                &protocol_params.quote_mint,
            )
            .unwrap()
        } else {
            protocol_params.pool_state
        };

        let is_wsol = protocol_params.base_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.quote_mint == crate::constants::WSOL_TOKEN_ACCOUNT;

        let is_usdc = protocol_params.base_mint == crate::constants::USDC_TOKEN_ACCOUNT
            || protocol_params.quote_mint == crate::constants::USDC_TOKEN_ACCOUNT;

        if !is_wsol && !is_usdc {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        }

        // ========================================
        // Trade calculation and account address preparation
        // ========================================
        let is_quote_out = protocol_params.quote_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.quote_mint == crate::constants::USDC_TOKEN_ACCOUNT;
        let mint_token_program = if is_quote_out {
            protocol_params.base_token_program
        } else {
            protocol_params.quote_token_program
        };

        let minimum_amount_out: u64 = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => {
                // 获取实际费率（从 amm_config 账户）
                let fees = match params.rpc.as_ref() {
                    Some(rpc) => get_amm_config_fees(rpc, &protocol_params.amm_config).await?,
                    None => {
                        return Err(anyhow!("RPC client is required for fee calculation"));
                    },
                };

                compute_swap_amount(
                    protocol_params.base_reserve,
                    protocol_params.quote_reserve,
                    is_quote_out,
                    params.input_amount.unwrap_or(0),
                    params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE),
                    fees.trade_fee_rate,
                    fees.protocol_fee_rate,
                    fees.fund_fee_rate,
                )
                .min_amount_out
            },
        };

        let output_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            if is_wsol {
                &crate::constants::WSOL_TOKEN_ACCOUNT
            } else {
                &crate::constants::USDC_TOKEN_ACCOUNT
            },
            get_input_token_program(is_wsol),
            params.open_seed_optimize,
        );
        let input_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            &params.input_mint,
            &mint_token_program,
            params.open_seed_optimize,
        );

        let output_vault_account = get_vault_account(
            &pool_state,
            if is_wsol {
                &crate::constants::WSOL_TOKEN_ACCOUNT
            } else {
                &crate::constants::USDC_TOKEN_ACCOUNT
            },
            protocol_params,
        );
        let input_vault_account =
            get_vault_account(&pool_state, &params.input_mint, protocol_params);

        let observation_state_account = if protocol_params.observation_state == Pubkey::default() {
            get_observation_state_pda(&pool_state).unwrap()
        } else {
            protocol_params.observation_state
        };

        // ========================================
        // Build instructions
        // ========================================
        let mut instructions = Vec::with_capacity(3);

        if params.create_output_mint_ata {
            instructions.extend(crate::trading::common::create_wsol_ata(&params.payer.pubkey()));
        }

        // Create sell instruction
        let output_token_program_meta =
            AccountMeta::new_readonly(*get_input_token_program(is_wsol), false);
        let accounts: [AccountMeta; 13] = [
            AccountMeta::new(params.payer.pubkey(), true), // Payer (signer)
            accounts::AUTHORITY_META,                      // Authority (readonly)
            AccountMeta::new(protocol_params.amm_config, false), // Amm Config (readonly)
            AccountMeta::new(pool_state, false),           // Pool State
            AccountMeta::new(input_token_account, false),  // Input Token Account
            AccountMeta::new(output_token_account, false), // Output Token Account
            AccountMeta::new(input_vault_account, false),  // Input Vault Account
            AccountMeta::new(output_vault_account, false), // Output Vault Account
            AccountMeta::new_readonly(mint_token_program, false), // Input Token Program (readonly)
            output_token_program_meta,                     // Output Token Program (readonly)
            AccountMeta::new_readonly(params.input_mint, false), // Input token mint (readonly)
            if is_wsol {
                crate::constants::WSOL_TOKEN_ACCOUNT_META
            } else {
                crate::constants::USDC_TOKEN_ACCOUNT_META
            }, // Output token mint (readonly)
            AccountMeta::new(observation_state_account, false), // Observation State Account
        ];
        // Create instruction data
        let mut data = [0u8; 24];
        data[..8].copy_from_slice(SWAP_BASE_IN_DISCRIMINATOR);
        data[8..16].copy_from_slice(&params.input_amount.unwrap_or(0).to_le_bytes());
        data[16..24].copy_from_slice(&minimum_amount_out.to_le_bytes());

        instructions.push(Instruction::new_with_bytes(
            accounts::RAYDIUM_CPMM,
            &data,
            accounts.to_vec(),
        ));

        if params.close_output_mint_ata {
            // Close wSOL ATA account, reclaim rent
            instructions.extend(crate::trading::common::close_wsol(&params.payer.pubkey()));
        }
        if params.close_input_mint_ata {
            instructions.push(crate::common::spl_token::close_account(
                &mint_token_program,
                &input_token_account,
                &params.payer.pubkey(),
                &params.payer.pubkey(),
                &[&params.payer.pubkey()],
            )?);
        }

        Ok(instructions)
    }
}
