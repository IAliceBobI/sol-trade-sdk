//! Raydium CPMM Exact In Buy 完整验证测试（使用框架）
//!
//! 使用通用 DEX 验证框架，测试 PIPE-WSOL Pool 的三阶段验证

mod test_helpers;
use test_helpers::create_test_client;

use sol_trade_sdk::{DexType, TradingClient};
use sol_trade_test_utils::{
    dex_verification::{
        cleanup_pool_cache, run_dex_three_stage_verification, verify_three_stage_accuracy,
        DexVerifyConfig, OperationType, ParamsBuilder, RaydiumCpmmPoolRegistry,
        TradeDirection,
    },
    ensure_pipe_pool_wsol_liquidity, ensure_token_balance,
    pipe_mint, wsol_mint, PipeWsolBuyParamsBuilder,
};

// 参数构建器结构体
struct PipeWsolParamsBuilder;

impl ParamsBuilder for PipeWsolParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(&self, client: &TradingClient, amount: u64) -> impl std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send {
        async move {
            PipeWsolBuyParamsBuilder::new(Some(amount))
                .slippage(1000) // 10% 滑点
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(cpmm_exact_in_buy_complete_framework)]
async fn test_cpmm_exact_in_buy_three_stage_verification_with_framework() {
    // ===== 测试配置（仅此部分需要修改）=====
    let input_amount = 1_000u64; // 0.001 SOL
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Pool 注册表获取配置
    let pool_config = RaydiumCpmmPoolRegistry::pipe_wsol();

    // 构建完整的验证配置
    let config = DexVerifyConfig {
        dex_type: DexType::RaydiumCpmm,
        pool: pool_config,
        operation: OperationType::BuyExactIn,
        direction: TradeDirection::Token1ToToken0, // WSOL -> PIPE
        input_amount,
    };

    // ===== 初始化 Client 和余额（框架外的准备）=====
    let client = create_test_client().await;

    // 确保 PIPE Pool 流动性
    if let Err(e) = ensure_pipe_pool_wsol_liquidity(
        &client.rpc,
        rpc_url,
        client.payer.as_ref(),
        10,
    )
    .await
    {
        println!("⚠️  警告: 确保 PIPE Pool 流动性失败: {}", e);
        println!("继续测试，但可能因为流动性不足而失败...");
    }

    // 确保 WSOL 余额
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

    // 确保 PIPE 余额（确保 ATA 存在）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        rpc_url,
        client.payer.as_ref(),
        &pipe_mint(),
        "1",
    )
    .await
    {
        panic!("❌ 确保 PIPE 余额失败: {}", e);
    }

    // ===== 运行三阶段验证（框架自动处理）=====
    let result = match run_dex_three_stage_verification(&client, config, PipeWsolParamsBuilder).await {
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
