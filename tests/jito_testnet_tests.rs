//! Jito Testnet 测试
//!
//! 需要真实网络连接的测试
//!
//! ## 测试环境要求
//!
//! ### 环境变量
//! - `SOLANA_TEST_KEY_PATH`: Testnet 密钥文件路径（用于 test_jito_bundle_send_example）
//! - `PROXY_URL`: 代理 URL（可选，默认 http://127.0.0.1:7891）
//!
//! ### 运行方式
//!
//! ```bash
//! # 1. 设置环境变量
//! export SOLANA_TEST_KEY_PATH=/path/to/testnet-keypair.json
//!
//! # 2. 运行所有 testnet 测试
//! cargo nextest run --test jito_testnet_tests -- --ignored
//!
//! # 3. 运行特定测试
//! cargo nextest run --test jito_testnet_tests -- test_jito_bundle_send_example --exact --nocapture --ignored
//! cargo nextest run --test jito_testnet_tests -- test_jito_dynamic_tip_floor --exact --nocapture --ignored
//! ```
//!
//! ## 📚 相关资源
//!
//! - [Jito 官方文档](https://docs.jito.wtf)
//! - [Tip Floor API](https://bundles.jito.wtf/api/v1/bundles/tip_floor)
//! - [Solana Testnet Faucet](https://faucet.solana.com/)

use solana_sdk::{
    pubkey::Pubkey,
    signature::{EncodableKey, Keypair, Signer},
};
use std::str::FromStr;

// 导入公共代理库
mod common;
use common::proxy_http::{get_latest_blockhash_with_proxy, get_solana_balance_with_proxy};

// ============================================================================
// Test 1: Jito Bundle Testnet 模拟测试
// ============================================================================

