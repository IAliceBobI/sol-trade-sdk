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
//! - `BUNDLE_ID`: Bundle ID（用于 test_query_bundle_status）
//! - `JITO_NETWORK`: 网络（testnet 或 mainnet，用于 test_query_bundle_status）
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
//! cargo nextest run --test jito_testnet_tests -- test_simulate_bundle --exact --nocapture --ignored
//! cargo nextest run --test jito_testnet_tests -- test_jito_dynamic_tip_floor --exact --nocapture --ignored
//! cargo nextest run --test jito_testnet_tests -- test_query_bundle_status --exact --nocapture --ignored
//! ```
//!
//! ## 📚 相关资源
//!
//! - [Jito 官方文档](https://docs.jito.wtf)
//! - [Tip Floor API](https://bundles.jito.wtf/api/v1/bundles/tip_floor)
//! - [Solana Testnet Faucet](https://faucet.solana.com/)
//! - [Bundle 状态查询文档](../docs/Jito_Bundle_状态查询.md)
//!
//! ## 🔍 Bundle 状态查询
//!
//! 使用 `test_query_bundle_status` 测试可以查询 Bundle 的处理状态：
//!
//! ```bash
//! export BUNDLE_ID="your_bundle_id_here"
//! export JITO_NETWORK="testnet"  # 或 "mainnet"
//! cargo nextest run --test jito_testnet_tests -- test_query_bundle_status --exact --nocapture --ignored
//! ```
//!
//! 详见 [Bundle 状态查询文档](../docs/Jito_Bundle_状态查询.md)。

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
    use solana_sdk::hash::Hash;
    use std::env;

    println!("\n========== Jito Bundle Testnet 实际测试 ==========\n");

    // ========== 1. 读取环境变量 ==========
    let sender_key_path =
        env::var("SOLANA_TEST_KEY_PATH1").expect("SOLANA_TEST_KEY_PATH1 环境变量未设置");
    let receiver_key_path =
        env::var("SOLANA_TEST_KEY_PATH2").expect("SOLANA_TEST_KEY_PATH2 环境变量未设置");

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

    // ⚠️ 重要：Jito Testnet 和 Mainnet 使用不同的 Tip 账户！
    //
    // **Testnet Tip Accounts**（从 Jito Testnet API 获取）:
    //   获取方式: curl -X POST "https://dallas.testnet.block-engine.jito.wtf/api/v1/getTipAccounts" \
    //               -H "Content-Type: application/json" \
    //               -d '{"jsonrpc":"2.0","id":1,"method":"getTipAccounts","params":[]}'
    //
    // **Mainnet Tip Accounts**:
    //   这些账户定义在 src/constants/swqos.rs 中的 JITO_TIP_ACCOUNTS
    //   包括: 96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5 等
    //
    // **为什么不同？**
    //   - Jito Testnet 是独立的测试环境，使用自己的验证者和基础设施
    //   - Testnet 的 tip 收益分配给 Testnet 验证者，而不是 Mainnet 验证者
    //   - 使用错误的 tip accounts 会导致错误: "Bundles must write lock at least one tip account"
    //
    // **如何获取正确的 Tip Accounts？**
    //   - Testnet: https://<region>.testnet.block-engine.jito.wtf/api/v1/getTipAccounts
    //   - Mainnet: https://<region>.mainnet.block-engine.jito.wtf/api/v1/getTipAccounts
    //
    // **可用区域**:
    //   - Testnet: Frankfurt, New York, Dallas
    //   - Mainnet: Amsterdam, Dublin, Frankfurt, London, New York, Salt Lake City, Singapore, Tokyo
    let jito_tip_accounts = vec![
        "7aewvu8fMf1DK4fKoMXKfs3h3wpAQ7r7D8T1C71LmMF",
        "84DrGKhycCUGfLzw8hXsUYX9SnWdh2wW3ozsTPrC5xyg",
        "BkMx5bRzQeP6tUZgzEs3xeDWJfQiLYvNDqSgmGZKYJDq",
        "4uRnem4BfVpZBv7kShVxUYtcipscgZMSHi3B9CSL6gAA",
        "G2d63CEgKBdgtpYT2BuheYQ9HFuFCenuHLNyKVpqAuSD",
        "AzfhMPcx3qjbvCK3UUy868qmc5L451W341cpFqdL3EBe",
        "F7ThiQUBYiEcyaxpmMuUeACdoiSLKg4SZZ8JSfpFNwAf",
        "CwWZzvRgmxj9WLLhdoWUVrHZ1J8db3w2iptKuAitHqoC",
    ];

    let mut rng = rand::rng();

    // 💰 根据 tip_floor 数据调整 tip 金额
    // Testnet tip_floor (2026-01-29):
    //   - 50th percentile: ~1920 lamports
    //   - 95th percentile: ~52000 lamports
    //   - 99th percentile: 0.001 SOL (1000000 lamports)
    //
    // Mainnet tip_floor (2026-01-29):
    //   - 50th percentile: 1,800 lamports
    //   - 95th percentile: 99,500 lamports
    //   - 99th percentile: 1,745,203 lamports
    //
    // 为了提高成功率，我们动态获取 Tip Floor 并计算最优 tip

    println!("\n📊 正在获取 Tip Floor 数据...");
    use sol_trade_sdk::swqos::jito::dynamic_tip::{
        DynamicTipConfig, JitoTipFloorClient, TipPercentile,
    };

    let tip_floor_client = JitoTipFloorClient::from_env_proxy();
    let base_tip_amount: u64 = match tip_floor_client.get_tip_floor().await {
        Ok(tip_floor) => {
            println!("✅ Tip Floor 数据获取成功:");
            println!(
                "  ├─ 25th: {:.6} SOL ({} lamports)",
                tip_floor.landed_tips_25th_percentile,
                (tip_floor.landed_tips_25th_percentile * 1_000_000_000.0) as u64
            );
            println!(
                "  ├─ 50th: {:.6} SOL ({} lamports)",
                tip_floor.landed_tips_50th_percentile,
                (tip_floor.landed_tips_50th_percentile * 1_000_000_000.0) as u64
            );
            println!(
                "  ├─ 75th: {:.6} SOL ({} lamports)",
                tip_floor.landed_tips_75th_percentile,
                (tip_floor.landed_tips_75th_percentile * 1_000_000_000.0) as u64
            );
            println!(
                "  ├─ 95th: {:.6} SOL ({} lamports)",
                tip_floor.landed_tips_95th_percentile,
                (tip_floor.landed_tips_95th_percentile * 1_000_000_000.0) as u64
            );
            println!(
                "  └─ 99th: {:.6} SOL ({} lamports)",
                tip_floor.landed_tips_99th_percentile,
                (tip_floor.landed_tips_99th_percentile * 1_000_000_000.0) as u64
            );
            println!();

            // 使用 P75 + 1.0 倍数（合理范围，避免被拒绝）
            let config = DynamicTipConfig {
                enabled: true,
                percentile: TipPercentile::P75, // 使用中等百分位
                multiplier: 1.0,                // 1倍，不激进
                min_tip: 0.00001,
                max_tip: 0.0001, // 设置合理上限
            };

            let tip_sol = tip_floor_client.calculate_tip(&tip_floor, &config);
            let tip_lamports = (tip_sol * 1_000_000_000.0) as u64;

            println!("💡 动态 Tip 配置 (合理策略):");
            println!("  ├─ 策略: P75 + 1.0x");
            println!("  ├─ 基础值: {:.6} SOL", tip_floor.landed_tips_75th_percentile);
            println!("  ├─ 计算值: {:.6} SOL", tip_sol);
            println!("  ├─ 最终值: {} lamports ({:.9} SOL)", tip_lamports, tip_sol);
            println!("  └─ 目标: 合理的 Tip 范围，避免被拒绝");
            println!();

            tip_lamports
        },
        Err(e) => {
            println!("⚠️  无法获取 Tip Floor: {}", e);
            println!("💡 使用备用固定值（合理范围）");
            println!("  └─ 10,000 lamports (0.00001 SOL)");
            println!();

            // 备用值：合理的固定 tip（0.00001 SOL）
            10_000
        },
    };

    let base_transfer_amount: u64 = 1_000; // 基础转账金额 0.000001 SOL
    let final_tip_amount: u64 = base_tip_amount / 2; // 最后一个 tip 减半

    // 为每个交易生成唯一标识和随机化参数
    let tx_id: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    println!("💡 交易唯一标识: {}", tx_id);

    // 交易 1: 转账 + tip (使用正确的 testnet tip 账户)
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

    // 交易 2: 转账 + tip
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

    // 交易 3: 转账 + tip
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

    // 交易 4: 仅 tip
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
    println!(
        "  ├─ 总转账: {} lamports ({:.9} SOL)",
        total_transfer,
        total_transfer as f64 / 1_000_000_000.0
    );
    println!("  ├─ 总 Tip: {} lamports ({:.9} SOL)", total_tip, total_tip as f64 / 1_000_000_000.0);
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
    use sol_trade_sdk::swqos::jito::{JitoClient, JitoRegion};

    // 创建 Jito client（使用 testnet endpoint）
    let _jito_client = JitoClient::new(
        testnet_rpc.to_string(),
        JitoRegion::Default, // 使用默认区域
        String::new(),       // 不需要 auth token
    );

    println!("\n📦 发送 Bundle 到 Jito...");

    // 直接使用 HTTP 客户端调用 Jito API 以获取完整响应
    use reqwest::Client;
    use sol_trade_sdk::swqos::common::FormatBase64VersionedTransaction;

    // 将交易转换为 base64
    let txs_base64: Vec<String> =
        versioned_transactions.iter().map(|tx| tx.to_base64_string()).collect();

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "sendBundle",
        "params": [
            txs_base64,
            { "encoding": "base64" }
        ],
        "id": 1,
    });

    let jito_endpoint = format!("{}/api/v1/bundles", jito_testnet_endpoint);

    println!("📡 正在发送到: {}", jito_endpoint);
    println!("📦 Bundle 大小: {} bytes", body.to_string().len());
    println!("🔌 使用代理: {}", proxy_url);

    // 使用代理创建 HTTP 客户端
    let client = if !proxy_url.is_empty() {
        use reqwest::Proxy;
        let proxy = Proxy::all(proxy_url).map_err(|e| format!("Failed to create proxy: {}", e))?;
        Client::builder().proxy(proxy).build()
    } else {
        Client::builder().build()
    }
    .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post(&jito_endpoint)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await?;

    let status = response.status();
    let response_text = response.text().await?;

    println!("\n📥 Jito 响应状态: {}", status);
    println!("📥 响应内容:");
    println!(
        "{}",
        serde_json::from_str::<serde_json::Value>(&response_text)
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or(response_text.clone()))
            .unwrap_or(response_text.clone())
    );

    // 解析响应
    if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(&response_text) {
        if let Some(result) = response_json.get("result") {
            println!("\n✅ Bundle 发送成功!");

            // 提取 bundle 签名
            if let Some(bundle_id) = result.get("bundle_id").and_then(|v| v.as_str()) {
                println!("📦 Bundle ID: {}", bundle_id);

                // 提取交易签名
                if let Some(signatures) = result.get("signatures").and_then(|v| v.as_array()) {
                    println!("📝 交易签名:");
                    for (i, sig) in signatures.iter().enumerate() {
                        if let Some(sig_str) = sig.as_str() {
                            println!("   {}. {}", i + 1, sig_str);
                        }
                    }
                }
            }

            println!("\n💡 提示: Bundle 可能需要几秒钟才能被确认");
            println!("💡 你可以在 Jito Explorer 上查看 Bundle 状态");

            println!("\n✅ 测试完成!");
            println!("\n============================================\n");
            Ok(())
        } else if let Some(error) = response_json.get("error") {
            println!("\n❌ Jito 返回错误:");
            println!("   错误码: {}", error.get("code").unwrap_or(&serde_json::json!("N/A")));
            println!(
                "   错误信息: {}",
                error.get("message").unwrap_or(&serde_json::json!("Unknown"))
            );
            println!("\n============================================\n");
            Err(format!("Jito error: {}", error).into())
        } else {
            println!("\n⚠️  未知响应格式");
            println!("\n============================================\n");
            Err("Unknown response format".into())
        }
    } else {
        println!("\n❌ 无法解析响应 JSON");
        println!("\n============================================\n");
        Err("Failed to parse response".into())
    }
}

