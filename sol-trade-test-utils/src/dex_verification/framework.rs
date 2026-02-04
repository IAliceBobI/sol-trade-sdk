//! DEX 三阶段验证框架核心逻辑
//!
//! 实现统一的三阶段验证流程：
//! 1. 本地计算（Quote）
//! 2. 链上模拟（Simulation）
//! 3. 实际执行（Execution）

use super::types::DexVerifyConfig;
use sol_trade_sdk::{
    instruction::utils::raydium_cpmm::clear_pool_cache,
    parser::DexParser,
    QuoteResult, SimulationResult,
    TradingClient,
};

/// 三阶段验证结果
#[derive(Debug)]
pub struct ThreeStageResult {
    /// 阶段 1: 本地计算结果
    pub quote_result: QuoteResult,
    /// 阶段 2: 链上模拟结果
    pub simulation_result: SimulationResult,
    /// 阶段 3: 实际执行结果
    pub execution_result: ExecutionResult,
}

/// 实际执行结果
#[derive(Debug)]
pub struct ExecutionResult {
    /// 交易签名
    pub signature: String,
    /// 输出金额（原始单位）
    pub amount_out: u64,
    /// 输出金额（UI 格式）
    pub amount_out_ui: f64,
    /// 是否解析成功
    pub parse_success: bool,
}

/// 参数构建器 Trait（Buy 版本）
///
/// 用于构建买入交易参数的异步函数
#[allow(clippy::manual_async_fn)]
pub trait BuyParamsBuilder: Send + Sync {
    /// 构建买入交易参数
    fn build(&self, client: &TradingClient, amount: u64) -> impl std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send;
}

/// 函数指针类型的买入参数构建器实现
impl<F, Fut> BuyParamsBuilder for F
where
    F: Fn(&TradingClient, u64) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send,
{
    #[allow(clippy::manual_async_fn)]
    fn build(&self, client: &TradingClient, amount: u64) -> impl std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send {
        async move {
            self(client, amount).await
        }
    }
}

/// 参数构建器 Trait（Sell 版本）
///
/// 用于构建卖出交易参数的异步函数
#[allow(clippy::manual_async_fn)]
pub trait SellParamsBuilder: Send + Sync {
    /// 构建卖出交易参数
    fn build(&self, client: &TradingClient, amount: u64) -> impl std::future::Future<Output = sol_trade_sdk::TradeSellParams> + Send;
}

/// 函数指针类型的卖出参数构建器实现
impl<F, Fut> SellParamsBuilder for F
where
    F: Fn(&TradingClient, u64) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = sol_trade_sdk::TradeSellParams> + Send,
{
    #[allow(clippy::manual_async_fn)]
    fn build(&self, client: &TradingClient, amount: u64) -> impl std::future::Future<Output = sol_trade_sdk::TradeSellParams> + Send {
        async move {
            self(client, amount).await
        }
    }
}

