//! 常用代币常量定义
//!
//! 用于硬编码已知代币的 mint 地址和 symbol

use solana_sdk::pubkey;

pub use solana_sdk::pubkey::Pubkey;

/// SOL Mint (Wrapped SOL)
pub const SOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

/// USDC Mint (mainnet)
pub const USDC_MINT: Pubkey = pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

/// USDT Mint (mainnet)
pub const USDT_MINT: Pubkey = pubkey!("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB");

/// RAY (Raydium) Mint
pub const RAY_MINT: Pubkey = pubkey!("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R");
