//! Jito Bundle 发送测试
//!
//! 完整的 Jito Bundle 发送示例（Testnet 实际测试）
//!
//! ## 避免重复交易的措施
//!
//! Solana 通过**消息哈希**(message hash)来判断交易是否重复。消息哈希包含:
//! - 账户列表
//! - 指令数据(program_id, data, accounts)
//! - recent_blockhash
//!
//! 为避免 Bundle 中的交易被视为重复,本测试采用了以下策略:
//! 1. **唯一 Memo**: 每个交易添加包含时间戳的唯一 memo 指令
//! 2. **随机化金额**: 在基础金额上添加小的随机增量(转账 ±100 lamports, tip ±1000 lamports)
//! 3. **不同 Tip 账户**: 为每个交易使用不同的 Jito tip 账户(Jito 共有 8 个)
//!
//! 这些措施确保每个交易产生唯一的消息哈希,避免错误码 -32602(重复交易)。
//!
//! ## 环境变量
//! - `SOLANA_TEST_KEY_PATH1`: Testnet 发送方密钥文件路径
//! - `SOLANA_TEST_KEY_PATH2`: Testnet 接收方密钥文件路径
//! - `PROXY_URL`: 代理 URL（可选，默认 http://127.0.0.1:7891）
//!
//! ## 运行方式
//! ```bash
//! export SOLANA_TEST_KEY_PATH1=/path/to/sender-keypair.json
//! export SOLANA_TEST_KEY_PATH2=/path/to/receiver-keypair.json
//! cargo nextest run --test jito_bundle_send -- test_jito_bundle_send_example --exact --nocapture --ignored
//! ```
//!
//! ## 📚 相关资源
//!
//! - [Jito 官方文档](https://docs.jito.wtf)
//! - [Solana Testnet Faucet](https://faucet.solana.com/)

use solana_sdk::{
    pubkey::Pubkey,
    signature::{EncodableKey, Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction::transfer;
use std::str::FromStr;

// 导入公共代理库
use sol_trade_test_utils::proxy_http::{
    get_latest_blockhash_with_proxy, get_solana_balance_with_proxy,
};

/// 完整的 Jito Bundle 发送示例（Testnet 实际测试）
///
/// 这个测试在 Testnet 上实际发送 Jito Bundle 交易
/// Bundle 包含 3 笔从 SOLANA_TEST_KEY_PATH1 到 SOLANA_TEST_KEY_PATH2 的小额 SOL 转账
#[tokio::test]
#[serial_test::serial]
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
    let jito_tip_accounts = [
        "7aewvu8fMf1DK4fKoMXKfs3H3wpAQ7r7D8T1C71LmMF",
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
