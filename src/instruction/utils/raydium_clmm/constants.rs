// Raydium CLMM 常量定义

use solana_sdk::{pubkey, pubkey::Pubkey};

/// Raydium CLMM WSOL-USDT 锚定池（用于 USD 价格计算）
/// 如果不传入锚定池参数，默认使用此池
pub const DEFAULT_WSOL_USDT_CLMM_POOL: Pubkey =
    pubkey!("ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6");

/// Seeds for PDA derivation
pub mod seeds {
    pub const TICK_ARRAY_SEED: &[u8] = b"tick_array";
    pub const POOL_TICK_ARRAY_BITMAP_SEED: &[u8] = b"pool_tick_array_bitmap_extension";
}

/// 常量偏移量
pub const TOKEN_MINT0_OFFSET: usize = 73;
pub const TOKEN_MINT1_OFFSET: usize = 105;

/// 缓存最大容量
pub const MAX_CACHE_SIZE: usize = 50_000;

/// 每个 Tick Array 包含的 tick 数量
pub const TICKS_PER_ARRAY: i32 = 60;
