use crate::trading::core::params::RaydiumClmmParams;

/// 滑点计算辅助函数
/// 根据官方 client 实现移植
///
/// # 参数
/// - `amount`: 基础金额
/// - `slippage_bps`: 滑点（单位：基点，10000 = 100%）
/// - `round_up`: 是否向上取整
///   - true: 用于计算最大输入 (max in: amount * (1 + slippage))
///   - false: 用于计算最小输出 (min out: amount * (1 - slippage))
///
/// # 返回
/// 应用滑点后的金额
pub fn amount_with_slippage(amount: u64, slippage_bps: u16, round_up: bool) -> u64 {
    let slippage_f64 = (slippage_bps as f64) / 10000.0; // 将 BP 转换为小数
    if round_up {
        // max in: amount * (1 + slippage), 向上取整
        ((amount as f64) * (1.0 + slippage_f64)).ceil() as u64
    } else {
        // min out: amount * (1 - slippage), 向下取整
        ((amount as f64) * (1.0 - slippage_f64)).floor() as u64
    }
}

/// 价格计算降级方案（当无法获取 tick arrays 时使用）
///
/// # 参数
/// - `amount_in`: 输入金额
/// - `sqrt_price_x64`: 当前平方根价格（x64 格式）
/// - `is_token0_in`: 是否为 token0 输入
/// - `input_decimals`: 输入代币的小数位数
/// - `output_decimals`: 输出代币的小数位数
/// - `protocol_params`: CLMM 协议参数
///
/// # 返回
/// 预期输出金额
pub fn fallback_price_calculation(
    amount_in: u64,
    sqrt_price_x64: u128,
    is_token0_in: bool,
    input_decimals: u8,
    output_decimals: u8,
    protocol_params: &RaydiumClmmParams,
) -> u64 {
    use crate::utils::price::raydium_clmm::{price_token0_in_token1, price_token1_in_token0};

    // 使用价格计算作为降级方案
    let price = if is_token0_in {
        price_token0_in_token1(
            sqrt_price_x64,
            protocol_params.token0_decimals,
            protocol_params.token1_decimals,
        )
    } else {
        price_token1_in_token0(
            sqrt_price_x64,
            protocol_params.token0_decimals,
            protocol_params.token1_decimals,
        )
    };

    let input_amount_f64 = amount_in as f64 / 10f64.powi(input_decimals as i32);
    let output_amount_f64 = input_amount_f64 * price;
    (output_amount_f64 * 10f64.powi(output_decimals as i32)) as u64
}
