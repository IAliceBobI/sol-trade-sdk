//! Solana DEX 交易解析模块

// 允许在整个解析器模块中使用 unwrap，因为：
// 1. 解析的二进制数据格式是已知的
// 2. 解析失败表示数据损坏，应该 panic
// 3. 这些是解析工具，输入应该是有效的交易数据
#![allow(clippy::unwrap_used)]

pub mod base_parser;
pub mod constants;
pub mod dex_parser;
pub mod discriminators;
pub mod instruction_data_parser;
pub mod transaction_adapter;
pub mod types;
pub mod utils;

pub mod pumpswap;
pub mod raydium;

pub use dex_parser::DexParser;
pub use discriminators::{DexProtocol, DiscriminatorRegistry, InstructionType};
pub use instruction_data_parser::{
    format_token_amount, parse_u64_from_offset, parse_u128_from_offset,
};
pub use types::*;
pub use utils::BinaryReader;