/// 运行 DEX 三阶段验证测试
///
/// # 参数
///
/// * `client` - TradingClient 实例
/// * `config` - 验证配置
/// * `params_builder` - 参数构建器
///
/// # 返回
///
/// 返回三阶段验证结果
///
/// # 示例
///
/// ```ignore
/// let config = DexVerifyConfig {
///     dex_type: DexType::RaydiumCpmm,
///     pool: PoolConfig::new(...),
///     operation: OperationType::BuyExactIn,
///     direction: TradeDirection::Token1ToToken0,
///     input_amount: 1_000,
/// };
///
/// let params_builder = |client: &TradingClient, amount: u64| async move {
///     PipeWsolBuyParamsBuilder::new(Some(amount))
///         .slippage(1000)
///         .build(client)
///         .await
/// };
///
/// let result = run_dex_three_stage_verification(&client, config, params_builder).await?;
/// verify_three_stage_accuracy(&result, 1.0)?;
/// ```
pub async fn run_dex_three_stage_verification<P>(
    client: &TradingClient,
    config: DexVerifyConfig,
    params_builder: P,
) -> Result<ThreeStageResult, Box<dyn std::error::Error>>
where
    P: BuyParamsBuilder,
{
    println!("==============================================");
    println!("DEX 三阶段验证测试");
    println!("==============================================\n");

    println!("📊 测试配置:");
    println!("  DEX: {:?}", config.dex_type);
    println!("  Pool: {}", config.pool);
    println!("  操作: {}", config.operation);
    println!("  方向: {}", config.direction);
    println!("  输入金额: {} (最小单位)\n", config.input_amount);

    // 检查 Pool 类型
    if config.pool.is_mixed_pool() {
        println!(
            "⚠️  混合 Pool 检测: {} + {}\n",
            config.pool.token0_program, config.pool.token1_program
        );
    }

    // ===== 阶段 1: 本地计算（Quote）=====
    println!("========================================");
    println!("阶段 1: 本地计算（client.buy_quote）");
    println!("========================================\n");

    let buy_params = params_builder.build(client, config.input_amount).await;

    let quote_result = match client.buy_quote(buy_params.clone()).await {
        Ok(quote) => quote,
        Err(e) => {
            return Err(format!("❌ 本地计算失败: {}", e).into());
        },
    };

    println!("✅ 本地计算结果:");
    println!("  输出金额: {}", quote_result.amount_out);
    println!("  手续费: {}", quote_result.fee_amount);
    println!("  计算时间: {} ms\n", quote_result.calculation_time_ms);

    // ===== 阶段 2: 链上模拟（Simulation）=====
    println!("========================================");
    println!("阶段 2: 链上模拟（client.buy_simulate）");
    println!("========================================\n");

    let simulation_result = match client.buy_simulate(buy_params.clone()).await {
        Ok(result) => result,
        Err(e) => {
            return Err(format!("❌ 链上模拟失败: {}", e).into());
        },
    };

    println!("✅ 链上模拟结果:");
    println!("  输出金额: {}", simulation_result.amount_out);
    println!("  手续费: {}", simulation_result.fee_amount);
    println!("  计算单元: {} CU", simulation_result.compute_units);
    println!(
        "  交易费用: {} lamports",
        simulation_result.transaction_fee
    );
    println!(
        "  状态: {}\n",
        if simulation_result.success {
            "成功"
        } else {
            "失败"
        }
    );

    // ===== 阶段 3: 实际执行 =====
    println!("========================================");
    println!("阶段 3: 实际执行（client.buy）");
    println!("========================================\n");

    println!("🚀 执行买入交易...");
    let (success, sigs, error) = client
        .buy(buy_params)
        .await
        .expect("买入交易执行失败");

    if !success {
        return Err(format!(
            "❌ 买入交易失败: {:?}\n  Pool: {}\n  输入金额: {}",
            error, config.pool.pool_address, config.input_amount
        )
        .into());
    }

    let signature = sigs.first().expect("交易成功但无签名");
    println!("✅ 买入成功，签名: {}\n", signature);

    // 解析买入交易
    println!("📋 解析买入交易...");
    let parser = DexParser::default();
    let sig_str = signature.to_string();
    let parse_result = parser.parse_transaction(&sig_str).await;

    let execution_result = if parse_result.success && !parse_result.trades.is_empty() {
        let trade = &parse_result.trades[0];
        println!("✅ 买入交易解析成功:");
        println!("  DEX: {}", trade.dex);
        println!("  用户: {}", trade.user);
        println!("  Pool: {}", trade.pool);
        println!("  交易类型: {:?}", trade.trade_type);
        println!(
            "  输入: {} {} ({} decimals)",
            trade.input_token.amount, trade.input_token.mint, trade.input_token.decimals
        );
        println!(
            "  输出: {} {} ({} decimals)",
            trade.output_token.amount,
            trade.output_token.mint,
            trade.output_token.decimals
        );
        if let Some(ref fee) = trade.fee {
            println!("  费用: {} {}", fee.amount, fee.mint);
        }
        println!();

        ExecutionResult {
            signature: sig_str,
            amount_out: trade.output_token.amount_raw.parse::<u64>().unwrap_or(0),
            amount_out_ui: trade.output_token.amount,
            parse_success: true,
        }
    } else {
        println!("⚠️  买入交易解析失败: {:?}\n", parse_result.error);
        ExecutionResult {
            signature: sig_str,
            amount_out: 0,
            amount_out_ui: 0.0,
            parse_success: false,
        }
    };

    // 等待链上状态更新
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    Ok(ThreeStageResult {
        quote_result,
        simulation_result,
        execution_result,
    })
}