/// 完整的 Jito Bundle 发送示例（Testnet 模拟）
///
/// 这个测试展示如何在 Testnet 上模拟构建 Jito Bundle 交易
/// 注意：这是模拟测试，不实际发送交易
///
/// ## 环境变量
/// - `SOLANA_TEST_KEY_PATH`: Testnet 密钥文件路径
/// - `PROXY_URL`: 代理 URL（可选，默认 http://127.0.0.1:7891）
///
/// ## 运行方式
/// ```bash
/// export SOLANA_TEST_KEY_PATH=/path/to/testnet-keypair.json
/// cargo test --test jito_testnet_tests -- test_jito_bundle_send_example --exact --nocapture --ignored
/// ```
#[tokio::test]
#[ignore] // 默认忽略，需要手动运行
async fn test_jito_bundle_send_example() -> Result<(), Box<dyn std::error::Error>> {
    use std::env;

    println!("\n========== Jito Bundle Testnet 模拟测试 ==========\n");

    // ========== 1. 读取环境变量 ==========
    let key_path = env::var("SOLANA_TEST_KEY_PATH").expect("SOLANA_TEST_KEY_PATH 环境变量未设置");

    let proxy_url = env::var("PROXY_URL").unwrap_or("http://127.0.0.1:7891".to_string());

    println!("📁 密钥路径: {}", key_path);
    println!("🔌 代理地址: {}", proxy_url);

    // ========== 2. 读取密钥 ==========
    let payer = Keypair::read_from_file(&key_path)?;
    println!("📍 Payer 地址: {}", payer.pubkey());

    // ========== 3. 配置 RPC ==========
    let testnet_rpc = "https://api.testnet.solana.com";
    let jito_testnet_endpoint = "https://dallas.testnet.block-engine.jito.wtf";

    println!("\n🌐 Testnet RPC: {}", testnet_rpc);
    println!("🚀 Jito Testnet: {}", jito_testnet_endpoint);

    // ========== 4. 创建 RPC 客户端（通过代理） ==========
    println!("\n📡 正在查询账户余额...");

    // 查询余额（使用公共代理库）
    let balance =
        get_solana_balance_with_proxy(testnet_rpc, Some(&proxy_url), &payer.pubkey().to_string())
            .await?;
    let sol_balance = balance as f64 / 1_000_000_000.0;

    println!("💰 账户余额: {:.9} SOL ({} lamports)", sol_balance, balance);

    if balance < 5_000_000 {
        println!("\n⚠️  余额不足（需要至少 0.005 SOL）");
        println!("💡 请从以下地址获取测试 SOL:");
        println!("   https://faucet.solana.com/");
        return Err("余额不足".into());
    }

    // ========== 5. 获取 recent blockhash ==========
    println!("\n📡 正在获取 recent blockhash...");

    let blockhash = get_latest_blockhash_with_proxy(testnet_rpc, Some(&proxy_url)).await?;
    println!("✅ Blockhash: {}", blockhash);

    // ========== 6. 创建 receiver 和 tip account ==========
    let receiver = Pubkey::from_str("GjJyeC3YDUU7TPCndhTUzbf3HqHYBH1JKQmWLH9nPqx").unwrap();
    let tip_account = Pubkey::from_str("HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe").unwrap();

    println!("\n👤 Receiver: {}", receiver);
    println!("💰 Tip Account: {}", tip_account);

    // ========== 7. 展示 Bundle 结构 ==========
    println!("\n📦 模拟构建 Bundle 交易（3 笔）...");

    let tip_amount = 10_000; // 0.00001 SOL
    let transfer_amount = 1_000; // 每笔转账 0.000001 SOL

    println!("  ✓ 交易 1: 转账 {} lamports 到 receiver", transfer_amount);
    println!("  ✓ 交易 2: 转账 {} lamports 到 receiver", transfer_amount);
    println!(
        "  ✓ 交易 3: 转账 {} lamports 到 receiver + Tip {} lamports",
        transfer_amount, tip_amount
    );

    // ========== 8. 展示 Bundle 详情 ==========
    println!("\n📋 Bundle 结构详情:");
    println!("  ├─ 交易数量: 3 / 5 (最大)");
    println!("  ├─ 总转账: {} lamports", transfer_amount * 3);
    println!(
        "  ├─ 总 Tip: {} lamports ({:.6} SOL)",
        tip_amount,
        tip_amount as f64 / 1_000_000_000.0
    );
    println!("  ├─ 预估交易费: ~15,000 lamports (5,000 × 3)");
    println!(
        "  ├─ 预估总花费: {} lamports ({:.9} SOL)",
        transfer_amount * 3 + tip_amount + 15_000,
        (transfer_amount * 3 + tip_amount + 15_000) as f64 / 1_000_000_000.0
    );
    println!("  └─ 原子性: 是（全部成功或全部失败）");

    // ========== 9. 展示如何实际发送 ==========
    println!("\n💡 如果要实际发送 Bundle，需要:");
    println!("  1. 使用 SDK 创建 JitoClient:");
    println!("     ```rust");
    println!(
        "     use sol_trade_sdk::swqos::{{SwqosClientTrait, jito::{{JitoClient, JitoRegion}}}};"
    );
    println!("     ");
    println!("     // 创建自定义 testnet client");
    println!("     let client = JitoClient::new(");
    println!("         testnet_rpc.to_string(),");
    println!("         JitoRegion::Custom(jito_testnet_endpoint),");
    println!("         String::new(),");
    println!("     );");
    println!("     ```");
    println!("\n  2. 构建交易并序列化:");
    println!("     ```rust");
    println!("     let transactions = vec![tx1, tx2, tx3];");
    println!("     let txs_base64: Vec<String> = transactions");
    println!("         .iter()");
    println!("         .map(|tx| tx.to_base64_string())");
    println!("         .collect();");
    println!("     ```");
    println!("\n  3. 发送到 Jito Testnet:");
    println!("     ```rust");
    println!("     client.send_transactions(");
    println!("         TradeType::Buy,");
    println!("         &transactions,");
    println!("         false, // 不等待确认");
    println!("     ).await?;");
    println!("     ```");
    println!("\n  或者使用 HTTP 直接发送:");
    println!("     POST {}/api/v1/bundles", jito_testnet_endpoint);
    println!("     Content-Type: application/json");
    println!("     ");
    println!("     {{");
    println!("       \"jsonrpc\": \"2.0\",");
    println!("       \"id\": 1,");
    println!("       \"method\": \"sendBundle\",");
    println!("       \"params\": [[tx1_base64, tx2_base64, tx3_base64]]");
    println!("     }}");

    println!("\n✅ 测试完成!");
    println!("📝 注意: 这是模拟测试，展示了构建流程，但未实际发送交易");
    println!("📝 所有交易使用相同的 blockhash: {}", blockhash);
    println!("📝 Tip 必须在最后一笔交易中");
    println!("\n============================================\n");

    Ok(())
}

