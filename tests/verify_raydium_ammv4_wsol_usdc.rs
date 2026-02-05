//! Raydium AMM V4 Exact In Buy 完整验证测试（使用框架）
//!
//! 使用通用 DEX 验证框架，测试 WSOL-USDC Pool 的三阶段验证

mod test_helpers;
use test_helpers::create_test_client;

use sol_trade_sdk::{DexType, TradingClient};
use sol_trade_test_utils::{
    dex_verification::{
        cleanup_pool_cache,
        run_dex_three_stage_verification,
        run_dex_three_stage_verification_sell,
        BuyParamsBuilder,
        DexVerifyConfig,
        OperationType,
        RaydiumAmmV4PoolRegistry,
        SellParamsBuilder,
        TradeDirection,
    },
    ensure_token_balance,
    usdc_mint, wsol_mint,
};

// ==================== Buy Exact In ====================

// 参数构建器结构体
struct WsolUsdcParamsBuilder;

impl BuyParamsBuilder for WsolUsdcParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(&self, client: &TradingClient, amount: u64) -> impl std::future::Future<Output = sol_trade_sdk::TradeBuyParams> + Send {
        async move {
            // 构建 AMM V4 买入参数
            let pool_config = RaydiumAmmV4PoolRegistry::wsol_usdc();

            let amm_v4_params = sol_trade_sdk::trading::core::params::RaydiumAmmV4Params::from_amm_address_by_rpc(
                &client.rpc,
                pool_config.pool_address,
            )
            .await
            .expect("Failed to build RaydiumAmmV4Params for WSOL-USDC");

            let recent_blockhash = client
                .rpc
                .get_latest_blockhash()
                .await
                .expect("Failed to get latest blockhash");

            sol_trade_sdk::TradeBuyParams {
                dex_type: sol_trade_sdk::DexType::RaydiumAmmV4,
                input_token_type: sol_trade_sdk::TradeTokenType::SOL,
                mint: pool_config.token1_mint, // USDC (token1)
                input_token_amount: amount,
                slippage_basis_points: Some(0), // 0% 滑点，验证 Quote 准确性
                recent_blockhash: Some(recent_blockhash),
                extension_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumAmmV4(amm_v4_params),
                address_lookup_table_account: None,
                wait_transaction_confirmed: true,
                create_input_token_ata: true,
                close_input_token_ata: false,
                create_mint_ata: true,
                durable_nonce: None,
                enable_jito_sandwich_protection: Some(false),
                fixed_output_token_amount: None,
                gas_fee_strategy: sol_trade_test_utils::create_test_gas_fee_strategy(),
                simulate: false,
                on_transaction_signed: None,
                callback_execution_mode: None,
            }
        }
    }
}

#[tokio::test]
#[serial_test::serial(ammv4_wsol_usdc_pool)] // 使用同一把锁，避免并行测试修改同一个 pool
async fn test_ammv4_exact_in_buy_three_stage_verification_with_framework() {
    // ===== 测试配置（仅此部分需要修改）=====
    let input_amount = 20_000_000u64; // 0.02 SOL（买入少量 USDC）
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Pool 注册表获取配置
    let pool_config = RaydiumAmmV4PoolRegistry::wsol_usdc();

    // 构建完整的验证配置
    let config = DexVerifyConfig {
        dex_type: DexType::RaydiumAmmV4,
        pool: pool_config,
        operation: OperationType::BuyExactIn,
        direction: TradeDirection::Token0ToToken1, // WSOL -> USDC
        input_amount,
    };

    // ===== 初始化 Client 和余额（框架外的准备）=====
    let client = create_test_client().await;

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

    // 确保 USDC 余额（确保 ATA 存在）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        rpc_url,
        client.payer.as_ref(),
        &usdc_mint(),
        "1000",
    )
    .await
    {
        panic!("❌ 确保 USDC 余额失败: {}", e);
    }

    // ===== 运行三阶段验证（框架自动处理）=====
    let result = match run_dex_three_stage_verification(&client, config, WsolUsdcParamsBuilder).await {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    // ===== 验证 Quote 准确性（只比较本地计算和实际执行）=====
    let local_output = result.quote_result.amount_out;
    let actual_output = result.execution_result.amount_out;

    println!("\n========================================");
    println!("裁判：Quote 准确性验证");
    println!("========================================\n");

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ 阶段                │ 输出 (原始单位) │ 说明                  │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!(
        "│ 1. 本地计算         │ {:>12} │ sell_quote             │",
        local_output
    );
    println!(
        "│ 2. 实际执行         │ {:>12} │ send_transaction       │",
        actual_output
    );
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    let diff = local_output.abs_diff(actual_output);
    let error_rate = if actual_output > 0 {
        (diff as f64 / actual_output as f64) * 100.0
    } else {
        0.0
    };

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ 差异分析                                                │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ 本地 vs 实际:                                            │");
    println!("│   绝对差异: {} (原始单位)                            │", diff);
    println!("│   误差率:   {:.4}%                                         │", error_rate);
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    if error_rate <= 1.0 {
        println!("✅ 裁判结果：Quote 计算准确");
        println!("   本地 vs 实际: {:.4}% ≤ 1.0% ✓\n", error_rate);
    } else {
        cleanup_pool_cache();
        panic!(
            "❌ 测试失败：Quote 计算误差过大\n   本地 vs 实际: {:.4}% > 1.0%",
            error_rate
        );
    }

    // 清理缓存
    cleanup_pool_cache();
}

