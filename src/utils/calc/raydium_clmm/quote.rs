// Copyright (c) Raydium Foundation
// Licensed under Apache 2.0

//! CLMM Quote 函数
//!
//! 提供 Exact In 和 Exact Out 的 quote 计算接口。

use super::swap_exact_out::calculate_swap_exact_out_with_tick_arrays;
use super::types::QuoteExactOutResult;
use crate::utils::calc::clmm_math::swap_math::FEE_RATE_DENOMINATOR_VALUE;

/// Quote an exact-out swap against a Raydium CLMM pool (简化版本)
///
/// 计算获得指定输出金额所需的输入金额。
///
/// # Arguments
///
/// * `sqrt_price_x64` - Current sqrt price
/// * `liquidity` - Current pool liquidity
/// * `amount_out` - Desired output amount
/// * `zero_for_one` - Direction of swap (true = token0->token1, false = token1->token0)
/// * `fee_rate` - Fee rate (as u64)
///
/// # Returns
///
/// Returns `QuoteExactOutResult` containing the required input amount and fees
///
/// # Errors
///
/// Returns error if:
/// - Insufficient liquidity
/// - Calculation overflow
///
/// # Limitations
///
/// 这是一个简化实现，假设交易不会跨越 tick array 边界。
/// 对于大额交易，请使用 exact_in 模式或实现完整的 tick array 遍历。
pub fn quote_exact_out_simplified(
    sqrt_price_x64: u128,
    liquidity: u128,
    amount_out: u64,
    zero_for_one: bool,
    fee_rate: u64,
) -> Result<QuoteExactOutResult, String> {
    if liquidity == 0 {
        return Err("No liquidity available in the pool".to_string());
    }

    if amount_out == 0 {
        return Err("amount_out must be greater than 0".to_string());
    }

    // 简化实现：使用恒定乘积公式近似计算
    // CLMM 可以近似看作 CPMM，价格 p = sqrt_price^2 / 2^128
    // token1/token0 = p，因此 token0 = token1 / p

    // 计算归一化价格（避免溢出）
    // normalized_price = (sqrt_price / 2^32)^2 = sqrt_price^2 / 2^64
    let price_shifted = sqrt_price_x64 >> 32; // 除以 2^32
    let normalized_price = price_shifted
        .checked_mul(price_shifted)
        .ok_or_else(|| "Price calculation overflow".to_string())?;

    // 根据方向计算输入金额
    let amount_in = if zero_for_one {
        // token0 -> token1: 输入 token0，输出 token1
        // amount_in = amount_out / price
        // 为了精确计算，使用：amount_in = (amount_out * SCALE) / price
        const SCALE: u128 = 1_000_000_000_000;

        let amount_scaled = (amount_out as u128)
            .checked_mul(SCALE)
            .ok_or_else(|| "Amount scaling overflow".to_string())?;

        // 确保 price != 0
        if normalized_price == 0 {
            return Err("Invalid price: price is zero".to_string());
        }

        amount_scaled
            .checked_div(normalized_price)
            .ok_or_else(|| "Amount division error".to_string())?
    } else {
        // token1 -> token0: 输入 token1，输出 token0
        // amount_in = amount_out * price
        (amount_out as u128)
            .checked_mul(normalized_price)
            .ok_or_else(|| "Amount multiplication overflow".to_string())?
    };

    // 转换回原始 scale（对于 zero_for_one 的情况）
    let amount_in = if zero_for_one {
        amount_in
            .checked_div(1_000_000_000_000u128)
            .ok_or_else(|| "Amount descaling overflow".to_string())?
    } else {
        amount_in
    };

    // 确保计算结果合理
    if amount_in == 0 {
        return Err(
            "Calculated amount_in is zero, insufficient liquidity or invalid price".to_string()
        );
    }

    // 检查是否超过流动性限制
    if amount_in as u64 > amount_out * 1000 {
        // 如果输入金额是输出金额的 1000 倍以上，可能流动性不足
        return Err("Insufficient liquidity for this trade".to_string());
    }

    // 计算手续费
    let fee_amount = amount_in
        .checked_mul(fee_rate as u128)
        .and_then(|p: u128| p.checked_div(FEE_RATE_DENOMINATOR_VALUE as u128))
        .ok_or_else(|| "Fee calculation overflow".to_string())? as u64;

    let total_amount_in = (amount_in as u64)
        .checked_add(fee_amount)
        .ok_or_else(|| "Total amount overflow".to_string())?;

    // 价格影响（简化：输出金额占总流动性比例）
    let price_impact_bps = (amount_out as u128)
        .checked_mul(10_000u128)
        .and_then(|p| p.checked_div(liquidity))
        .map(|impact| impact as u64);

    Ok(QuoteExactOutResult { amount_in: total_amount_in, fee_amount, price_impact_bps })
}

/// Quote an exact-out swap against a Raydium CLMM pool (完整版本)
///
/// 使用完整的 tick array 遍历算法，支持大额交易和跨 tick 边界。
///
/// # Arguments
///
/// * `amount_out` - 期望的输出金额（固定）
/// * `sqrt_price_x64` - 当前平方根价格
/// * `liquidity` - 当前流动性
/// * `tick_current` - 当前 tick
/// * `tick_spacing` - tick 间距
/// * `fee_rate` - 手续费率
/// * `zero_for_one` - 交易方向 (true = token0->token1, false = token1->token0)
/// * `tick_arrays` - tick array 数据 (从 RPC 获取)
///
/// # Returns
///
/// 返回 `QuoteExactOutResult` 包含所需的输入金额和手续费
///
/// # Errors
///
/// 如果流动性不足或计算溢出则返回错误
pub fn quote_exact_out(
    amount_out: u64,
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current: i32,
    tick_spacing: u16,
    fee_rate: u32,
    zero_for_one: bool,
    tick_arrays: &[(i32, Vec<(i32, i128, u128)>)],
) -> Result<QuoteExactOutResult, String> {
    // 调用完整版计算函数
    let result = calculate_swap_exact_out_with_tick_arrays(
        amount_out,
        sqrt_price_x64,
        liquidity,
        tick_current,
        tick_spacing,
        fee_rate,
        zero_for_one,
        tick_arrays,
    )
    .map_err(|e| e.to_string())?;

    // 注意：calculate_swap_exact_out_with_tick_arrays 已经返回包含手续费的结果
    // price_impact_bps 在内部设置为 None，可以根据需要计算
    Ok(result)
}
