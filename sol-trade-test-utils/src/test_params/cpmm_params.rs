//! CPMM 交易参数构建器
//!
//! 提供 Raydium CPMM 的交易参数构建器，支持：
//! - PIPE-WSOL
//! - USDC-PRTS

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use sol_trade_sdk::{
    trading::core::params::{DexParamEnum, RaydiumCpmmParams},
    DexType, TradeBuyParams, TradeSellParams, TradeTokenType, TradingClient,
};

use super::{constants::*, gas_fee::create_test_gas_fee_strategy};

// ==================== PIPE-WSOL 交易参数 ====================

/// PIPE-WSOL 买入参数构建器
pub struct PipeWsolBuyParamsBuilder {
    input_amount: u64,
    slippage_bps: Option<u64>,
}

impl PipeWsolBuyParamsBuilder {
    /// 创建新的构建器
    ///
    /// # 参数
    /// - `input_amount`: 输入金额（lamports），默认 20_000_000 (0.02 SOL)
    pub fn new(input_amount: Option<u64>) -> Self {
        Self {
            input_amount: input_amount.unwrap_or(20_000_000),
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
        let pool_address = Pubkey::from_str(PIPE_WSOL_POOL).unwrap();
        let target_mint = Pubkey::from_str(PIPE_MINT).unwrap();

        let cpmm_params = RaydiumCpmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
            .await
            .expect("Failed to build RaydiumCpmmParams for PIPE-WSOL");

        let recent_blockhash =
            client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

        TradeBuyParams {
            dex_type: DexType::RaydiumCpmm,
            input_token_type: TradeTokenType::SOL,
            mint: target_mint,
            input_token_amount: self.input_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            extension_params: DexParamEnum::RaydiumCpmm(cpmm_params),
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

/// PIPE-WSOL 卖出参数构建器
pub struct PipeWsolSellParamsBuilder {
    sell_amount: u64,
    slippage_bps: Option<u64>,
}

impl PipeWsolSellParamsBuilder {
    /// 创建新的构建器
    ///
    /// # 参数
    /// - `sell_amount`: 卖出数量（PIPE token 最小单位）
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
        let pool_address = Pubkey::from_str(PIPE_WSOL_POOL).unwrap();
        let target_mint = Pubkey::from_str(PIPE_MINT).unwrap();

        let cpmm_params = RaydiumCpmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
            .await
            .expect("Failed to build RaydiumCpmmParams for PIPE-WSOL");

        let recent_blockhash =
            client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

        TradeSellParams {
            dex_type: DexType::RaydiumCpmm,
            output_token_type: TradeTokenType::SOL,
            mint: target_mint,
            input_token_amount: self.sell_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            with_tip: false,
            extension_params: DexParamEnum::RaydiumCpmm(cpmm_params),
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

// ==================== USDC-PRTS 交易参数 ====================

/// USDC-PRTS 买入参数构建器
pub struct UsdcPrtsBuyParamsBuilder {
    input_amount: u64,
    slippage_bps: Option<u64>,
}

impl UsdcPrtsBuyParamsBuilder {
    /// 创建新的构建器
    ///
    /// # 参数
    /// - `input_amount`: 输入金额（USDC 最小单位），默认 100_000_000 (100 USDC)
    pub fn new(input_amount: Option<u64>) -> Self {
        Self {
            input_amount: input_amount.unwrap_or(100_000_000),
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
        let pool_address = Pubkey::from_str(USDC_PRTS_POOL).unwrap();
        let prts_mint = Pubkey::from_str(PRTS_MINT).unwrap();

        let cpmm_params = RaydiumCpmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
            .await
            .expect("Failed to build RaydiumCpmmParams for USDC-PRTS");

        let recent_blockhash =
            client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

        TradeBuyParams {
            dex_type: DexType::RaydiumCpmm,
            input_token_type: TradeTokenType::USDC,
            mint: prts_mint,
            input_token_amount: self.input_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            extension_params: DexParamEnum::RaydiumCpmm(cpmm_params),
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

/// USDC-PRTS 卖出参数构建器
pub struct UsdcPrtsSellParamsBuilder {
    sell_amount: u64,
    slippage_bps: Option<u64>,
}

impl UsdcPrtsSellParamsBuilder {
    /// 创建新的构建器
    ///
    /// # 参数
    /// - `sell_amount`: 卖出数量（PRTS token 最小单位）
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
        let pool_address = Pubkey::from_str(USDC_PRTS_POOL).unwrap();
        let prts_mint = Pubkey::from_str(PRTS_MINT).unwrap();

        let cpmm_params = RaydiumCpmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
            .await
            .expect("Failed to build RaydiumCpmmParams for USDC-PRTS");

        let recent_blockhash =
            client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

        TradeSellParams {
            dex_type: DexType::RaydiumCpmm,
            output_token_type: TradeTokenType::USDC,
            mint: prts_mint,
            input_token_amount: self.sell_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            with_tip: false,
            extension_params: DexParamEnum::RaydiumCpmm(cpmm_params),
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
