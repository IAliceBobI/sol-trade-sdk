//! CLMM 交易参数构建器
//!
//! 提供 Raydium CLMM 的交易参数构建器，支持：
//! - USDT-WSOL
//! - SOLETT-WSOL

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use sol_trade_sdk::{
    instruction::utils::raydium_clmm::get_pool_by_address,
    trading::core::params::{DexParamEnum, RaydiumClmmParams},
    DexType, TradeBuyParams, TradeSellParams, TradeTokenType, TradingClient,
};

use super::{constants::*, gas_fee::create_test_gas_fee_strategy};

// ==================== USDT-WSOL CLMM 参数构建器 ====================

/// USDT-WSOL CLMM 买入参数构建器
pub struct UsdtWsolClmmBuyParamsBuilder {
    input_amount: u64,
    slippage_bps: Option<u64>,
}

impl UsdtWsolClmmBuyParamsBuilder {
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
        let pool_address = Pubkey::from_str(USDT_WSOL_POOL).unwrap();
        let usdt_mint = Pubkey::from_str(USDT_MINT).unwrap();

        // 获取 Pool 状态
        let pool_state = get_pool_by_address(&client.rpc, &pool_address)
            .await
            .expect("Failed to get CLMM pool state for USDT-WSOL");

        // 获取 Token Program（自动检测）
        let token0_program = sol_trade_sdk::utils::token::get_token_program_with_cache(
            &client.rpc,
            &pool_state.token_mint0,
        )
        .await
        .expect("Failed to get token0 program");
        let token1_program = sol_trade_sdk::utils::token::get_token_program_with_cache(
            &client.rpc,
            &pool_state.token_mint1,
        )
        .await
        .expect("Failed to get token1 program");

        let clmm_params = RaydiumClmmParams {
            pool_state: pool_address,
            amm_config: pool_state.amm_config,
            token0_mint: pool_state.token_mint0,
            token1_mint: pool_state.token_mint1,
            token0_vault: pool_state.token_vault0,
            token1_vault: pool_state.token_vault1,
            observation_state: pool_state.observation_key,
            token0_decimals: pool_state.mint_decimals0,
            token1_decimals: pool_state.mint_decimals1,
            token0_program,
            token1_program,
        };

        let recent_blockhash =
            client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

