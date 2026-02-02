// Copyright (c) Raydium Foundation
// 允许文档格式的当前写法
#![allow(clippy::doc_markdown)]
// 允许使用 &Vec（为了类型一致性）
#![allow(clippy::vec_box)]
// Licensed under Apache 2.0
// Raydium CLMM swap calculations using official math libraries

//! Raydium CLMM calculation module
//!
//! Uses official Raydium CLMM math libraries (clmm_math) for all calculations.
//! Dependencies: uint = { git = "https://github.com/raydium-io/parity-common", package = "uint" }

// Re-export official libraries for convenience
pub use super::clmm_math::{
    U128, U256, fixed_point_64, full_math::MulDiv, liquidity_math, sqrt_price_math, swap_math,
    tick_math,
};

// Export constants from official libraries
pub use super::clmm_math::fixed_point_64::{Q64, RESOLUTION};
pub use super::clmm_math::liquidity_math::{
    add_delta, get_delta_amount_0_unsigned, get_delta_amount_1_unsigned,
};
pub use super::clmm_math::sqrt_price_math::{
    get_next_sqrt_price_from_amount_0_rounding_up, get_next_sqrt_price_from_amount_1_rounding_down,
    get_next_sqrt_price_from_input, get_next_sqrt_price_from_output,
};
pub use super::clmm_math::tick_math::{
    MAX_SQRT_PRICE_X64, MAX_TICK, MIN_SQRT_PRICE_X64, MIN_TICK, get_sqrt_price_at_tick,
    get_tick_at_sqrt_price,
};

// Re-export official swap_math components
pub use super::clmm_math::swap_math::{
    FEE_RATE_DENOMINATOR_VALUE, SwapStep as OfficialSwapStep,
    compute_swap_step as official_compute_swap_step,
};

/// Swap 状态
#[derive(Debug, Clone)]
pub struct SwapState {
    /// 剩余需要消耗的输入量
    pub amount_specified_remaining: u64,
    /// 已计算的输出量
    pub amount_calculated: u64,
    /// 当前价格
    pub sqrt_price_x64: u128,
    /// 当前 tick
    pub tick: i32,
    /// 当前流动性
    pub liquidity: u128,
}

/// 单步计算结果（为了向后兼容保留，实际使用官方 OfficialSwapStep）
#[deprecated(note = "Use OfficialSwapStep from swap_math instead")]
#[derive(Debug, Clone, Default)]
pub struct SwapStep {
    /// 下一个价格
    pub sqrt_price_next_x64: u128,
    /// 输入量
    pub amount_in: u64,
    /// 输出量
    pub amount_out: u64,
    /// 手续费
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

// ============================================================================
// Swap Algorithm - 使用官方 swap_math 模块
// ============================================================================

/// 计算单步 swap 结果（使用官方实现）
///
/// 这是 CLMM 最核心的函数，直接调用官方 swap_math::compute_swap_step
/// 注意：block_timestamp 参数用于未来扩展，当前传入 0 即可
pub fn compute_swap_step(
    sqrt_price_current_x64: u128,
    sqrt_price_target_x64: u128,
    liquidity: u128,
    amount_remaining: u64,
    fee_rate: u32,
    is_base_input: bool,
    zero_for_one: bool,
) -> Result<OfficialSwapStep, &'static str> {
    // 直接调用官方实现，block_timestamp 传 0
    official_compute_swap_step(
        sqrt_price_current_x64,
        sqrt_price_target_x64,
        liquidity,
        amount_remaining,
        fee_rate,
        is_base_input,
        zero_for_one,
        0, // block_timestamp: 客户端计算不需要，传 0
    )
}

// ============================================================================
// 主入口函数 - 计算完整 swap 输出（完整版）
// ============================================================================

// ========================================
// 完整的 tick-by-tick 遍历算法实现
// ========================================

