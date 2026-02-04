/// Quote / simulation helpers for swap routing.
///
/// This module provides a common result type so higher layers can compare pools
/// using a consistent shape.
use solana_sdk::pubkey::Pubkey;

/// Quote exact-in 参数
#[derive(Debug, Clone)]
pub struct QuoteExactInParams {
    /// Pool 地址
    pub pool_address: Pubkey,
    /// 输入代币的 mint 地址
    pub input_mint: Pubkey,
    /// 输出代币的 mint 地址
    pub output_mint: Pubkey,
    /// 输入金额（最小单位）
    pub amount_in: u64,
}

/// Quote exact-out 参数
#[derive(Debug, Clone)]
pub struct QuoteExactOutParams {
    /// Pool 地址
    pub pool_address: Pubkey,
    /// 输入代币的 mint 地址
    pub input_mint: Pubkey,
    /// 输出代币的 mint 地址
    pub output_mint: Pubkey,
    /// 输出金额（最小单位）
    pub amount_out: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct QuoteExactInResult {
    /// Output amount for an exact-in swap (in smallest units).
    pub amount_out: u64,
    /// Total fee amount paid (in input token units, smallest units).
    pub fee_amount: u64,
    /// Optional price impact estimation in basis points.
    pub price_impact_bps: Option<u64>,
    /// Number of extra on-chain accounts read to produce this quote.
    pub extra_accounts_read: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct QuoteExactOutResult {
    /// Required input amount for an exact-out swap (in smallest units).
    pub amount_in: u64,
    /// Total fee amount paid (in input token units, smallest units).
    pub fee_amount: u64,
    /// Optional price impact estimation in basis points.
    pub price_impact_bps: Option<u64>,
    /// Number of extra on-chain accounts read to produce this quote.
    pub extra_accounts_read: usize,
}
