use crate::{
    constants::trade_consts::DEFAULT_SLIPPAGE,
    instruction::utils::raydium_amm_v4::{SWAP_BASE_IN_DISCRIMINATOR, accounts},
    trading::core::{
        params::{RaydiumAmmV4Params, SwapParams},
        traits::InstructionBuilder,
    },
    utils::calc::raydium_amm_v4::compute_swap_amount,
};
use anyhow::{Result, anyhow};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signer::Signer,
};

/// Instruction builder for Raydium AMM V4 (Raydium Liquidity Pool V4) protocol
///
/// Raydium AMM V4 使用恒定乘积公式（x * y = k）进行流动性提供和交易
/// 程序地址: 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
///
/// # 参考源码
/// - SwapBaseIn 指令定义: temp/raydium-amm/program/src/instruction.rs:72-93
/// - SwapBaseIn 处理逻辑: temp/raydium-amm/program/src/processor.rs:2210-2406
/// - 账户布局 (17 个账户): temp/raydium-amm/program/src/instruction.rs (指令文档)
///
/// # Serum Market 子账户
/// 当前实现使用 `amm` 地址作为所有 Serum 子账户的占位符
/// 对于大多数 Pool 这种方式可以正常工作
///
/// # 完整 Serum Market 支持（可选）
/// 如需支持更复杂的 Orderbook Pool，可使用 serum_market.rs 解析器：
/// - Market 结构: temp/serum-dex/dex/src/state.rs:293-343
/// - 解析函数: parse_market_account()
/// - Nonce 解析: parse_vault_signer_nonce()
pub struct RaydiumAmmV4InstructionBuilder;

#[async_trait::async_trait]
impl InstructionBuilder for RaydiumAmmV4InstructionBuilder {
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
            .downcast_ref::<RaydiumAmmV4Params>()
            .ok_or_else(|| anyhow!("Invalid protocol params for RaydiumCpmm"))?;

        let is_wsol = protocol_params.coin_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.pc_mint == crate::constants::WSOL_TOKEN_ACCOUNT;

        let is_usdc = protocol_params.coin_mint == crate::constants::USDC_TOKEN_ACCOUNT
            || protocol_params.pc_mint == crate::constants::USDC_TOKEN_ACCOUNT;

        if !is_wsol && !is_usdc {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        }

