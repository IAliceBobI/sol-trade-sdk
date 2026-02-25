// Copyright (c) Raydium Foundation
// Licensed under Apache 2.0

//! Exact Out Swap 计算
//!
//! 固定输出金额，计算输入金额。

use super::helpers::{find_next_initialized_tick, needs_next_tick_array};
use super::swap_exact_in::compute_swap_step;
use super::types::QuoteExactOutResult;
use crate::utils::calc::clmm_math::{
    liquidity_math::add_delta,
    tick_math::{
        MAX_SQRT_PRICE_X64, MAX_TICK, MIN_SQRT_PRICE_X64, MIN_TICK, get_sqrt_price_at_tick,
        get_tick_at_sqrt_price,
    },
};

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
) -> Result<QuoteExactOutResult, &'static str> {
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
    // - fee_amount_calculated: 累计的手续费
    let mut amount_out_remaining = amount_out;
    let mut amount_in_calculated: u64 = 0;
    let mut fee_amount_calculated: u64 = 0;

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
            let step_fee = swap_step.fee_amount;

            // 累计手续费
            fee_amount_calculated =
                fee_amount_calculated.checked_add(step_fee).ok_or("fee amount overflow")?;

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

    Ok(QuoteExactOutResult {
        amount_in: amount_in_calculated,
        fee_amount: fee_amount_calculated,
        price_impact_bps: None,
    })
}
