// 允许文档格式的当前写法
#![allow(clippy::doc_markdown)]
// 允许经过类型检查后的 unwrap（已验证变量类型）
#![allow(clippy::unwrap_used)]

//! Raydium CPMM 模块
//!
//! 提供 Raydium CPMM DEX 的 Pool 查询、Quote 计算等功能

mod cache;
mod constants;
mod fee_queries;
mod helpers;
mod pool_queries;
mod quotes;

// 导出公共 API
pub use constants::{DEFAULT_WSOL_USDT_CLMM_POOL, accounts, seeds};

// 内部重新导出（crate 内部可访问）
pub(crate) use constants::SWAP_BASE_IN_DISCRIMINATOR;

// helpers 函数现在是内部实现（pub(crate)）
pub(crate) use helpers::{get_pool_pda, get_vault_account};

pub use pool_queries::{
    clear_pool_cache, get_pool_by_address, get_pool_by_address_force, get_pool_by_mint,
    get_pool_by_mint_force, get_pool_token_balances, list_pools_by_mint,
};

pub use quotes::{get_token_price_in_usd, get_token_price_in_usd_with_pool};

// 内部重新导出（crate 内部可访问）
pub(crate) use quotes::{quote_exact_in, quote_exact_out};

pub use fee_queries::{FeeRates, clear_amm_config_cache, get_amm_config_fees};

// 重新导出类型
pub use crate::instruction::utils::raydium_cpmm_types::{PoolState, pool_state_decode};
