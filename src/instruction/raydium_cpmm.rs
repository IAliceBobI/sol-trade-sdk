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

/// ⚠️ 重要说明：Raydium CPMM 指令构建逻辑
///
/// ## 当前实现（老版本逻辑）
/// 参考：git rev fcf947bfa3d57d0927239fc3de9a5519c5a0f002
///
/// 当前代码使用简化逻辑，**假设交易总是涉及 WSOL 或 USDC 作为输入或输出**：
/// - 买入：输入固定为 WSOL/USDC，输出为目标代币
/// - 卖出：输入为目标代币，输出固定为 WSOL/USDC
/// - Token Program 固定为 TOKEN_PROGRAM（不支持 Token-2022）
///
/// ### 买入逻辑示例：
/// ```rust
/// // 假设：如果 base_mint 是 WSOL/USDC，则输入是 base
/// let is_base_in = protocol_params.base_mint == WSOL/USDC;
/// // 输入 token 固定为 WSOL/USDC
/// // 输出 token 为目标代币
/// ```
///
/// ### 卖出逻辑示例：
/// ```rust
/// // 假设：如果 quote_mint 是 WSOL/USDC，则输出是 quote
/// let is_quote_out = protocol_params.quote_mint == WSOL/USDC;
/// // 输入 token 为目标代币
/// // 输出 token 固定为 WSOL/USDC
/// ```
///
/// ## 这种逻辑的限制
/// 1. 只支持 WSOL/USDC 作为基础货币
/// 2. 不支持代币到代币的直接交换（必须经过 WSOL/USDC）
/// 3. 不支持 Token-2022 程序
/// 4. 无法灵活处理任意代币对的 swap
///
/// ## 未来改进方向（Swap Output 估算）
/// 后续需要支持更通用的 swap output 估算功能时，需要修改为：
///
/// ### 1. 动态输入/输出检测
/// ```rust
/// // 买入：根据实际输入代币判断
/// let is_base_in = params.input_mint == protocol_params.base_mint;
///
/// // 卖出：根据实际输出代币判断
/// let is_base_out = params.output_mint == protocol_params.base_mint;
/// ```
///
/// ### 2. 动态 Token Program 支持
/// ```rust
/// // 支持检测 Token-2022
/// let input_token_program = get_token_program_cached(&params.input_mint)
///     .unwrap_or(TOKEN_PROGRAM);
/// let output_token_program = get_token_program_cached(&params.output_mint)
///     .unwrap_or(TOKEN_PROGRAM);
/// ```
///
/// ### 3. 任意代币对支持
/// - 不再假设 WSOL/USDC 必须是输入或输出
/// - 支持代币到代币的直接交换
/// - 支持 Token-2022 程序
///
/// ## 修改影响范围
/// 修改逻辑时需要注意以下测试和代码：
/// - `verify_raydium_cpmm_exact_in_*` 测试（链上模拟验证）
/// - `verify_raydium_cpmm_exact_out_*` 测试（链上模拟验证）
/// - `test_raydium_cpmm_buy_sell_complete`（集成测试）
/// - 所有依赖 CPMM swap 的代码
///
/// ## 修改建议
/// 在修改为通用逻辑前，建议：
/// 1. 添加新的配置选项（如 `support_token_2022: bool`）
/// 2. 保持向后兼容（保留旧逻辑作为 fallback）
/// 3. 逐步迁移到新逻辑
/// 4. 完整测试所有支持的代币对
/// 5. 确保 verify 测试仍然通过
///
/// ## 参考代码
/// - 老版本：git rev fcf947bfa3d57d0927239fc3de9a5519c5a0f002
/// - 官方代码：/opt/projects/sol-trade-sdk/temp/dex/raydium-cp-swap/client/src/instructions/amm_instructions.rs
///
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
        // 🔴 需要修改：Swap Output 估算功能需要这里改为动态检测
        //
        // 当前逻辑（老版本）：假设 base_mint 是 WSOL/USDC，则输入是 base
        // 参考：git rev fcf947bfa3d57d0927239fc3de9a5519c5a0f002
        //
        // 未来逻辑：
        // let is_base_in = params.input_mint == protocol_params.base_mint;
        //
        // 影响：
        // - compute_swap_amount 的方向参数
        // - mint_token_program 的选择
        // - 输入/输出 token 账户的确定
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

        // 🔴 需要修改：Swap Output 估算功能需要这里改为动态检测
        //
        // 当前逻辑（老版本）：输入固定为 WSOL/USDC，使用 TOKEN_PROGRAM
        // 参考：git rev fcf947bfa3d57d0927239fc3de9a5519c5a0f002
        //
        // 未来逻辑：
        // let input_token_program = get_token_program_cached(&params.input_mint)?;
        // let input_token_account = get_associated_token_address_with_program_id_fast_use_seed(
        //     &params.payer.pubkey(),
        //     &params.input_mint,  // 使用实际输入代币
        //     &input_token_program,
        //     params.open_seed_optimize,
        // );
        let input_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            if is_wsol {
                &crate::constants::WSOL_TOKEN_ACCOUNT
            } else {
                &crate::constants::USDC_TOKEN_ACCOUNT
            },
            &crate::constants::TOKEN_PROGRAM,
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

        // 直接使用 protocol_params 中的 observation_state（来自 pool.observation_key）
        // 参考官方代码: /opt/projects/sol-trade-sdk/temp/dex/raydium-cp-swap/client/src/main.rs:572
        let observation_state_account = protocol_params.observation_state;

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
        // 🔴 需要修改：Swap Output 估算功能需要这里改为动态检测
        //
        // 当前逻辑（老版本）：Input Token Program 固定为 TOKEN_PROGRAM
        // 参考：git rev fcf947bfa3d57d0927239fc3de9a5519c5a0f002
        //
        // 未来逻辑：
        // AccountMeta::new_readonly(input_token_program, false),   // 动态输入程序
        // AccountMeta::new_readonly(output_token_program, false),  // 动态输出程序
        // AccountMeta::new_readonly(params.input_mint, false),    // 动态输入 mint
        // AccountMeta::new_readonly(params.output_mint, false),   // 动态输出 mint
        //
        // 影响：
        // - 支持 Token-2022 程序
        // - 支持任意代币对
        // - verify_raydium_cpmm_* 测试需要更新
        let accounts: [AccountMeta; 13] = [
            AccountMeta::new(params.payer.pubkey(), true), // Payer (signer)
            accounts::AUTHORITY_META,                      // Authority (readonly)
            AccountMeta::new(protocol_params.amm_config, false), // Amm Config (readonly)
            AccountMeta::new(pool_state, false),           // Pool State
            AccountMeta::new(input_token_account, false),  // Input Token Account
            AccountMeta::new(output_token_account, false), // Output Token Account
            AccountMeta::new(input_vault_account, false),  // Input Vault Account
            AccountMeta::new(output_vault_account, false), // Output Vault Account
            crate::constants::TOKEN_PROGRAM_META,          // Input Token Program (readonly)
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
        // 🔴 需要修改：Swap Output 估算功能需要这里改为动态检测
        //
        // 当前逻辑（老版本）：如果 quote_mint 是 WSOL/USDC，则输出是 quote
        // 参考：git rev fcf947bfa3d57d0927239fc3de9a5519c5a0f002
        //
        // 未来逻辑：
        // let is_base_out = params.output_mint == protocol_params.base_mint;
        //
        // 影响：
        // - compute_swap_amount 的方向参数
        // - mint_token_program 的选择
        // - 输入/输出 token 账户的确定
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

        // 老版本逻辑：输出固定为 WSOL/USDC，使用 TOKEN_PROGRAM
        // 参考：git rev fcf947bfa3d57d0927239fc3de9a5519c5a0f002
        let output_token_account = get_associated_token_address_with_program_id_fast_use_seed(
            &params.payer.pubkey(),
            if is_wsol {
                &crate::constants::WSOL_TOKEN_ACCOUNT
            } else {
                &crate::constants::USDC_TOKEN_ACCOUNT
            },
            &crate::constants::TOKEN_PROGRAM,
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

        // 直接使用 protocol_params 中的 observation_state（来自 pool.observation_key）
        // 参考官方代码: /opt/projects/sol-trade-sdk/temp/dex/raydium-cp-swap/client/src/main.rs:572
        let observation_state_account = protocol_params.observation_state;

        // ========================================
        // Build instructions
        // ========================================
        let mut instructions = Vec::with_capacity(3);

        if params.create_output_mint_ata {
            instructions.extend(crate::trading::common::create_wsol_ata(&params.payer.pubkey()));
        }

        // Create sell instruction
        // 🔴 需要修改：Swap Output 估算功能需要这里改为动态检测
        //
        // 当前逻辑（老版本）：Input Token Program 使用 mint_token_program，Output Token Program 固定为 TOKEN_PROGRAM
        // 参考：git rev fcf947bfa3d57d0927239fc3de9a5519c5a0f002
        //
        // 未来逻辑：
        // AccountMeta::new_readonly(input_token_program, false),   // 动态输入程序
        // AccountMeta::new_readonly(output_token_program, false),  // 动态输出程序
        // AccountMeta::new_readonly(params.input_mint, false),    // 动态输入 mint
        // AccountMeta::new_readonly(params.output_mint, false),   // 动态输出 mint
        //
        // 影响：
        // - 支持 Token-2022 程序
        // - 支持任意代币对
        // - verify_raydium_cpmm_* 测试需要更新
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
            crate::constants::TOKEN_PROGRAM_META,          // Output Token Program (readonly)
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
