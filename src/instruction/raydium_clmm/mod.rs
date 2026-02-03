//! Raydium CLMM (Concentrated Liquidity Market Maker) instruction builder
//!
//! 此模块提供 Raydium CLMM 协议的交易指令构建功能。
//!
//! ## 架构
//!
//! - **builder.rs**: InstructionBuilder trait 的实现
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
mod helpers;

pub use builder::RaydiumClmmInstructionBuilder;
pub use helpers::{amount_with_slippage, fallback_price_calculation};