        // ========================================
        // Trade calculation and account address preparation
        // ========================================
        let is_base_in = protocol_params.coin_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.coin_mint == crate::constants::USDC_TOKEN_ACCOUNT;
        // 🔧 修复：使用已经解包的 input_amount
        let amount_in: u64 = input_amount;
        let slippage = params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE);
        let swap_result = compute_swap_amount(
            protocol_params.coin_reserve,
            protocol_params.pc_reserve,
            is_base_in,
            amount_in,
            slippage,
        );
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => {
                // Exact Out 模式下应用滑点：允许实际输出比期望输出少一定的百分比
                // 使用 u128 避免乘法溢出
                let min_out = (fixed as u128 * (10_000 - slippage) as u128 / 10_000) as u64;
                // 确保 min_out 不会为 0（当 fixed 很小时）
                if min_out == 0 && fixed > 0 { fixed } else { min_out }
            },
            None => swap_result.min_amount_out,
        };

        let user_source_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed(
                &params.payer.pubkey(),
                if is_wsol {
                    &crate::constants::WSOL_TOKEN_ACCOUNT
                } else {
                    &crate::constants::USDC_TOKEN_ACCOUNT
                },
                &crate::constants::TOKEN_PROGRAM,
                params.open_seed_optimize,
            );
        let user_destination_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed(
                &params.payer.pubkey(),
                &params.output_mint,
                &crate::constants::TOKEN_PROGRAM,
                params.open_seed_optimize,
            );

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
                    &crate::constants::TOKEN_PROGRAM,
                    params.open_seed_optimize,
                ),
            );
        }

        // Create buy instruction
        let accounts: [AccountMeta; 17] = [
            crate::constants::TOKEN_PROGRAM_META, // Token Program (readonly)
            AccountMeta::new(protocol_params.amm, false), // Amm
            accounts::AUTHORITY_META,             // Authority (readonly)
            AccountMeta::new(protocol_params.amm, false), // Amm Open Orders
            AccountMeta::new(protocol_params.token_coin, false), // Pool Coin Token Account
            AccountMeta::new(protocol_params.token_pc, false), // Pool Pc Token Account
            AccountMeta::new(protocol_params.amm, false), // Serum Program
            AccountMeta::new(protocol_params.amm, false), // Serum Market
            AccountMeta::new(protocol_params.amm, false), // Serum Bids
            AccountMeta::new(protocol_params.amm, false), // Serum Asks
            AccountMeta::new(protocol_params.amm, false), // Serum Event Queue
            AccountMeta::new(protocol_params.amm, false), // Serum Coin Vault Account
            AccountMeta::new(protocol_params.amm, false), // Serum Pc Vault Account
            AccountMeta::new(protocol_params.amm, false), // Serum Vault Signer
            AccountMeta::new(user_source_token_account, false), // User Source Token Account
            AccountMeta::new(user_destination_token_account, false), // User Destination Token Account
            AccountMeta::new(params.payer.pubkey(), true),           // User Source Owner
        ];
        // Create instruction data
        let mut data = [0u8; 17];
        data[..1].copy_from_slice(SWAP_BASE_IN_DISCRIMINATOR);
        data[1..9].copy_from_slice(&amount_in.to_le_bytes());
        data[9..17].copy_from_slice(&minimum_amount_out.to_le_bytes());

        instructions.push(Instruction::new_with_bytes(
            accounts::RAYDIUM_AMM_V4,
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
            .downcast_ref::<RaydiumAmmV4Params>()
            .ok_or_else(|| anyhow!("Invalid protocol params for RaydiumCpmm"))?;

        // 🔧 修复：改进 Option 检查的清晰度
        if params.input_amount.is_none_or(|a| a == 0) {
            return Err(anyhow!("Token amount is not set"));
        }

        let is_wsol = protocol_params.coin_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.pc_mint == crate::constants::WSOL_TOKEN_ACCOUNT;

        let is_usdc = protocol_params.coin_mint == crate::constants::USDC_TOKEN_ACCOUNT
            || protocol_params.pc_mint == crate::constants::USDC_TOKEN_ACCOUNT;

        if !is_wsol && !is_usdc {
            return Err(anyhow!("Pool must contain WSOL or USDC"));
        }

        // ========================================
        // Trade calculation and account address preparation
        // ========================================
        let is_base_in = protocol_params.pc_mint == crate::constants::WSOL_TOKEN_ACCOUNT
            || protocol_params.pc_mint == crate::constants::USDC_TOKEN_ACCOUNT;
        let slippage = params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE);
        let swap_result = compute_swap_amount(
            protocol_params.coin_reserve,
            protocol_params.pc_reserve,
            is_base_in,
            params.input_amount.unwrap_or(0),
            slippage,
        );
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => {
                // Exact Out 模式下应用滑点：允许实际输出比期望输出少一定的百分比
                // 使用 u128 避免乘法溢出
                let min_out = (fixed as u128 * (10_000 - slippage) as u128 / 10_000) as u64;
                // 确保 min_out 不会为 0（当 fixed 很小时）
                if min_out == 0 && fixed > 0 { fixed } else { min_out }
            },
            None => swap_result.min_amount_out,
        };

        let user_source_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed(
                &params.payer.pubkey(),
                &params.input_mint,
                &crate::constants::TOKEN_PROGRAM,
                params.open_seed_optimize,
            );
        let user_destination_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed(
                &params.payer.pubkey(),
                if is_wsol {
                    &crate::constants::WSOL_TOKEN_ACCOUNT
                } else {
                    &crate::constants::USDC_TOKEN_ACCOUNT
                },
                &crate::constants::TOKEN_PROGRAM,
                params.open_seed_optimize,
            );

        // ========================================
        // Build instructions
        // ========================================
        let mut instructions = Vec::with_capacity(3);

        if params.create_output_mint_ata {
            instructions.extend(crate::trading::common::create_wsol_ata(&params.payer.pubkey()));
        }

        // Create buy instruction
        let accounts: [AccountMeta; 17] = [
            crate::constants::TOKEN_PROGRAM_META, // Token Program (readonly)
            AccountMeta::new(protocol_params.amm, false), // Amm
            accounts::AUTHORITY_META,             // Authority (readonly)
            AccountMeta::new(protocol_params.amm, false), // Amm Open Orders
            AccountMeta::new(protocol_params.token_coin, false), // Pool Coin Token Account
            AccountMeta::new(protocol_params.token_pc, false), // Pool Pc Token Account
            AccountMeta::new(protocol_params.amm, false), // Serum Program
            AccountMeta::new(protocol_params.amm, false), // Serum Market
            AccountMeta::new(protocol_params.amm, false), // Serum Bids
            AccountMeta::new(protocol_params.amm, false), // Serum Asks
            AccountMeta::new(protocol_params.amm, false), // Serum Event Queue
            AccountMeta::new(protocol_params.amm, false), // Serum Coin Vault Account
            AccountMeta::new(protocol_params.amm, false), // Serum Pc Vault Account
            AccountMeta::new(protocol_params.amm, false), // Serum Vault Signer
            AccountMeta::new(user_source_token_account, false), // User Source Token Account
            AccountMeta::new(user_destination_token_account, false), // User Destination Token Account
            AccountMeta::new(params.payer.pubkey(), true),           // User Source Owner
        ];
        // Create instruction data
        let mut data = [0u8; 17];
        data[..1].copy_from_slice(SWAP_BASE_IN_DISCRIMINATOR);
        data[1..9].copy_from_slice(&params.input_amount.unwrap_or(0).to_le_bytes());
        data[9..17].copy_from_slice(&minimum_amount_out.to_le_bytes());

        instructions.push(Instruction::new_with_bytes(
            accounts::RAYDIUM_AMM_V4,
            &data,
            accounts.to_vec(),
        ));

        if params.close_output_mint_ata {
            instructions.extend(crate::trading::common::close_wsol(&params.payer.pubkey()));
        }
        if params.close_input_mint_ata {
            instructions.push(crate::common::spl_token::close_account(
                &crate::constants::TOKEN_PROGRAM,
                &user_source_token_account,
                &params.payer.pubkey(),
                &params.payer.pubkey(),
                &[&params.payer.pubkey()],
            )?);
        }

        Ok(instructions)
    }
}
