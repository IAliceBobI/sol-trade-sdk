//! PumpSwap 交易参数构建器
//!
//! 提供 PumpSwap 的交易参数构建器，支持：
//! - PUMP-WSOL
//! - BONK-WSOL

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use sol_trade_sdk::{DexType, TradeBuyParams, TradeSellParams, TradeTokenType, TradingClient};

use super::{constants::*, gas_fee::create_test_gas_fee_strategy};

// ==================== PUMP-WSOL 交易参数 ====================

/// PUMP-WSOL 买入参数构建器
pub struct PumpWsolBuyParamsBuilder {
    input_amount: u64,
    slippage_bps: Option<u64>,
}

impl PumpWsolBuyParamsBuilder {
    /// 创建新的构建器
    ///
    /// # 参数
    /// - `input_amount`: 输入金额（WSOL lamports），默认 1_000_000 (0.001 SOL)
    pub fn new(input_amount: Option<u64>) -> Self {
        Self {
            input_amount: input_amount.unwrap_or(1_000_000),
            slippage_bps: Some(10000), // 默认 10%
        }
    }

    /// 设置滑点容忍度（基点）
    pub fn slippage(mut self, bps: u64) -> Self {
        self.slippage_bps = Some(bps);
        self
    }

    /// 构建买入参数
    pub async fn build(self, client: &TradingClient) -> TradeBuyParams {
        let pool_address = Pubkey::from_str(PUMP_WSOL_POOL).unwrap();
        let pump_mint = Pubkey::from_str(PUMP_MINT).unwrap();

        let pumpswap_params =
            sol_trade_sdk::trading::core::params::PumpSwapParams::from_pool_address_by_rpc(
                &client.rpc,
                &pool_address,
            )
            .await
            .expect("Failed to build PumpSwapParams for PUMP-WSOL");

        let recent_blockhash = client
            .rpc
            .get_latest_blockhash()
            .await
            .expect("Failed to get latest blockhash");

        TradeBuyParams {
            dex_type: DexType::PumpSwap,
            input_token_type: TradeTokenType::WSOL,
            mint: pump_mint,
            input_token_amount: self.input_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            extension_params: sol_trade_sdk::trading::core::params::DexParamEnum::PumpSwap(
                pumpswap_params,
            ),
            address_lookup_table_account: None,
            wait_transaction_confirmed: true,
            create_input_token_ata: true,
            close_input_token_ata: false,
            create_mint_ata: true,
            durable_nonce: None,
            enable_jito_sandwich_protection: Some(false),
            fixed_output_token_amount: None,
            gas_fee_strategy: create_test_gas_fee_strategy(),
            simulate: false,
            on_transaction_signed: None,
            callback_execution_mode: None,
        }
    }
}

/// PUMP-WSOL 卖出参数构建器
pub struct PumpWsolSellParamsBuilder {
    sell_amount: u64,
    slippage_bps: Option<u64>,
}

impl PumpWsolSellParamsBuilder {
    /// 创建新的构建器
    ///
    /// # 参数
    /// - `sell_amount`: 卖出数量（PUMP token 最小单位）
    pub fn new(sell_amount: u64) -> Self {
        Self {
            sell_amount,
            slippage_bps: Some(10000), // 默认 10%
        }
    }

    /// 设置滑点容忍度（基点）
    pub fn slippage(mut self, bps: u64) -> Self {
        self.slippage_bps = Some(bps);
        self
    }

    /// 构建卖出参数
    pub async fn build(self, client: &TradingClient) -> TradeSellParams {
        let pool_address = Pubkey::from_str(PUMP_WSOL_POOL).unwrap();
        let pump_mint = Pubkey::from_str(PUMP_MINT).unwrap();

        let pumpswap_params =
            sol_trade_sdk::trading::core::params::PumpSwapParams::from_pool_address_by_rpc(
                &client.rpc,
                &pool_address,
            )
            .await
            .expect("Failed to build PumpSwapParams for PUMP-WSOL");

        let recent_blockhash = client
            .rpc
            .get_latest_blockhash()
            .await
            .expect("Failed to get latest blockhash");

        TradeSellParams {
            dex_type: DexType::PumpSwap,
            output_token_type: TradeTokenType::WSOL,
            mint: pump_mint,
            input_token_amount: self.sell_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            with_tip: false,
            extension_params: sol_trade_sdk::trading::core::params::DexParamEnum::PumpSwap(
                pumpswap_params,
            ),
            address_lookup_table_account: None,
            wait_transaction_confirmed: true,
            create_output_token_ata: true,
            close_output_token_ata: false,
            close_mint_token_ata: false,
            durable_nonce: None,
            enable_jito_sandwich_protection: Some(false),
            fixed_output_token_amount: None,
            gas_fee_strategy: create_test_gas_fee_strategy(),
            simulate: false,
            on_transaction_signed: None,
            callback_execution_mode: None,
        }
    }
}