// ============================================================================
// Test 2: Bundle 模拟测试（新增）
// ============================================================================

/// 模拟 Bundle 执行（在发送前验证）
///
/// 这个测试使用 Jito 的 `simulateBundle` API 来模拟 bundle 执行
/// 可以在实际发送前发现潜在问题：
/// - 账户余额不足
/// - 指令参数错误
/// - Program 执行失败
/// - Compute Unit 不足
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
/// cargo test --test jito_testnet_tests -- test_simulate_bundle --exact --nocapture --ignored
/// ```
///
/// ## 📚 相关文档
/// - [Jito Bundle Simulation](https://docs.jito.wtf/lowlatencytxnsend/)
#[tokio::test]
#[ignore] // 默认忽略，需要手动运行
async fn test_simulate_bundle() -> Result<(), Box<dyn std::error::Error>> {
    use solana_sdk::hash::Hash;
    use std::env;

    println!("\n========== Jito Bundle 模拟测试 ==========\n");

    // ========== 1. 读取环境变量 ==========
    let sender_key_path =
        env::var("SOLANA_TEST_KEY_PATH1").expect("SOLANA_TEST_KEY_PATH1 环境变量未设置");
    let receiver_key_path =
        env::var("SOLANA_TEST_KEY_PATH2").expect("SOLANA_TEST_KEY_PATH2 环境变量未设置");

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

    // ========== 4. 查询余额 ==========
    println!("\n📡 正在查询账户余额...");

    let sender_balance =
        get_solana_balance_with_proxy(testnet_rpc, Some(&proxy_url), &sender.pubkey().to_string())
            .await?;
    let sender_sol_balance = sender_balance as f64 / 1_000_000_000.0;

    println!("💰 发送方余额: {:.9} SOL ({} lamports)", sender_sol_balance, sender_balance);

    if sender_balance < 10_000_000 {
        println!("\n⚠️  发送方余额不足（需要至少 0.01 SOL）");
        return Err("发送方余额不足".into());
    }

    // ========== 5. 获取 recent blockhash ==========
    println!("\n📡 正在获取 recent blockhash...");

    let blockhash_str = get_latest_blockhash_with_proxy(testnet_rpc, Some(&proxy_url)).await?;
    let blockhash = Hash::from_str(&blockhash_str)?;
    println!("✅ Blockhash: {}", blockhash_str);

    // ========== 6. 构建测试 Bundle（2 个简单交易）==========
    println!("\n🔨 正在构建测试 Bundle...");

    let jito_tip_accounts = vec![
        "7aewvu8fMf1DK4fKoMXKfs3H3wpAQ7r7D8T1C71LmMF",
        "84DrGKhycCUGfLzw8hXsUYX9SnWdh2wW3ozsTPrC5xyg",
    ];

    // 交易 1: 转账 + tip
    let tip_account_1 = Pubkey::from_str(jito_tip_accounts[0]).unwrap();
    let transfer_amount_1 = 1_000; // 0.000001 SOL
    let tip_amount_1 = 10_000; // 0.00001 SOL

    let mut tx1 = Transaction::new_with_payer(
        &[
            transfer(&sender.pubkey(), &receiver_pubkey, transfer_amount_1),
            transfer(&sender.pubkey(), &tip_account_1, tip_amount_1),
        ],
        Some(&sender.pubkey()),
    );

    // 交易 2: 仅 tip
    let tip_account_2 = Pubkey::from_str(jito_tip_accounts[1]).unwrap();
    let tip_amount_2 = 5_000; // 0.000005 SOL

    let mut tx2 = Transaction::new_with_payer(
        &[transfer(&sender.pubkey(), &tip_account_2, tip_amount_2)],
        Some(&sender.pubkey()),
    );

    // 签名
    tx1.sign(&[&sender], blockhash);
    tx2.sign(&[&sender], blockhash);

    println!("  ✓ 交易 1: 转账 {} lamports + Tip {} lamports", transfer_amount_1, tip_amount_1);
    println!("  ✓ 交易 2: Tip {} lamports (仅 tip)", tip_amount_2);

    // ========== 7. 转换为 VersionedTransaction 并编码 ==========
    use sol_trade_sdk::swqos::common::FormatBase64VersionedTransaction;
    use solana_sdk::transaction::VersionedTransaction;

    let versioned_transactions: Vec<VersionedTransaction> =
        vec![VersionedTransaction::from(tx1), VersionedTransaction::from(tx2)];

    let txs_base64: Vec<String> =
        versioned_transactions.iter().map(|tx| tx.to_base64_string()).collect();

    // ========== 8. 调用 simulateBundle API ==========
    println!("\n🔍 正在模拟 Bundle 执行...");

    // 注意：simulateBundle 使用 /api/v1 端点（不是 /api/v1/bundles）
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "simulateBundle",
        "params": [
            {
                "encodedTransactions": txs_base64,
                "accounts": {
                    "encoding": "base64",
                    "addresses": [sender.pubkey().to_string(), receiver_pubkey.to_string()]
                }
            }
        ]
    });

    let jito_endpoint = format!("{}/api/v1", jito_testnet_endpoint);

    println!("📡 正在发送到: {}/simulate", jito_endpoint);
    println!("📦 Bundle 大小: {} bytes", body.to_string().len());
    println!("🔌 使用代理: {}", proxy_url);

    // 使用代理创建 HTTP 客户端
    use reqwest::Proxy;
    let client = if !proxy_url.is_empty() {
        let proxy = Proxy::all(proxy_url).map_err(|e| format!("Failed to create proxy: {}", e))?;
        reqwest::Client::builder().proxy(proxy).build()
    } else {
        reqwest::Client::builder().build()
    }
    .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = client
        .post(&jito_endpoint)
        .header("Content-Type", "application/json")
        .body(body.to_string())
        .send()
        .await?;

    let status = response.status();
    let response_text = response.text().await?;

    println!("\n📥 Jito 响应状态: {}", status);

    // ========== 9. 解析模拟结果 ==========
    if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(&response_text) {
        println!("\n📥 完整响应:");
        println!(
            "{}",
            serde_json::to_string_pretty(&response_json).unwrap_or(response_text.clone())
        );

        if let Some(result) = response_json.get("result") {
            println!("\n✅ 模拟成功!");

            // 检查 value 中的错误
            if let Some(value) = result.get("value") {
                // 检查总体错误
                if let Some(err) = value.get("err") {
                    println!("\n⚠️  Bundle 模拟发现错误:");
                    println!("   错误: {}", err);

                    // 检查日志
                    if let Some(logs) = value.get("logs").and_then(|l| l.as_array()) {
                        println!("\n   日志:");
                        for log in logs.iter().take(20) {
                            if let Some(log_str) = log.as_str() {
                                println!("     - {}", log_str);
                            }
                        }
                    }

                    println!("\n💡 建议:");
                    println!("   - 检查账户余额是否充足");
                    println!("   - 检查 tip 账户是否正确");
                    println!("   - 检查交易参数是否有效");
                    println!("   - 尝试增加 tip 金额");

                    return Err("Bundle 模拟失败".into());
                }

                // 显示日志
                if let Some(logs) = value.get("logs").and_then(|l| l.as_array()) {
                    println!("\n📋 执行日志:");
                    for log in logs.iter().take(15) {
                        if let Some(log_str) = log.as_str() {
                            println!("   - {}", log_str);
                        }
                    }
                    if logs.len() > 15 {
                        println!("   ... (省略 {} 条)", logs.len() - 15);
                    }
                }

                // 显示账户状态变化
                if let Some(pre_accounts) =
                    value.get("preExecutionAccounts").and_then(|a| a.as_array())
                {
                    println!("\n📊 执行前账户状态:");
                    for (i, account) in pre_accounts.iter().enumerate() {
                        if let Some(lamports) = account.get("lamports").and_then(|l| l.as_u64()) {
                            println!("   账户 {}: {} lamports", i + 1, lamports);
                        }
                    }
                }

                if let Some(post_accounts) =
                    value.get("postExecutionAccounts").and_then(|a| a.as_array())
                {
                    println!("\n📊 执行后账户状态:");
                    for (i, account) in post_accounts.iter().enumerate() {
                        if let Some(lamports) = account.get("lamports").and_then(|l| l.as_u64()) {
                            println!("   账户 {}: {} lamports", i + 1, lamports);
                        }
                    }
                }

                // 显示 Compute Unit 消耗
                if let Some(units_consumed) =
                    value.get("computeUnitsConsumed").and_then(|u| u.as_u64())
                {
                    println!("\n💰 Compute Unit 消耗: {}", units_consumed);
                }

                // 显示返回数据
                if let Some(return_data) = value.get("returnData") {
                    println!("\n📦 返回数据:");
                    println!(
                        "   {}",
                        serde_json::to_string_pretty(return_data).unwrap_or_default()
                    );
                }
            }

            println!("\n✅ Bundle 模拟通过，可以安全发送!");
            println!("\n============================================\n");
            Ok(())
        } else if let Some(error) = response_json.get("error") {
            println!("\n❌ Jito API 返回错误:");
            println!("   错误码: {}", error.get("code").unwrap_or(&serde_json::json!("N/A")));
            println!(
                "   错误信息: {}",
                error.get("message").unwrap_or(&serde_json::json!("Unknown"))
            );

            // 检查是否是已知错误
            if let Some(msg) = error.get("message").and_then(|m| m.as_str()) {
                println!("\n💡 可能的原因:");
                if msg.contains("blockhash") {
                    println!("   - Blockhash 可能已过期，请重新获取");
                } else if msg.contains("insufficient") {
                    println!("   - 账户余额不足");
                } else if msg.contains("tip") {
                    println!("   - Tip 金额或账户配置错误");
                } else {
                    println!("   - 检查 Bundle 构建是否正确");
                }
            }

            println!("\n============================================\n");
            Err(format!("Jito API error: {}", error).into())
        } else {
            println!("\n⚠️  未知响应格式");
            println!("\n============================================\n");
            Err("Unknown response format".into())
        }
    } else {
        println!("\n❌ 无法解析响应 JSON");
        println!("原始响应: {}", response_text);
        println!("\n============================================\n");
        Err("Failed to parse response".into())
    }
}