/// 验证三阶段结果的准确性
///
/// # 参数
///
/// * `result` - 三阶段验证结果
/// * `max_error_percent` - 最大允许误差百分比（例如 1.0 表示 1%）
///
/// # 返回
///
/// 如果误差在可接受范围内返回 Ok(())，否则返回错误
pub fn verify_three_stage_accuracy(
    result: &ThreeStageResult,
    max_error_percent: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    // ===== 裁判：比较三个阶段的结果 =====
    println!("========================================");
    println!("裁判：三阶段结果对比");
    println!("========================================\n");

    let local_output = result.quote_result.amount_out;
    let sim_output = result.simulation_result.amount_out;
    let actual_output = result.execution_result.amount_out;

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ 阶段                │ 输出 (原始单位) │ 说明                  │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!(
        "│ 1. 本地计算         │ {:>12} │ buy_quote              │",
        local_output
    );
    println!(
        "│ 2. 链上模拟         │ {:>12} │ buy_simulate           │",
        sim_output
    );
    println!(
        "│ 3. 实际执行         │ {:>12} │ send_transaction       │",
        actual_output
    );
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    // 计算差异
    let diff_sim_local = local_output.abs_diff(sim_output);
    let diff_actual_sim = sim_output.abs_diff(actual_output);
    let diff_actual_local = local_output.abs_diff(actual_output);

    let error_rate_sim_local = if sim_output > 0 {
        (diff_sim_local as f64 / sim_output as f64) * 100.0
    } else {
        0.0
    };

    let error_rate_actual_sim = if actual_output > 0 {
        (diff_actual_sim as f64 / actual_output as f64) * 100.0
    } else {
        0.0
    };

    let error_rate_actual_local = if actual_output > 0 {
        (diff_actual_local as f64 / actual_output as f64) * 100.0
    } else {
        0.0
    };

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ 差异分析                                                │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ 本地 vs 模拟:                                            │");
    println!(
        "│   绝对差异: {} (原始单位)                            │",
        diff_sim_local
    );
    println!("│   误差率:   {:.4}%                                         │", error_rate_sim_local);
    println!("│                                                         │");
    println!("│ 模拟 vs 实际:                                            │");
    println!(
        "│   绝对差异: {} (原始单位)                            │",
        diff_actual_sim
    );
    println!(
        "│   误差率:   {:.4}%                                         │",
        error_rate_actual_sim
    );
    println!("│                                                         │");
    println!("│ 本地 vs 实际:                                            │");
    println!(
        "│   绝对差异: {} (原始单位)                            │",
        diff_actual_local
    );
    println!(
        "│   误差率:   {:.4}%                                         │",
        error_rate_actual_local
    );
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    // 判断：误差是否在可接受范围内
    let local_sim_ok = error_rate_sim_local <= max_error_percent;
    let sim_actual_ok = error_rate_actual_sim <= max_error_percent;
    let local_actual_ok = error_rate_actual_local <= max_error_percent;

    if local_sim_ok && sim_actual_ok && local_actual_ok {
        println!("✅ 裁判结果：三阶段结果一致");
        println!(
            "   本地 vs 模拟: {:.4}% ≤ {:.1}% ✓",
            error_rate_sim_local, max_error_percent
        );
        println!(
            "   模拟 vs 实际: {:.4}% ≤ {:.1}% ✓",
            error_rate_actual_sim, max_error_percent
        );
        println!(
            "   本地 vs 实际: {:.4}% ≤ {:.1}% ✓",
            error_rate_actual_local, max_error_percent
        );
        println!("✅ 测试通过\n");
        Ok(())
    } else {
        println!("❌ 裁判结果：三阶段结果不一致");
        if !local_sim_ok {
            println!(
                "   ❌ 本地 vs 模拟: {:.4}% > {:.1}%",
                error_rate_sim_local, max_error_percent
            );
        }
        if !sim_actual_ok {
            println!(
                "   ❌ 模拟 vs 实际: {:.4}% > {:.1}%",
                error_rate_actual_sim, max_error_percent
            );
        }
        if !local_actual_ok {
            println!(
                "   ❌ 本地 vs 实际: {:.4}% > {:.1}%",
                error_rate_actual_local, max_error_percent
            );
        }
        println!();
        println!("🔍 可能的原因：");
        println!("  1. 本地计算公式与链上逻辑不一致");
        println!("  2. 储备金在查询和执行之间发生了变化");
        println!("  3. 费用计算方式不同");
        println!("  4. Program data 解析错误");
        println!();
        Err("❌ 测试失败：三阶段结果误差过大".into())
    }
}

