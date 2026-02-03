// Raydium AMM V4 常量定义

use solana_sdk::{pubkey, pubkey::Pubkey};

/// Raydium CLMM WSOL-USDT 锚定池（用于 USD 价格计算）
/// 如果不传入锚定池参数，默认使用此池
pub const DEFAULT_WSOL_USDT_CLMM_POOL: Pubkey =
    pubkey!("ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6");

/// Constants used as seeds for deriving PDAs (Program Derived Addresses)
pub mod seeds {
    pub const POOL_SEED: &[u8] = b"pool";
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};

    pub const AUTHORITY: Pubkey = pubkey!("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    pub const RAYDIUM_AMM_V4: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");

    pub const TRADE_FEE_NUMERATOR: u64 = 25;
    pub const TRADE_FEE_DENOMINATOR: u64 = 10000;
    pub const SWAP_FEE_NUMERATOR: u64 = 25;
    pub const SWAP_FEE_DENOMINATOR: u64 = 10000;

    // META

    pub const AUTHORITY_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: AUTHORITY,
            is_signer: false,
            is_writable: false,
        };
}

pub const SWAP_BASE_IN_DISCRIMINATOR: &[u8] = &[9];
pub const SWAP_BASE_OUT_DISCRIMINATOR: &[u8] = &[11];

/// Pool 状态常量
pub mod pool_status {
    /// 未初始化
    pub const UNINITIALIZED: u64 = 0;
    /// 已初始化
    pub const INITIALIZED: u64 = 1;
    /// 已禁用
    pub const DISABLED: u64 = 2;
    /// 只能提现
    pub const WITHDRAW_ONLY: u64 = 3;
    /// 只能订单簿
    pub const ORDER_BOOK_ONLY: u64 = 4;
    /// 只能交易
    pub const SWAP_ONLY: u64 = 5;
    /// 活跃状态
    pub const ACTIVE: u64 = 6;
}

/// coin_mint 在 AmmInfo 结构中的偏移量
///
/// 根据 AmmInfo 字段顺序与 Borsh 编码规则计算：
/// - 16 个 u64 字段 (16 * 8 = 128 字节)
/// - Fees (8 个 u64, 8 * 8 = 64 字节)
/// - OutPutData (10 个 u64 与 4 个 u128, 共 144 字节)
/// - token_coin (Pubkey, 32 字节)
/// - token_pc (Pubkey, 32 字节)
///   因此 coin_mint 起始偏移量为 128 + 64 + 144 + 32 + 32 = 400 字节。
pub const COIN_MINT_OFFSET: usize = 400;

/// pc_mint 在 AmmInfo 结构中的偏移量
/// 即 coin_mint 之后再偏移一个 Pubkey (32 字节)
pub const PC_MINT_OFFSET: usize = 432;

/// 缓存最大容量
pub const MAX_CACHE_SIZE: usize = 50_000;