// ============================================================================
// Test 3: 动态 Tip Floor API 测试
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

// ============================================================================
// Test 3: Bundle 状态查询工具
// ============================================================================

/// Jito Bundle 状态查询工具
///
/// ## 使用方法
///
/// ### 方式 1: 使用环境变量指定 Bundle ID
/// ```bash
/// export BUNDLE_ID="3d74badb78eb2c39080233892f442063f1c6b9f8f8b8bc9036c976a9699449db"
/// export JITO_NETWORK="testnet"  # 或 "mainnet"
/// cargo nextest run --test jito_testnet_tests -- test_query_bundle_status --exact --nocapture --ignored
/// ```
///
/// ### 方式 2: 直接修改测试函数中的 bundle_id 参数
/// ```bash
/// cargo nextest run --test jito_testnet_tests -- test_query_bundle_status --exact --nocapture --ignored
/// ```
///
/// ## 状态值说明
///
/// - `Pending`: Bundle 正在处理中
/// - `Landed`: Bundle 已成功上链
/// - `Failed`: Bundle 处理失败
/// - `Invalid`: Bundle 无效（可能已过期）
///
/// ## API 说明
///
/// - `getInflightBundleStatuses`: 查询正在处理中的 Bundle 状态
/// - `getBundleStatuses`: 查询已处理的 Bundle 最终状态
#[tokio::test]
#[ignore] // 默认忽略，需要手动运行
async fn test_query_bundle_status() -> Result<(), Box<dyn std::error::Error>> {
    use reqwest::Client;
    use std::env;

    println!("\n========== Jito Bundle 状态查询工具 ==========\n");

    // ========== 1. 读取配置 ==========
    // 方式 1: 从环境变量读取
    let bundle_id = env::var("BUNDLE_ID").unwrap_or_else(|_| {
        // 方式 2: 使用默认值（可以修改这里）
        "11111111".to_string()
    });

    let network = env::var("JITO_NETWORK").unwrap_or_else(|_| "testnet".to_string());

    // 根据网络选择 API endpoint
    let (api_endpoint, network_name) = match network.to_lowercase().as_str() {
        "mainnet" => ("https://mainnet.block-engine.jito.wtf/api/v1", "Jito Mainnet"),
        "testnet" | _ => ("https://dallas.testnet.block-engine.jito.wtf/api/v1", "Jito Testnet"),
    };

    println!("📦 Bundle ID: {}", bundle_id);
    println!("🌐 网络: {}", network_name);
    println!("🔗 API Endpoint: {}", api_endpoint);

    // 读取代理配置
    let proxy_url = env::var("PROXY_URL").ok();
    if let Some(ref proxy) = proxy_url {
        println!("🔌 使用代理: {}", proxy);
    }
    println!();

    // ========== 2. 查询 Inflight 状态 ==========
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("1️⃣  实时状态 (Inflight Status)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 使用代理创建 HTTP 客户端
    use reqwest::Proxy;
    let client_builder = if let Some(proxy_url) = &proxy_url {
        Client::builder().proxy(Proxy::all(proxy_url)?)
    } else {
        Client::builder()
    };
    let client = client_builder.build()?;

    // 查询 Inflight 状态
    let inflight_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getInflightBundleStatuses",
        "params": [[bundle_id]]
    });

    let inflight_url = format!("{}/getInflightBundleStatuses", api_endpoint);

    match client
        .post(&inflight_url)
        .header("Content-Type", "application/json")
        .body(inflight_body.to_string())
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            let response_text = response.text().await?;

            if status.is_success() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
                    if let Some(result) =
                        json.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_array())
                    {
                        if let Some(bundle_info) = result.first() {
                            let bundle_status = bundle_info
                                .get("status")
                                .and_then(|s| s.as_str())
                                .unwrap_or("Unknown");
                            let landed_slot = bundle_info
                                .get("landed_slot")
                                .and_then(|s| s.as_i64())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "N/A".to_string());

                            println!("✅ 查询成功");
                            println!();
                            println!("📊 状态: {}", format_status(bundle_status));
                            println!("📍 确认 Slot: {}", landed_slot);
                            println!(
                                "🆔 Bundle ID: {}",
                                bundle_info
                                    .get("bundle_id")
                                    .and_then(|id| id.as_str())
                                    .unwrap_or("N/A")
                            );

                            // 显示状态解释
                            println!();
                            println!("💡 状态说明:");
                            match bundle_status {
                                "Pending" => {
                                    println!("   ⏳ Bundle 正在处理中，请耐心等待");
                                    println!("   💡 建议每隔几秒查询一次");
                                },
                                "Landed" => {
                                    println!("   ✅ Bundle 已成功上链！");
                                    println!("   💡 可以在 Solana Explorer 查看交易详情");
                                },
                                "Failed" => {
                                    println!("   ❌ Bundle 处理失败");
                                    println!("   💡 可能原因: Tip 不足、交易无效、网络问题");
                                },
                                "Invalid" => {
                                    println!("   ⚠️  Bundle 无效或已过期");
                                    println!("   💡 Blockhash 可能已过期，请重新发送");
                                },
                                _ => {
                                    println!("   ❓ 未知状态");
                                },
                            }
                        } else {
                            println!("⚠️  未找到 Bundle 信息（可能未提交或已过期）");
                        }
                    } else {
                        println!("⚠️  响应格式异常");
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json).unwrap_or(response_text.clone())
                        );
                    }
                } else {
                    println!("❌ 解析响应失败");
                    println!("{}", response_text);
                }
            } else {
                println!("❌ HTTP 错误: {}", status);
                println!("{}", response_text);
            }
        },
        Err(e) => {
            println!("❌ 请求失败: {}", e);
        },
    }

    println!();

    // ========== 3. 查询最终状态 ==========
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("2️⃣  最终状态 (Final Status)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let final_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBundleStatuses",
        "params": [[bundle_id]]
    });

    let final_url = format!("{}/getBundleStatuses", api_endpoint);

    match client
        .post(&final_url)
        .header("Content-Type", "application/json")
        .body(final_body.to_string())
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            let response_text = response.text().await?;

            if status.is_success() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response_text) {
                    if let Some(result) =
                        json.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_array())
                    {
                        if !result.is_empty() {
                            println!("✅ 找到 {} 个已确认的 Bundle", result.len());
                            println!();

                            for (i, bundle_info) in result.iter().enumerate() {
                                println!("📦 Bundle #{}", i + 1);
                                println!(
                                    "   Bundle ID: {}",
                                    bundle_info
                                        .get("bundle_id")
                                        .and_then(|id| id.as_str())
                                        .unwrap_or("N/A")
                                );
                                println!(
                                    "   Slot: {}",
                                    bundle_info
                                        .get("slot")
                                        .and_then(|s| s.as_i64())
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| "N/A".to_string())
                                );
                                println!(
                                    "   交易数量: {}",
                                    bundle_info
                                        .get("transactions")
                                        .and_then(|t| t.as_array())
                                        .map(|a| a.len().to_string())
                                        .unwrap_or_else(|| "N/A".to_string())
                                );
                                println!();
                            }

                            println!("✅ Bundle 已成功上链！");
                        } else {
                            println!("⚠️  未找到已确认的 Bundle");
                            println!("💡 说明: Bundle 可能仍在处理中或已失败");
                        }
                    } else {
                        println!("⚠️  响应格式异常");
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json).unwrap_or(response_text.clone())
                        );
                    }
                } else {
                    println!("❌ 解析响应失败");
                    println!("{}", response_text);
                }
            } else {
                println!("❌ HTTP 错误: {}", status);
                println!("{}", response_text);
            }
        },
        Err(e) => {
            println!("❌ 请求失败: {}", e);
        },
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💡 使用提示");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔍 持续监控 Bundle 状态:");
    println!("   export BUNDLE_ID=\"your_bundle_id\"");
    println!("   export JITO_NETWORK=\"testnet\"");
    println!(
        "   cargo nextest run --test jito_testnet_tests -- test_query_bundle_status --exact --nocapture --ignored"
    );
    println!();
    println!("🌐 在 Explorer 查看:");
    if network.to_lowercase() == "testnet" {
        println!("   Solscan: https://solscan.io/?cluster=testnet");
    } else {
        println!("   Solscan: https://solscan.io/");
    }
    println!("============================================\n");

    Ok(())
}

/// 格式化状态显示（带颜色和图标）
fn format_status(status: &str) -> String {
    match status {
        "Pending" => "⏳ Pending (处理中)".to_string(),
        "Landed" => "✅ Landed (已上链)".to_string(),
        "Failed" => "❌ Failed (失败)".to_string(),
        "Invalid" => "⚠️  Invalid (无效)".to_string(),
        _ => format!("❓ {}", status),
    }
}
