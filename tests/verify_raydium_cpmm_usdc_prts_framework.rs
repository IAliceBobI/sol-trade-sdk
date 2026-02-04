//! Raydium CPMM USDC-PRTS Exact In Buy 验证测试（使用框架）
//!
//! 使用通用 DEX 验证框架，测试 Token2022 Pool 的三阶段验证

mod test_helpers;
use test_helpers::create_test_client;

use sol_trade_sdk::{DexType, TradingClient};
use sol_trade_test_utils::{
    dex_verification::{
        cleanup_pool_cache, run_dex_three_stage_verification, verify_three_stage_accuracy,
        DexVerifyConfig, OperationType, ParamsBuilder, RaydiumCpmmPoolRegistry,
        TradeDirection,
    },
    ensure_token_balance, usdc_mint, UsdcPrtsBuyParamsBuilder,
};

// 参数构建器结构体
struct UsdcPrtsParamsBuilder;

impl ParamsBuilder for UsdcPrtsParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(&self, client: &TradingClient, amount: u64) -> impl std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send {
        async move {
            UsdcPrtsBuyParamsBuilder::new(Some(amount))
                .slippage(1000) // 10% 滑点
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(cpmm_usdc_prts_framework)]
async fn test_cpmm_usdc_prts_exact_in_buy_with_framework() {
    // ===== 测试配置（仅此部分需要修改）=====
    let input_amount = 1_000_000u64; // 1 USDC
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Pool 注册表获取配置
    let pool_config = RaydiumCpmmPoolRegistry::usdc_prts();

    // 构建完整的验证配置
    let config = DexVerifyConfig {
        dex_type: DexType::RaydiumCpmm,
        pool: pool_config,
        operation: OperationType::BuyExactIn,
        direction: TradeDirection::Token0ToToken1, // USDC -> PRTS
        input_amount,
    };

    // ===== 初始化 Client 和余额（框架外的准备）=====
    let client = create_test_client().await;

    // 确保 USDC 余额（Token Program）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        rpc_url,
        client.payer.as_ref(),
        &usdc_mint(),
        "10",
    )
    .await
    {
        panic!("❌ 确保 USDC 余额失败: {}", e);
    }

    // ===== 运行三阶段验证（框架自动处理）=====
    let result = match run_dex_three_stage_verification(&client, config, UsdcPrtsParamsBuilder).await {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    // ===== 验证结果（框架自动对比）=====
    if let Err(e) = verify_three_stage_accuracy(&result, 1.0) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    // 清理缓存
    cleanup_pool_cache();
}
