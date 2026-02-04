// Raydium CPMM 常量定义

use solana_sdk::{pubkey, pubkey::Pubkey};

/// Raydium CLMM WSOL-USDT 锚定池（用于 USD 价格计算）
/// 如果不传入锚定池参数，默认使用此池
pub const DEFAULT_WSOL_USDT_CLMM_POOL: Pubkey =
    pubkey!("ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6");

/// Constants used as seeds for deriving PDAs (Program Derived Addresses)
pub mod seeds {
    pub const POOL_SEED: &[u8] = b"pool";
    pub const POOL_VAULT_SEED: &[u8] = b"pool_vault";
    pub const OBSERVATION_STATE_SEED: &[u8] = b"observation";
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};
    pub const AUTHORITY: Pubkey = pubkey!("GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL");
    pub const RAYDIUM_CPMM: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
    pub const FEE_RATE_DENOMINATOR_VALUE: u128 = 1_000_000;
    pub const TRADE_FEE_RATE: u64 = 2500;
    pub const CREATOR_FEE_RATE: u64 = 0;
    pub const PROTOCOL_FEE_RATE: u64 = 120000;
    pub const FUND_FEE_RATE: u64 = 40000;
    // META
    pub const AUTHORITY_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: AUTHORITY,
            is_signer: false,
            is_writable: false,
        };
}

pub(crate) const SWAP_BASE_IN_DISCRIMINATOR: &[u8] = &[143, 190, 90, 218, 196, 30, 51, 222];
pub(crate) const SWAP_BASE_OUT_DISCRIMINATOR: &[u8] = &[55, 217, 98, 86, 163, 74, 180, 173];

/// 缓存大小限制
pub(crate) const MAX_CACHE_SIZE: usize = 50_000;

/// 常量偏移量（包含 discriminator）
/// 根据实际数据解析结果，mintA 在 offset 160（不包含 discriminator），mintB 在 offset 192（不包含 discriminator）
/// RPC 查询时使用包含 discriminator 的偏移量，所以需要加 8
pub(crate) const TOKEN0_MINT_OFFSET: usize = 168; // mintA offset (160 + 8 discriminator)
pub(crate) const TOKEN1_MINT_OFFSET: usize = 200; // mintB offset (192 + 8 discriminator)
