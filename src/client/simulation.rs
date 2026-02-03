//! TradingClient 模拟相关方法

use super::helpers::{get_input_mint, get_output_mint};
use super::types::{TradingClient, TradeBuyParams, TradeSellParams};
use crate::trading::core::params::SwapParams;
use crate::trading::core::traits::InstructionBuilder;
use crate::{DexType, SimulationResult, TradeTokenType, UnifiedResult, UnifiedTradingError};
use solana_sdk::signature::Signer;

impl TradingClient {
    /// 链上模拟（准确验证）
    ///
    /// 通过链上模拟提供准确的交易结果，不发送真实交易。
    /// 支持所有 DEX。
    ///
    /// # 参数
    ///
    /// * `params` - 交易参数
    ///
    /// # 返回
    ///
    /// 返回 `SimulationResult` 包含模拟的输出金额、CU 消耗、交易费用等
    pub async fn buy_simulate(&self, params: TradeBuyParams) -> UnifiedResult<SimulationResult> {
        // 1. 参数验证（支持 exact_in 和 exact_out）
        if let Some(fixed_output) = params.fixed_output_token_amount {
            // exact_out 模式验证
            if fixed_output == 0 {
                return Err(UnifiedTradingError::InvalidParameters(
                    "fixed_output_token_amount must be > 0".into(),
                ));
            }
        } else {
            // exact_in 模式验证（现有逻辑）
            if params.input_token_amount == 0 {
                return Err(UnifiedTradingError::InvalidParameters("amount must be > 0".into()));
            }
        }

        if params.input_token_type == TradeTokenType::USD1 && params.dex_type != DexType::Bonk {
            return Err(UnifiedTradingError::InvalidParameters(
                "USD1 only supported on Bonk".into(),
            ));
        }

        // 2. 获取 input_mint
        let input_mint = get_input_mint(&params.input_token_type);

        // 3. 构建 SwapParams（完全复用 buy 中的逻辑）
        let protocol_params = params.extension_params;

        let swap_params = SwapParams {
            rpc: Some(self.rpc.clone()),
            payer: self.payer.clone(),
            trade_type: crate::swqos::TradeType::Buy,
            input_mint,
            output_mint: params.mint,
            input_token_program: None,
            output_token_program: None,
            input_amount: Some(params.input_token_amount),
            slippage_basis_points: params.slippage_basis_points,
            address_lookup_table_account: params.address_lookup_table_account,
            recent_blockhash: params.recent_blockhash,
            wait_transaction_confirmed: false, // 模拟不需要等待确认
            protocol_params: protocol_params.clone(),
            open_seed_optimize: self.use_seed_optimize,
            swqos_clients: self.swqos_clients.clone(),
            middleware_manager: self.middleware_manager.clone(),
            durable_nonce: params.durable_nonce,
            with_tip: true,
            create_input_mint_ata: params.create_input_token_ata,
            close_input_mint_ata: params.close_input_token_ata,
            create_output_mint_ata: params.create_mint_ata,
            close_output_mint_ata: false,
            fixed_output_amount: params.fixed_output_token_amount,
            gas_fee_strategy: params.gas_fee_strategy,
            simulate: true, // 关键：设置模拟模式
            on_transaction_signed: None,
            callback_execution_mode: None,
            enable_jito_sandwich_protection: None,
        };

        // 4. 构建指令（根据 DEX 类型使用对应的 InstructionBuilder）
        let instructions = match params.dex_type {
            DexType::RaydiumClmm => {
                crate::instruction::raydium_clmm::RaydiumClmmInstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::RaydiumCpmm => {
                crate::instruction::raydium_cpmm::RaydiumCpmmInstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::RaydiumAmmV4 => {
                crate::instruction::raydium_amm_v4::RaydiumAmmV4InstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::PumpSwap => {
                crate::instruction::pumpswap::PumpSwapInstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::PumpFun => {
                crate::instruction::pumpfun::PumpFunInstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::Bonk => {
                crate::instruction::bonk::BonkInstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::MeteoraDammV2 => {
                crate::instruction::meteora_damm_v2::MeteoraDammV2InstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
        }
        .map_err(|e| UnifiedTradingError::TransactionBuildError(e.to_string()))?;

        // 5. 获取用户 ATA
        let user_input_ata = spl_associated_token_account::get_associated_token_address(
            &self.payer.pubkey(),
            &input_mint,
        );
        let user_output_ata = spl_associated_token_account::get_associated_token_address(
            &self.payer.pubkey(),
            &params.mint,
        );

        // 6. 调用链上模拟
        let sim_result = crate::utils::simulation_based_calc::simulate_swap_transaction(
            &self.rpc,
            &self.payer,
            instructions,
            user_input_ata,
            user_output_ata,
            input_mint,
            params.mint,
        )
        .await
        .map_err(|e| UnifiedTradingError::SimulationFailed(e.to_string()))?;

        // 7. 转换返回值
        Ok(SimulationResult {
            amount_out: sim_result.actual_output_amount,
            amount_in: params.input_token_amount, // 添加此行：支持 exact_out 模式
            fee_amount: 0,                        // TODO: 从 sim_result 计算
            compute_units: sim_result.units_consumed.unwrap_or(0),
            transaction_fee: sim_result.transaction_fee,
            success: sim_result.success,
            error: sim_result.error,
            logs: sim_result.logs,
            dex_type: params.dex_type,
        })
    }

    /// 卖出模拟（exact_in 和 exact_out）
    ///
    /// 模拟卖出操作，返回链上模拟结果。
    ///
    /// # 参数
    ///
    /// * `params` - 卖出参数
    ///   - `input_token_amount`: 要卖出的代币数量（exact_in 模式）
    ///   - `fixed_output_token_amount`: 期望获得的输出代币数量（exact_out 模式，可选）
    ///
    /// # 返回
    ///
    /// 返回 `SimulationResult` 包含：
    /// - `amount_in`: 实际卖出的数量
    /// - `amount_out`: 获得的输出数量
    /// - `compute_units`: 计算单元消耗
    /// - `transaction_fee`: 交易费用
    pub async fn sell_simulate(&self, params: TradeSellParams) -> UnifiedResult<SimulationResult> {
        // 1. 参数验证
        if let Some(fixed_output) = params.fixed_output_token_amount {
            if fixed_output == 0 {
                return Err(UnifiedTradingError::InvalidParameters(
                    "fixed_output_token_amount must be > 0".into(),
                ));
            }
        } else {
            if params.input_token_amount == 0 {
                return Err(UnifiedTradingError::InvalidParameters(
                    "input_token_amount must be > 0".into(),
                ));
            }
        }

        // 2. 获取 output_mint
        let output_mint = get_output_mint(&params.output_token_type);

        // 3. 构建 SwapParams
        let swap_params = SwapParams {
            rpc: Some(self.rpc.clone()),
            payer: self.payer.clone(),
            trade_type: crate::swqos::TradeType::Sell,
            input_mint: params.mint,
            output_mint,
            input_token_program: None,
            output_token_program: None,
            input_amount: Some(params.input_token_amount),
            slippage_basis_points: params.slippage_basis_points,
            address_lookup_table_account: params.address_lookup_table_account,
            recent_blockhash: params.recent_blockhash,
            wait_transaction_confirmed: false,
            protocol_params: params.extension_params.clone(),
            open_seed_optimize: self.use_seed_optimize,
            swqos_clients: self.swqos_clients.clone(),
            middleware_manager: self.middleware_manager.clone(),
            durable_nonce: params.durable_nonce,
            with_tip: params.with_tip,
            create_input_mint_ata: false,
            close_input_mint_ata: false,
            create_output_mint_ata: params.create_output_token_ata,
            close_output_mint_ata: params.close_output_token_ata,
            fixed_output_amount: params.fixed_output_token_amount,
            gas_fee_strategy: params.gas_fee_strategy,
            simulate: true,
            on_transaction_signed: params.on_transaction_signed,
            callback_execution_mode: params.callback_execution_mode,
            enable_jito_sandwich_protection: None,
        };

        // 4. 构建指令
        let instructions = match params.dex_type {
            DexType::RaydiumClmm => {
                crate::instruction::raydium_clmm::RaydiumClmmInstructionBuilder
                    .build_sell_instructions(&swap_params)
                    .await
            },
            DexType::RaydiumCpmm => {
                crate::instruction::raydium_cpmm::RaydiumCpmmInstructionBuilder
                    .build_sell_instructions(&swap_params)
                    .await
            },
            DexType::RaydiumAmmV4 => {
                crate::instruction::raydium_amm_v4::RaydiumAmmV4InstructionBuilder
                    .build_sell_instructions(&swap_params)
                    .await
            },
            DexType::PumpSwap => {
                crate::instruction::pumpswap::PumpSwapInstructionBuilder
                    .build_sell_instructions(&swap_params)
                    .await
            },
            _ => {
                return Err(UnifiedTradingError::UnsupportedDex(params.dex_type));
            },
        }
        .map_err(|e| UnifiedTradingError::TransactionBuildError(e.to_string()))?;

        // 5. 获取用户 ATA
        let user_input_ata = spl_associated_token_account::get_associated_token_address(
            &self.payer.pubkey(),
            &params.mint,
        );
        let user_output_ata = spl_associated_token_account::get_associated_token_address(
            &self.payer.pubkey(),
            &output_mint,
        );

        // 6. 调用链上模拟
        let sim_result = crate::utils::simulation_based_calc::simulate_swap_transaction(
            &self.rpc,
            &self.payer,
            instructions,
            user_input_ata,
            user_output_ata,
            params.mint,
            output_mint,
        )
        .await
        .map_err(|e| UnifiedTradingError::SimulationFailed(e.to_string()))?;

        // 7. 转换返回值
        Ok(SimulationResult {
            amount_out: sim_result.actual_output_amount,
            amount_in: params.input_token_amount,
            fee_amount: 0,
            compute_units: sim_result.units_consumed.unwrap_or(0),
            transaction_fee: sim_result.transaction_fee,
            success: sim_result.success,
            error: sim_result.error,
            logs: sim_result.logs,
            dex_type: params.dex_type,
        })
    }
}
