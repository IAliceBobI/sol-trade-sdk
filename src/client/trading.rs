//! TradingClient 核心交易方法（buy/sell）

use super::types::{TradeBuyParams, TradeSellParams, TradingClient};
use crate::trading::core::params::SwapParams;
use crate::trading::core::params::{
    BonkParams, MeteoraDammV2Params, PumpFunParams, PumpSwapParams, RaydiumAmmV4Params,
    RaydiumClmmParams, RaydiumCpmmParams,
};
use crate::trading::factory::TradeFactory;
use solana_sdk::signature::Signature;

impl TradingClient {
    /// 执行指定代币的买入订单
    ///
    /// 🔧 修复：返回Vec<Signature>支持多SWQOS并发交易
    /// - bool: 是否至少有一个交易成功
    /// - Vec<Signature>: 所有提交的交易签名（按SWQOS顺序）
    /// - Option<TradeError>: 最后一个错误（如果全部失败）
    ///
    /// # Arguments
    ///
    /// * `params` - 包含所有必要交易配置的买入交易参数
    ///
    /// # Returns
    ///
    /// Returns `Ok((bool, Vec<Signature>, Option<TradeError>))` with success flag and all transaction signatures,
    /// or an error if the transaction fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Invalid protocol parameters are provided for the specified DEX type
    /// - The transaction fails to execute
    /// - Network or RPC errors occur
    /// - Insufficient SOL balance for the purchase
    /// - Required accounts cannot be created or accessed
    #[inline]
    pub async fn buy(
        &self,
        params: TradeBuyParams,
    ) -> Result<(bool, Vec<Signature>, Option<crate::TradeError>), anyhow::Error> {
        #[cfg(feature = "perf-trace")]
        if params.slippage_basis_points.is_none() {
            log::debug!(
                "slippage_basis_points is none, use default slippage basis points: {}",
                crate::constants::trade_consts::DEFAULT_SLIPPAGE
            );
        }
        if params.input_token_type == crate::TradeTokenType::USD1
            && params.dex_type != crate::DexType::Bonk
        {
            return Err(anyhow::anyhow!(
                " Current version only support USD1 trading on Bonk protocols"
            ));
        }
        let input_token_mint = if params.input_token_type == crate::TradeTokenType::SOL {
            crate::NATIVE_SOL_MARKER
        } else if params.input_token_type == crate::TradeTokenType::WSOL {
            crate::WSOL_TOKEN_ACCOUNT
        } else if params.input_token_type == crate::TradeTokenType::USDC {
            crate::USDC_TOKEN_ACCOUNT
        } else if params.input_token_type == crate::TradeTokenType::USDT {
            crate::USDT_TOKEN_ACCOUNT
        } else {
            crate::USD1_TOKEN_ACCOUNT
        };
        let executor = TradeFactory::create_executor(params.dex_type);
        let protocol_params = params.extension_params;
        let buy_params = SwapParams {
            rpc: Some(self.rpc.clone()),
            payer: self.payer.clone(),
            trade_type: crate::swqos::TradeType::Buy,
            input_mint: input_token_mint,
            output_mint: params.mint,
            input_token_program: None,
            output_token_program: None,
            input_amount: Some(params.input_token_amount),
            slippage_basis_points: params.slippage_basis_points,
            address_lookup_table_account: params.address_lookup_table_account,
            recent_blockhash: params.recent_blockhash,
            wait_transaction_confirmed: params.wait_transaction_confirmed,
            protocol_params: protocol_params.clone(),
            open_seed_optimize: self.use_seed_optimize, // 使用全局seed优化配置
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
            simulate: params.simulate,
            on_transaction_signed: params.on_transaction_signed,
            callback_execution_mode: params
                .callback_execution_mode
                .or(Some(self.callback_execution_mode)),
            enable_jito_sandwich_protection: params
                .enable_jito_sandwich_protection
                .or(Some(self.enable_jito_sandwich_protection)),
        };

        // Validate protocol params
        let is_valid_params = match params.dex_type {
            crate::DexType::PumpFun => {
                protocol_params.as_any().downcast_ref::<PumpFunParams>().is_some()
            },
            crate::DexType::PumpSwap => {
                protocol_params.as_any().downcast_ref::<PumpSwapParams>().is_some()
            },
            crate::DexType::Bonk => protocol_params.as_any().downcast_ref::<BonkParams>().is_some(),
            crate::DexType::RaydiumCpmm => {
                protocol_params.as_any().downcast_ref::<RaydiumCpmmParams>().is_some()
            },
            crate::DexType::RaydiumAmmV4 => {
                protocol_params.as_any().downcast_ref::<RaydiumAmmV4Params>().is_some()
            },
            crate::DexType::RaydiumClmm => {
                protocol_params.as_any().downcast_ref::<RaydiumClmmParams>().is_some()
            },
            crate::DexType::MeteoraDammV2 => {
                protocol_params.as_any().downcast_ref::<MeteoraDammV2Params>().is_some()
            },
        };

        if !is_valid_params {
            return Err(anyhow::anyhow!("Invalid protocol params for Trade"));
        }

        // 🔧 预热 Token Program 缓存（确保 calculate_ata_sync 能命中缓存）
        // 这是一次性操作，后续交易会使用缓存
        use crate::utils::token::get_token_program_with_cache;
        get_token_program_with_cache(&self.rpc, &input_token_mint).await?;
        get_token_program_with_cache(&self.rpc, &params.mint).await?;

        let swap_result = executor.swap(buy_params).await;

        swap_result.map(|(success, sigs, err)| (success, sigs, err.map(crate::TradeError::from)))
    }

