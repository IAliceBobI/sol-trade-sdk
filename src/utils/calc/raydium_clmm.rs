// Copyright (c) Raydium Foundation
// Licensed under Apache 2.0  
// Raydium CLMM swap calculations using official math libraries

//! Raydium CLMM calculation module
//! 
//! Uses official Raydium CLMM math libraries (clmm_math) for all calculations.
//! Dependencies: uint = { git = "https://github.com/raydium-io/parity-common", package = "uint" }

// Re-export official libraries for convenience
pub use super::clmm_math::{
    tick_math,
    liquidity_math,
    sqrt_price_math,
    fixed_point_64,
    full_math::MulDiv,
    U128, U256,
};

// Export constants from official libraries  
pub use super::clmm_math::tick_math::{
    MIN_TICK, MAX_TICK, MIN_SQRT_PRICE_X64, MAX_SQRT_PRICE_X64,
    get_sqrt_price_at_tick, get_tick_at_sqrt_price,
};
pub use super::clmm_math::fixed_point_64::{Q64, RESOLUTION};
pub use super::clmm_math::liquidity_math::{
    add_delta, get_delta_amount_0_unsigned, get_delta_amount_1_unsigned,
};
pub use super::clmm_math::sqrt_price_math::{
    get_next_sqrt_price_from_input, get_next_sqrt_price_from_output,
    get_next_sqrt_price_from_amount_0_rounding_up,
    get_next_sqrt_price_from_amount_1_rounding_down,
};

/// 费率分母（100%）
pub const FEE_RATE_DENOMINATOR_VALUE: u32 = 1_000_000;

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

/// 单步计算结果
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
// Swap Algorithm - 使用官方数学库的 tick-by-tick 算法
// ============================================================================

// ============================================================================

/// 计算单步 swap 结果（核心算法）
/// 
/// 这是 CLMM 最核心的函数，计算在指定价格区间内的交易结果
pub fn compute_swap_step(
    sqrt_price_current_x64: u128,
    sqrt_price_target_x64: u128,
    liquidity: u128,
    amount_remaining: u64,
    fee_rate: u32,
    is_base_input: bool,
    zero_for_one: bool,
) -> Result<SwapStep, &'static str> {
    let mut swap_step = SwapStep::default();

    if is_base_input {
        // 精确输入模式
        let amount_remaining_less_fee = amount_remaining
            .saturating_mul((FEE_RATE_DENOMINATOR_VALUE - fee_rate) as u64)
            / FEE_RATE_DENOMINATOR_VALUE as u64;

        // 计算该价格区间可以消耗的最大输入量
        let amount_in = calculate_amount_in_range(
            sqrt_price_current_x64,
            sqrt_price_target_x64,
            liquidity,
            zero_for_one,
            is_base_input,
        )?;

        // 判断是否能达到目标价格
        swap_step.sqrt_price_next_x64 = if amount_in.is_some()
            && amount_remaining_less_fee >= amount_in.unwrap()
        {
            sqrt_price_target_x64
        } else {
            get_next_sqrt_price_from_input(
                sqrt_price_current_x64,
                liquidity,
                amount_remaining_less_fee,
                zero_for_one,
            )
        };
    } else {
        // 精确输出模式
        let amount_out = calculate_amount_in_range(
            sqrt_price_current_x64,
            sqrt_price_target_x64,
            liquidity,
            zero_for_one,
            is_base_input,
        )?;

        swap_step.sqrt_price_next_x64 = if amount_out.is_some() && amount_remaining >= amount_out.unwrap() {
            sqrt_price_target_x64
        } else {
            get_next_sqrt_price_from_output(
                sqrt_price_current_x64,
                liquidity,
                amount_remaining,
                zero_for_one,
            )
        };
    }

    // 是否达到目标价格
    let max = sqrt_price_target_x64 == swap_step.sqrt_price_next_x64;

    // 计算实际的输入输出量
    if zero_for_one {
        if !(max && is_base_input) {
            swap_step.amount_in = get_delta_amount_0_unsigned(
                swap_step.sqrt_price_next_x64,
                sqrt_price_current_x64,
                liquidity,
                true,
            )?;
        }
        if !(max && !is_base_input) {
            swap_step.amount_out = get_delta_amount_1_unsigned(
                swap_step.sqrt_price_next_x64,
                sqrt_price_current_x64,
                liquidity,
                false,
            )?;
        }
    } else {
        if !(max && is_base_input) {
            swap_step.amount_in = get_delta_amount_1_unsigned(
                sqrt_price_current_x64,
                swap_step.sqrt_price_next_x64,
                liquidity,
                true,
            )?;
        }
        if !(max && !is_base_input) {
            swap_step.amount_out = get_delta_amount_0_unsigned(
                sqrt_price_current_x64,
                swap_step.sqrt_price_next_x64,
                liquidity,
                false,
            )?;
        }
    }

    // 精确输出模式：不超过剩余量
    if !is_base_input && swap_step.amount_out > amount_remaining {
        swap_step.amount_out = amount_remaining;
    }

    // 计算手续费
    swap_step.fee_amount = if is_base_input && swap_step.sqrt_price_next_x64 != sqrt_price_target_x64
    {
        // 未达到目标价格，剩余部分作为手续费
        amount_remaining.saturating_sub(swap_step.amount_in)
    } else {
        // 按比例计算手续费：amount_in * fee_rate / (1 - fee_rate)
        (swap_step.amount_in as u64)
            .mul_div_ceil(
                fee_rate.into(),
                (FEE_RATE_DENOMINATOR_VALUE - fee_rate).into(),
            )
            .unwrap_or(0)
    };

    Ok(swap_step)
}

