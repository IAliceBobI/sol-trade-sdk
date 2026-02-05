//! TradingClient 报价相关方法

use super::helpers::{get_input_mint, get_output_mint, supports_quote};
use super::types::{TradeBuyParams, TradeSellParams, TradingClient};
use crate::{QuoteResult, UnifiedResult, UnifiedTradingError, utils::quote::QuoteExactInParams};

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
                // 推断输出 mint
                let output_mint = if input_mint == clmm_params.token0_mint {
                    clmm_params.token1_mint
                } else {
                    clmm_params.token0_mint
                };

                let quote_params = QuoteExactInParams {
                    pool_address: clmm_params.pool_state,
                    input_mint,
                    output_mint,
                    amount_in: params.input_token_amount,
                };

                let quote = crate::instruction::utils::raydium_clmm::quote_exact_in(
                    &self.rpc,
                    quote_params,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::RaydiumCpmm(cpmm_params) => {
                // 推断输出 mint
                let output_mint = if input_mint == cpmm_params.base_mint {
                    cpmm_params.quote_mint
                } else {
                    cpmm_params.base_mint
                };

                let quote_params = QuoteExactInParams {
                    pool_address: cpmm_params.pool_state,
                    input_mint,
                    output_mint,
                    amount_in: params.input_token_amount,
                };

                let quote = crate::instruction::utils::raydium_cpmm::quote_exact_in(
                    &self.rpc,
                    quote_params,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::RaydiumAmmV4(amm_params) => {
                // 推断输出 mint
                let output_mint = if input_mint == amm_params.coin_mint {
                    amm_params.pc_mint
                } else {
                    amm_params.coin_mint
                };

                let quote_params = QuoteExactInParams {
                    pool_address: amm_params.amm,
                    input_mint,
                    output_mint,
                    amount_in: params.input_token_amount,
                };

                let quote = crate::instruction::utils::raydium_amm_v4::quote_exact_in(
                    &self.rpc,
                    quote_params,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::PumpSwap(pump_params) => {
                // 推断输出 mint
                let output_mint = if input_mint == pump_params.base_mint {
                    pump_params.quote_mint
                } else {
                    pump_params.base_mint
                };

                let quote_params = QuoteExactInParams {
                    pool_address: pump_params.pool,
                    input_mint,
                    output_mint,
                    amount_in: params.input_token_amount,
                };

                let quote =
                    crate::instruction::utils::pumpswap::quote_exact_in(&self.rpc, quote_params)
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
                // 推断输出 mint
                let output_mint = if params.mint == clmm_params.token0_mint {
                    clmm_params.token1_mint
                } else {
                    clmm_params.token0_mint
                };

                let quote_params = QuoteExactInParams {
                    pool_address: clmm_params.pool_state,
                    input_mint: params.mint,
                    output_mint,
                    amount_in: params.input_token_amount,
                };

                let quote = crate::instruction::utils::raydium_clmm::quote_exact_in(
                    &self.rpc,
                    quote_params,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::RaydiumCpmm(cpmm_params) => {
                // 推断输出 mint
                let output_mint = if params.mint == cpmm_params.base_mint {
                    cpmm_params.quote_mint
                } else {
                    cpmm_params.base_mint
                };

                let quote_params = QuoteExactInParams {
                    pool_address: cpmm_params.pool_state,
                    input_mint: params.mint,
                    output_mint,
                    amount_in: params.input_token_amount,
                };

                let quote = crate::instruction::utils::raydium_cpmm::quote_exact_in(
                    &self.rpc,
                    quote_params,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::RaydiumAmmV4(amm_params) => {
                // 推断输出 mint
                let output_mint = if params.mint == amm_params.coin_mint {
                    amm_params.pc_mint
                } else {
                    amm_params.coin_mint
                };

                let quote_params = QuoteExactInParams {
                    pool_address: amm_params.amm,
                    input_mint: params.mint,
                    output_mint,
                    amount_in: params.input_token_amount,
                };

                let quote = crate::instruction::utils::raydium_amm_v4::quote_exact_in(
                    &self.rpc,
                    quote_params,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            crate::DexParamEnum::PumpSwap(pump_params) => {
                // 推断输出 mint
                let output_mint = if params.mint == pump_params.base_mint {
                    pump_params.quote_mint
                } else {
                    pump_params.base_mint
                };

                let quote_params = QuoteExactInParams {
                    pool_address: pump_params.pool,
                    input_mint: params.mint,
                    output_mint,
                    amount_in: params.input_token_amount,
                };

                let quote =
                    crate::instruction::utils::pumpswap::quote_exact_in(&self.rpc, quote_params)
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