    /// 执行指定代币的卖出订单
    ///
    /// 🔧 修复：返回Vec<Signature>支持多SWQOS并发交易
    /// - bool: 是否至少有一个交易成功
    /// - Vec<Signature>: 所有提交的交易签名（按SWQOS顺序）
    /// - Option<TradeError>: 最后一个错误（如果全部失败）
    ///
    /// # Arguments
    ///
    /// * `params` - 包含所有必要交易配置的卖出交易参数
    ///
    /// # Returns
    ///
    /// Returns `Ok((bool, Vec<Signature>, Option<TradeError>))` with success flag and all transaction signatures,
    /// or an error if the transaction fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Invalid protocol parameters are provided for the specified DEX type
    /// - The transaction fails to execute
    /// - Network or RPC errors occur
    /// - Insufficient token balance for the sale
    /// - Token account doesn't exist or is not properly initialized
    /// - Required accounts cannot be created or accessed
    #[inline]
    pub async fn sell(
        &self,
        params: TradeSellParams,
    ) -> Result<(bool, Vec<Signature>, Option<crate::TradeError>), anyhow::Error> {
        #[cfg(feature = "perf-trace")]
        if params.slippage_basis_points.is_none() {
            log::debug!(
                "slippage_basis_points is none, use default slippage basis points: {}",
                crate::constants::trade_consts::DEFAULT_SLIPPAGE
            );
        }
        if params.output_token_type == crate::TradeTokenType::USD1
            && params.dex_type != crate::DexType::Bonk
        {
            return Err(anyhow::anyhow!(
                " Current version only support USD1 trading on Bonk protocols"
            ));
        }
        let executor = TradeFactory::create_executor(params.dex_type);
        let protocol_params = params.extension_params;
        let output_token_mint = if params.output_token_type == crate::TradeTokenType::SOL {
            crate::NATIVE_SOL_MARKER
        } else if params.output_token_type == crate::TradeTokenType::WSOL {
            crate::WSOL_TOKEN_ACCOUNT
        } else if params.output_token_type == crate::TradeTokenType::USDC {
            crate::USDC_TOKEN_ACCOUNT
        } else if params.output_token_type == crate::TradeTokenType::USDT {
            crate::USDT_TOKEN_ACCOUNT
        } else {
            crate::USD1_TOKEN_ACCOUNT
        };
        let sell_params = SwapParams {
            rpc: Some(self.rpc.clone()),
            payer: self.payer.clone(),
            trade_type: crate::swqos::TradeType::Sell,
            input_mint: params.mint,
            output_mint: output_token_mint,
            input_token_program: None,
            output_token_program: None,
            input_amount: Some(params.input_token_amount),
            slippage_basis_points: params.slippage_basis_points,
            address_lookup_table_account: params.address_lookup_table_account,
            recent_blockhash: params.recent_blockhash,
            wait_transaction_confirmed: params.wait_transaction_confirmed,
            protocol_params: protocol_params.clone(),
            with_tip: params.with_tip,
            open_seed_optimize: self.use_seed_optimize, // 使用全局seed优化配置
            swqos_clients: self.swqos_clients.clone(),
            middleware_manager: self.middleware_manager.clone(),
            durable_nonce: params.durable_nonce,
            create_input_mint_ata: false,
            close_input_mint_ata: params.close_mint_token_ata,
            create_output_mint_ata: params.create_output_token_ata,
            close_output_mint_ata: params.close_output_token_ata,
            fixed_output_amount: params.fixed_output_token_amount,
            gas_fee_strategy: params.gas_fee_strategy,
            simulate: params.simulate,
            on_transaction_signed: params.on_transaction_signed,
            callback_execution_mode: params
                .callback_execution_mode
                .or(Some(self.callback_execution_mode)),
            enable_jito_sandwich_protection: params
                .enable_jito_sandwich_protection
                .or(Some(self.enable_jito_sandwich_protection)),
        };

        // Validate protocol params
        let is_valid_params = match params.dex_type {
            crate::DexType::PumpFun => {
                protocol_params.as_any().downcast_ref::<PumpFunParams>().is_some()
            },
            crate::DexType::PumpSwap => {
                protocol_params.as_any().downcast_ref::<PumpSwapParams>().is_some()
            },
            crate::DexType::Bonk => protocol_params.as_any().downcast_ref::<BonkParams>().is_some(),
            crate::DexType::RaydiumCpmm => {
                protocol_params.as_any().downcast_ref::<RaydiumCpmmParams>().is_some()
            },
            crate::DexType::RaydiumAmmV4 => {
                protocol_params.as_any().downcast_ref::<RaydiumAmmV4Params>().is_some()
            },
            crate::DexType::RaydiumClmm => {
                protocol_params.as_any().downcast_ref::<RaydiumClmmParams>().is_some()
            },
            crate::DexType::MeteoraDammV2 => {
                protocol_params.as_any().downcast_ref::<MeteoraDammV2Params>().is_some()
            },
        };

        if !is_valid_params {
            return Err(anyhow::anyhow!("Invalid protocol params for Trade"));
        }

        // 🔧 预热 Token Program 缓存（确保 calculate_ata_sync 能命中缓存）
        // 这是一次性操作，后续交易会使用缓存
        use crate::utils::token::get_token_program_with_cache;
        get_token_program_with_cache(&self.rpc, &params.mint).await?;
        get_token_program_with_cache(&self.rpc, &output_token_mint).await?;

        // Execute sell based on tip preference
        let swap_result = executor.swap(sell_params).await;

        swap_result.map(|(success, sigs, err)| (success, sigs, err.map(crate::TradeError::from)))
    }

