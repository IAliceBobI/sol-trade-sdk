// 允许在库初始化代码中使用 expect，因为：
// 1. 关键资源初始化失败是严重错误，应该 panic
// 2. 用户提供的配置已经在其他地方验证
#![allow(clippy::expect_used)]

pub mod client;
pub mod common;
pub mod constants;
pub mod instruction;
pub mod parser;
pub mod perf;
pub mod swqos;
pub mod trading;
pub mod utils;

// 导出模块
mod exports;
mod infrastructure;

// 重导出公共接口
pub use exports::*;

// 重导出 TradingClient 和相关类型
pub use client::{SolanaTrade, TradeBuyParams, TradeSellParams, TradingClient};
pub use infrastructure::TradingInfrastructure;

/// Type of the token to buy
#[derive(Clone, PartialEq)]
pub enum TradeTokenType {
    SOL,
    WSOL,
    USD1,
    USDC,
}