// ==================== Sell Exact In ====================

struct WsolUsdcSellExactInParamsBuilder;

impl SellParamsBuilder for WsolUsdcSellExactInParamsBuilder {
    #[allow(clippy::manual_async_fn)]
    fn build(
        &self,
        client: &TradingClient,
        amount: u64,
    ) -> impl std::future::Future<Output = sol_trade_sdk::TradeSellParams> + Send {
        async move {
            // 构建 AMM V4 卖出参数
            let pool_config = RaydiumAmmV4PoolRegistry::wsol_usdc();

            let amm_v4_params = sol_trade_sdk::trading::core::params::RaydiumAmmV4Params::from_amm_address_by_rpc(
                &client.rpc,
                pool_config.pool_address,
            )
            .await
            .expect("Failed to build RaydiumAmmV4Params for WSOL-USDC");

            let recent_blockhash = client
                .rpc
                .get_latest_blockhash()
                .await
                .expect("Failed to get latest blockhash");

            sol_trade_sdk::TradeSellParams {
                dex_type: sol_trade_sdk::DexType::RaydiumAmmV4,
                output_token_type: sol_trade_sdk::TradeTokenType::SOL,
                mint: pool_config.token1_mint, // USDC (token1)
                input_token_amount: amount,
                slippage_basis_points: Some(0), // 0% 滑点，验证 Quote 准确性
                recent_blockhash: Some(recent_blockhash),
                with_tip: false,
                extension_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumAmmV4(amm_v4_params),
                address_lookup_table_account: None,
                wait_transaction_confirmed: true,
                create_output_token_ata: true,
                close_output_token_ata: false,
                close_mint_token_ata: false,
                durable_nonce: None,
                enable_jito_sandwich_protection: Some(false),
                fixed_output_token_amount: None,
                gas_fee_strategy: sol_trade_test_utils::create_test_gas_fee_strategy(),
                simulate: false,
                on_transaction_signed: None,
                callback_execution_mode: None,
            }
        }
    }
}

#[tokio::test]
#[serial_test::serial(ammv4_wsol_usdc_pool)] // 使用同一把锁，避免并行测试修改同一个 pool
async fn test_ammv4_exact_in_sell_three_stage_verification_with_framework() {
    // ===== 测试配置（仅此部分需要修改）=====
    let input_amount = 1_000_000u64; // 卖出 1 USDC
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Pool 注册表获取配置
    let pool_config = RaydiumAmmV4PoolRegistry::wsol_usdc();

    // 构建完整的验证配置
    let config = DexVerifyConfig {
        dex_type: DexType::RaydiumAmmV4,
        pool: pool_config,
        operation: OperationType::SellExactIn,
        direction: TradeDirection::Token1ToToken0, // USDC -> WSOL
        input_amount,
    };

    // ===== 初始化 Client 和余额（框架外的准备）=====
    let client = create_test_client().await;

    // 确保 USDC 余额（卖出需要持有 USDC）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        rpc_url,
        client.payer.as_ref(),
        &usdc_mint(),
        "10000", // 确保有足够的 USDC
    )
    .await
    {
        panic!("❌ 确保 USDC 余额失败: {}", e);
    }

    // 确保 WSOL 余额（确保 WSOL ATA 存在，用于接收卖出的 WSOL）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        rpc_url,
        client.payer.as_ref(),
        &wsol_mint(),
        "1",
    )
    .await
    {
        panic!("❌ 确保 WSOL 余额失败: {}", e);
    }

    // ===== 运行三阶段验证（框架自动处理）=====
    let result = match run_dex_three_stage_verification_sell(&client, config, WsolUsdcSellExactInParamsBuilder).await {
        Ok(r) => r,
        Err(e) => {
            cleanup_pool_cache();
            panic!("❌ 三阶段验证失败: {}", e);
        },
    };

    // ===== 验证 Quote 准确性（只比较本地计算和实际执行）=====
    let local_output = result.quote_result.amount_out;
    let actual_output = result.execution_result.amount_out;

    println!("\n========================================");
    println!("裁判：Quote 准确性验证");
    println!("========================================\n");

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ 阶段                │ 输出 (原始单位) │ 说明                  │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!(
        "│ 1. 本地计算         │ {:>12} │ sell_quote             │",
        local_output
    );
    println!(
        "│ 2. 实际执行         │ {:>12} │ send_transaction       │",
        actual_output
    );
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    let diff = local_output.abs_diff(actual_output);
    let error_rate = if actual_output > 0 {
        (diff as f64 / actual_output as f64) * 100.0
    } else {
        0.0
    };

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ 差异分析                                                │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ 本地 vs 实际:                                            │");
    println!("│   绝对差异: {} (原始单位)                            │", diff);
    println!("│   误差率:   {:.4}%                                         │", error_rate);
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    if error_rate <= 1.0 {
        println!("✅ 裁判结果：Quote 计算准确");
        println!("   本地 vs 实际: {:.4}% ≤ 1.0% ✓\n", error_rate);
    } else {
        cleanup_pool_cache();
        panic!(
            "❌ 测试失败：Quote 计算误差过大\n   本地 vs 实际: {:.4}% > 1.0%",
            error_rate
        );
    }

    // 清理缓存
    cleanup_pool_cache();
}
