// Copyright (c) Raydium Foundation
// Licensed under Apache 2.0
// Source: https://github.com/raydium-io/raydium-clmm/programs/amm/src/libraries/
// Complete copy of official Raydium CLMM math libraries for client-side use
// All anchor dependencies removed, calculation logic preserved

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
pub use swap_math::{SwapStep, compute_swap_step, FEE_RATE_DENOMINATOR_VALUE};
pub use unsafe_math::UnsafeMathTrait;
