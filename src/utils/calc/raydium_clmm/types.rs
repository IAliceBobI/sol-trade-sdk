// Copyright (c) Raydium Foundation
// Licensed under Apache 2.0

//! CLMM 类型定义
//!
//! 包含 swap 计算所需的所有数据结构。

/// Swap 状态
#[derive(Debug, Clone)]
pub struct SwapState {
    /// 剩余需要消耗的输入量
    pub amount_specified_remaining: u64,
    /// 已计算的输出量
    pub amount_calculated: u64,
    /// 累计手续费
    pub fee_amount: u64,
    /// 当前价格
    pub sqrt_price_x64: u128,
    /// 当前 tick
    pub tick: i32,
    /// 当前流动性
    pub liquidity: u128,
}

/// CLMM Swap 计算结果
#[derive(Debug, Clone, Copy)]
pub struct SwapCalculationResult {
    /// 输出金额
    pub amount_out: u64,
    /// 累计手续费
    pub fee_amount: u64,
}

/// Step 计算状态
#[derive(Debug, Clone, Default)]
pub struct StepComputations {
    pub sqrt_price_start_x64: u128,
    pub tick_next: i32,
    pub initialized: bool,
    pub sqrt_price_next_x64: u128,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee_amount: u64,
}

/// 简化的 Tick 状态（客户端版本）
#[derive(Debug, Clone, Default)]
pub struct TickState {
    pub tick: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
}

impl TickState {
    pub fn is_initialized(&self) -> bool {
        self.liquidity_gross != 0
    }
}

/// Result of an exact-out swap calculation
#[derive(Debug, Clone)]
pub struct QuoteExactOutResult {
    /// Required input amount (including fees)
    pub amount_in: u64,
    /// Fee amount charged
    pub fee_amount: u64,
    /// Price impact in basis points (optional)
    pub price_impact_bps: Option<u64>,
}