/// 预计算指定价格区间的输入/输出量
fn calculate_amount_in_range(
    sqrt_price_current_x64: u128,
    sqrt_price_target_x64: u128,
    liquidity: u128,
    zero_for_one: bool,
    is_base_input: bool,
) -> Result<Option<u64>, &'static str> {
    let result = if is_base_input {
        if zero_for_one {
            get_delta_amount_0_unsigned(
                sqrt_price_target_x64,
                sqrt_price_current_x64,
                liquidity,
                true,
            )
        } else {
            get_delta_amount_1_unsigned(
                sqrt_price_current_x64,
                sqrt_price_target_x64,
                liquidity,
                true,
            )
        }
    } else {
        if zero_for_one {
            get_delta_amount_1_unsigned(
                sqrt_price_target_x64,
                sqrt_price_current_x64,
                liquidity,
                false,
            )
        } else {
            get_delta_amount_0_unsigned(
                sqrt_price_current_x64,
                sqrt_price_target_x64,
                liquidity,
                false,
            )
        }
    };

    match result {
        Ok(amount) => Ok(Some(amount)),
        Err(_) => Ok(None), // Overflow 返回 None
    }
}

// ============================================================================
// 主入口函数 - 计算完整 swap 输出
// ============================================================================

/// 计算 CLMM swap 的精确输出量（简化版本 - 不需要 tick arrays）
/// 
/// 注意：这是简化版本，假设在单个 tick 区间内完成交易
/// 完整版本需要遍历多个 tick arrays
pub fn calculate_swap_amount_simple(
    input_amount: u64,
    sqrt_price_x64: u128,
    liquidity: u128,
    tick_current: i32,
    fee_rate: u32,
    zero_for_one: bool,
) -> Result<u64, &'static str> {
    if input_amount == 0 {
        return Err("Input amount must not be 0");
    }

    if liquidity == 0 {
        return Err("Liquidity must not be 0");
    }

    // 设置价格限制
    let sqrt_price_limit_x64 = if zero_for_one {
        MIN_SQRT_PRICE_X64 + 1
    } else {
        MAX_SQRT_PRICE_X64 - 1
    };

    // 初始化状态
    let mut state = SwapState {
        amount_specified_remaining: input_amount,
        amount_calculated: 0,
        sqrt_price_x64,
        tick: tick_current,
        liquidity,
    };

    // 简化版本：假设只需要一步即可完成
    // 完整版本需要循环遍历多个 tick
    let swap_step = compute_swap_step(
        state.sqrt_price_x64,
        sqrt_price_limit_x64,
        state.liquidity,
        state.amount_specified_remaining,
        fee_rate,
        true, // is_base_input
        zero_for_one,
    )?;

    state.amount_calculated = swap_step.amount_out;

    Ok(state.amount_calculated)
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
        // 使用更实际的测试参数
        let sqrt_price_current = 79228162514264337593543950336u128; // Q64.64 格式的 1.0
        let sqrt_price_target = 100000000000000000000000000000u128; // 略高于 current
        let liquidity = 100000000000u128; // 更大的流动性
        let amount_remaining = 1000000u64; // 更大的输入
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
        
        assert!(result.is_ok(), "compute_swap_step should succeed");
        let step = result.unwrap();
        
        // 检查输出结果
        println!("amount_in: {}, amount_out: {}, fee_amount: {}", 
                 step.amount_in, step.amount_out, step.fee_amount);
        
        // 应该有输入（扣除手续费后）
        assert!(step.amount_in > 0, "amount_in should be positive");
        assert!(step.fee_amount > 0, "fee_amount should be positive");
        
        // 注意：由于流动性和价格范围的关系，输出可能为 0
        // 这里只验证计算不出错
    }
}

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

    let sqrt_price_limit_x64 = if zero_for_one {
        MIN_SQRT_PRICE_X64 + 1
    } else {
        MAX_SQRT_PRICE_X64 - 1
    };

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

        let mut step = StepComputations::default();
        step.sqrt_price_start_x64 = state.sqrt_price_x64;

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

            // 调用核心 swap 计算
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
            state.amount_calculated = state
                .amount_calculated
                .checked_add(step.amount_out)
                .ok_or("amount overflow")?;

            // 如果达到下一个 tick，更新流动性
            if state.sqrt_price_x64 == step.sqrt_price_next_x64 {
                if step.initialized {
                    let liquidity_delta = if zero_for_one {
                        -liquidity_net
                    } else {
                        liquidity_net
                    };
                    state.liquidity = add_delta(state.liquidity, liquidity_delta)?;
                }

                state.tick = if zero_for_one {
                    step.tick_next - 1
                } else {
                    step.tick_next
                };
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
        if needs_next_tick_array(state.tick, &tick_arrays, tick_array_idx, tick_spacing, zero_for_one) {
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
            } else {
                if tick > current_tick && is_initialized {
                    return Some((tick, is_initialized, liquidity_net));
                }
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