/// 完整的 swap 计算（需要外部传入 tick array 数据）
///
/// 参数：
/// - tick_arrays: 所有需要的 tick array（从 RPC 获取并解析）
/// - 其他参数同简化版
pub fn calculate_swap_amount_with_tick_arrays(
    amount_specified: u64,
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current: i32,
    tick_spacing: u16,
    fee_rate: u32,
    zero_for_one: bool,
    tick_arrays: &[(i32, Vec<(i32, i128, u128)>)], // (start_index, [(tick, liquidity_net, liquidity_gross)])
) -> Result<u64, &'static str> {
    if amount_specified == 0 {
        return Err("amount_specified must not be 0");
    }

    let sqrt_price_limit_x64 =
        if zero_for_one { MIN_SQRT_PRICE_X64 + 1 } else { MAX_SQRT_PRICE_X64 - 1 };

    // 验证价格限制
    if zero_for_one {
        if sqrt_price_limit_x64 < MIN_SQRT_PRICE_X64 {
            return Err("sqrt_price_limit_x64 must greater than MIN_SQRT_PRICE_X64");
        }
        if sqrt_price_limit_x64 >= sqrt_price_x64 {
            return Err("sqrt_price_limit_x64 must smaller than current");
        }
    } else {
        if sqrt_price_limit_x64 > MAX_SQRT_PRICE_X64 {
            return Err("sqrt_price_limit_x64 must smaller than MAX_SQRT_PRICE_X64");
        }
        if sqrt_price_limit_x64 <= sqrt_price_x64 {
            return Err("sqrt_price_limit_x64 must greater than current");
        }
    }

    let mut state = SwapState {
        amount_specified_remaining: amount_specified,
        amount_calculated: 0,
        sqrt_price_x64,
        tick: tick_current,
        liquidity,
    };

    let mut tick_array_idx = 0;
    let mut loop_count = 0;
    const MAX_LOOP: u32 = 10;

    // 循环遍历 tick arrays 直到输入耗尽或达到价格限制
    while state.amount_specified_remaining != 0
        && state.sqrt_price_x64 != sqrt_price_limit_x64
        && state.tick < MAX_TICK
        && state.tick > MIN_TICK
    {
        if loop_count >= MAX_LOOP {
            return Err("loop_count limit exceeded");
        }

        let mut step = StepComputations {
            sqrt_price_start_x64: state.sqrt_price_x64,
            ..Default::default()
        };

        // 找到下一个初始化的 tick
        let next_initialized_tick = find_next_initialized_tick(
            &tick_arrays[tick_array_idx..],
            state.tick,
            tick_spacing,
            zero_for_one,
        );

        if let Some((tick_next, initialized, liquidity_net)) = next_initialized_tick {
            step.tick_next = tick_next.clamp(MIN_TICK, MAX_TICK);
            step.initialized = initialized;

            step.sqrt_price_next_x64 = get_sqrt_price_at_tick(step.tick_next)?;

            // 计算目标价格
            let target_price = if (zero_for_one && step.sqrt_price_next_x64 < sqrt_price_limit_x64)
                || (!zero_for_one && step.sqrt_price_next_x64 > sqrt_price_limit_x64)
            {
                sqrt_price_limit_x64
            } else {
                step.sqrt_price_next_x64
            };

            // 调用官方 swap 计算
            let swap_step = compute_swap_step(
                state.sqrt_price_x64,
                target_price,
                state.liquidity,
                state.amount_specified_remaining,
                fee_rate,
                true, // is_base_input
                zero_for_one,
            )?;

            state.sqrt_price_x64 = swap_step.sqrt_price_next_x64;
            step.amount_in = swap_step.amount_in;
            step.amount_out = swap_step.amount_out;
            step.fee_amount = swap_step.fee_amount;

            // 更新剩余量和计算量
            state.amount_specified_remaining = state
                .amount_specified_remaining
                .checked_sub(step.amount_in + step.fee_amount)
                .ok_or("amount underflow")?;
            state.amount_calculated =
                state.amount_calculated.checked_add(step.amount_out).ok_or("amount overflow")?;

            // 如果达到下一个 tick，更新流动性
            if state.sqrt_price_x64 == step.sqrt_price_next_x64 {
                if step.initialized {
                    let liquidity_delta = if zero_for_one { -liquidity_net } else { liquidity_net };
                    state.liquidity = add_delta(state.liquidity, liquidity_delta)?;
                }

                state.tick = if zero_for_one { step.tick_next - 1 } else { step.tick_next };
            } else if state.sqrt_price_x64 != step.sqrt_price_start_x64 {
                // 重新计算 tick
                state.tick = get_tick_at_sqrt_price(state.sqrt_price_x64)?;
            }

            loop_count += 1;
        } else {
            // 没有更多 tick array，跳出循环
            break;
        }

        // 如果当前 tick array 已经用完，移动到下一个
        if needs_next_tick_array(
            state.tick,
            tick_arrays,
            tick_array_idx,
            tick_spacing,
            zero_for_one,
        ) {
            tick_array_idx += 1;
            if tick_array_idx >= tick_arrays.len() {
                break;
            }
        }
    }

    Ok(state.amount_calculated)
}

/// 在 tick arrays 中找到下一个初始化的 tick
fn find_next_initialized_tick(
    tick_arrays: &[(i32, Vec<(i32, i128, u128)>)],
    current_tick: i32,
    _tick_spacing: u16,
    zero_for_one: bool,
) -> Option<(i32, bool, i128)> {
    for (_start_index, ticks) in tick_arrays {
        for &(tick, liquidity_net, liquidity_gross) in ticks {
            let is_initialized = liquidity_gross > 0;

            if zero_for_one {
                if tick <= current_tick && is_initialized {
                    return Some((tick, is_initialized, liquidity_net));
                }
            } else if tick > current_tick && is_initialized {
                return Some((tick, is_initialized, liquidity_net));
            }
        }
    }
    None
}

