// Copyright (c) Raydium Foundation
// Licensed under Apache 2.0
// Source: https://github.com/raydium-io/raydium-clmm/programs/amm/src/libraries/
// Complete copy of official Raydium CLMM math libraries for client-side use
// All anchor dependencies removed, calculation logic preserved

// 允许在整个 CLMM 数学库中使用 unwrap，因为：
// 1. 这些是来自官方 Raydium CLMM 的数学库
// 2. 数学计算经过 checked_* 操作后是安全的
// 3. 如果计算失败表示数据不一致，应该 panic
#![allow(clippy::unwrap_used)]

pub mod big_num;
pub mod fixed_point_64;
pub mod full_math;
pub mod liquidity_math;
pub mod sqrt_price_math;
pub mod swap_math;
pub mod tick_math;
pub mod unsafe_math;

// Re-exports for convenience
pub use big_num::{U128, U256, U512};
pub use full_math::MulDiv;
pub use swap_math::{FEE_RATE_DENOMINATOR_VALUE, SwapStep, compute_swap_step};
pub use unsafe_math::UnsafeMathTrait;
