// 允许文档格式的当前写法
#![allow(clippy::doc_markdown)]

//! Raydium AMM V4 工具模块
//!
//! 提供 Pool 查询、Quote 计算等功能

mod constants;
mod helpers;
mod pool_queries;
mod quotes;
mod serum_market;

// 重新导出公共 API
pub use constants::{DEFAULT_WSOL_USDT_CLMM_POOL, accounts, pool_status, seeds};

// 内部重新导出（crate 内部可访问）
pub(crate) use constants::SWAP_BASE_IN_DISCRIMINATOR;

pub use pool_queries::{
    clear_pool_cache, get_pool_by_address, get_pool_by_address_force, get_pool_by_mint,
    get_pool_by_mint_force, get_token_price_in_usd, get_token_price_in_usd_with_pool,
    list_pools_by_mint,
};

// quote_exact_in 和 quote_exact_out 现在是内部实现，使用 client.buy_quote() / client.sell_quote() 代替
// 内部重新导出（crate 内部可访问）
pub(crate) use quotes::{quote_exact_in, quote_exact_out};

// helpers 函数 - 用户可能需要检查 pool 状态
pub use helpers::{is_hot_mint, is_pool_active, is_pool_tradeable};

pub use serum_market::{derive_vault_signer, parse_market_account, MarketState};
