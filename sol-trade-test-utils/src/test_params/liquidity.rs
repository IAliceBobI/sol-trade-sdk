//! 流动性添加构建器
//!
//! 提供向 CPMM 池子添加流动性的参数构建器

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use sol_trade_sdk::liquidity::cpmm::{
    build_deposit_instruction, calculate_deposit_amounts, CpmmDepositParams,
};

use super::constants::{PIPE_MINT, PIPE_WSOL_POOL, WSOL_MINT};

// ==================== 通用 CPMM 流动性构建器 ====================

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
        let owner_lp_token =
            spl_associated_token_account::get_associated_token_address_with_program_id(
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

// ==================== PIPE-WSOL 专用流动性构建器 ====================

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
                Pubkey::from_str(PIPE_WSOL_POOL).unwrap(),
                Pubkey::from_str(PIPE_MINT).unwrap(),
                Pubkey::from_str(WSOL_MINT).unwrap(),
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
        self.inner.build(payer, pool_state, token_0_vault_amount, token_1_vault_amount)
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