// ==================== BONK-WSOL 交易参数 ====================

/// BONK-WSOL 买入参数构建器
pub struct BonkWsolBuyParamsBuilder {
    input_amount: u64,
    slippage_bps: Option<u64>,
}

impl BonkWsolBuyParamsBuilder {
    /// 创建新的构建器
    ///
    /// # 参数
    /// - `input_amount`: 输入金额（WSOL lamports），默认 1_000_000 (0.001 SOL)
    pub fn new(input_amount: Option<u64>) -> Self {
        Self {
            input_amount: input_amount.unwrap_or(1_000_000),
            slippage_bps: Some(10000), // 默认 10%
        }
    }

    /// 设置滑点容忍度（基点）
    pub fn slippage(mut self, bps: u64) -> Self {
        self.slippage_bps = Some(bps);
        self
    }

    /// 构建买入参数
    pub async fn build(self, client: &TradingClient) -> TradeBuyParams {
        let pool_address = Pubkey::from_str(BONK_WSOL_POOL).unwrap();
        let bonk_mint = Pubkey::from_str(BONK_MINT).unwrap();

        let pumpswap_params =
            sol_trade_sdk::trading::core::params::PumpSwapParams::from_pool_address_by_rpc(
                &client.rpc,
                &pool_address,
            )
            .await
            .expect("Failed to build PumpSwapParams for BONK-WSOL");

        let recent_blockhash = client
            .rpc
            .get_latest_blockhash()
            .await
            .expect("Failed to get latest blockhash");

        TradeBuyParams {
            dex_type: DexType::PumpSwap,
            input_token_type: TradeTokenType::WSOL,
            mint: bonk_mint,
            input_token_amount: self.input_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            extension_params: sol_trade_sdk::trading::core::params::DexParamEnum::PumpSwap(
                pumpswap_params,
            ),
            address_lookup_table_account: None,
            wait_transaction_confirmed: true,
            create_input_token_ata: true,
            close_input_token_ata: false,
            create_mint_ata: true,
            durable_nonce: None,
            enable_jito_sandwich_protection: Some(false),
            fixed_output_token_amount: None,
            gas_fee_strategy: create_test_gas_fee_strategy(),
            simulate: false,
            on_transaction_signed: None,
            callback_execution_mode: None,
        }
    }
}

/// BONK-WSOL 卖出参数构建器
pub struct BonkWsolSellParamsBuilder {
    sell_amount: u64,
    slippage_bps: Option<u64>,
}

impl BonkWsolSellParamsBuilder {
    /// 创建新的构建器
    ///
    /// # 参数
    /// - `sell_amount`: 卖出数量（BONK token 最小单位）
    pub fn new(sell_amount: u64) -> Self {
        Self {
            sell_amount,
            slippage_bps: Some(10000), // 默认 10%
        }
    }

    /// 设置滑点容忍度（基点）
    pub fn slippage(mut self, bps: u64) -> Self {
        self.slippage_bps = Some(bps);
        self
    }

    /// 构建卖出参数
    pub async fn build(self, client: &TradingClient) -> TradeSellParams {
        let pool_address = Pubkey::from_str(BONK_WSOL_POOL).unwrap();
        let bonk_mint = Pubkey::from_str(BONK_MINT).unwrap();

        let pumpswap_params =
            sol_trade_sdk::trading::core::params::PumpSwapParams::from_pool_address_by_rpc(
                &client.rpc,
                &pool_address,
            )
            .await
            .expect("Failed to build PumpSwapParams for BONK-WSOL");

        let recent_blockhash = client
            .rpc
            .get_latest_blockhash()
            .await
            .expect("Failed to get latest blockhash");

        TradeSellParams {
            dex_type: DexType::PumpSwap,
            output_token_type: TradeTokenType::WSOL,
            mint: bonk_mint,
            input_token_amount: self.sell_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            with_tip: false,
            extension_params: sol_trade_sdk::trading::core::params::DexParamEnum::PumpSwap(
                pumpswap_params,
            ),
            address_lookup_table_account: None,
            wait_transaction_confirmed: true,
            create_output_token_ata: true,
            close_output_token_ata: false,
            close_mint_token_ata: false,
            durable_nonce: None,
            enable_jito_sandwich_protection: Some(false),
            fixed_output_token_amount: None,
            gas_fee_strategy: create_test_gas_fee_strategy(),
            simulate: false,
            on_transaction_signed: None,
            callback_execution_mode: None,
        }
    }
}
