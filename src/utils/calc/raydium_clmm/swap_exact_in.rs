// Copyright (c) Raydium Foundation
// Licensed under Apache 2.0

//! Exact In Swap 计算
//!
//! 固定输入金额，计算输出金额。

use super::helpers::{find_next_initialized_tick, needs_next_tick_array};
use super::types::{StepComputations, SwapCalculationResult, SwapState};
use crate::utils::calc::clmm_math::{
    liquidity_math::add_delta,
    swap_math::{SwapStep, compute_swap_step},
    tick_math::{
        MAX_SQRT_PRICE_X64, MAX_TICK, MIN_SQRT_PRICE_X64, MIN_TICK, get_sqrt_price_at_tick,
        get_tick_at_sqrt_price,
    },
};

/// 计算单步 swap 结果（使用官方实现）
///
/// 这是 CLMM 最核心的函数，直接调用官方 swap_math::compute_swap_step
/// 注意：block_timestamp 参数用于未来扩展，当前传入 0 即可
pub fn compute_swap_step_wrapper(
    sqrt_price_current_x64: u128,
    sqrt_price_target_x64: u128,
    liquidity: u128,
    amount_remaining: u64,
    fee_rate: u32,
    is_base_input: bool,
    zero_for_one: bool,
) -> Result<SwapStep, &'static str> {
    // 直接调用官方实现，block_timestamp 传 0
    compute_swap_step(
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
) -> Result<SwapCalculationResult, &'static str> {
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
        fee_amount: 0,
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
            let swap_step = compute_swap_step_wrapper(
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

            // 累计手续费
            state.fee_amount =
                state.fee_amount.checked_add(step.fee_amount).ok_or("fee amount overflow")?;

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
            // 没有找到下一个初始化的 tick
            // 在当前价格区间完成交易（使用当前价格作为目标，价格不变）
            let swap_step = compute_swap_step_wrapper(
                state.sqrt_price_x64,
                state.sqrt_price_x64,
                state.liquidity,
                state.amount_specified_remaining,
                fee_rate,
                true,
                zero_for_one,
            )?;

            // 更新剩余量和计算量
            state.amount_specified_remaining = state
                .amount_specified_remaining
                .checked_sub(swap_step.amount_in + swap_step.fee_amount)
                .ok_or("amount underflow")?;
            state.amount_calculated = state
                .amount_calculated
                .checked_add(swap_step.amount_out)
                .ok_or("amount overflow")?;
            state.fee_amount = state
                .fee_amount
                .checked_add(swap_step.fee_amount)
                .ok_or("fee amount overflow")?;

            // 跳出循环
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

    Ok(SwapCalculationResult {
        amount_out: state.amount_calculated,
        fee_amount: state.fee_amount,
    })
}

/// Exact In Buy 方向的内部计算（用 quote 买 base）
///
/// 已知输入的 quote 数量，计算能获得多少 base。
///
/// 对于 CLMM pool，假设：
/// - token1 是 quote token（WSOL/USDC）
/// - token0 是 base token（其他代币）
/// - Buy = 用 quote 买 base = token1 -> token0 = zero_for_one = false
///
/// # Arguments
///
/// * `amount_in` - 输入的 quote 数量（固定）
/// * `sqrt_price_x64` - 当前平方根价格
/// * `liquidity` - 当前流动性
/// * `tick_current` - 当前 tick
/// * `tick_spacing` - tick 间距
/// * `fee_rate` - 手续费率
/// * `tick_arrays` - tick array 数据 (从 RPC 获取)
///
/// # 返回
///
/// `SwapCalculationResult` 包含输出金额和费用
pub fn buy_exact_in_internal(
    amount_in: u64,
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current: i32,
    tick_spacing: u16,
    fee_rate: u32,
    tick_arrays: &[(i32, Vec<(i32, i128, u128)>)],
) -> Result<SwapCalculationResult, &'static str> {
    calculate_swap_amount_with_tick_arrays(
        amount_in,
        sqrt_price_x64,
        liquidity,
        tick_current,
        tick_spacing,
        fee_rate,
        false, // zero_for_one = false, token1 -> token0 (quote -> base)
        tick_arrays,
    )
}

/// Exact In Sell 方向的内部计算（用 base 卖成 quote）
///
/// 已知输入的 base 数量，计算能获得多少 quote。
///
/// 对于 CLMM pool，假设：
/// - token1 是 quote token（WSOL/USDC）
/// - token0 是 base token（其他代币）
/// - Sell = 用 base 卖成 quote = token0 -> token1 = zero_for_one = true
///
/// # Arguments
///
/// * `amount_in` - 输入的 base 数量（固定）
/// * `sqrt_price_x64` - 当前平方根价格
/// * `liquidity` - 当前流动性
/// * `tick_current` - 当前 tick
/// * `tick_spacing` - tick 间距
/// * `fee_rate` - 手续费率
/// * `tick_arrays` - tick array 数据 (从 RPC 获取)
///
/// # 返回
///
/// `SwapCalculationResult` 包含输出金额和费用
pub fn sell_exact_in_internal(
    amount_in: u64,
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current: i32,
    tick_spacing: u16,
    fee_rate: u32,
    tick_arrays: &[(i32, Vec<(i32, i128, u128)>)],
) -> Result<SwapCalculationResult, &'static str> {
    calculate_swap_amount_with_tick_arrays(
        amount_in,
        sqrt_price_x64,
        liquidity,
        tick_current,
        tick_spacing,
        fee_rate,
        true, // zero_for_one = true, token0 -> token1 (base -> quote)
        tick_arrays,
    )
}
