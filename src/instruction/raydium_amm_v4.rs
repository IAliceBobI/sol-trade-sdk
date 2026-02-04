use crate::{
    constants::trade_consts::DEFAULT_SLIPPAGE,
    instruction::utils::raydium_amm_v4::{
        SWAP_BASE_IN_DISCRIMINATOR, SWAP_BASE_OUT_DISCRIMINATOR, accounts,
    },
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

/// 获取输入 Token（WSOL/USDC）的 Token Program
///
/// WSOL 固定使用 TOKEN_PROGRAM
/// USDC 自动检测（支持 Token-2022）
#[allow(dead_code)]
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

/// Instruction builder for Raydium AMM V4 (Raydium Liquidity Pool V4) protocol
///
/// Raydium AMM V4 使用恒定乘积公式（x * y = k）进行流动性提供和交易
/// 程序地址: 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
pub struct RaydiumAmmV4InstructionBuilder;

#[async_trait::async_trait]
impl InstructionBuilder for RaydiumAmmV4InstructionBuilder {
    async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
        // ========================================
        // Parameter validation and basic data preparation
        // ========================================
        // 检查是否为 exact_out 模式
        let _has_fixed_output = params.fixed_output_amount.is_some();

        // 对于 exact_in 模式，input_amount 是用户指定的输入量
        // 对于 exact_out 模式，input_amount 是通过 quote_exact_out 计算的所需输入量
        let amount = params.input_amount.ok_or_else(|| anyhow!("Input amount is required"))?;
        if amount == 0 {
            return Err(anyhow!("Amount cannot be zero"));
        }
        let input_amount = amount;

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
        // is_base_in: true = coin 作为输入 (Coin -> PC), false = pc 作为输入 (PC -> Coin)
        let is_base_in = params.input_mint == protocol_params.coin_mint;

        // 实时获取 Pool 状态（包含 Serum 相关账户和 PNL 信息）
        let amm_info = if let Some(ref rpc) = params.rpc {
            match crate::instruction::utils::raydium_amm_v4::get_pool_by_address(
                rpc,
                &protocol_params.amm,
            )
            .await
            {
                Ok(info) => Some(info),
                Err(e) => {
                    tracing::warn!("Failed to get pool info: {}", e);
                    None
                },
            }
        } else {
            None
        };

        // 如果储备金为 0 或有 Pool 状态，实时获取储备金（带 PNL 调整）
        let (coin_reserve, pc_reserve) = if protocol_params.coin_reserve == 0
            || protocol_params.pc_reserve == 0
            || amm_info.is_some()
        {
            if let Some(ref rpc) = params.rpc {
                match (
                    rpc.get_token_account_balance(&protocol_params.token_coin).await,
                    rpc.get_token_account_balance(&protocol_params.token_pc).await,
                ) {
                    (Ok(coin), Ok(pc)) => {
                        let mut coin_amt = coin.amount.parse::<u64>().unwrap_or(0);
                        let mut pc_amt = pc.amount.parse::<u64>().unwrap_or(0);

                        // 如果有 Pool 状态，应用 PNL 调整
                        if let Some(ref info) = amm_info {
                            // ========================================
                            // TODO: 支持 Orderbook Pool (status=1 INITIALIZED, status=6 ACTIVE)
                            // ========================================
                            // 当前只支持 SWAP_ONLY Pool (status=5)
                            //
                            // 对于启用 Orderbook 的 Pool:
                            // - 需要使用 SwapBaseIn (18个账户) 而非 SwapBaseInV2 (8个账户)
                            // - 需要解析 Serum Market 账户获取子账户地址
                            // - 参考: /opt/projects/sol-trade-sdk/temp/dex/raydium-amm/program/src/processor.rs:2210-2406
                            //
                            // 实现步骤:
                            // 1. 添加 serum_dex crate 依赖
                            // 2. 解析 Serum Market 账户数据 (见 /opt/projects/sol-trade-sdk/temp/dex/openbook-dex)
                            // 3. 提取 bids, asks, event_queue, coin_vault, pc_vault, vault_signer
                            // 4. 修改指令构建使用 SwapBaseIn
                            // ========================================
                            if info.status != 5 {
                                return Err(anyhow!(
                                    "Raydium AMM V4 Pool status is {} (only SWAP_ONLY=5 supported).\n\
                                    Pool with Orderbook enabled (INITIALIZED=1, ACTIVE=6) requires Serum Market parsing.\n\
n\
                                    TODO: Implement Serum Market support:\n\
                                    1. Add serum_dex crate dependency\n\
                                    2. Parse Serum Market account data\n\
                                    3. Use SwapBaseIn (18 accounts) instead of SwapBaseInV2\n\
                                    \n\
                                    Reference: /opt/projects/sol-trade-sdk/temp/dex/raydium-amm/program/src/instruction.rs:314-334\n\
                                    Reference: /opt/projects/sol-trade-sdk/temp/dex/raydium-amm/program/src/processor.rs:2210-2406",
                                    info.status
                                ));
                            }

                            coin_amt = coin_amt
                                .checked_sub(info.out_put.need_take_pnl_coin)
                                .unwrap_or(coin_amt);
                            pc_amt =
                                pc_amt.checked_sub(info.out_put.need_take_pnl_pc).unwrap_or(pc_amt);
                        }

                        (coin_amt, pc_amt)
                    },
                    _ => (protocol_params.coin_reserve, protocol_params.pc_reserve),
                }
            } else {
                (protocol_params.coin_reserve, protocol_params.pc_reserve)
            }
        } else {
            (protocol_params.coin_reserve, protocol_params.pc_reserve)
        };

        // 🔧 修复：使用已经解包的 input_amount
        let amount_in: u64 = input_amount;
        let swap_result = compute_swap_amount(
            coin_reserve,
            pc_reserve,
            is_base_in,
            amount_in,
            params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE),
        );
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => swap_result.min_amount_out,
        };

        // 获取输入 token 的 program（支持 Token-2022）
        let input_token_program = crate::utils::token::get_token_program_cached(&params.input_mint)
            .unwrap_or(crate::constants::TOKEN_PROGRAM);
        let user_source_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed(
                &params.payer.pubkey(),
                &params.input_mint,
                &input_token_program,
                params.open_seed_optimize,
            );
        // 获取输出 token 的 program（支持 Token-2022）
        let output_token_program =
            crate::utils::token::get_token_program_cached(&params.output_mint)
                .unwrap_or(crate::constants::TOKEN_PROGRAM);
        let user_destination_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed(
                &params.payer.pubkey(),
                &params.output_mint,
                &output_token_program,
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
                    &output_token_program,
                    params.open_seed_optimize,
                ),
            );
        }

        // Create buy instruction
        // 使用 amm_info 中的 Serum 相关账户（如果可用），否则使用默认值
        let (serum_program, serum_market, open_orders) = if let Some(ref info) = amm_info {
            (info.serum_dex, info.market, info.open_orders)
        } else {
            // 如果没有 Pool 状态，使用旧的逻辑（可能导致错误）
            (protocol_params.amm, protocol_params.amm, protocol_params.amm)
        };

        let accounts: [AccountMeta; 17] = [
            crate::constants::TOKEN_PROGRAM_META, // Token Program (readonly)
            AccountMeta::new(protocol_params.amm, false), // Amm
            accounts::AUTHORITY_META,             // Authority (readonly)
            AccountMeta::new(open_orders, false), // Amm Open Orders
            AccountMeta::new(protocol_params.token_coin, false), // Pool Coin Token Account
            AccountMeta::new(protocol_params.token_pc, false), // Pool Pc Token Account
            AccountMeta::new_readonly(serum_program, false), // Serum Program
            AccountMeta::new(serum_market, false), // Serum Market
            AccountMeta::new(serum_market, false), // Serum Bids (从 market 派生，这里简化处理)
            AccountMeta::new(serum_market, false), // Serum Asks (从 market 派生，这里简化处理)
            AccountMeta::new(serum_market, false), // Serum Event Queue (从 market 派生，这里简化处理)
            AccountMeta::new(serum_market, false), // Serum Coin Vault Account (从 market 派生，这里简化处理)
            AccountMeta::new(serum_market, false), // Serum Pc Vault Account (从 market 派生，这里简化处理)
            AccountMeta::new(serum_market, false), // Serum Vault Signer (从 market 派生，这里简化处理)
            AccountMeta::new(user_source_token_account, false), // User Source Token Account
            AccountMeta::new(user_destination_token_account, false), // User Destination Token Account
            AccountMeta::new(params.payer.pubkey(), true),           // User Source Owner
        ];
        // Create instruction data
        // 根据模式选择正确的指令类型和参数
        // - exact_in 模式：使用 SWAP_BASE_IN (固定输入，计算最小输出)
        //   参数: [amount_in, minimum_amount_out]
        // - exact_out 模式：使用 SWAP_BASE_OUT (固定输出，计算最大输入)
        //   参数: [max_amount_in, amount_out]
        let discriminator = if params.fixed_output_amount.is_some() {
            SWAP_BASE_OUT_DISCRIMINATOR
        } else {
            SWAP_BASE_IN_DISCRIMINATOR
        };

        // 在 exact_out 模式下，需要添加滑点缓冲到 max_amount_in
        // 因为链上会从 max_amount_in 中扣除费用后计算输出
        // 如果 max_amount_in 刚好等于计算值，扣除费用后可能无法得到足够的输出
        let (param1, param2) = if params.fixed_output_amount.is_some() {
            // exact_out: 添加滑点缓冲到 max_amount_in
            let slippage_bps = params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE);
            let max_amount_in = amount_in.saturating_add((amount_in * slippage_bps / 10000).max(1));
            (max_amount_in, minimum_amount_out)
        } else {
            // exact_in: 直接使用计算的值
            (amount_in, minimum_amount_out)
        };

        let mut data = [0u8; 17];
        data[..1].copy_from_slice(discriminator);
        data[1..9].copy_from_slice(&param1.to_le_bytes());
        data[9..17].copy_from_slice(&param2.to_le_bytes());

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
        // 检查是否为 exact_out 模式
        let has_fixed_output = params.fixed_output_amount.is_some();

        if !has_fixed_output {
            // exact_in 模式：需要 input_amount
            if params.input_amount.is_none_or(|a| a == 0) {
                return Err(anyhow!("Token amount is not set"));
            }
        }
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
        // is_base_in: true = coin 作为输入 (Coin -> PC), false = pc 作为输入 (PC -> Coin)
        let is_base_in = params.input_mint == protocol_params.coin_mint;

        // 实时获取 Pool 状态（包含 Serum 相关账户和 PNL 信息）
        let amm_info = if let Some(ref rpc) = params.rpc {
            match crate::instruction::utils::raydium_amm_v4::get_pool_by_address(
                rpc,
                &protocol_params.amm,
            )
            .await
            {
                Ok(info) => Some(info),
                Err(e) => {
                    tracing::warn!("Failed to get pool info: {}", e);
                    None
                },
            }
        } else {
            None
        };

        // 如果储备金为 0 或有 Pool 状态，实时获取储备金（带 PNL 调整）
        let (coin_reserve, pc_reserve) = if protocol_params.coin_reserve == 0
            || protocol_params.pc_reserve == 0
            || amm_info.is_some()
        {
            if let Some(ref rpc) = params.rpc {
                match (
                    rpc.get_token_account_balance(&protocol_params.token_coin).await,
                    rpc.get_token_account_balance(&protocol_params.token_pc).await,
                ) {
                    (Ok(coin), Ok(pc)) => {
                        let mut coin_amt = coin.amount.parse::<u64>().unwrap_or(0);
                        let mut pc_amt = pc.amount.parse::<u64>().unwrap_or(0);

                        // 如果有 Pool 状态，应用 PNL 调整
                        if let Some(ref info) = amm_info {
                            // ========================================
                            // TODO: 支持 Orderbook Pool (status=1 INITIALIZED, status=6 ACTIVE)
                            // ========================================
                            // 当前只支持 SWAP_ONLY Pool (status=5)
                            //
                            // 对于启用 Orderbook 的 Pool:
                            // - 需要使用 SwapBaseIn (18个账户) 而非 SwapBaseInV2 (8个账户)
                            // - 需要解析 Serum Market 账户获取子账户地址
                            // - 参考: /opt/projects/sol-trade-sdk/temp/dex/raydium-amm/program/src/processor.rs:2210-2406
                            //
                            // 实现步骤:
                            // 1. 添加 serum_dex crate 依赖
                            // 2. 解析 Serum Market 账户数据 (见 /opt/projects/sol-trade-sdk/temp/dex/openbook-dex)
                            // 3. 提取 bids, asks, event_queue, coin_vault, pc_vault, vault_signer
                            // 4. 修改指令构建使用 SwapBaseIn
                            // ========================================
                            if info.status != 5 {
                                return Err(anyhow!(
                                    "Raydium AMM V4 Pool status is {} (only SWAP_ONLY=5 supported).\n\
                                    Pool with Orderbook enabled (INITIALIZED=1, ACTIVE=6) requires Serum Market parsing.\n\
n\
                                    TODO: Implement Serum Market support:\n\
                                    1. Add serum_dex crate dependency\n\
                                    2. Parse Serum Market account data\n\
                                    3. Use SwapBaseIn (18 accounts) instead of SwapBaseInV2\n\
                                    \n\
                                    Reference: /opt/projects/sol-trade-sdk/temp/dex/raydium-amm/program/src/instruction.rs:314-334\n\
                                    Reference: /opt/projects/sol-trade-sdk/temp/dex/raydium-amm/program/src/processor.rs:2210-2406",
                                    info.status
                                ));
                            }

                            coin_amt = coin_amt
                                .checked_sub(info.out_put.need_take_pnl_coin)
                                .unwrap_or(coin_amt);
                            pc_amt =
                                pc_amt.checked_sub(info.out_put.need_take_pnl_pc).unwrap_or(pc_amt);
                        }

                        (coin_amt, pc_amt)
                    },
                    _ => (protocol_params.coin_reserve, protocol_params.pc_reserve),
                }
            } else {
                (protocol_params.coin_reserve, protocol_params.pc_reserve)
            }
        } else {
            (protocol_params.coin_reserve, protocol_params.pc_reserve)
        };

        let swap_result = compute_swap_amount(
            coin_reserve,
            pc_reserve,
            is_base_in,
            params.input_amount.unwrap_or(0),
            params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE),
        );
        let minimum_amount_out = match params.fixed_output_amount {
            Some(fixed) => fixed,
            None => swap_result.min_amount_out,
        };

        // 获取输入 token 的 program（支持 Token-2022）
        let input_token_program = crate::utils::token::get_token_program_cached(&params.input_mint)
            .unwrap_or(crate::constants::TOKEN_PROGRAM);
        let user_source_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed(
                &params.payer.pubkey(),
                &params.input_mint,
                &input_token_program,
                params.open_seed_optimize,
            );
        // 获取输出 token 的 program（支持 Token-2022）
        let output_token_program =
            crate::utils::token::get_token_program_cached(&params.output_mint)
                .unwrap_or(crate::constants::TOKEN_PROGRAM);
        let user_destination_token_account =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast_use_seed(
                &params.payer.pubkey(),
                &params.output_mint,
                &output_token_program,
                params.open_seed_optimize,
            );

        // ========================================
        // Build instructions
        // ========================================
        let mut instructions = Vec::with_capacity(3);

        if params.create_output_mint_ata {
            instructions.extend(crate::trading::common::create_wsol_ata(&params.payer.pubkey()));
        }

        // Create sell instruction
        // 使用 amm_info 中的 Serum 相关账户（如果可用），否则使用默认值
        let (serum_program, serum_market, open_orders) = if let Some(ref info) = amm_info {
            (info.serum_dex, info.market, info.open_orders)
        } else {
            // 如果没有 Pool 状态，使用旧的逻辑（可能导致错误）
            (protocol_params.amm, protocol_params.amm, protocol_params.amm)
        };

        let accounts: [AccountMeta; 17] = [
            crate::constants::TOKEN_PROGRAM_META, // Token Program (readonly)
            AccountMeta::new(protocol_params.amm, false), // Amm
            accounts::AUTHORITY_META,             // Authority (readonly)
            AccountMeta::new(open_orders, false), // Amm Open Orders
            AccountMeta::new(protocol_params.token_coin, false), // Pool Coin Token Account
            AccountMeta::new(protocol_params.token_pc, false), // Pool Pc Token Account
            AccountMeta::new_readonly(serum_program, false), // Serum Program
            AccountMeta::new(serum_market, false), // Serum Market
            AccountMeta::new(serum_market, false), // Serum Bids (从 market 派生，这里简化处理)
            AccountMeta::new(serum_market, false), // Serum Asks (从 market 派生，这里简化处理)
            AccountMeta::new(serum_market, false), // Serum Event Queue (从 market 派生，这里简化处理)
            AccountMeta::new(serum_market, false), // Serum Coin Vault Account (从 market 派生，这里简化处理)
            AccountMeta::new(serum_market, false), // Serum Pc Vault Account (从 market 派生，这里简化处理)
            AccountMeta::new(serum_market, false), // Serum Vault Signer (从 market 派生，这里简化处理)
            AccountMeta::new(user_source_token_account, false), // User Source Token Account
            AccountMeta::new(user_destination_token_account, false), // User Destination Token Account
            AccountMeta::new(params.payer.pubkey(), true),           // User Source Owner
        ];
        // Create instruction data
        // 根据模式选择正确的指令类型和参数
        // - exact_in 模式：使用 SWAP_BASE_IN (固定输入，计算最小输出)
        //   参数: [amount_in, minimum_amount_out]
        // - exact_out 模式：使用 SWAP_BASE_OUT (固定输出，计算最大输入)
        //   参数: [max_amount_in, amount_out]
        let discriminator = if params.fixed_output_amount.is_some() {
            SWAP_BASE_OUT_DISCRIMINATOR
        } else {
            SWAP_BASE_IN_DISCRIMINATOR
        };

        let amount_in = params.input_amount.unwrap_or(0);

        // 在 exact_out 模式下，需要添加滑点缓冲到 max_amount_in
        let (param1, param2) = if params.fixed_output_amount.is_some() {
            // exact_out: 添加滑点缓冲到 max_amount_in
            let slippage_bps = params.slippage_basis_points.unwrap_or(DEFAULT_SLIPPAGE);
            let max_amount_in = amount_in.saturating_add((amount_in * slippage_bps / 10000).max(1));
            (max_amount_in, minimum_amount_out)
        } else {
            // exact_in: 直接使用计算的值
            (amount_in, minimum_amount_out)
        };

        let mut data = [0u8; 17];
        data[..1].copy_from_slice(discriminator);
        data[1..9].copy_from_slice(&param1.to_le_bytes());
        data[9..17].copy_from_slice(&param2.to_le_bytes());

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
