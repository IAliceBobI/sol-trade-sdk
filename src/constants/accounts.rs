use solana_sdk::{pubkey, pubkey::Pubkey};

pub const SYSTEM_PROGRAM: Pubkey = pubkey!("11111111111111111111111111111111");
pub const SYSTEM_PROGRAM_META: solana_sdk::instruction::AccountMeta =
    solana_sdk::instruction::AccountMeta {
        pubkey: SYSTEM_PROGRAM,
        is_signer: false,
        is_writable: false,
    };

pub const TOKEN_PROGRAM: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_PROGRAM_META: solana_sdk::instruction::AccountMeta =
    solana_sdk::instruction::AccountMeta {
        pubkey: TOKEN_PROGRAM,
        is_signer: false,
        is_writable: false,
    };

pub const TOKEN_PROGRAM_2022: Pubkey = pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const TOKEN_PROGRAM_2022_META: solana_sdk::instruction::AccountMeta =
    solana_sdk::instruction::AccountMeta {
        pubkey: TOKEN_PROGRAM_2022,
        is_signer: false,
        is_writable: false,
    };

/// Alias for TOKEN_PROGRAM_2022 to match usage in code
pub const TOKEN_2022_PROGRAM: Pubkey = TOKEN_PROGRAM_2022;

/// 原生 SOL 的 API 占位符标记（以 11 结尾）
///
/// ⚠️ 注意：这不是一个真实的 Token Mint 地址！
/// 它仅作为 TradingClient API 的标记，表示"用户希望使用原生 SOL 进行交易"
/// 在实际链上操作中，会自动转换为 WSOL_TOKEN_ACCOUNT（以 12 结尾）
///
/// 真实 WSOL Token Mint 地址请使用 `WSOL_TOKEN_ACCOUNT`（以 12 结尾）
pub const NATIVE_SOL_MARKER: Pubkey = pubkey!("So11111111111111111111111111111111111111111");

/// Wrapped SOL (WSOL) Token Mint 地址（以 12 结尾）
///
/// 这是 Solana 上真实的 WSOL Token Mint 地址，Pool 中使用的就是这个地址
/// 注意：NATIVE_SOL_MARKER（以 11 结尾）只是 API 占位符，不是真实 Token Mint
pub const WSOL_TOKEN_ACCOUNT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
pub const WSOL_TOKEN_ACCOUNT_META: solana_sdk::instruction::AccountMeta =
    solana_sdk::instruction::AccountMeta {
        pubkey: WSOL_TOKEN_ACCOUNT,
        is_signer: false,
        is_writable: false,
    };

pub const USD1_TOKEN_ACCOUNT: Pubkey = pubkey!("USD1ttGY1N17NEEHLmELoaybftRBUSErhqYiQzvEmuB");
pub const USD1_TOKEN_ACCOUNT_META: solana_sdk::instruction::AccountMeta =
    solana_sdk::instruction::AccountMeta {
        pubkey: USD1_TOKEN_ACCOUNT,
        is_signer: false,
        is_writable: false,
    };

// USDC (mainnet) mint and meta
pub const USDC_TOKEN_ACCOUNT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
pub const USDC_TOKEN_ACCOUNT_META: solana_sdk::instruction::AccountMeta =
    solana_sdk::instruction::AccountMeta {
        pubkey: USDC_TOKEN_ACCOUNT,
        is_signer: false,
        is_writable: false,
    };

// USDT (mainnet) mint and meta
pub const USDT_TOKEN_ACCOUNT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");
pub const USDT_TOKEN_ACCOUNT_META: solana_sdk::instruction::AccountMeta =
    solana_sdk::instruction::AccountMeta {
        pubkey: USDT_TOKEN_ACCOUNT,
        is_signer: false,
        is_writable: false,
    };

pub const RENT: Pubkey = solana_sdk::sysvar::rent::id();
pub const RENT_META: solana_sdk::instruction::AccountMeta =
    solana_sdk::instruction::AccountMeta { pubkey: RENT, is_signer: false, is_writable: false };

pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub const MEMO_PROGRAM: Pubkey = pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
pub const MEMO_PROGRAM_META: solana_sdk::instruction::AccountMeta =
    solana_sdk::instruction::AccountMeta {
        pubkey: MEMO_PROGRAM,
        is_signer: false,
        is_writable: false,
    };
