//! Raydium CPMM USDC-PRTS 验证测试（Exact In）
//!
//! 使用通用 DEX 验证框架，测试 Token-2022 Pool 的三阶段验证
//!
//! # Pool 特性
//!
//! - **Pool 类型**: 混合 Pool（Token Program + Token-2022）
//!   - USDC: 标准 Token Program
//!   - PRTS: Token-2022 Program（启用 Transfer Fee 扩展，但费率为 0%）
//!
//! # 精度说明
//!
//! - 本地计算 vs 链上执行误差：**约 0.04%**
//! - Token-2022 扩展（Transfer Fee 等）不影响计算（费率为 0%）
//! - 误差来自内部状态计算差异和累积手续费扣除精度

mod test_helpers;
use test_helpers::create_test_client;

use sol_trade_sdk::{DexType, TradingClient};
use sol_trade_test_utils::{
    UsdcPrtsBuyParamsBuilder, UsdcPrtsSellParamsBuilder,
    dex_verification::{
        BuyParamsBuilder, DexVerifyConfig, OperationType, RaydiumCpmmPoolRegistry,
        SellParamsBuilder, TradeDirection, cleanup_pool_cache, run_dex_three_stage_verification,
        run_dex_three_stage_verification_sell, verify_three_stage_accuracy,
    },
    ensure_token_balance, prts_mint, usdc_mint,
};

// 参数构建器结构体
struct UsdcPrtsParamsBuilder;

impl BuyParamsBuilder for UsdcPrtsParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(
        &self,
        client: &TradingClient,
        amount: u64,
    ) -> impl std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send {
        async move {
            UsdcPrtsBuyParamsBuilder::new(Some(amount))
                .slippage(1000) // 10% 滑点
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(cpmm_usdc_prts_pool)] // 使用同一把锁，避免并行测试修改同一个 pool
async fn test_cpmm_usdc_prts_exact_in_buy_with_framework() {
    // ===== 测试配置（仅此部分需要修改）=====
    // ⚠️ 注意：USDC decimals = 6，所以：
    // - 1 USDC = 1,000,000 units
    let input_amount = 1_000_000u64; // 1 USDC
    let _rpc_url = "http://127.0.0.1:8899";

    // 使用 Pool 注册表获取配置
    let pool_config = RaydiumCpmmPoolRegistry::usdc_prts();

    // 构建完整的验证配置
    let config = DexVerifyConfig {
        dex_type: DexType::RaydiumCpmm,
        pool: pool_config,
        operation: OperationType::BuyExactIn,
        direction: TradeDirection::Token0ToToken1, // USDC -> PRTS
        input_amount,
        skip_local_quote: false, // CPMM 本地 Quote 准确，不需要跳过
    };

    // ===== 初始化 Client 和余额（框架外的准备）=====
    let client = create_test_client().await;

    // 确保 USDC 余额（Token Program）
    if let Err(e) =
        ensure_token_balance(&client.rpc, client.payer.as_ref(), &usdc_mint(), "10").await
    {
        panic!("❌ 确保 USDC 余额失败: {}", e);
    }

    // ===== 运行三阶段验证（框架自动处理）=====
    let result =
        match run_dex_three_stage_verification(&client, config, UsdcPrtsParamsBuilder).await {
            Ok(r) => r,
            Err(e) => {
                cleanup_pool_cache();
                panic!("❌ 三阶段验证失败: {}", e);
            },
        };

    // ===== 验证结果（框架自动对比）=====
    // 混合 Pool（Token-2022 + Token），本地计算可能有 0.04% 误差，使用 1% 容错
    if let Err(e) = verify_three_stage_accuracy(&result, 1.0, false) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    // 清理缓存
    cleanup_pool_cache();
}

// ==================== Sell Exact In ====================
//
// ⚠️  重要：PRTS Token 的 decimals = 9
// - 1 PRTS = 1,000,000,000 最小单位
// - Pool 流动性极不平衡：~700M PRTS : ~13K USDC
// - 1 PRTS ≈ 0.000019 USDC（约 0.019 cents）
//
// 因此需要较大的交易量才能获得有意义的输出：
// - 100,000 PRTS → 约 1.9 USDC
// - 交易量太小会导致输出 < Transfer Fee，触发 require_gt!(amount_received, 0) 错误
//
// 2025-02-05 调试记录：
// - 原始错误：input_amount = 10_000_000 (误以为是 10 PRTS，实际是 0.01 PRTS)
// - 0.01 PRTS 换到的 USDC ≈ 0，扣除 5% Transfer Fee 后 = 0
// - 错误码：RequireGtViolated (2505), Left: 0, Right: 0
// - 修复：增加金额到 100,000 PRTS，测试通过

struct UsdcPrtsSellExactInParamsBuilder;

impl SellParamsBuilder for UsdcPrtsSellExactInParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(
        &self,
        client: &TradingClient,
        amount: u64,
    ) -> impl std::future::Future<Output = sol_trade_sdk::TradeSellParams> + Send {
        async move {
            UsdcPrtsSellParamsBuilder::new(amount)
                .slippage(1000) // 10% 滑点
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(cpmm_usdc_prts_pool)] // 使用同一把锁，避免并行测试修改同一个 pool
async fn test_cpmm_usdc_prts_sell_exact_in() {
    // ⚠️ 注意：PRTS decimals = 9，所以：
    // - 1 PRTS = 1,000,000,000 units
    // - 100,000 PRTS = 100,000,000,000,000 units
    let input_amount = 100_000_000_000_000u64; // 卖出 100,000 PRTS (PRTS decimals = 9)
    let _rpc_url = "http://127.0.0.1:8899";
    let pool_config = RaydiumCpmmPoolRegistry::usdc_prts();

    let config = DexVerifyConfig {
        dex_type: DexType::RaydiumCpmm,
        pool: pool_config,
        operation: OperationType::SellExactIn,
        direction: TradeDirection::Token1ToToken0, // PRTS -> USDC
        input_amount,
        skip_local_quote: false, // CPMM 本地 Quote 准确，不需要跳过
    };

    let client = create_test_client().await;

    // 确保 PRTS 余额（Token-2022，卖出需要持有 PRTS）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        client.payer.as_ref(),
        &prts_mint(),
        "200000", // 200,000 PRTS（足够卖出 100,000 PRTS）
    )
    .await
    {
        panic!("❌ 确保 PRTS 余额失败: {}", e);
    }

    // 注意：USDC-PRTS 是混合 Pool（Token-2022），本地 vs 链行误差约 0.04%
    let result = match run_dex_three_stage_verification_sell(
        &client,
        config,
        UsdcPrtsSellExactInParamsBuilder,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    // 使用更宽松的误差容忍度（1%），因为 Token-2022 混合 Pool 有已知精度问题
    // 不跳过本地计算验证
    if let Err(e) = verify_three_stage_accuracy(&result, 1.0, false) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    cleanup_pool_cache();
}
