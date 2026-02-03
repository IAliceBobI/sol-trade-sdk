//! TradingClient 报价相关方法

use super::helpers::{get_input_mint, get_output_mint, supports_quote};
use super::types::{TradeBuyParams, TradeSellParams, TradingClient};
use crate::{QuoteResult, UnifiedResult, UnifiedTradingError};

impl TradingClient {
    /// 本地计算（快速估算）
    ///
    /// 提供快速的本地价格估算，不发送交易到链上。
    /// 支持的 DEX：Raydium CLMM, Raydium CPMM, Raydium AMM V4, PumpSwap
    ///
    /// # 参数
    ///
    /// * `params` - 交易参数
    ///
    /// # 返回
    ///
    /// 返回 `QuoteResult` 包含预期的输出金额、手续费等信息
    pub async fn buy_quote(&self, params: TradeBuyParams) -> UnifiedResult<QuoteResult> {
        let start = std::time::Instant::now();

        // 1. 参数验证
        if params.input_token_amount == 0 {
            return Err(UnifiedTradingError::InvalidParameters("amount must be > 0".into()));
        }

        if !supports_quote(&params.dex_type) {
            return Err(UnifiedTradingError::UnsupportedDex(params.dex_type));
        }

        // 2. 获取 input_mint
        let input_mint = get_input_mint(&params.input_token_type);

        // 3. 根据 DEX 类型调用对应的 quote_exact_in
        let (amount_out, fee_amount) = match &params.extension_params {
            crate::DexParamEnum::RaydiumClmm(clmm_params) => {
                // 推断方向：input_mint 是否是 token0
                let zero_for_one = input_mint == clmm_params.token0_mint;

                let quote = crate::instruction::utils::raydium_clmm::quote_exact_in(
                    &self.rpc,
                    &clmm_params.pool_state,
                    params.input_token_amount,
                    zero_for_one,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::RaydiumCpmm(cpmm_params) => {
                let is_token0_in = input_mint == cpmm_params.base_mint;

                let quote = crate::instruction::utils::raydium_cpmm::quote_exact_in(
                    &self.rpc,
                    &cpmm_params.pool_state,
                    params.input_token_amount,
                    is_token0_in,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::RaydiumAmmV4(amm_params) => {
                let is_coin_in = input_mint == amm_params.coin_mint;

                let quote = crate::instruction::utils::raydium_amm_v4::quote_exact_in(
                    &self.rpc,
                    &amm_params.amm,
                    params.input_token_amount,
                    is_coin_in,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::PumpSwap(pump_params) => {
                let is_base_in = input_mint == pump_params.base_mint;

                let quote = crate::instruction::utils::pumpswap::quote_exact_in(
                    &self.rpc,
                    &pump_params.pool,
                    params.input_token_amount,
                    is_base_in,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            _ => return Err(UnifiedTradingError::UnsupportedDex(params.dex_type)),
        };

        Ok(QuoteResult {
            amount_out,
            fee_amount,
            price_impact_bps: None,
            calculation_time_ms: start.elapsed().as_millis() as u64,
            dex_type: params.dex_type,
        })
    }

    /// 卖出报价（本地计算）
    ///
    /// 提供快速的本地价格估算，不发送交易到链上。
    /// 支持的 DEX：Raydium CLMM, Raydium CPMM, Raydium AMM V4, PumpSwap
    ///
    /// # 参数
    ///
    /// * `params` - 卖出参数
    ///
    /// # 返回
    ///
    /// 返回 `QuoteResult` 包含预期的输出金额、手续费等信息
    pub async fn sell_quote(&self, params: TradeSellParams) -> UnifiedResult<QuoteResult> {
        let start = std::time::Instant::now();

        // 1. 参数验证
        if params.input_token_amount == 0 {
            return Err(UnifiedTradingError::InvalidParameters("amount must be > 0".into()));
        }

        if !supports_quote(&params.dex_type) {
            return Err(UnifiedTradingError::UnsupportedDex(params.dex_type));
        }

        // 2. 获取 output_mint（卖出时，mint 是输入，output_token_type 是输出）
        let _output_mint = get_output_mint(&params.output_token_type);

        // 3. 根据 DEX 类型调用对应的 quote_exact_in
        let (amount_out, fee_amount) = match &params.extension_params {
            crate::DexParamEnum::RaydiumClmm(clmm_params) => {
                // 推断方向：mint 是否是 token0
                // 卖出 mint 时，如果 mint 是 token0，则方向是 token0 -> token1 (zero_for_one = true)
                let zero_for_one = params.mint == clmm_params.token0_mint;

                let quote = crate::instruction::utils::raydium_clmm::quote_exact_in(
                    &self.rpc,
                    &clmm_params.pool_state,
                    params.input_token_amount,
                    zero_for_one,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::RaydiumCpmm(cpmm_params) => {
                // 推断方向：mint 是否是 base_mint (token0)
                // 卖出 mint 时，如果 mint 是 token0，则方向是 token0 -> token1 (is_token0_in = true)
                let is_token0_in = params.mint == cpmm_params.base_mint;

                let quote = crate::instruction::utils::raydium_cpmm::quote_exact_in(
                    &self.rpc,
                    &cpmm_params.pool_state,
                    params.input_token_amount,
                    is_token0_in,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::RaydiumAmmV4(amm_params) => {
                // 推断方向：mint 是否是 coin_mint
                // 卖出 mint 时，如果 mint 是 coin，则方向是 coin -> pc (is_coin_in = true)
                let is_coin_in = params.mint == amm_params.coin_mint;

                let quote = crate::instruction::utils::raydium_amm_v4::quote_exact_in(
                    &self.rpc,
                    &amm_params.amm,
                    params.input_token_amount,
                    is_coin_in,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::PumpSwap(pump_params) => {
                // 推断方向：mint 是否是 base_mint
                // 卖出 mint 时，如果 mint 是 base，则方向是 base -> quote (is_base_in = true)
                let is_base_in = params.mint == pump_params.base_mint;

                let quote = crate::instruction::utils::pumpswap::quote_exact_in(
                    &self.rpc,
                    &pump_params.pool,
                    params.input_token_amount,
                    is_base_in,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            _ => return Err(UnifiedTradingError::UnsupportedDex(params.dex_type)),
        };

        Ok(QuoteResult {
            amount_out,
            fee_amount,
            price_impact_bps: None,
            calculation_time_ms: start.elapsed().as_millis() as u64,
            dex_type: params.dex_type,
        })
    }
}
