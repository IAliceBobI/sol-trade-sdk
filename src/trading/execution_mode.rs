//! 交易执行模式
//!
//! 提供灵活的交易执行策略：
//! - 本地计算（快速估算）
//! - 链上模拟（准确验证）
//! - 真实执行（实际交易）

use serde::{Deserialize, Serialize};

/// 交易执行模式
///
/// 定义交易的执行方式，支持从快速估算到真实执行的完整流程
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ExecutionMode {
    /// 本地计算模式
    ///
    /// **特点**：
    /// - 最快速，无网络延迟
    /// - 使用离线数学计算
    /// - 适合价格估算、UI 显示
    ///
    /// **适用场景**：
    /// - 价格查询
    /// - 数量估算
    /// - UI 预览
    ///
    /// **示例**：
    /// ```ignore
    /// let result = client.quote(QuoteParams {
    ///     dex_type: DexType::RaydiumClmm,
    ///     mint_in: wsol_mint,
    ///     mint_out: jup_mint,
    ///     amount_in: 100_000_000,
    ///     mode: ExecutionMode::LocalCalculation,
    /// }).await?;
    /// println!("预估获得: {} JUP", result.amount_out);
    /// ```
    #[default]
    LocalCalculation,

    /// 链上模拟模式
    ///
    /// **特点**：
    /// - 完全准确（使用链上逻辑）
    /// - 不消耗真实费用
    /// - 需要 RPC 调用（约 1-2 秒）
    ///
    /// **适用场景**：
    /// - 验证本地计算
    /// - 测试交易
    /// - 确认滑点设置
    ///
    /// **示例**：
    /// ```ignore
    /// let result = client.quote(QuoteParams {
    ///     dex_type: DexType::RaydiumClmm,
    ///     mint_in: wsol_mint,
    ///     mint_out: jup_mint,
    ///     amount_in: 100_000_000,
    ///     mode: ExecutionMode::Simulation,
    /// }).await?;
    /// println!("模拟输出: {} JUP", result.amount_out);
    /// println!("CU 消耗: {:?}", result.compute_units);
    /// ```
    Simulation,

    /// 真实执行模式
    ///
    /// **特点**：
    /// - 实际执行交易
    /// - 消耗真实费用
    /// - 改变链上状态
    ///
    /// **适用场景**：
    /// - 正式交易
    /// - 需要立即执行
    ///
    /// **示例**：
    /// ```ignore
    /// let result = client.quote(QuoteParams {
    ///     dex_type: DexType::RaydiumClmm,
    ///     mint_in: wsol_mint,
    ///     mint_out: jup_mint,
    ///     amount_in: 100_000_000,
    ///     mode: ExecutionMode::RealExecution,
    /// }).await?;
    /// println!("交易签名: {:?}", result.signature);
    /// println!("实际获得: {} JUP", result.amount_out);
    /// ```
    RealExecution,
}

impl ExecutionMode {
    /// 是否为本地计算模式
    pub fn is_local_calculation(&self) -> bool {
        matches!(self, ExecutionMode::LocalCalculation)
    }

    /// 是否为模拟模式
    pub fn is_simulation(&self) -> bool {
        matches!(self, ExecutionMode::Simulation)
    }

    /// 是否为真实执行模式
    pub fn is_real_execution(&self) -> bool {
        matches!(self, ExecutionMode::RealExecution)
    }

    /// 获取模式的描述
    pub fn description(&self) -> &'static str {
        match self {
            ExecutionMode::LocalCalculation => "本地计算（快速估算）",
            ExecutionMode::Simulation => "链上模拟（准确验证）",
            ExecutionMode::RealExecution => "真实执行（实际交易）",
        }
    }

    /// 获取模式的速度等级（1=最快，3=最慢）
    pub fn speed_level(&self) -> u8 {
        match self {
            ExecutionMode::LocalCalculation => 1,
            ExecutionMode::Simulation => 2,
            ExecutionMode::RealExecution => 3,
        }
    }

    /// 获取模式的准确性等级（1=估算，3=实际）
    pub fn accuracy_level(&self) -> u8 {
        match self {
            ExecutionMode::LocalCalculation => 1,
            ExecutionMode::Simulation => 2,
            ExecutionMode::RealExecution => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_properties() {
        // 本地计算
        assert!(ExecutionMode::LocalCalculation.is_local_calculation());
        assert!(!ExecutionMode::LocalCalculation.is_simulation());
        assert!(!ExecutionMode::LocalCalculation.is_real_execution());
        assert_eq!(ExecutionMode::LocalCalculation.speed_level(), 1);
        assert_eq!(ExecutionMode::LocalCalculation.accuracy_level(), 1);

        // 模拟
        assert!(!ExecutionMode::Simulation.is_local_calculation());
        assert!(ExecutionMode::Simulation.is_simulation());
        assert!(!ExecutionMode::Simulation.is_real_execution());
        assert_eq!(ExecutionMode::Simulation.speed_level(), 2);
        assert_eq!(ExecutionMode::Simulation.accuracy_level(), 2);

        // 真实执行
        assert!(!ExecutionMode::RealExecution.is_local_calculation());
        assert!(!ExecutionMode::RealExecution.is_simulation());
        assert!(ExecutionMode::RealExecution.is_real_execution());
        assert_eq!(ExecutionMode::RealExecution.speed_level(), 3);
        assert_eq!(ExecutionMode::RealExecution.accuracy_level(), 3);
    }

    #[test]
    fn test_execution_mode_description() {
        assert_eq!(ExecutionMode::LocalCalculation.description(), "本地计算（快速估算）");
        assert_eq!(ExecutionMode::Simulation.description(), "链上模拟（准确验证）");
        assert_eq!(ExecutionMode::RealExecution.description(), "真实执行（实际交易）");
    }

    #[test]
    fn test_default() {
        assert_eq!(ExecutionMode::default(), ExecutionMode::LocalCalculation);
    }
}
