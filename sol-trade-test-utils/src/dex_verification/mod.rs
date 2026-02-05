//! DEX 三阶段验证测试框架
//!
//! 提供通用的 DEX 交易验证框架，支持：
//! - 多种 DEX 类型（CLMM, CPMM, AMM V4, PumpSwap）
//! - 多种操作类型（Buy/Sell × Exact In/Out）
//! - 多种 Pool Program 类型（Token/Token, Token/Token2022, Token2022/Token2022）
//!
//! # 使用示例
//!
//! ```ignore
//! use sol_trade_test_utils::dex_verification::{
//!     run_dex_three_stage_verification, DexVerifyConfig, OperationType,
//!     PoolConfig, TokenProgramType, TradeDirection,
//! };
//! use sol_trade_sdk::DexType;
//!
//! let config = DexVerifyConfig {
//!     dex_type: DexType::RaydiumCpmm,
//!     pool: PoolConfig::new(
//!         pool_address,
//!         "PIPE-WSOL",
//!         token0_mint,
//!         TokenProgramType::Token,
//!         token1_mint,
//!         TokenProgramType::Token,
//!         10,
//!     ),
//!     operation: OperationType::BuyExactIn,
//!     direction: TradeDirection::Token1ToToken0,
//!     input_amount: 1_000,
//! };
//!
//! run_dex_three_stage_verification(&client, config).await?;
//! ```

mod framework;
mod pool_registry;
mod types;

// 重新导出常用类型
pub use framework::{
    cleanup_pool_cache, run_dex_three_stage_verification, run_dex_three_stage_verification_sell,
    verify_three_stage_accuracy, BuyParamsBuilder, ExecutionResult, SellParamsBuilder,
    ThreeStageResult, TransactionType,
};
pub use pool_registry::{
    PumpSwapPoolRegistry, RaydiumAmmV4PoolRegistry, RaydiumClmmPoolRegistry,
    RaydiumCpmmPoolRegistry,
};
pub use types::{DexVerifyConfig, OperationType, PoolConfig, TokenProgramType, TradeDirection};