/// 清理 Pool 缓存
pub fn cleanup_pool_cache() {
    clear_pool_cache();
}

/// 运行 DEX 三阶段验证测试（Sell 版本）
///
/// # 参数
///
/// * `client` - TradingClient 实例
/// * `config` - 验证配置
/// * `params_builder` - 参数构建器
///
/// # 返回
///
/// 返回三阶段验证结果
pub async fn run_dex_three_stage_verification_sell<P>(
    client: &TradingClient,
    config: DexVerifyConfig,
    params_builder: P,
) -> Result<ThreeStageResult, Box<dyn std::error::Error>>
where
    P: SellParamsBuilder,
{
    println!("==============================================");
    println!("DEX 三阶段验证测试（Sell）");
    println!("==============================================\n");

    println!("📊 测试配置:");
    println!("  DEX: {:?}", config.dex_type);
    println!("  Pool: {}", config.pool);
    println!("  操作: {}", config.operation);
    println!("  方向: {}", config.direction);
    println!("  输入金额: {} (最小单位)\n", config.input_amount);

    // 检查 Pool 类型
    if config.pool.is_mixed_pool() {
        println!(
            "⚠️  混合 Pool 检测: {} + {}\n",
            config.pool.token0_program, config.pool.token1_program
        );
    }

    // ===== 阶段 1: 本地计算（Quote）=====
    println!("========================================");
    println!("阶段 1: 本地计算（client.sell_quote）");
    println!("========================================\n");

    let sell_params = params_builder.build(client, config.input_amount).await;

    let quote_result = match client.sell_quote(sell_params.clone()).await {
        Ok(quote) => quote,
        Err(e) => {
            return Err(format!("❌ 本地计算失败: {}", e).into());
        },
    };

    println!("✅ 本地计算结果:");
    println!("  输出金额: {}", quote_result.amount_out);
    println!("  手续费: {}", quote_result.fee_amount);
    println!("  计算时间: {} ms\n", quote_result.calculation_time_ms);

    // ===== 阶段 2: 链上模拟（Simulation）=====
    println!("========================================");
    println!("阶段 2: 链上模拟（client.sell_simulate）");
    println!("========================================\n");

    let simulation_result = match client.sell_simulate(sell_params.clone()).await {
        Ok(result) => result,
        Err(e) => {
            return Err(format!("❌ 链上模拟失败: {}", e).into());
        },
    };

    println!("✅ 链上模拟结果:");
    println!("  输出金额: {}", simulation_result.amount_out);
    println!("  手续费: {}", simulation_result.fee_amount);
    println!("  计算单元: {} CU", simulation_result.compute_units);
    println!(
        "  交易费用: {} lamports",
        simulation_result.transaction_fee
    );
    println!(
        "  状态: {}\n",
        if simulation_result.success {
            "成功"
        } else {
            "失败"
        }
    );

    // ===== 阶段 3: 实际执行 =====
    println!("========================================");
    println!("阶段 3: 实际执行（client.sell）");
    println!("========================================\n");

    println!("🚀 执行卖出交易...");
    let (success, sigs, error) = client
        .sell(sell_params)
        .await
        .expect("卖出交易执行失败");

    if !success {
        return Err(format!(
            "❌ 卖出交易失败: {:?}\n  Pool: {}\n  输入金额: {}",
            error, config.pool.pool_address, config.input_amount
        )
        .into());
    }

    let signature = sigs.first().expect("交易成功但无签名");
    println!("✅ 卖出成功，签名: {}\n", signature);

    // 解析卖出交易
    println!("📋 解析卖出交易...");
    let parser = DexParser::default();
    let sig_str = signature.to_string();
    let parse_result = parser.parse_transaction(&sig_str).await;

    let execution_result = if parse_result.success && !parse_result.trades.is_empty() {
        let trade = &parse_result.trades[0];
        println!("✅ 卖出交易解析成功:");
        println!("  DEX: {}", trade.dex);
        println!("  用户: {}", trade.user);
        println!("  Pool: {}", trade.pool);
        println!("  交易类型: {:?}", trade.trade_type);
        println!(
            "  输入: {} {} ({} decimals)",
            trade.input_token.amount, trade.input_token.mint, trade.input_token.decimals
        );
        println!(
            "  输出: {} {} ({} decimals)",
            trade.output_token.amount,
            trade.output_token.mint,
            trade.output_token.decimals
        );
        if let Some(ref fee) = trade.fee {
            println!("  费用: {} {}", fee.amount, fee.mint);
        }
        println!();

        ExecutionResult {
            signature: sig_str,
            amount_out: trade.output_token.amount_raw.parse::<u64>().unwrap_or(0),
            amount_out_ui: trade.output_token.amount,
            parse_success: true,
        }
    } else {
        println!("⚠️  卖出交易解析失败: {:?}\n", parse_result.error);
        ExecutionResult {
            signature: sig_str,
            amount_out: 0,
            amount_out_ui: 0.0,
            parse_success: false,
        }
    };

    // 等待链上状态更新
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    Ok(ThreeStageResult {
        quote_result,
        simulation_result,
        execution_result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_accuracy_with_zero_diff() {
        let result = ThreeStageResult {
            quote_result: QuoteResult {
                amount_out: 1000,
                fee_amount: 10,
                price_impact_bps: None,
                calculation_time_ms: 10,
                dex_type: sol_trade_sdk::DexType::RaydiumCpmm,
            },
            simulation_result: SimulationResult {
                amount_out: 1000,
                amount_in: 1000,
                fee_amount: 10,
                compute_units: 100000,
                transaction_fee: 5000,
                success: true,
                error: None,
                logs: None,
                dex_type: sol_trade_sdk::DexType::RaydiumCpmm,
            },
            execution_result: ExecutionResult {
                signature: "test".to_string(),
                amount_out: 1000,
                amount_out_ui: 0.001,
                parse_success: true,
            },
        };

        assert!(verify_three_stage_accuracy(&result, 1.0).is_ok());
    }

    #[test]
    fn test_verify_accuracy_with_small_diff() {
        let result = ThreeStageResult {
            quote_result: QuoteResult {
                amount_out: 1000,
                fee_amount: 10,
                price_impact_bps: None,
                calculation_time_ms: 10,
                dex_type: sol_trade_sdk::DexType::RaydiumCpmm,
            },
            simulation_result: SimulationResult {
                amount_out: 1005,
                amount_in: 1000,
                fee_amount: 10,
                compute_units: 100000,
                transaction_fee: 5000,
                success: true,
                error: None,
                logs: None,
                dex_type: sol_trade_sdk::DexType::RaydiumCpmm,
            },
            execution_result: ExecutionResult {
                signature: "test".to_string(),
                amount_out: 1005,
                amount_out_ui: 0.001,
                parse_success: true,
            },
        };

        // 0.5% 误差应该通过 1% 的容忍度
        assert!(verify_three_stage_accuracy(&result, 1.0).is_ok());
    }

    #[test]
    fn test_verify_accuracy_with_large_diff() {
        let result = ThreeStageResult {
            quote_result: QuoteResult {
                amount_out: 1000,
                fee_amount: 10,
                price_impact_bps: None,
                calculation_time_ms: 10,
                dex_type: sol_trade_sdk::DexType::RaydiumCpmm,
            },
            simulation_result: SimulationResult {
                amount_out: 1050,
                amount_in: 1000,
                fee_amount: 10,
                compute_units: 100000,
                transaction_fee: 5000,
                success: true,
                error: None,
                logs: None,
                dex_type: sol_trade_sdk::DexType::RaydiumCpmm,
            },
            execution_result: ExecutionResult {
                signature: "test".to_string(),
                amount_out: 1050,
                amount_out_ui: 0.001,
                parse_success: true,
            },
        };

        // 5% 误差应该超过 1% 的容忍度
        assert!(verify_three_stage_accuracy(&result, 1.0).is_err());
    }
}