    /// 执行指定代币数量百分比的卖出订单
    ///
    /// 这是一个便捷函数，根据代币总量的百分比计算要卖出的确切数量，
    /// 然后调用 `sell` 函数。
    ///
    /// # Arguments
    ///
    /// * `params` - 卖出交易参数（将使用计算出的代币数量进行修改）
    /// * `amount_token` - 可用的代币总量（最小代币单位）
    /// * `percent` - 要卖出的代币百分比（1-100，其中 100 = 100%）
    ///
    /// # Returns
    ///
    /// Returns `Ok((bool, Vec<Signature>, Option<TradeError>))` with success flag and all transaction signatures,
    /// or an error if the transaction fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - `percent` 为 0 或大于 100
    /// - 为指定的 DEX 类型提供了无效的协议参数
    /// - 交易执行失败
    /// - 网络或 RPC 错误
    /// - 计算出的销售金额的代币余额不足
    /// - 代币账户不存在或未正确初始化
    /// - 无法创建或访问所需账户
    pub async fn sell_by_percent(
        &self,
        mut params: TradeSellParams,
        amount_token: u64,
        percent: u64,
    ) -> Result<(bool, Vec<Signature>, Option<crate::TradeError>), anyhow::Error> {
        if percent == 0 || percent > 100 {
            return Err(anyhow::anyhow!("Percentage must be between 1 and 100"));
        }
        let amount = amount_token * percent / 100;
        params.input_token_amount = amount;
        self.sell(params).await
    }
}
