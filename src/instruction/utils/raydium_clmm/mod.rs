// 允许文档格式的当前写法
#![allow(clippy::doc_markdown)]
// 允许经过类型检查后的 unwrap_err（已验证变量类型）
#![allow(clippy::unwrap_used)]
// 允许未使用的初始化（为了代码可读性）
#![allow(unused_assignments)]

// Raydium CLMM 模块
// 将原来的单文件拆分为多个子模块以提高可维护性

mod cache;
mod constants;
mod helpers;
mod pool_queries;
mod price;
mod quotes;
mod tick_array_bitmap;

// 重新导出所有公共接口
pub use constants::*;
pub use helpers::*;
pub use pool_queries::*;
pub use price::*;
// quote_exact_in 和 quote_exact_out 现在是内部实现，使用 client.buy_quote() / client.sell_quote() 代替
pub use tick_array_bitmap::*;

// 显式公开导出 get_tick_array_start_index（用于测试）
pub use helpers::get_tick_array_start_index;

// 内部重新导出（crate 内部可访问）
pub(crate) use quotes::{quote_exact_in, quote_exact_out};