        TradeBuyParams {
            dex_type: DexType::RaydiumClmm,
            input_token_type: TradeTokenType::WSOL,
            mint: usdt_mint,
            input_token_amount: self.input_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            extension_params: DexParamEnum::RaydiumClmm(clmm_params),
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

/// USDT-WSOL CLMM 卖出参数构建器
pub struct UsdtWsolClmmSellParamsBuilder {
    sell_amount: u64,
    slippage_bps: Option<u64>,
}

impl UsdtWsolClmmSellParamsBuilder {
    /// 创建新的构建器
    ///
    /// # 参数
    /// - `sell_amount`: 卖出数量（USDT token 最小单位）
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
        let pool_address = Pubkey::from_str(USDT_WSOL_POOL).unwrap();
        let usdt_mint = Pubkey::from_str(USDT_MINT).unwrap();

        // 获取 Pool 状态
        let pool_state = get_pool_by_address(&client.rpc, &pool_address)
            .await
            .expect("Failed to get CLMM pool state for USDT-WSOL");

        // 获取 Token Program（自动检测）
        let token0_program = sol_trade_sdk::utils::token::get_token_program_with_cache(
            &client.rpc,
            &pool_state.token_mint0,
        )
        .await
        .expect("Failed to get token0 program");
        let token1_program = sol_trade_sdk::utils::token::get_token_program_with_cache(
            &client.rpc,
            &pool_state.token_mint1,
        )
        .await
        .expect("Failed to get token1 program");

        let clmm_params = RaydiumClmmParams {
            pool_state: pool_address,
            amm_config: pool_state.amm_config,
            token0_mint: pool_state.token_mint0,
            token1_mint: pool_state.token_mint1,
            token0_vault: pool_state.token_vault0,
            token1_vault: pool_state.token_vault1,
            observation_state: pool_state.observation_key,
            token0_decimals: pool_state.mint_decimals0,
            token1_decimals: pool_state.mint_decimals1,
            token0_program,
            token1_program,
        };

        let recent_blockhash =
            client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

        TradeSellParams {
            dex_type: DexType::RaydiumClmm,
            output_token_type: TradeTokenType::WSOL,
            mint: usdt_mint,
            input_token_amount: self.sell_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            with_tip: false,
            extension_params: DexParamEnum::RaydiumClmm(clmm_params),
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

// ==================== SOLETT-WSOL CLMM 参数构建器 ====================

/// SOLETT-WSOL CLMM 买入参数构建器
pub struct SolettWsolClmmBuyParamsBuilder {
    input_amount: u64,
    slippage_bps: Option<u64>,
}

impl SolettWsolClmmBuyParamsBuilder {
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
        let pool_address = Pubkey::from_str(SOLETT_WSOL_POOL).unwrap();
        let solett_mint = Pubkey::from_str(SOLETT_MINT).unwrap();

        // 获取 Pool 状态
        let pool_state = get_pool_by_address(&client.rpc, &pool_address)
            .await
            .expect("Failed to get CLMM pool state for SOLETT-WSOL");

        // 获取 Token Program（自动检测）
        let token0_program = sol_trade_sdk::utils::token::get_token_program_with_cache(
            &client.rpc,
            &pool_state.token_mint0,
        )
        .await
        .expect("Failed to get token0 program");
        let token1_program = sol_trade_sdk::utils::token::get_token_program_with_cache(
            &client.rpc,
            &pool_state.token_mint1,
        )
        .await
        .expect("Failed to get token1 program");

        let clmm_params = RaydiumClmmParams {
            pool_state: pool_address,
            amm_config: pool_state.amm_config,
            token0_mint: pool_state.token_mint0,
            token1_mint: pool_state.token_mint1,
            token0_vault: pool_state.token_vault0,
            token1_vault: pool_state.token_vault1,
            observation_state: pool_state.observation_key,
            token0_decimals: pool_state.mint_decimals0,
            token1_decimals: pool_state.mint_decimals1,
            token0_program,
            token1_program,
        };

        let recent_blockhash =
            client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

        TradeBuyParams {
            dex_type: DexType::RaydiumClmm,
            input_token_type: TradeTokenType::WSOL,
            mint: solett_mint,
            input_token_amount: self.input_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            extension_params: DexParamEnum::RaydiumClmm(clmm_params),
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

/// SOLETT-WSOL CLMM 卖出参数构建器
pub struct SolettWsolClmmSellParamsBuilder {
    sell_amount: u64,
    slippage_bps: Option<u64>,
}

impl SolettWsolClmmSellParamsBuilder {
    /// 创建新的构建器
    ///
    /// # 参数
    /// - `sell_amount`: 卖出数量（SOLETT token 最小单位）
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
        let pool_address = Pubkey::from_str(SOLETT_WSOL_POOL).unwrap();
        let solett_mint = Pubkey::from_str(SOLETT_MINT).unwrap();

        // 获取 Pool 状态
        let pool_state = get_pool_by_address(&client.rpc, &pool_address)
            .await
            .expect("Failed to get CLMM pool state for SOLETT-WSOL");

        // 获取 Token Program（自动检测）
        let token0_program = sol_trade_sdk::utils::token::get_token_program_with_cache(
            &client.rpc,
            &pool_state.token_mint0,
        )
        .await
        .expect("Failed to get token0 program");
        let token1_program = sol_trade_sdk::utils::token::get_token_program_with_cache(
            &client.rpc,
            &pool_state.token_mint1,
        )
        .await
        .expect("Failed to get token1 program");

        let clmm_params = RaydiumClmmParams {
            pool_state: pool_address,
            amm_config: pool_state.amm_config,
            token0_mint: pool_state.token_mint0,
            token1_mint: pool_state.token_mint1,
            token0_vault: pool_state.token_vault0,
            token1_vault: pool_state.token_vault1,
            observation_state: pool_state.observation_key,
            token0_decimals: pool_state.mint_decimals0,
            token1_decimals: pool_state.mint_decimals1,
            token0_program,
            token1_program,
        };

        let recent_blockhash =
            client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

        TradeSellParams {
            dex_type: DexType::RaydiumClmm,
            output_token_type: TradeTokenType::WSOL,
            mint: solett_mint,
            input_token_amount: self.sell_amount,
            slippage_basis_points: self.slippage_bps,
            recent_blockhash: Some(recent_blockhash),
            with_tip: false,
            extension_params: DexParamEnum::RaydiumClmm(clmm_params),
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
