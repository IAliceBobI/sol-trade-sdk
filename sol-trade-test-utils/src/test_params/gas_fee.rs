//! Gas 费策略
//!
//! 提供测试用的 Gas 费策略配置

use sol_trade_sdk::common::GasFeeStrategy;

/// 创建测试用的 Gas 费策略
///
/// # 返回
/// 返回一个配置了测试参数的 `GasFeeStrategy`：
/// - buy/sell priority fee: 150,000 lamports
/// - buy/sell compute unit limit: 500,000
/// - buy/sell compute unit price: 0.001 micro-lamports per CU
pub fn create_test_gas_fee_strategy() -> GasFeeStrategy {
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(
        150_000, // buy_priority_fee
        150_000, // sell_priority_fee
        500_000, // buy_compute_unit_limit
        500_000, // sell_compute_unit_limit
        0.001,   // buy_compute_unit_price
        0.001,   // sell_compute_unit_price
    );
    gas_fee_strategy
}
