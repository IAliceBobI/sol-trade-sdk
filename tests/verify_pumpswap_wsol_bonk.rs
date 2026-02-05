//! PumpSwap BONK-WSOL 验证测试（三阶段对比）
//!
//! 使用通用 DEX 验证框架，测试 BONK-WSOL Pool 的三阶段验证
//!
//! # Pool 特性
//!
//! - **Pool 类型**: 纯 Token Pool（Token + Token）
//!   - BONK: 标准 Token Program
//!   - WSOL: 标准 Token Program
//!
//! # 精度说明
//!
//! - 本地计算 vs 链上执行误差：期望 0%（纯 Token Pool）

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
    bonk_mint,
    wsol_mint,
    BonkWsolBuyParamsBuilder,
    BonkWsolSellParamsBuilder,
};

// 参数构建器结构体
struct BonkWsolParamsBuilder;

impl BuyParamsBuilder for BonkWsolParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(&self, client: &TradingClient, amount: u64) -> impl std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send {
        async move {
            BonkWsolBuyParamsBuilder::new(Some(amount))
                .slippage(1000) // 10% 滑点
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(pumpswap_wsol_bonk_pool)] // 使用同一把锁，避免并行测试修改同一个 pool
async fn test_pumpswap_wsol_bonk_exact_in_buy_with_framework() {
    // ===== 测试配置（仅此部分需要修改）=====
    // ⚠️ 注意：WSOL decimals = 9，所以：
    // - 1 WSOL = 1,000,000,000 lamports
    // - 0.001 SOL = 1,000,000 lamports
    let input_amount = 1_000_000u64; // 0.001 SOL
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Pool 注册表获取配置
    let pool_config = PumpSwapPoolRegistry::bonk_wsol();

    // 构建完整的验证配置
    let config = DexVerifyConfig {
        dex_type: DexType::PumpSwap,
        pool: pool_config,
        operation: OperationType::BuyExactIn,
        direction: TradeDirection::Token1ToToken0, // WSOL -> BONK
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
    let result = match run_dex_three_stage_verification(&client, config, BonkWsolParamsBuilder).await {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    // ===== 验证结果（框架自动对比）=====
    // 纯 Token Pool，期望 0% 误差
    if let Err(e) = verify_three_stage_accuracy(&result, 1.0, false) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    // 清理缓存
    cleanup_pool_cache();
}

// ==================== Sell Exact In ====================
//
// ⚠️  重要：BONK Token 的 decimals = 5
// - 1 BONK = 100,000 最小单位
//
// 因此需要较大的交易量才能获得有意义的输出：
// - 1,000,000 BONK → 约 X WSOL
// - 交易量太小会导致输出 < 最小值

struct BonkWsolSellExactInParamsBuilder;

impl SellParamsBuilder for BonkWsolSellExactInParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(
        &self,
        client: &TradingClient,
        amount: u64,
    ) -> impl std::future::Future<Output = sol_trade_sdk::TradeSellParams> + Send {
        async move {
            BonkWsolSellParamsBuilder::new(amount)
                .slippage(1000) // 10% 滑点
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(pumpswap_wsol_bonk_pool)] // 使用同一把锁，避免并行测试修改同一个 pool
async fn test_pumpswap_wsol_bonk_sell_exact_in() {
    // ⚠️ 注意：BONK decimals = 5，所以：
    // - 1 BONK = 100,000 units
    // - 1,000,000 BONK = 100,000,000,000 units
    let input_amount = 100_000_000_000u64; // 卖出 1,000,000 BONK (BONK decimals = 5)
    let rpc_url = "http://127.0.0.1:8899";
    let pool_config = PumpSwapPoolRegistry::bonk_wsol();

    let config = DexVerifyConfig {
        dex_type: DexType::PumpSwap,
        pool: pool_config,
        operation: OperationType::SellExactIn,
        direction: TradeDirection::Token0ToToken1, // BONK -> WSOL
        input_amount,
        skip_local_quote: false, // 本地 Quote 准确，不需要跳过
    };

    let client = create_test_client().await;

    // 确保 BONK 余额（Token Program，卖出需要持有 BONK）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        rpc_url,
        client.payer.as_ref(),
        &bonk_mint(),
        "10000000",  // 10,000,000 BONK（足够卖出 1,000,000 BONK）
    )
    .await
    {
        panic!("❌ 确保 BONK 余额失败: {}", e);
    }

    // 纯 Token Pool，期望 0% 误差
    let result = match run_dex_three_stage_verification_sell(&client, config, BonkWsolSellExactInParamsBuilder).await {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    // 纯 Token Pool，期望 0% 误差
    if let Err(e) = verify_three_stage_accuracy(&result, 1.0, false) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    cleanup_pool_cache();
}
