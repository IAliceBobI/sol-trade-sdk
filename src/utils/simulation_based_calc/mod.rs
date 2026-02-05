//! # 基于链上模拟的 Swap 计算验证工具
//!
//! 这个模块提供了一种可靠的方法来验证各种 DEX 的 swap 计算准确性：
//! 1. 构造实际的 swap 交易指令
//! 2. 通过 RPC 模拟交易执行（不消耗真实费用）
//! 3. 从模拟结果中解析实际的 token 余额变化
//! 4. 与离线计算结果对比验证
//!
//! ## 优势
//! - **100% 准确**: 使用链上实际逻辑，无需维护复杂的数学公式
//! - **自动更新**: DEX 升级时自动适配，无需修改计算代码
//! - **费用准确**: 包含所有实际费用（协议费、LP 费等）
//!
//! ## 使用场景
//! - 验证离线计算的准确性
//! - 获取精确的输出数量（用于滑点设置）
//! - 测试新 DEX 协议的 swap 逻辑

mod helpers;
mod simulator;
mod types;

pub use types::SimulatedSwapResult;
pub use simulator::simulate_swap_transaction;

use anyhow::{Result, anyhow};

/// 验证离线计算的准确性
///
/// # 使用示例
/// ```ignore
/// use sol_trade_sdk::utils::simulation_based_calc::*;
///
/// // 1. 离线计算
/// let calculated_output = raydium_cpmm::compute_swap_amount(
///     base_reserve, quote_reserve, true, amount_in, 100
/// );
///
/// // 2. 链上模拟
/// let simulated = simulate_swap_transaction(
///     &rpc, &payer, instructions,
///     input_ata, output_ata,
///     input_mint, output_mint
/// ).await?;
///
/// // 3. 对比验证
/// let diff = if simulated.actual_output_amount > calculated_output.amount_out {
///     simulated.actual_output_amount - calculated_output.amount_out
/// } else {
///     calculated_output.amount_out - simulated.actual_output_amount
/// };
///
/// let error_rate = (diff as f64 / simulated.actual_output_amount as f64) * 100.0;
/// println!("离线计算: {}", calculated_output.amount_out);
/// println!("链上模拟: {}", simulated.actual_output_amount);
/// println!("误差率: {}%", error_rate);
///
/// assert!(error_rate < 0.1, "误差超过 0.1%");
/// ```
pub fn verify_calculation_accuracy(
    calculated_output: u64,
    simulated_output: u64,
    max_error_percentage: f64,
) -> Result<bool> {
    if simulated_output == 0 {
        return Err(anyhow!("模拟输出为 0，无法验证"));
    }

    let diff = calculated_output.abs_diff(simulated_output);

    let error_rate = (diff as f64 / simulated_output as f64) * 100.0;

    if error_rate > max_error_percentage {
        Err(anyhow!(
            "计算误差过大: {}% (最大允许: {}%) | 计算: {}, 实际: {}",
            error_rate,
            max_error_percentage,
            calculated_output,
            simulated_output
        ))
    } else {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_calculation_accuracy() {
        // 完全匹配
        assert!(verify_calculation_accuracy(1000, 1000, 0.1).is_ok());

        // 0.01% 误差
        assert!(verify_calculation_accuracy(1000, 1001, 0.1).is_ok());

        // 0.1% 误差
        assert!(verify_calculation_accuracy(1000, 1001, 0.1).is_ok());

        // 超过 0.1% 误差
        assert!(verify_calculation_accuracy(1000, 1002, 0.1).is_err());
    }
}
