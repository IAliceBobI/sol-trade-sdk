// 允许在此模块中使用 unwrap，因为：
// 1. PDA 地址计算对有效输入不应该失败
// 2. 参数在调用前已经过验证
#![allow(clippy::unwrap_used)]

use crate::{
    common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed,
    constants::trade_consts::DEFAULT_SLIPPAGE,
    instruction::utils::raydium_cpmm::{
        SWAP_BASE_IN_DISCRIMINATOR, accounts, get_amm_config_fees, get_pool_pda, get_vault_account,
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

// ==================== 辅助函数 ====================

/// 根据 mint 地址从 Pool 参数获取对应的 Token Program
///
/// 直接通过 mint 匹配，避免复杂的方向判断
fn get_token_program_for_mint(mint: &Pubkey, params: &RaydiumCpmmParams) -> Pubkey {
    if mint == &params.base_mint {
        params.base_token_program
    } else if mint == &params.quote_mint {
        params.quote_token_program
    } else {
        panic!(
            "Mint {} 不在 Pool 中 (base={}, quote={})",
            mint, params.base_mint, params.quote_mint
        )
    }
}

// ==================== Instruction Builder ====================

/// Instruction builder for RaydiumCpmm protocol
pub struct RaydiumCpmmInstructionBuilder;

#[async_trait::async_trait]
impl InstructionBuilder for RaydiumCpmmInstructionBuilder {
    async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // ========================================
        // 参数验证和基础准备
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

        // 验证 Pool 包含 WSOL 或 USDC
        let is_wsol = protocol_params.base_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.quote_mint == crate::constants::WSOL_TOKEN_ACCOUNT;
        let is_usdc = protocol_params.base_mint == crate::constants::USDC_TOKEN_ACCOUNT
            || protocol_params.quote_mint == crate::constants::USDC_TOKEN_ACCOUNT;

        if !is_wsol && !is_usdc {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        }

        // ========================================
        // Token Program 和 ATA 计算（简化版）
        // ========================================
        // 处理 SOL_TOKEN_ACCOUNT 和 WSOL_TOKEN_ACCOUNT 的标准化
        // 必须在获取 Token Program 之前进行标准化，因为 Pool 中使用 WSOL_TOKEN_ACCOUNT
        let normalized_input_mint = if params.input_mint == crate::constants::SOL_TOKEN_ACCOUNT {
            crate::constants::WSOL_TOKEN_ACCOUNT
        } else {
            params.input_mint
        };

        let normalized_output_mint = if params.output_mint == crate::constants::SOL_TOKEN_ACCOUNT {
            crate::constants::WSOL_TOKEN_ACCOUNT
        } else {
            params.output_mint
        };

        // 直接通过 mint 匹配获取 Token Program（使用标准化后的 mint）
        let input_token_program =
            get_token_program_for_mint(&normalized_input_mint, protocol_params);
        let output_token_program =
            get_token_program_for_mint(&normalized_output_mint, protocol_params);

        // Swap 方向：输入是否为 base token（使用标准化后的 mint）
        let is_base_in = normalized_input_mint == protocol_params.base_mint;

        // 计算 ATA
        let input_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
                &params.payer.pubkey(),
                &normalized_input_mint,
                &input_token_program,
            );

        let output_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            &params.output_mint,
            &output_token_program,
            params.open_seed_optimize,
        );

        let input_vault_account =
            get_vault_account(&pool_state, &normalized_input_mint, protocol_params);
        let output_vault_account =
            get_vault_account(&pool_state, &normalized_output_mint, protocol_params);

        // ========================================
        // Swap 数量计算
        // ========================================
        let fees = match params.rpc.as_ref() {
            Some(rpc) => get_amm_config_fees(rpc, &protocol_params.amm_config).await?,
            None => return Err(anyhow!("RPC client is required for fee calculation")),
        };

        let result = compute_swap_amount(
            protocol_params.base_reserve,
            protocol_params.quote_reserve,
            is_base_in,
            input_amount,
            params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE),
            fees.trade_fee_rate,
            fees.protocol_fee_rate,
            fees.fund_fee_rate,
        );

        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => result.min_amount_out,
        };

        // ========================================
        // 构建指令
        // ========================================
        let mut instructions = Vec::with_capacity(6);

        // 创建输入 token ATA（如果需要）
        if params.create_input_mint_ata {
            instructions
                .extend(crate::trading::common::handle_wsol(&params.payer.pubkey(), input_amount));
        }

        // 创建输出 token ATA（如果需要）
        if params.create_output_mint_ata {
            instructions.extend(
                crate::common::fast_fn::create_associated_token_account_idempotent_fast_use_seed(
                    &params.payer.pubkey(),
                    &params.payer.pubkey(),
                    &params.output_mint,
                    &output_token_program,
                    params.open_seed_optimize,
                ),
            );
        }

        // Swap 指令
        let accounts: [AccountMeta; 13] = [
            AccountMeta::new(params.payer.pubkey(), true), // Payer
            accounts::AUTHORITY_META,                      // Authority
            AccountMeta::new(protocol_params.amm_config, false), // Amm Config
            AccountMeta::new(pool_state, false),           // Pool State
            AccountMeta::new(input_token_account, false),  // Input Token Account
            AccountMeta::new(output_token_account, false), // Output Token Account
            AccountMeta::new(input_vault_account, false),  // Input Vault
            AccountMeta::new(output_vault_account, false), // Output Vault
            AccountMeta::new_readonly(input_token_program, false), // Input Token Program
            AccountMeta::new_readonly(output_token_program, false), // Output Token Program
            AccountMeta::new_readonly(normalized_input_mint, false), // Input Mint (使用标准化后的地址)
            AccountMeta::new_readonly(normalized_output_mint, false), // Output Mint (使用标准化后的地址)
            AccountMeta::new(protocol_params.observation_state, false), // Observation State
        ];

        let mut data = [0u8; 24];
        data[..8].copy_from_slice(SWAP_BASE_IN_DISCRIMINATOR);
        data[8..16].copy_from_slice(&input_amount.to_le_bytes());
        data[16..24].copy_from_slice(&minimum_amount_out.to_le_bytes());

        instructions.push(Instruction::new_with_bytes(
            accounts::RAYDIUM_CPMM,
            &data,
            accounts.to_vec(),
        ));

        // 关闭输入 token ATA（如果需要）
        if params.close_input_mint_ata {
            instructions.extend(crate::trading::common::close_wsol(&params.payer.pubkey()));
        }

        Ok(instructions)
    }

    async fn build_sell_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // ========================================
        // 参数验证和基础准备
        // ========================================
        let protocol_params = params
            .protocol_params
            .as_any()
            .downcast_ref::<RaydiumCpmmParams>()
            .ok_or_else(|| anyhow!("Invalid protocol params for RaydiumCpmm"))?;

        let has_fixed_output = params.fixed_output_amount.is_some();

        if !has_fixed_output && params.input_amount.is_none_or(|a| a == 0) {
            return Err(anyhow!("Token amount is not set"));
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

        // 验证 Pool 包含 WSOL 或 USDC
        let is_wsol = protocol_params.base_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.quote_mint == crate::constants::WSOL_TOKEN_ACCOUNT;
        let is_usdc = protocol_params.base_mint == crate::constants::USDC_TOKEN_ACCOUNT
            || protocol_params.quote_mint == crate::constants::USDC_TOKEN_ACCOUNT;

        if !is_wsol && !is_usdc {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        }

        // ========================================
        // Token Program 和 ATA 计算（简化版）
        // ========================================
        // 处理 SOL_TOKEN_ACCOUNT 和 WSOL_TOKEN_ACCOUNT 的标准化
        // 必须在获取 Token Program 之前进行标准化，因为 Pool 中使用 WSOL_TOKEN_ACCOUNT
        let normalized_input_mint = if params.input_mint == crate::constants::SOL_TOKEN_ACCOUNT {
            crate::constants::WSOL_TOKEN_ACCOUNT
        } else {
            params.input_mint
        };

        let normalized_output_mint = if params.output_mint == crate::constants::SOL_TOKEN_ACCOUNT {
            crate::constants::WSOL_TOKEN_ACCOUNT
        } else {
            params.output_mint
        };

        // 直接通过 mint 匹配获取 Token Program（使用标准化后的 mint）
        let input_token_program =
            get_token_program_for_mint(&normalized_input_mint, protocol_params);
        let output_token_program =
            get_token_program_for_mint(&normalized_output_mint, protocol_params);

        // Swap 方向：输入是否为 base token（使用标准化后的 mint）
        let is_base_in = normalized_input_mint == protocol_params.base_mint;

        // 计算 ATA
        let input_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            &normalized_input_mint,
            &input_token_program,
            params.open_seed_optimize,
        );

        let output_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            &normalized_output_mint,
            &output_token_program,
            params.open_seed_optimize,
        );

        let input_vault_account =
            get_vault_account(&pool_state, &normalized_input_mint, protocol_params);
        let output_vault_account =
            get_vault_account(&pool_state, &normalized_output_mint, protocol_params);

        // ========================================
        // Swap 数量计算
        // ========================================
        let minimum_amount_out: u64 = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => {
                let fees = match params.rpc.as_ref() {
                    Some(rpc) => get_amm_config_fees(rpc, &protocol_params.amm_config).await?,
                    None => return Err(anyhow!("RPC client is required for fee calculation")),
                };

                compute_swap_amount(
                    protocol_params.base_reserve,
                    protocol_params.quote_reserve,
                    is_base_in,
                    params.input_amount.unwrap_or(0),
                    params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE),
                    fees.trade_fee_rate,
                    fees.protocol_fee_rate,
                    fees.fund_fee_rate,
                )
                .min_amount_out
            },
        };

        // ========================================
        // 构建指令
        // ========================================
        let mut instructions = Vec::with_capacity(3);

        // 创建输出 token ATA（如果需要）
        if params.create_output_mint_ata {
            if is_wsol {
                instructions
                    .extend(crate::trading::common::create_wsol_ata(&params.payer.pubkey()));
            } else if is_usdc {
                // USDC ATA 创建使用对应的 Token Program
                instructions.extend(
                    crate::common::fast_fn::create_associated_token_account_idempotent_fast_use_seed(
                        &params.payer.pubkey(),
                        &params.payer.pubkey(),
                        &crate::constants::USDC_TOKEN_ACCOUNT,
                        &output_token_program,
                        params.open_seed_optimize,
                    ),
                );
            }
        }

        // Swap 指令
        let accounts: [AccountMeta; 13] = [
            AccountMeta::new(params.payer.pubkey(), true), // Payer
            accounts::AUTHORITY_META,                      // Authority
            AccountMeta::new(protocol_params.amm_config, false), // Amm Config
            AccountMeta::new(pool_state, false),           // Pool State
            AccountMeta::new(input_token_account, false),  // Input Token Account
            AccountMeta::new(output_token_account, false), // Output Token Account
            AccountMeta::new(input_vault_account, false),  // Input Vault
            AccountMeta::new(output_vault_account, false), // Output Vault
            AccountMeta::new_readonly(input_token_program, false), // Input Token Program
            AccountMeta::new_readonly(output_token_program, false), // Output Token Program
            AccountMeta::new_readonly(normalized_input_mint, false), // Input Mint (使用标准化后的地址)
            AccountMeta::new_readonly(normalized_output_mint, false), // Output Mint (使用标准化后的地址)
            AccountMeta::new(protocol_params.observation_state, false), // Observation State
        ];

        let mut data = [0u8; 24];
        data[..8].copy_from_slice(SWAP_BASE_IN_DISCRIMINATOR);
        data[8..16].copy_from_slice(&params.input_amount.unwrap_or(0).to_le_bytes());
        data[16..24].copy_from_slice(&minimum_amount_out.to_le_bytes());

        instructions.push(Instruction::new_with_bytes(
            accounts::RAYDIUM_CPMM,
            &data,
            accounts.to_vec(),
        ));

        // 关闭输出 token ATA（如果需要）
        if params.close_output_mint_ata {
            instructions.extend(crate::trading::common::close_wsol(&params.payer.pubkey()));
        }

        // 关闭输入 token ATA（如果需要）
        if params.close_input_mint_ata {
            instructions.push(crate::common::spl_token::close_account(
                &input_token_program,
                &input_token_account,
                &params.payer.pubkey(),
                &params.payer.pubkey(),
                &[&params.payer.pubkey()],
            )?);
        }

        Ok(instructions)
    }
}
