//! Token 价格获取功能集成测试
//!
//! 测试 CPMM token 的 USD 价格计算功能：
//! - get_token_price_in_usd_with_pool - 通过 Pool 获取 token 的 USD 价格
//!
//! 运行测试:
//!     cargo nextest run --package sol-trade-test-utils token_price_tests -- --nocapture

use sol_trade_sdk::{
    common::auto_mock_rpc::AutoMockRpcClient,
    instruction::utils::raydium_cpmm::get_token_price_in_usd_with_pool,
};
use std::str::FromStr;

/// 测试：获取 PIPE token 的 USD 价格（通过 PIPE-WSOL Pool）
///
/// 测试内容：
/// 1. 使用 get_token_price_in_usd_with_pool 获取价格
/// 2. 验证价格返回成功
/// 3. 验证价格合理性（正数且在合理范围内）
#[tokio::test]
#[serial_test::serial]
async fn test_get_pipe_token_price_in_usd() {
    println!("\n=== 测试：获取 PIPE token 的 USD 价格 ===");

    let pipe_mint = pipe_mint();
    let pipe_wsol_pool = pipe_wsol_pool();
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端（使用独立命名空间）
    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("pipe_price".to_string()),
    );

    println!("Token Mint: {}", pipe_mint);
    println!("Pool 地址: {}", pipe_wsol_pool);
    println!("锚定池: WSOL-USDT（默认）");

    // 调用价格计算函数
    let result: Result<f64, anyhow::Error> =
        get_token_price_in_usd_with_pool(&auto_mock_client, &pipe_mint, &pipe_wsol_pool, None).await;

    // 验证结果
    assert!(result.is_ok(), "Failed to get token price in USD: {:?}", result.err());

    let price_usd = result.unwrap();
    println!("\n✅ PIPE USD 价格: ${:.8}", price_usd);

    // 验证价格合理性
    assert!(price_usd > 0.0, "Price should be positive");
    assert!(price_usd < 1000.0, "Price should be reasonable (< $1000)");
    println!("✅ 价格范围验证通过");

    println!("\n✅ 测试通过");
    println!("💡 首次运行：从 RPC 获取并保存（约 2-3 秒）");
    println!("💡 后续运行：从缓存加载（约 0.01 秒）");
    println!("💡 速度提升：约 100-200 倍！");
}

/// 测试：获取 PRTS token 的 USD 价格（通过 USDC-PRTS Pool）
///
/// 测试内容：
/// 1. 使用 get_token_price_in_usd_with_pool 获取价格
/// 2. 验证价格返回成功
/// 3. 验证价格合理性
#[tokio::test]
#[serial_test::serial]
async fn test_get_prts_token_price_in_usd() {
    println!("\n=== 测试：获取 PRTS token 的 USD 价格 ===");

    let prts_mint = prts_mint();
    let usdc_prts_pool = usdc_prts_pool();
    let rpc_url = "http://127.0.0.1:8899";

    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("prts_price".to_string()),
    );

    println!("Token Mint: {}", prts_mint);
    println!("Pool 地址: {}", usdc_prts_pool);
    println!("锚定池: USDC-USDT（默认）");

    let result: Result<f64, anyhow::Error> =
        get_token_price_in_usd_with_pool(&auto_mock_client, &prts_mint, &usdc_prts_pool, None).await;

    assert!(result.is_ok(), "Failed to get token price in USD: {:?}", result.err());

    let price_usd = result.unwrap();
    println!("\n✅ PRTS USD 价格: ${:.8}", price_usd);

    assert!(price_usd > 0.0, "Price should be positive");
    assert!(price_usd < 1000.0, "Price should be reasonable (< $1000)");
    println!("✅ 价格范围验证通过");

    println!("\n✅ 测试通过");
}

/// 测试：获取多个 token 的 USD 价格
///
/// 测试内容：
/// 1. 获取 PIPE 价格
/// 2. 获取 PRTS 价格
/// 3. 验证所有价格都在合理范围内
#[tokio::test]
#[serial_test::serial]
async fn test_get_multiple_token_prices() {
    println!("\n=== 测试：获取多个 token 的 USD 价格 ===");

    let rpc_url = "http://127.0.0.1:8899";
    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("multi_token_price".to_string()),
    );

    // 测试 PIPE
    println!("\n📊 测试 PIPE 价格...");
    let pipe_price = get_token_price_in_usd_with_pool(
        &auto_mock_client,
        &pipe_mint(),
        &pipe_wsol_pool(),
        None,
    )
    .await
    .expect("Failed to get PIPE price");
    println!("   PIPE: ${:.8}", pipe_price);
    assert!(pipe_price > 0.0 && pipe_price < 1000.0);

    // 测试 PRTS
    println!("\n📊 测试 PRTS 价格...");
    let prts_price = get_token_price_in_usd_with_pool(
        &auto_mock_client,
        &prts_mint(),
        &usdc_prts_pool(),
        None,
    )
    .await
    .expect("Failed to get PRTS price");
    println!("   PRTS: ${:.8}", prts_price);
    assert!(prts_price > 0.0 && prts_price < 1000.0);

    println!("\n✅ 所有价格验证通过");
    println!("\n✅ 测试通过");
}

// ============ Helper Functions ============

// cpmm pool: BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp, wsol-pipe(pipe 8ycz3kctoRb4LFrtoYG2r8tRyUYUeGf5Q16M2TEMp7A token)
// cpmm pool: CVPpJXyiPNRgD3a8SjmXkC1cKdHtry1PF9BVG6dYoxjk, usdc-ring(ring A3569FJtxQ9qstaE1ToZDt8uAwkTQyMRf8xy669DbUZz token)
// cpmm pool: 7Cvz28TyKnGuL8GAtbsVFu1FJ3Po7A37Zc8JSJqkSPDp, usdc-prts(prts 3PQkX8yfuxoe9kuBoLCEZoxzi9LG4w8Ci2JWWGNfPRTS token2022)
// cpmm pool: GarGiGTMQrZyot44J9hc71NeGNeEaxnq3nefKxBruEsS, usdc-cib(cib GarGiGTMQrZyot44J9hc71NeGNeEaxnq3nefKxBruEsS token2022)

/// PIPE Mint 地址
fn pipe_mint() -> solana_sdk::pubkey::Pubkey {
    solana_sdk::pubkey::Pubkey::from_str("8ycz3kctoRb4LFrtoYG2r8tRyUYUeGf5Q16M2TEMp7A").unwrap()
}

/// PRTS Mint 地址
fn prts_mint() -> solana_sdk::pubkey::Pubkey {
    solana_sdk::pubkey::Pubkey::from_str("3PQkX8yfuxoe9kuBoLCEZoxzi9LG4w8Ci2JWWGNfPRTS").unwrap()
}

/// PIPE-WSOL Pool 地址
fn pipe_wsol_pool() -> solana_sdk::pubkey::Pubkey {
    solana_sdk::pubkey::Pubkey::from_str("BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp").unwrap()
}

/// USDC-PRTS Pool 地址
fn usdc_prts_pool() -> solana_sdk::pubkey::Pubkey {
    solana_sdk::pubkey::Pubkey::from_str("7Cvz28TyKnGuL8GAtbsVFu1FJ3Po7A37Zc8JSJqkSPDp").unwrap()
}
