//! TradingClient 模块
//!
//! 将 TradingClient 及其相关方法拆分到多个子模块以提高代码组织性。

mod constructor;
mod helpers;
mod pumpfun;
mod quote;
mod simulation;
mod trading;
mod types;
mod wsol;

// 重导出所有公共接口
pub use constructor::SolanaTrade;
pub use types::{TradeBuyParams, TradeSellParams, TradingClient};
