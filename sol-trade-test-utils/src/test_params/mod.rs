//! DEX 测试参数构造工具
//!
//! 提供可复用的测试参数构造函数，用于不同 DEX 测试场景
//!
//! # 支持的 DEX
//!
//! - Raydium CPMM: PIPE-WSOL, USDC-PRTS
//! - Raydium CLMM: USDT-WSOL, SOLETT-WSOL
//! - PumpSwap: PUMP-WSOL, BONK-WSOL
//!
//! # 使用示例
//!
//! ```ignore
//! use sol_trade_test_utils::{PumpWsolBuyParamsBuilder, PumpWsolSellParamsBuilder};
//!
//! // 构建买入参数
//! let buy_params = PumpWsolBuyParamsBuilder::new(Some(1_000_000))
//!     .slippage(1000) // 1% 滑点
//!     .build(&client)
//!     .await;
//!
//! // 构建卖出参数
//! let sell_params = PumpWsolSellParamsBuilder::new(10_000_000_000)
//!     .slippage(1000) // 1% 滑点
//!     .build(&client)
//!     .await;
//! ```

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// 导出所有子模块
pub mod clmm_params;
pub mod constants;
pub mod cpmm_params;
pub mod gas_fee;
pub mod liquidity;
pub mod pumpswap_params;

// 重新导出常用的构建器
pub use clmm_params::{
    SolettWsolClmmBuyParamsBuilder, SolettWsolClmmSellParamsBuilder, UsdtWsolClmmBuyParamsBuilder,
    UsdtWsolClmmSellParamsBuilder,
};
pub use constants::*;
pub use cpmm_params::{
    PipeWsolBuyParamsBuilder, PipeWsolSellParamsBuilder, UsdcPrtsBuyParamsBuilder,
    UsdcPrtsSellParamsBuilder,
};
pub use gas_fee::create_test_gas_fee_strategy;
pub use liquidity::{CpmmLiquidityBuilder, PipeWsolLiquidityBuilder};
pub use pumpswap_params::{
    BonkWsolBuyParamsBuilder, BonkWsolSellParamsBuilder, PumpWsolBuyParamsBuilder,
    PumpWsolSellParamsBuilder,
};

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

/// 获取 PUMP-WSOL Pool 地址
pub fn pump_wsol_pool() -> Pubkey {
    Pubkey::from_str(PUMP_WSOL_POOL).unwrap()
}

/// 获取 BONK-WSOL Pool 地址
pub fn bonk_wsol_pool() -> Pubkey {
    Pubkey::from_str(BONK_WSOL_POOL).unwrap()
}

/// 获取 PUMP Mint
pub fn pump_mint() -> Pubkey {
    Pubkey::from_str(PUMP_MINT).unwrap()
}

/// 获取 BONK Mint
pub fn bonk_mint() -> Pubkey {
    Pubkey::from_str(BONK_MINT).unwrap()
}

/// 获取 USDT-WSOL Pool 地址
pub fn usdt_wsol_pool() -> Pubkey {
    Pubkey::from_str(USDT_WSOL_POOL).unwrap()
}

/// 获取 USDT Mint
pub fn usdt_mint() -> Pubkey {
    Pubkey::from_str(USDT_MINT).unwrap()
}

/// 获取 SOLETT-WSOL Pool 地址
pub fn solett_wsol_pool() -> Pubkey {
    Pubkey::from_str(SOLETT_WSOL_POOL).unwrap()
}

/// 获取 SOLETT Mint
pub fn solett_mint() -> Pubkey {
    Pubkey::from_str(SOLETT_MINT).unwrap()
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
        assert_ne!(pump_wsol_pool(), Pubkey::default());
        assert_ne!(pump_mint(), Pubkey::default());
        assert_ne!(bonk_wsol_pool(), Pubkey::default());
        assert_ne!(bonk_mint(), Pubkey::default());
        assert_ne!(usdt_wsol_pool(), Pubkey::default());
        assert_ne!(usdt_mint(), Pubkey::default());
        assert_ne!(solett_wsol_pool(), Pubkey::default());
        assert_ne!(solett_mint(), Pubkey::default());
    }
}
