//! PumpSwap PUMP-WSOL 验证测试（三阶段对比）
//!
//! 使用通用 DEX 验证框架，测试 PUMP-WSOL Pool 的三阶段验证
//!
//! # Pool 特性
//!
//! - **Pool 类型**: 混合 Pool（Token-2022 + Token）
//!   - PUMP: Token-2022 Program
//!   - WSOL: 标准 Token Program
//!
//! # 精度说明
//!
//! - 本地计算 vs 链上执行误差：根据测试结果确定

mod test_helpers;
use test_helpers::create_test_client;

use sol_trade_sdk::{DexType, TradingClient};
use sol_trade_test_utils::{
    dex_verification::{
        cleanup_pool_cache,
        run_dex_three_stage_verification,
        run_dex_three_stage_verification_sell,
        verify_three_stage_accuracy,
        BuyParamsBuilder,
        DexVerifyConfig,
        OperationType,
        PumpSwapPoolRegistry,
        SellParamsBuilder,
        TradeDirection,
    },
    ensure_token_balance,
    pump_mint,
    wsol_mint,
    PumpWsolBuyParamsBuilder,
    PumpWsolSellParamsBuilder,
};

// 参数构建器结构体
struct PumpWsolParamsBuilder;

impl BuyParamsBuilder for PumpWsolParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(&self, client: &TradingClient, amount: u64) -> impl std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send {
        async move {
            PumpWsolBuyParamsBuilder::new(Some(amount))
                .slippage(1000) // 10% 滑点
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(pumpswap_wsol_pump_pool)] // 使用同一把锁，避免并行测试修改同一个 pool
async fn test_pumpswap_wsol_pump_exact_in_buy_with_framework() {
    // ===== 测试配置（仅此部分需要修改）=====
    // ⚠️ 注意：WSOL decimals = 9，所以：
    // - 1 WSOL = 1,000,000,000 lamports
    // - 0.001 SOL = 1,000,000 lamports
    let input_amount = 1_000_000u64; // 0.001 SOL
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Pool 注册表获取配置
    let pool_config = PumpSwapPoolRegistry::pump_wsol();

    // 构建完整的验证配置
    let config = DexVerifyConfig {
        dex_type: DexType::PumpSwap,
        pool: pool_config,
        operation: OperationType::BuyExactIn,
        direction: TradeDirection::Token1ToToken0, // WSOL -> PUMP
        input_amount,
        skip_local_quote: false, // 本地 Quote 准确，不需要跳过
    };

    // ===== 初始化 Client 和余额（框架外的准备）=====
    let client = create_test_client().await;

    // 确保 WSOL 余额（Token Program）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        rpc_url,
        client.payer.as_ref(),
        &wsol_mint(),
        "10",
    )
    .await
    {
        panic!("❌ 确保 WSOL 余额失败: {}", e);
    }

    // ===== 运行三阶段验证（框架自动处理）=====
    let result = match run_dex_three_stage_verification(&client, config, PumpWsolParamsBuilder).await {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    // ===== 验证结果（框架自动对比）=====
    if let Err(e) = verify_three_stage_accuracy(&result, 1.0, false) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    // 清理缓存
    cleanup_pool_cache();
}

// ==================== Sell Exact In ====================
//
// ⚠️  重要：PUMP Token 的 decimals = 6
// - 1 PUMP = 1,000,000 最小单位
//
// 因此需要较大的交易量才能获得有意义的输出：
// - 10,000 PUMP → 约 X WSOL
// - 交易量太小会导致输出 < 最小值

struct PumpWsolSellExactInParamsBuilder;

impl SellParamsBuilder for PumpWsolSellExactInParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(
        &self,
        client: &TradingClient,
        amount: u64,
    ) -> impl std::future::Future<Output = sol_trade_sdk::TradeSellParams> + Send {
        async move {
            PumpWsolSellParamsBuilder::new(amount)
                .slippage(1000) // 10% 滑点
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(pumpswap_wsol_pump_pool)] // 使用同一把锁，避免并行测试修改同一个 pool
async fn test_pumpswap_wsol_pump_sell_exact_in() {
    // ⚠️ 注意：PUMP decimals = 6，所以：
    // - 1 PUMP = 1,000,000 units
    // - 10,000 PUMP = 10,000,000,000 units
    let input_amount = 10_000_000_000u64; // 卖出 10,000 PUMP (PUMP decimals = 6)
    let rpc_url = "http://127.0.0.1:8899";
    let pool_config = PumpSwapPoolRegistry::pump_wsol();

    let config = DexVerifyConfig {
        dex_type: DexType::PumpSwap,
        pool: pool_config,
        operation: OperationType::SellExactIn,
        direction: TradeDirection::Token0ToToken1, // PUMP -> WSOL
        input_amount,
        skip_local_quote: false, // 本地 Quote 准确，不需要跳过
    };

    let client = create_test_client().await;

    // 确保 PUMP 余额（Token-2022，卖出需要持有 PUMP）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        rpc_url,
        client.payer.as_ref(),
        &pump_mint(),
        "100000",  // 100,000 PUMP（足够卖出 10,000 PUMP）
    )
    .await
    {
        panic!("❌ 确保 PUMP 余额失败: {}", e);
    }

    // 注意：PUMP-WSOL 是混合 Pool（Token-2022 + Token），本地 vs 链行误差待测试确定
    let result = match run_dex_three_stage_verification_sell(&client, config, PumpWsolSellExactInParamsBuilder).await {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    // 使用更宽松的误差容忍度（1%），因为 Token-2022 混合 Pool 可能有精度问题
    if let Err(e) = verify_three_stage_accuracy(&result, 1.0, false) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    cleanup_pool_cache();
}
