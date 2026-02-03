//! Raydium CLMM (Concentrated Liquidity Market Maker) instruction builder
//!
//! 此模块提供 Raydium CLMM 协议的交易指令构建功能。
//!
//! ## 架构
//!
//! - **builder.rs**: InstructionBuilder trait 的实现
//! - **builder_helpers.rs**: 指令构建辅助函数（tick arrays、账户列表等）
//! - **helpers.rs**: 辅助函数（滑点计算、价格计算等）
//!
//! ## 使用示例
//!
//! ```rust
//! use sol_trade_sdk::instruction::raydium_clmm::RaydiumClmmInstructionBuilder;
//! use sol_trade_sdk::trading::core::traits::InstructionBuilder;
//!
//! let builder = RaydiumClmmInstructionBuilder;
//! let instructions = builder.build_buy_instructions(&params).await?;
//! ```

mod builder;
mod builder_helpers;
mod helpers;

pub use builder::RaydiumClmmInstructionBuilder;
pub use builder_helpers::{calculate_slippage_amount, get_swap_tick_arrays, MAX_SQRT_PRICE_X64, MIN_SQRT_PRICE_X64};
pub use helpers::{amount_with_slippage, fallback_price_calculation};
