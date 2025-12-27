/// Quote / simulation helpers for swap routing.
///
/// This module provides a common result type so higher layers can compare pools
/// using a consistent shape.

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

