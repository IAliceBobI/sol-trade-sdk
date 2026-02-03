//! Raydium AMM V4 Swap 计算工具
//!
//! 完全复制自 Raydium 官方实现
//! 源码: https://github.com/raydium-io/raydium-amm/blob/master/program/src/math.rs

use crate::instruction::utils::raydium_amm_v4::accounts::{
    SWAP_FEE_DENOMINATOR, SWAP_FEE_NUMERATOR,
};

/// Swap 方向枚举
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwapDirection {
    /// 输入 PC，输出 Coin
    PC2Coin = 1,
    /// 输入 Coin，输出 PC
    Coin2PC = 2,
}

/// Ceiling division trait
trait CheckedCeilDiv: Sized {
    fn checked_ceil_div(&self, rhs: Self) -> Option<Self>;
}

impl CheckedCeilDiv for u128 {
    fn checked_ceil_div(&self, rhs: Self) -> Option<Self> {
        let mut quotient = self.checked_div(rhs)?;
        let remainder = self.checked_rem(rhs)?;
        if remainder != 0 {
            quotient = quotient.checked_add(1)?;
        }
        Some(quotient)
    }
}

/// 完全复制 Raydium 官方的 swap_token_amount_base_in 函数
///
/// 源码位置: math.rs:373-409
///
/// # 参数说明
/// * `amount_in`: 输入金额（已扣除 swap_fee）
/// * `total_pc_without_take_pnl`: PC 储备金
/// * `total_coin_without_take_pnl`: Coin 储备金
/// * `swap_direction`: Swap 方向
///
/// # 返回
/// 输出金额
pub fn swap_token_amount_base_in(
    amount_in: u128,
    total_pc_without_take_pnl: u128,
    total_coin_without_take_pnl: u128,
    swap_direction: SwapDirection,
) -> u128 {
    match swap_direction {
        SwapDirection::Coin2PC => {
            // (x + delta_x) * (y + delta_y) = x * y
            // (coin + amount_in) * (pc - amount_out) = coin * pc
            // => amount_out = pc - coin * pc / (coin + amount_in)
            // => amount_out = ((pc * coin + pc * amount_in) - coin * pc) / (coin + amount_in)
            // => amount_out =  pc * amount_in / (coin + amount_in)
            let denominator = total_coin_without_take_pnl
                .checked_add(amount_in)
                .unwrap();
            total_pc_without_take_pnl
                .checked_mul(amount_in)
                .unwrap()
                .checked_div(denominator)
                .unwrap()
        }
        SwapDirection::PC2Coin => {
            // (x + delta_x) * (y + delta_y) = x * y
            // (pc + amount_in) * (coin - amount_out) = coin * pc
            // => amount_out = coin - coin * pc / (pc + amount_in)
            // => amount_out = (coin * pc + coin * amount_in - coin * pc) / (pc + amount_in)
            // => amount_out = coin * amount_in / (pc + amount_in)
            let denominator = total_pc_without_take_pnl
                .checked_add(amount_in)
                .unwrap();
            total_coin_without_take_pnl
                .checked_mul(amount_in)
                .unwrap()
                .checked_div(denominator)
                .unwrap()
        }
    }
}

/// 完全复制 Raydium 官方的 swap fee 计算和 swap 逻辑
///
/// 源码位置: processor.rs:2393-2405
///
/// # 参数
/// * `amount_in`: 原始输入金额
/// * `swap_fee_numerator`: Swap fee 分子（默认 25）
/// * `swap_fee_denominator`: swap fee 分母（默认 10000）
/// * `total_pc_without_take_pnl`: PC 储备金
/// * `total_coin_without_take_pnl`: Coin 储备金
/// * `swap_direction`: Swap 方向
///
/// # 返回
/// (swap_fee, output_amount)
pub fn calculate_swap_with_fee(
    amount_in: u64,
    swap_fee_numerator: u64,
    swap_fee_denominator: u64,
    total_pc_without_take_pnl: u64,
    total_coin_without_take_pnl: u64,
    swap_direction: SwapDirection,
) -> (u64, u64) {
    // 1. 计算 swap_fee（使用向上取整）
    // 源码: processor.rs:2393-2397
    let swap_fee = (amount_in as u128)
        .checked_mul(swap_fee_numerator as u128)
        .unwrap()
        .checked_ceil_div(swap_fee_denominator as u128)
        .unwrap() as u64;

    // 2. 从输入扣除费用
    // 源码: processor.rs:2398
    let swap_in_after_deduct_fee = (amount_in as u128).checked_sub(swap_fee as u128).unwrap() as u64;

    // 3. 计算输出金额
    // 源码: processor.rs:2399-2405
    let swap_amount_out = swap_token_amount_base_in(
        swap_in_after_deduct_fee as u128,
        total_pc_without_take_pnl as u128,
        total_coin_without_take_pnl as u128,
        swap_direction,
    ) as u64;

    (swap_fee, swap_amount_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checked_ceil_div() {
        assert_eq!(128u128.checked_ceil_div(100).unwrap(), 2); // 1.28 -> 2
        assert_eq!(100u128.checked_ceil_div(100).unwrap(), 1); // 1.0 -> 1
        assert_eq!(101u128.checked_ceil_div(100).unwrap(), 2); // 1.01 -> 2
    }

    #[test]
    fn test_swap_fee_calculation() {
        // 测试标准费用计算
        let amount_in = 1_000_000u64;
        let (swap_fee, _) = calculate_swap_with_fee(
            amount_in,
            25,    // 0.25%
            10000,
            5_531_095_839_846,
            53_692_923_475_369,
            SwapDirection::Coin2PC,
        );

        // swap_fee = ceil(1_000_000 * 25 / 10_000) = ceil(2500) = 2500
        assert_eq!(swap_fee, 2500);
    }
}
