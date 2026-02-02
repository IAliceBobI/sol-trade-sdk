//! 交易结果类型
//!
//! 定义了本地计算和链上模拟的结果结构

use crate::trading::factory::DexType;

/// 本地计算结果
///
/// 由 `buy_quote` 返回，提供快速的本地估算
#[derive(Debug, Clone)]
pub struct QuoteResult {
    /// 预期输出金额（最小单位）
    pub amount_out: u64,

    /// 手续费金额（输入代币单位）
    pub fee_amount: u64,

    /// 价格影响（基点，可选）
    pub price_impact_bps: Option<u64>,

    /// 计算耗时（毫秒）
    pub calculation_time_ms: u64,

    /// 使用的 DEX 类型
    pub dex_type: DexType,
}

/// 链上模拟结果
///
/// 由 `buy_simulate` 和 `sell_simulate` 返回，提供准确的链上验证结果
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// 输出金额
    /// - exact_in 模式：计算得到的输出
    /// - exact_out 模式：用户请求的输出
    pub amount_out: u64,

    /// 新增：输入金额
    /// - exact_in 模式：用户输入
    /// - exact_out 模式：计算得到的输入
    pub amount_in: u64,

    /// 手续费金额
    pub fee_amount: u64,

    /// 计算单元消耗
    pub compute_units: u64,

    /// 交易费用
    pub transaction_fee: u64,

    /// 模拟是否成功
    pub success: bool,

    /// 错误信息（如果失败）
    pub error: Option<String>,

    /// 交易日志（用于调试）
    pub logs: Option<Vec<String>>,

    /// 使用的 DEX 类型
    pub dex_type: DexType,
}
