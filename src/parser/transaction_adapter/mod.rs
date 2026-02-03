//! 交易适配器模块
//!
//! 将交易数据适配为统一的内部格式，提供便捷的访问接口

mod adapter;
mod errors;
mod parsers;
mod types;

pub use adapter::TransactionAdapter;
pub use errors::AdapterError;
pub use types::{InnerInstructionInfo, InstructionInfo, TokenAmount, TransferData};
