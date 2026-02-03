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

// 重新导出所有公共接口
pub use constants::*;
pub use helpers::*;
pub use pool_queries::*;
pub use price::*;
pub use quotes::*;
