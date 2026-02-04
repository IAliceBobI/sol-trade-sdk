//! Raydium CPMM 测试参数构造工具
//!
//! 提供可复用的测试参数构造函数，用于不同测试场景

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use sol_trade_sdk::{
    common::GasFeeStrategy,
    liquidity::cpmm::{build_deposit_instruction, calculate_deposit_amounts, CpmmDepositParams},
    trading::core::params::{DexParamEnum, RaydiumCpmmParams},
    DexType, TradeBuyParams, TradeSellParams, TradeTokenType, TradingClient,
};

// ==================== 常量定义 ====================

/// WSOL Mint
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// USDC Mint
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// PIPE Token Mint
pub const PIPE_MINT: &str = "8ycz3kctoRb4LFrtoYG2r8tRyUYUeGf5Q16M2TEMp7A";

/// PIPE-WSOL CPMM Pool
pub const PIPE_WSOL_POOL: &str = "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp";

/// PRTS Token Mint (Token-2022)
pub const PRTS_MINT: &str = "3PQkX8yfuxoe9kuBoLCEZoxzi9LG4w8Ci2JWWGNfPRTS";

/// USDC-PRTS CPMM Pool
pub const USDC_PRTS_POOL: &str = "7Cvz28TyKnGuL8CAtbsVFu1FJ3Po7A37Zc8JSJqkSPDp";

// ==================== Gas 费策略 ====================

/// 创建测试用的 Gas 费策略
pub fn create_test_gas_fee_strategy() -> GasFeeStrategy {
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);
    gas_fee_strategy
}

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

        let recent_blockhash = client
            .rpc
            .get_latest_blockhash()
            .await
            .expect("Failed to get latest blockhash");

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

        let recent_blockhash = client
            .rpc
            .get_latest_blockhash()
            .await
            .expect("Failed to get latest blockhash");

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

        let recent_blockhash = client
            .rpc
            .get_latest_blockhash()
            .await
            .expect("Failed to get latest blockhash");

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

        let recent_blockhash = client
            .rpc
            .get_latest_blockhash()
            .await
            .expect("Failed to get latest blockhash");

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

// ==================== 便捷函数 ====================

/// 获取 PIPE-WSOL Pool 地址
pub fn pipe_wsol_pool() -> Pubkey {
    Pubkey::from_str(PIPE_WSOL_POOL).unwrap()
}

/// 获取 PIPE Mint
pub fn pipe_mint() -> Pubkey {
    Pubkey::from_str(PIPE_MINT).unwrap()
}

/// 获取 USDC-PRTS Pool 地址
pub fn usdc_prts_pool() -> Pubkey {
    Pubkey::from_str(USDC_PRTS_POOL).unwrap()
}

/// 获取 PRTS Mint
pub fn prts_mint() -> Pubkey {
    Pubkey::from_str(PRTS_MINT).unwrap()
}

/// 获取 WSOL Mint
pub fn wsol_mint() -> Pubkey {
    Pubkey::from_str(WSOL_MINT).unwrap()
}

/// 获取 USDC Mint
pub fn usdc_mint() -> Pubkey {
    Pubkey::from_str(USDC_MINT).unwrap()
}

// ==================== 流动性添加构建器 ====================

/// CPMM 流动性添加参数构建器
///
/// 简化 CPMM 池子流动性添加的参数构造
pub struct CpmmLiquidityBuilder {
    lp_token_amount: u64,
    maximum_token_0_amount: Option<u64>,
    maximum_token_1_amount: Option<u64>,
    pool_address: Pubkey,
    token_0_mint: Pubkey,
    token_1_mint: Pubkey,
}

impl CpmmLiquidityBuilder {
    /// 创建新的流动性添加构建器
    ///
    /// # 参数
    /// - `lp_token_amount`: 要铸造的 LP 代币数量
    /// - `pool_address`: CPMM 池子地址
    /// - `token_0_mint`: Token0 mint 地址
    /// - `token_1_mint`: Token1 mint 地址
    pub fn new(
        lp_token_amount: u64,
        pool_address: Pubkey,
        token_0_mint: Pubkey,
        token_1_mint: Pubkey,
    ) -> Self {
        Self {
            lp_token_amount,
            maximum_token_0_amount: None,
            maximum_token_1_amount: None,
            pool_address,
            token_0_mint,
            token_1_mint,
        }
    }

    /// 设置最大 token_0 数量（滑点保护）
    pub fn max_token0(mut self, amount: u64) -> Self {
        self.maximum_token_0_amount = Some(amount);
        self
    }

    /// 设置最大 token_1 数量（滑点保护）
    pub fn max_token1(mut self, amount: u64) -> Self {
        self.maximum_token_1_amount = Some(amount);
        self
    }

    /// 构建流动性添加参数
    ///
    /// # 参数
    /// - `payer`: 用户公钥（用于派生 ATA）
    /// - `pool_state`: Pool 状态（包含 vault 和 lp_mint 信息）
    /// - `token_0_vault_amount`: Token0 金库余额
    /// - `token_1_vault_amount`: Token1 金库余额
    ///
    /// # 返回
    /// 返回 `CpmmDepositParams`、计算出的代币数量和 LP ATA 地址
    pub fn build(
        self,
        payer: Pubkey,
        pool_state: &sol_trade_sdk::instruction::utils::raydium_cpmm_types::PoolState,
        token_0_vault_amount: u64,
        token_1_vault_amount: u64,
    ) -> (CpmmDepositParams, Option<(u64, u64)>, Pubkey) {
        // 派生用户 ATA 地址
        let owner_lp_token = spl_associated_token_account::get_associated_token_address_with_program_id(
            &payer,
            &pool_state.lp_mint,
            &spl_token::id(),
        );

        let token_0_account =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &payer,
                &self.token_0_mint,
                &spl_token::id(),
            );