// ============================================================================
// Test 2: 动态 Tip Floor API 测试
// ============================================================================

/// 测试动态 Tip Floor API
///
/// 这个测试展示如何从 Jito Tip Floor API 获取实时 tip 数据
/// 这是 Jito 官方推荐的动态 tip 策略
#[tokio::test]
#[ignore] // 默认忽略，需要网络连接
async fn test_jito_dynamic_tip_floor() {
    use sol_trade_sdk::swqos::jito::{
        DynamicTipConfig,
        dynamic_tip::{JitoTipFloorClient, TipPercentile},
    };

    println!("\n========== Jito 动态 Tip Floor 测试 ==========\n");

    // 创建 Tip Floor 客户端（使用环境变量 PROXY_URL 中的代理，如果设置）
    let tip_client = JitoTipFloorClient::from_env_proxy();

    println!("📡 正在获取 Jito Tip Floor 数据...");

    match tip_client.get_tip_floor().await {
        Ok(tip_data) => {
            println!("✅ 成功获取 Tip Floor 数据!\n");

            println!("📊 Tip Floor 统计 (基于已成功的交易):");
            println!("  ├─ P25:  {:.6} SOL (25% 的交易)", tip_data.landed_tips_25th_percentile);
            println!("  ├─ P50:  {:.6} SOL (中位数)", tip_data.landed_tips_50th_percentile);
            println!("  ├─ P75:  {:.6} SOL (75% 的交易)", tip_data.landed_tips_75th_percentile);
            println!("  ├─ P95:  {:.6} SOL (95% 的交易)", tip_data.landed_tips_95th_percentile);
            println!("  ├─ P99:  {:.6} SOL (99% 的交易)", tip_data.landed_tips_99th_percentile);
            println!(
                "  └─ EMA: {:.6} SOL (指数移动平均)",
                tip_data.ema_landed_tips_50th_percentile
            );

            println!("\n💡 策略建议:");
            println!("  - 保守策略（低成本）: P25-P50");
            println!("  - 平衡策略（推荐）: P50-P75");
            println!("  - 激进策略（高优先级）: P95-P99");

            // 测试不同配置的动态 tip 计算
            println!("\n🧮 不同配置的计算结果:");

            let configs = vec![
                (
                    DynamicTipConfig {
                        enabled: true,
                        percentile: TipPercentile::P25,
                        multiplier: 1.0,
                        min_tip: 0.000001,
                        max_tip: 0.001,
                    },
                    "保守策略 (P25)",
                ),
                (
                    DynamicTipConfig {
                        enabled: true,
                        percentile: TipPercentile::P50,
                        multiplier: 1.0,
                        min_tip: 0.000001,
                        max_tip: 0.001,
                    },
                    "平衡策略 (P50)",
                ),
                (
                    DynamicTipConfig {
                        enabled: true,
                        percentile: TipPercentile::P75,
                        multiplier: 1.0,
                        min_tip: 0.000001,
                        max_tip: 0.001,
                    },
                    "平衡策略 (P75)",
                ),
                (
                    DynamicTipConfig {
                        enabled: true,
                        percentile: TipPercentile::P95,
                        multiplier: 1.0,
                        min_tip: 0.000001,
                        max_tip: 0.001,
                    },
                    "激进策略 (P95)",
                ),
            ];

            for (config, strategy_name) in configs {
                let calculated_tip = tip_client.calculate_tip(&tip_data, &config);
                println!("  - {}: {:.6} SOL", strategy_name, calculated_tip);
            }
        },
        Err(e) => {
            println!("❌ 获取 Tip Floor 失败: {}", e);
            println!("💡 可能的原因:");
            println!("   - 网络连接问题");
            println!("   - Jito API 暂时不可用");
        },
    }

    println!("\n============================================\n");
}
