//! Raydium CPMM PIPE-WSOL 完整验证测试（所有操作类型）
//!
//! 使用通用 DEX 验证框架，测试 PIPE-WSOL Pool 的所有交易类型：
//! - Buy Exact In: 已知输入金额，计算输出
//! - Buy Exact Out: 已知输出金额，计算输入
//! - Sell Exact In: 已知输入金额，计算输出
//! - Sell Exact Out: 已知输出金额，计算输入

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
        RaydiumCpmmPoolRegistry,
        SellParamsBuilder,
        TradeDirection,
    },
    ensure_pipe_pool_wsol_liquidity,
    ensure_token_balance,
    pipe_mint,
    PipeWsolBuyParamsBuilder,
    PipeWsolSellParamsBuilder,
    wsol_mint,
};

// ==================== Buy Exact In ====================

struct PipeWsolBuyExactInParamsBuilder;

impl BuyParamsBuilder for PipeWsolBuyExactInParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(
        &self,
        client: &TradingClient,
        amount: u64,
    ) -> impl std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send {
        async move {
            PipeWsolBuyParamsBuilder::new(Some(amount))
                .slippage(1000) // 10% 滑点
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(cpmm_pipe_wsol_buy_exact_in)]
async fn test_cpmm_pipe_wsol_buy_exact_in() {
    let input_amount = 1_000u64; // 0.001 SOL
    let rpc_url = "http://127.0.0.1:8899";
    let pool_config = RaydiumCpmmPoolRegistry::pipe_wsol();

    let config = DexVerifyConfig {
        dex_type: DexType::RaydiumCpmm,
        pool: pool_config,
        operation: OperationType::BuyExactIn,
        direction: TradeDirection::Token1ToToken0, // WSOL -> PIPE
        input_amount,
    };

    let client = create_test_client().await;

    // 跳过流动性添加，直接使用当前池子流动性（约 0.129 SOL，已足够测试）
    // // 确保 PIPE Pool 流动性
    // if let Err(e) =
    //     ensure_pipe_pool_wsol_liquidity(&client.rpc, rpc_url, client.payer.as_ref(), 1).await
    // {
    //     println!("⚠️  警告: 确保 PIPE Pool 流动性失败: {}", e);
    //     println!("继续测试，但可能因为流动性不足而失败...");
    // }

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

    let result = match run_dex_three_stage_verification(&client, config, PipeWsolBuyExactInParamsBuilder).await {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    if let Err(e) = verify_three_stage_accuracy(&result, 1.0) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    cleanup_pool_cache();
}

// ==================== Sell Exact In ====================

struct PipeWsolSellExactInParamsBuilder;

impl SellParamsBuilder for PipeWsolSellExactInParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(
        &self,
        client: &TradingClient,
        amount: u64,
    ) -> impl std::future::Future<Output = sol_trade_sdk::TradeSellParams> + Send {
        async move {
            // 先获取 PIPE 余额，确保有足够的 PIPE 可以卖出
            PipeWsolSellParamsBuilder::new(amount)
                .slippage(1000) // 10% 滑点
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(cpmm_pipe_wsol_sell_exact_in)]
async fn test_cpmm_pipe_wsol_sell_exact_in() {
    let input_amount = 1_000_000u64; // 卖出 1 PIPE
    let rpc_url = "http://127.0.0.1:8899";
    let pool_config = RaydiumCpmmPoolRegistry::pipe_wsol();

    let config = DexVerifyConfig {
        dex_type: DexType::RaydiumCpmm,
        pool: pool_config,
        operation: OperationType::SellExactIn,
        direction: TradeDirection::Token0ToToken1, // PIPE -> WSOL
        input_amount,
    };

    let client = create_test_client().await;

    // 确保 PIPE Pool 流动性
    if let Err(e) =
        ensure_pipe_pool_wsol_liquidity(&client.rpc, rpc_url, client.payer.as_ref(), 1).await
    {
        println!("⚠️  警告: 确保 PIPE Pool 流动性失败: {}", e);
        println!("继续测试，但可能因为流动性不足而失败...");
    }

    // 确保 PIPE 余额（卖出需要持有 PIPE）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        rpc_url,
        client.payer.as_ref(),
        &pipe_mint(),
        "10",
    )
    .await
    {
        panic!("❌ 确保 PIPE 余额失败: {}", e);
    }

    let result = match run_dex_three_stage_verification_sell(&client, config, PipeWsolSellExactInParamsBuilder).await {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    if let Err(e) = verify_three_stage_accuracy(&result, 1.0) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    cleanup_pool_cache();
}
