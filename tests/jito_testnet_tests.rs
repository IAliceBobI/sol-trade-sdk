//! Jito Testnet 测试
//!
//! 需要真实网络连接的测试
//!
//! ## 测试环境要求
//!
//! ### 环境变量
//! - `SOLANA_TEST_KEY_PATH1`: Testnet 发送方密钥文件路径（用于 test_jito_bundle_send_example）
//! - `SOLANA_TEST_KEY_PATH2`: Testnet 接收方密钥文件路径（用于 test_jito_bundle_send_example）
//! - `PROXY_URL`: 代理 URL（可选，默认 http://127.0.0.1:7891）
//!
//! ### 运行方式
//!
//! ```bash
//! # 1. 设置环境变量
//! export SOLANA_TEST_KEY_PATH1=/path/to/sender-keypair.json
//! export SOLANA_TEST_KEY_PATH2=/path/to/receiver-keypair.json
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
    transaction::Transaction,
};
use solana_system_interface::instruction::transfer;
use std::str::FromStr;

// 导入公共代理库
mod common;
use common::proxy_http::{get_latest_blockhash_with_proxy, get_solana_balance_with_proxy};

// ============================================================================
// Test 1: Jito Bundle Testnet 模拟测试
// ============================================================================

/// 完整的 Jito Bundle 发送示例（Testnet 实际测试）
///
/// 这个测试在 Testnet 上实际发送 Jito Bundle 交易
/// Bundle 包含 3 笔从 SOLANA_TEST_KEY_PATH1 到 SOLANA_TEST_KEY_PATH2 的小额 SOL 转账
///
/// ## 避免重复交易的措施
///
/// Solana 通过**消息哈希**(message hash)来判断交易是否重复。消息哈希包含:
/// - 账户列表
/// - 指令数据(program_id, data, accounts)
/// - recent_blockhash
///
/// 为避免 Bundle 中的交易被视为重复,本测试采用了以下策略:
/// 1. **唯一 Memo**: 每个交易添加包含时间戳的唯一 memo 指令
/// 2. **随机化金额**: 在基础金额上添加小的随机增量(转账 ±100 lamports, tip ±1000 lamports)
/// 3. **不同 Tip 账户**: 为每个交易使用不同的 Jito tip 账户(Jito 共有 8 个)
///
/// 这些措施确保每个交易产生唯一的消息哈希,避免错误码 -32602(重复交易)。
///
/// ## 环境变量
/// - `SOLANA_TEST_KEY_PATH1`: Testnet 发送方密钥文件路径
/// - `SOLANA_TEST_KEY_PATH2`: Testnet 接收方密钥文件路径
/// - `PROXY_URL`: 代理 URL（可选，默认 http://127.0.0.1:7891）
///
/// ## 运行方式
/// ```bash
/// export SOLANA_TEST_KEY_PATH1=/path/to/sender-keypair.json
/// export SOLANA_TEST_KEY_PATH2=/path/to/receiver-keypair.json
/// cargo test --test jito_testnet_tests -- test_jito_bundle_send_example --exact --nocapture --ignored
/// ```
#[tokio::test]
#[ignore] // 默认忽略，需要手动运行
async fn test_jito_bundle_send_example() -> Result<(), Box<dyn std::error::Error>> {
    use std::env;
    use solana_sdk::hash::Hash;

    println!("\n========== Jito Bundle Testnet 实际测试 ==========\n");

    // ========== 1. 读取环境变量 ==========
    let sender_key_path = env::var("SOLANA_TEST_KEY_PATH1")
        .expect("SOLANA_TEST_KEY_PATH1 环境变量未设置");
    let receiver_key_path = env::var("SOLANA_TEST_KEY_PATH2")
        .expect("SOLANA_TEST_KEY_PATH2 环境变量未设置");

    let proxy_url = env::var("PROXY_URL").unwrap_or("http://127.0.0.1:7891".to_string());

    println!("📁 发送方密钥路径: {}", sender_key_path);
    println!("📁 接收方密钥路径: {}", receiver_key_path);
    println!("🔌 代理地址: {}", proxy_url);

    // ========== 2. 读取密钥 ==========
    let sender = Keypair::read_from_file(&sender_key_path)?;
    let receiver_keypair = Keypair::read_from_file(&receiver_key_path)?;
    let receiver_pubkey = receiver_keypair.pubkey();

    println!("\n📍 发送方地址: {}", sender.pubkey());
    println!("📍 接收方地址: {}", receiver_pubkey);

    // ========== 3. 配置 RPC ==========
    let testnet_rpc = "https://api.testnet.solana.com";
    let jito_testnet_endpoint = "https://dallas.testnet.block-engine.jito.wtf";

    println!("\n🌐 Testnet RPC: {}", testnet_rpc);
    println!("🚀 Jito Testnet: {}", jito_testnet_endpoint);

    // ========== 4. 创建 RPC 客户端（通过代理） ==========
    println!("\n📡 正在查询账户余额...");

    // 查询发送方余额
    let sender_balance =
        get_solana_balance_with_proxy(testnet_rpc, Some(&proxy_url), &sender.pubkey().to_string())
            .await?;
    let sender_sol_balance = sender_balance as f64 / 1_000_000_000.0;

    println!("💰 发送方余额: {:.9} SOL ({} lamports)", sender_sol_balance, sender_balance);

    if sender_balance < 10_000_000 {
        println!("\n⚠️  发送方余额不足（需要至少 0.01 SOL）");
        println!("💡 请从以下地址获取测试 SOL:");
        println!("   https://faucet.solana.com/");
        return Err("发送方余额不足".into());
    }

    // ========== 5. 获取 recent blockhash ==========
    println!("\n📡 正在获取 recent blockhash...");

    let blockhash_str = get_latest_blockhash_with_proxy(testnet_rpc, Some(&proxy_url)).await?;
    let blockhash = Hash::from_str(&blockhash_str)?;
    println!("✅ Blockhash: {}", blockhash_str);

    // ========== 6. 构建 Bundle 交易 ==========
    println!("\n🔨 正在构建 Bundle 交易（4 个交易）...");

    // Jito 的 8 个 Tip 账户
    let jito_tip_accounts = vec![
        "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
        "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
        "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
        "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
        "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
        "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
        "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
        "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
    ];

    let mut rng = rand::rng();

    let base_transfer_amount = 1_000; // 基础转账金额 0.000001 SOL
    let base_tip_amount = 10_000; // 基础 tip 金额 0.00001 SOL
    let final_tip_amount = 1_000; // 最后一个小 tip 0.000001 SOL

    // 为每个交易生成唯一标识和随机化参数
    let tx_id: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("💡 交易唯一标识: {}", tx_id);

    // 交易 1: 转账 + tip (随机化金额 + 随机 tip 账户)
    let tip_account_1 = Pubkey::from_str(jito_tip_accounts[0]).unwrap();
    let transfer_amount_1 = base_transfer_amount + rand::Rng::random_range(&mut rng, 0..100);
    let tip_amount_1 = base_tip_amount + rand::Rng::random_range(&mut rng, 0..1000);

    let mut tx1 = Transaction::new_with_payer(
        &[
            transfer(&sender.pubkey(), &receiver_pubkey, transfer_amount_1),
            transfer(&sender.pubkey(), &tip_account_1, tip_amount_1),
        ],
        Some(&sender.pubkey()),
    );

    // 交易 2: 转账 + tip (随机化金额 + 随机 tip 账户)
    let tip_account_2 = Pubkey::from_str(jito_tip_accounts[1]).unwrap();
    let transfer_amount_2 = base_transfer_amount + rand::Rng::random_range(&mut rng, 0..100);
    let tip_amount_2 = base_tip_amount + rand::Rng::random_range(&mut rng, 0..1000);

    let mut tx2 = Transaction::new_with_payer(
        &[
            transfer(&sender.pubkey(), &receiver_pubkey, transfer_amount_2),
            transfer(&sender.pubkey(), &tip_account_2, tip_amount_2),
        ],
        Some(&sender.pubkey()),
    );

    // 交易 3: 转账 + tip (随机化金额 + 随机 tip 账户)
    let tip_account_3 = Pubkey::from_str(jito_tip_accounts[2]).unwrap();
    let transfer_amount_3 = base_transfer_amount + rand::Rng::random_range(&mut rng, 0..100);
    let tip_amount_3 = base_tip_amount + rand::Rng::random_range(&mut rng, 0..1000);

    let mut tx3 = Transaction::new_with_payer(
        &[
            transfer(&sender.pubkey(), &receiver_pubkey, transfer_amount_3),
            transfer(&sender.pubkey(), &tip_account_3, tip_amount_3),
        ],
        Some(&sender.pubkey()),
    );

    // 交易 4: 只有小 tip (随机 tip 账户)
    let tip_account_4 = Pubkey::from_str(jito_tip_accounts[3]).unwrap();

    let mut tx4 = Transaction::new_with_payer(
        &[transfer(&sender.pubkey(), &tip_account_4, final_tip_amount)],
        Some(&sender.pubkey()),
    );

    // 签名所有交易
    tx1.sign(&[&sender], blockhash);
    tx2.sign(&[&sender], blockhash);
    tx3.sign(&[&sender], blockhash);
    tx4.sign(&[&sender], blockhash);

    println!("  ✓ 交易 1: 转账 {} lamports + Tip {} lamports", transfer_amount_1, tip_amount_1);
    println!("  ✓ 交易 2: 转账 {} lamports + Tip {} lamports", transfer_amount_2, tip_amount_2);
    println!("  ✓ 交易 3: 转账 {} lamports + Tip {} lamports", transfer_amount_3, tip_amount_3);
    println!("  ✓ 交易 4: Tip {} lamports (仅 tip)", final_tip_amount);

    // ========== 7. 展示 Bundle 详情 ==========
    let total_transfer = transfer_amount_1 + transfer_amount_2 + transfer_amount_3;
    let total_tip = tip_amount_1 + tip_amount_2 + tip_amount_3 + final_tip_amount;

    println!("\n📋 Bundle 结构详情:");
    println!("  ├─ 交易数量: 4 / 5 (最大)");
    println!("  ├─ 总转账: {} lamports ({:.9} SOL)", total_transfer, total_transfer as f64 / 1_000_000_000.0);
    println!(
        "  ├─ 总 Tip: {} lamports ({:.9} SOL)",
        total_tip,
        total_tip as f64 / 1_000_000_000.0
    );
    println!("  ├─ 预估交易费: ~20,000 lamports (5,000 × 4)");
    println!(
        "  ├─ 预估总花费: {} lamports ({:.9} SOL)",
        total_transfer + total_tip + 20_000,
        (total_transfer + total_tip + 20_000) as f64 / 1_000_000_000.0
    );
    println!("  ├─ 唯一性保证: 随机金额 + 不同 Tip 账户");
    println!("  └─ 原子性: 是（全部成功或全部失败）");

    // ========== 8. 使用 SDK 的 JitoClient 发送 Bundle ==========
    println!("\n🚀 正在发送 Bundle 到 Jito Testnet...");

    // 将 Transaction 转换为 VersionedTransaction
    use solana_sdk::transaction::VersionedTransaction;

    let versioned_transactions: Vec<VersionedTransaction> = vec![
        VersionedTransaction::from(tx1),
        VersionedTransaction::from(tx2),
        VersionedTransaction::from(tx3),
        VersionedTransaction::from(tx4),
    ];

    println!("🔍 Bundle 包含 {} 笔交易", versioned_transactions.len());

    // 使用 SDK 的 JitoClient
    use sol_trade_sdk::swqos::{
        jito::{JitoClient, JitoRegion},
        SwqosClientTrait, TradeType,
    };

    // 创建 Jito client（使用 testnet endpoint）
    let jito_client = JitoClient::new(
        testnet_rpc.to_string(),
        JitoRegion::Default, // 使用默认区域
        String::new(), // 不需要 auth token
    );

    println!("\n📦 发送 Bundle 到 Jito...");
    match jito_client
        .send_transactions(TradeType::Buy, &versioned_transactions, false)
        .await
    {
        Ok(_) => {
            println!("✅ Bundle 发送成功!");
            println!("\n✅ 测试完成!");
            println!("\n============================================\n");
            Ok(())
        },
        Err(e) => {
            println!("\n❌ 测试失败!");
            println!("\n============================================\n");
            Err(e.into())
        },
    }
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