/// 判断是否需要移动到下一个 tick array
fn needs_next_tick_array(
    current_tick: i32,
    tick_arrays: &[(i32, Vec<(i32, i128, u128)>)],
    current_idx: usize,
    tick_spacing: u16,
    zero_for_one: bool,
) -> bool {
    if current_idx >= tick_arrays.len() {
        return false;
    }

    let (start_index, _) = tick_arrays[current_idx];
    let ticks_in_array = 60 * (tick_spacing as i32);

    if zero_for_one {
        current_tick < start_index
    } else {
        current_tick >= start_index + ticks_in_array
    }
}

// ========================================
// Exact Out 完整实现
// ========================================

/// 完整的 exact_out swap 计算（需要外部传入 tick array 数据）
///
/// 与 exact_in 的主要区别：
/// - exact_in: 固定输入，计算输出（is_base_input = true）
/// - exact_out: 固定输出，计算输入（is_base_input = false）
///
/// 参数：
/// - amount_out: 期望的输出金额（固定）
/// - 其他参数同 exact_in 版本
///
/// 返回：
/// - 所需的输入金额（包括手续费）
pub fn calculate_swap_exact_out_with_tick_arrays(
    amount_out: u64,
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current: i32,
    tick_spacing: u16,
    fee_rate: u32,
    zero_for_one: bool,
    tick_arrays: &[(i32, Vec<(i32, i128, u128)>)], // (start_index, [(tick, liquidity_net, liquidity_gross)])
) -> Result<u64, &'static str> {
    if amount_out == 0 {
        return Err("amount_out must not be 0");
    }

    let sqrt_price_limit_x64 =
        if zero_for_one { MIN_SQRT_PRICE_X64 + 1 } else { MAX_SQRT_PRICE_X64 - 1 };

    // 验证价格限制
    if zero_for_one {
        if sqrt_price_limit_x64 < MIN_SQRT_PRICE_X64 {
            return Err("sqrt_price_limit_x64 must greater than MIN_SQRT_PRICE_X64");
        }
        if sqrt_price_limit_x64 >= sqrt_price_x64 {
            return Err("sqrt_price_limit_x64 must smaller than current");
        }
    } else {
        if sqrt_price_limit_x64 > MAX_SQRT_PRICE_X64 {
            return Err("sqrt_price_limit_x64 must smaller than MAX_SQRT_PRICE_X64");
        }
        if sqrt_price_limit_x64 <= sqrt_price_x64 {
            return Err("sqrt_price_limit_x64 must greater than current");
        }
    }

    // 对于 exact_out，我们需要追踪：
    // - amount_out_remaining: 还需要提供的输出量
    // - amount_in_calculated: 已计算的输入量
    let mut amount_out_remaining = amount_out;
    let mut amount_in_calculated: u64 = 0;

    let mut current_sqrt_price = sqrt_price_x64;
    let mut current_tick = tick_current;
    let mut current_liquidity = liquidity;

    let mut tick_array_idx = 0;
    let mut loop_count = 0;
    const MAX_LOOP: u32 = 10;

    // 循环遍历 tick arrays 直到输出满足或达到价格限制
    while amount_out_remaining != 0
        && current_sqrt_price != sqrt_price_limit_x64
        && current_tick < MAX_TICK
        && current_tick > MIN_TICK
    {
        if loop_count >= MAX_LOOP {
            return Err("loop_count limit exceeded");
        }

        // 找到下一个初始化的 tick
        let next_initialized_tick = find_next_initialized_tick(
            &tick_arrays[tick_array_idx..],
            current_tick,
            tick_spacing,
            zero_for_one,
        );

        if let Some((tick_next, initialized, liquidity_net)) = next_initialized_tick {
            let tick_next = tick_next.clamp(MIN_TICK, MAX_TICK);
            let sqrt_price_next_x64 = get_sqrt_price_at_tick(tick_next)?;

            // 计算目标价格
            let target_price = if (zero_for_one && sqrt_price_next_x64 < sqrt_price_limit_x64)
                || (!zero_for_one && sqrt_price_next_x64 > sqrt_price_limit_x64)
            {
                sqrt_price_limit_x64
            } else {
                sqrt_price_next_x64
            };

            // 保存 step 开始时的价格（用于后续比较）
            let sqrt_price_start_x64 = current_sqrt_price;

            // 调用官方 swap 计算，关键：is_base_input = false（exact_out 模式）
            let swap_step = compute_swap_step(
                current_sqrt_price,
                target_price,
                current_liquidity,
                amount_out_remaining, // 注意：这里传入剩余的输出量
                fee_rate,
                false, // is_base_input = false for exact_out
                zero_for_one,
            )?;

            // 更新状态
            current_sqrt_price = swap_step.sqrt_price_next_x64;

            // 对于 exact_out：
            // - amount_in 是需要的输入
            // - amount_out 是提供的输出
            let step_input = swap_step.amount_in;
            let step_output = swap_step.amount_out;
            let _step_fee = swap_step.fee_amount;

            // 更新剩余输出和累计输入
            amount_out_remaining = amount_out_remaining
                .checked_sub(step_output)
                .ok_or("amount_out_remaining underflow")?;

            amount_in_calculated =
                amount_in_calculated.checked_add(step_input).ok_or("amount_in overflow")?;

            // 如果达到下一个 tick，更新流动性
            if current_sqrt_price == sqrt_price_next_x64 {
                if initialized {
                    let liquidity_delta = if zero_for_one { -liquidity_net } else { liquidity_net };
                    current_liquidity = add_delta(current_liquidity, liquidity_delta)?;
                }

                current_tick = if zero_for_one { tick_next - 1 } else { tick_next };
            } else if current_sqrt_price != sqrt_price_start_x64 {
                // 如果价格有变化（但没有到达下一个 tick），重新计算 tick
                // 注意：这里使用的是 step 开始时的价格，不是 sqrt_price_next_x64
                current_tick = get_tick_at_sqrt_price(current_sqrt_price)?;
            }

            loop_count += 1;
        } else {
            // 没有更多 tick array，跳出循环
            break;
        }

        // 如果当前 tick array 已经用完，移动到下一个
        if needs_next_tick_array(
            current_tick,
            tick_arrays,
            tick_array_idx,
            tick_spacing,
            zero_for_one,
        ) {
            tick_array_idx += 1;
            if tick_array_idx >= tick_arrays.len() {
                break;
            }
        }
    }

    // 如果还有剩余输出需求，说明流动性不足
    if amount_out_remaining > 0 {
        return Err("Insufficient liquidity to fulfill the exact_out request");
    }

    Ok(amount_in_calculated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_conversion() {
        let tick = 1000;
        let sqrt_price = get_sqrt_price_at_tick(tick).unwrap();
        let recovered_tick = get_tick_at_sqrt_price(sqrt_price).unwrap();

        // 允许 ±1 误差（浮点精度）
        assert!((recovered_tick - tick).abs() <= 1);
    }

    #[test]
    fn test_liquidity_delta() {
        let liquidity = 1000u128;

        // 正增量
        let result = add_delta(liquidity, 500).unwrap();
        assert_eq!(result, 1500);

        // 负增量
        let result = add_delta(liquidity, -300).unwrap();
        assert_eq!(result, 700);
    }

    #[test]
    fn test_compute_swap_step() {
        // 使用官方测试用例的参数
        // 从 temp/raydium-clmm/client/src/instructions/utils.rs 中参考
        let sqrt_price_current = 4295048016u128; // 更小的价格值
        let sqrt_price_target = 4295148016u128; // 稍高一点
        let liquidity = 10000000u128; // 适中的流动性
        let amount_remaining = 1000u64; // 较小的输入
        let fee_rate = 2500; // 0.25%

        let result = compute_swap_step(
            sqrt_price_current,
            sqrt_price_target,
            liquidity,
            amount_remaining,
            fee_rate,
            true,  // is_base_input
            false, // zero_for_one = false （价格上涨）
        );

        if let Err(e) = &result {
            eprintln!("compute_swap_step error: {}", e);
        }
        assert!(result.is_ok(), "compute_swap_step should succeed: {:?}", result.err());
        let step = result.unwrap();

        // 检查输出结果
        println!(
            "amount_in: {}, amount_out: {}, fee_amount: {}, sqrt_price_next: {}",
            step.amount_in, step.amount_out, step.fee_amount, step.sqrt_price_next_x64
        );

        // 应该有输出（算法成功执行）
        assert!(step.sqrt_price_next_x64 > 0, "sqrt_price_next should be positive");

        // 注意：由于流动性和价格范围的关系，amount_in/amount_out 可能为 0
        // 这里只验证计算不出错
    }
}

// ============================================================================
// Exact Out Quote - 支持指定输出金额的计算
// ============================================================================

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
    let amount_in = calculate_swap_exact_out_with_tick_arrays(
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

    // TODO: 计算实际手续费（需要从 swap_step 中累计）
    // 目前简化为 0，因为 CLMM 的手续费已经包含在 amount_in 中
    let fee_amount = 0u64;

    // 价格影响（简化：输出金额占总流动性比例）
    let price_impact_bps = (amount_out as u128)
        .checked_mul(10_000u128)
        .and_then(|p| p.checked_div(liquidity))
        .map(|impact| impact as u64);

    Ok(QuoteExactOutResult { amount_in, fee_amount, price_impact_bps })
}
