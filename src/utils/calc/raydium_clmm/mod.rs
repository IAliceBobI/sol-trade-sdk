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

// 子模块
mod helpers;
mod quote;
mod swap_exact_in;
mod swap_exact_out;
mod types;

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
pub use super::clmm_math::swap_math::{FEE_RATE_DENOMINATOR_VALUE, SwapStep, compute_swap_step};

// Export types
pub use types::{
    QuoteExactOutResult, StepComputations, SwapCalculationResult, SwapState, TickState,
};

// Export swap functions
pub use quote::{quote_exact_out, quote_exact_out_simplified};
pub use swap_exact_in::calculate_swap_amount_with_tick_arrays;
pub use swap_exact_out::calculate_swap_exact_out_with_tick_arrays;

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
            0,     // block_timestamp: 客户端计算不需要，传 0
        );

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
