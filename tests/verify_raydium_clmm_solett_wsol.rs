//! Raydium CLMM SOLETT-WSOL 验证测试（三阶段对比）
//!
//! 使用通用 DEX 验证框架，测试 SOLETT-WSOL Pool 的三阶段验证
//!
//! # Pool 特性
//!
//! - **Pool 类型**: CLMM Pool（集中流动性）
//!   - SOLETT: Token-2022 Program
//!   - WSOL: 标准 Token Program
//!
//! # Pool 配置
//!
//! - Pool: CYJQ19fbryujjHFDiik6GZmVpPuqi4Ew31orj43cAupT
//! - SOLETT: RRiB8JNqJvSQ3YJqFASQ3h5BBHPK1KHFrHgCFhxHjoM (Token-2022)
//! - WSOL: So11111111111111111111111111111111111111112 (Token Program)
//!
//! # 测试说明
//!
//! 这是 Token-2022 + Token Program 的混合 CLMM Pool，测试重点：
//! 1. 验证混合 Token Program 的正确处理
//! 2. 验证链上模拟和实际执行的一致性
//! 3. 由于 CLMM 本地计算的已知限制，使用较大的容错率

mod test_helpers;
use test_helpers::create_test_client;

use sol_trade_sdk::instruction::utils::raydium_clmm::get_pool_by_address;
use sol_trade_sdk::{DexType, TradingClient};
use sol_trade_test_utils::{
    SolettWsolClmmBuyParamsBuilder, SolettWsolClmmSellParamsBuilder,
    dex_verification::{
        BuyParamsBuilder, DexVerifyConfig, OperationType, RaydiumClmmPoolRegistry,
        SellParamsBuilder, TradeDirection, cleanup_pool_cache, run_dex_three_stage_verification,
        run_dex_three_stage_verification_sell, verify_three_stage_accuracy,
    },
    ensure_token_balance, solett_mint, wsol_mint,
};

// 参数构建器结构体
struct SolettWsolParamsBuilder;