        let token_1_account =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &payer,
                &self.token_1_mint,
                &spl_token::id(),
            );

        // 计算需要的代币数量（用于调试和设置最大值）
        let calculated_amounts = calculate_deposit_amounts(
            self.lp_token_amount,
            pool_state,
            token_0_vault_amount,
            token_1_vault_amount,
        );

        // 如果没有设置最大值，使用计算值的 1.2 倍作为默认值（20% 缓冲）
        let (max_token0, max_token1) = if let Some((t0, t1)) = calculated_amounts {
            (
                self.maximum_token_0_amount.unwrap_or(t0.saturating_mul(12) / 10),
                self.maximum_token_1_amount.unwrap_or(t1.saturating_mul(12) / 10),
            )
        } else {
            // 如果无法计算，使用用户提供的值或默认值
            (
                self.maximum_token_0_amount.unwrap_or(u64::MAX),
                self.maximum_token_1_amount.unwrap_or(u64::MAX),
            )
        };

        let deposit_params = CpmmDepositParams {
            pool_state: self.pool_address,
            owner_lp_token,
            token_0_account,
            token_1_account,
            token_0_vault: pool_state.token0_vault,
            token_1_vault: pool_state.token1_vault,
            token_0_mint: self.token_0_mint,
            token_1_mint: self.token_1_mint,
            lp_mint: pool_state.lp_mint,
            lp_token_amount: self.lp_token_amount,
            maximum_token_0_amount: max_token0,
            maximum_token_1_amount: max_token1,
            token_program: spl_token::id(),
        };

        (deposit_params, calculated_amounts, owner_lp_token)
    }

    /// 构建流动性添加指令（便捷方法）
    ///
    /// # 参数
    /// - `payer`: 用户公钥（用于派生 ATA）
    /// - `pool_state`: Pool 状态
    /// - `token_0_vault_amount`: Token0 金库余额
    /// - `token_1_vault_amount`: Token1 金库余额
    ///
    /// # 返回
    /// 返回构建好的 Instruction、计算出的代币数量和 LP ATA 地址
    pub fn build_instruction(
        self,
        payer: Pubkey,
        pool_state: &sol_trade_sdk::instruction::utils::raydium_cpmm_types::PoolState,
        token_0_vault_amount: u64,
        token_1_vault_amount: u64,
    ) -> (solana_sdk::instruction::Instruction, Option<(u64, u64)>, Pubkey) {
        let (deposit_params, calculated, owner_lp_token) =
            self.build(payer, pool_state, token_0_vault_amount, token_1_vault_amount);
        let instruction = build_deposit_instruction(deposit_params, payer);
        (instruction, calculated, owner_lp_token)
    }
}

/// PIPE-WSOL 流动性添加便捷构建器
///
/// 专门为 PIPE-WSOL 池子优化的流动性添加构建器
pub struct PipeWsolLiquidityBuilder {
    inner: CpmmLiquidityBuilder,
}

impl PipeWsolLiquidityBuilder {
    /// 创建新的 PIPE-WSOL 流动性添加构建器
    ///
    /// # 参数
    /// - `lp_token_amount`: 要铸造的 LP 代币数量
    pub fn new(lp_token_amount: u64) -> Self {
        Self {
            inner: CpmmLiquidityBuilder::new(
                lp_token_amount,
                pipe_wsol_pool(),
                pipe_mint(),
                wsol_mint(),
            ),
        }
    }

    /// 设置最大 PIPE 数量
    pub fn max_pipe(mut self, amount: u64) -> Self {
        self.inner = self.inner.max_token0(amount);
        self
    }

    /// 设置最大 WSOL 数量
    pub fn max_wsol(mut self, amount: u64) -> Self {
        self.inner = self.inner.max_token1(amount);
        self
    }

    /// 构建流动性添加参数
    pub fn build(
        self,
        payer: Pubkey,
        pool_state: &sol_trade_sdk::instruction::utils::raydium_cpmm_types::PoolState,
        token_0_vault_amount: u64,
        token_1_vault_amount: u64,
    ) -> (CpmmDepositParams, Option<(u64, u64)>, Pubkey) {
        self.inner
            .build(payer, pool_state, token_0_vault_amount, token_1_vault_amount)
    }

    /// 构建流动性添加指令
    pub fn build_instruction(
        self,
        payer: Pubkey,
        pool_state: &sol_trade_sdk::instruction::utils::raydium_cpmm_types::PoolState,
        token_0_vault_amount: u64,
        token_1_vault_amount: u64,
    ) -> (solana_sdk::instruction::Instruction, Option<(u64, u64)>, Pubkey) {
        self.inner
            .build_instruction(payer, pool_state, token_0_vault_amount, token_1_vault_amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubkey_constants() {
        // 验证常量解析正确
        assert_ne!(pipe_wsol_pool(), Pubkey::default());
        assert_ne!(pipe_mint(), Pubkey::default());
        assert_ne!(usdc_prts_pool(), Pubkey::default());
        assert_ne!(prts_mint(), Pubkey::default());
        assert_ne!(wsol_mint(), Pubkey::default());
        assert_ne!(usdc_mint(), Pubkey::default());
    }
}