impl BuyParamsBuilder for SolettWsolParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(
        &self,
        client: &TradingClient,
        amount: u64,
    ) -> impl std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send {
        async move {
            SolettWsolClmmBuyParamsBuilder::new(Some(amount))
                .slippage(10000) // 100% 滑点（CLMM 负数 tick 的本地 Quote 不准确）
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(raydium_clmm_solett_wsol_pool)] // 使用同一把锁，避免并行测试修改同一个 pool
async fn test_raydium_clmm_solett_wsol_exact_in_buy_with_framework() {
    // ===== 测试配置（仅此部分需要修改）=====
    // ⚠️ 注意：WSOL decimals = 9，所以：
    // - 1 WSOL = 1,000,000,000 lamports
    // - 0.01 SOL = 10,000,000 lamports
    let input_amount = 10_000_000u64; // 0.01 SOL
    let _rpc_url = "http://127.0.0.1:8899";

    // 使用 Pool 注册表获取配置
    let pool_config = RaydiumClmmPoolRegistry::solett_wsol();

    // 构建完整的验证配置
    let config = DexVerifyConfig {
        dex_type: DexType::RaydiumClmm,
        pool: pool_config,
        operation: OperationType::BuyExactIn,
        direction: TradeDirection::Token1ToToken0, // WSOL -> SOLETT
        input_amount,
        skip_local_quote: true, // CLMM 负数 tick 本地计算不准确，跳过本地 Quote
    };

    // ===== 初始化 Client 和余额（框架外的准备）=====
    let client = create_test_client().await;

    // 调试：查看 Pool 状态
    let pool_address =
        std::str::FromStr::from_str("CYJQ19fbryujjHFDiik6GZmVpPuqi4Ew31orj43cAupT").unwrap();
    if let Ok(pool_state) = get_pool_by_address(&client.rpc, &pool_address).await {
        println!("🔍 Pool 状态:");
        println!("  token0_mint: {}", pool_state.token_mint0);
        println!("  token1_mint: {}", pool_state.token_mint1);
        println!("  sqrt_price_x64: {}", pool_state.sqrt_price_x64);
        println!("  liquidity: {}", pool_state.liquidity);
        println!("  tick_current: {}", pool_state.tick_current);
        println!("  tick_spacing: {}", pool_state.tick_spacing);
    }

    // 确保 WSOL 余额（Token Program）
    if let Err(e) =
        ensure_token_balance(&client.rpc, client.payer.as_ref(), &wsol_mint(), "10").await
    {
        panic!("❌ 确保 WSOL 余额失败: {}", e);
    }

    // ===== 运行三阶段验证（框架自动处理）=====
    let result =
        match run_dex_three_stage_verification(&client, config, SolettWsolParamsBuilder).await {
            Ok(r) => r,
            Err(e) => {
                cleanup_pool_cache();
                panic!("❌ 三阶段验证失败: {}", e);
            },
        };

    // ===== 验证结果（框架自动对比）=====
    // 注意：SOLETT-WSOL 是 Token-2022 + Token Program 混合 Pool
    // CLMM 本地计算对负数 tick 不准确，我们重点验证链上模拟和实际执行的一致性
    // 使用 skip_local_check=true 跳过本地计算验证
    if let Err(e) = verify_three_stage_accuracy(&result, 0.1, true) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    // 清理缓存
    cleanup_pool_cache();
}

// ==================== Sell Exact In ====================
//
// ⚠️  重要：SOLETT Token 的 decimals 需要根据实际情况确认
// 可以通过 Pool 状态查询获取

struct SolettWsolSellExactInParamsBuilder;

impl SellParamsBuilder for SolettWsolSellExactInParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(
        &self,
        client: &TradingClient,
        amount: u64,
    ) -> impl std::future::Future<Output = sol_trade_sdk::TradeSellParams> + Send {
        async move {
            SolettWsolClmmSellParamsBuilder::new(amount)
                .slippage(10000) // 100% 滑点（CLMM 负数 tick 的本地 Quote 不准确）
                .build(client)
                .await
        }
    }
}

#[tokio::test]
#[serial_test::serial(raydium_clmm_solett_wsol_pool)] // 使用同一把锁，避免并行测试修改同一个 pool
async fn test_raydium_clmm_solett_wsol_sell_exact_in() {
    // ⚠️ 注意：SOLETT 的 decimals = 9（与 WSOL 相同）
    // 增加卖出金额以避免 "amount too small" 错误
    let input_amount = 1_000_000_000u64; // 卖出 1 SOLETT (SOLETT decimals = 9)
    let _rpc_url = "http://127.0.0.1:8899";
    let pool_config = RaydiumClmmPoolRegistry::solett_wsol();

    let config = DexVerifyConfig {
        dex_type: DexType::RaydiumClmm,
        pool: pool_config,
        operation: OperationType::SellExactIn,
        direction: TradeDirection::Token0ToToken1, // SOLETT -> WSOL
        input_amount,
        skip_local_quote: true, // CLMM 负数 tick 本地计算不准确，跳过本地 Quote
    };

    let client = create_test_client().await;

    // 确保 SOLETT 余额（Token-2022，卖出需要持有 SOLETT）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        client.payer.as_ref(),
        &solett_mint(),
        "10000", // 10,000 SOLETT（足够卖出）
    )
    .await
    {
        panic!("❌ 确保 SOLETT 余额失败: {}", e);
    }

    // Token-2022 + Token 混合 Pool，使用较大的容错率
    let result = match run_dex_three_stage_verification_sell(
        &client,
        config,
        SolettWsolSellExactInParamsBuilder,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    // 注意：由于 CLMM local quote 的已知问题,
    // 跳过本地计算验证。重点验证链上模拟和实际执行的一致性。
    if let Err(e) = verify_three_stage_accuracy(&result, 0.1, true) {
        cleanup_pool_cache();
        panic!("{}", e);
    }

    cleanup_pool_cache();
}
