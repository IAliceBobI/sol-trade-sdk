//! Jito Bundle 模拟测试
//!
//! 这个测试使用 Jito 的 `simulateBundle` API 来模拟 bundle 执行
//! 可以在实际发送前发现潜在问题：
//! - 账户余额不足
//! - 指令参数错误
//! - Program 执行失败
//! - Compute Unit 不足
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
//! cargo nextest run --test jito_simulate_bundle -- test_simulate_bundle --exact --nocapture --ignored
//! ```
//!
//! ## 📚 相关文档
//! - [Jito Bundle Simulation](https://docs.jito.wtf/lowlatencytxsend/)

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

/// 模拟 Bundle 执行（在发送前验证）
#[tokio::test]
#[serial_test::serial]
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

    let jito_tip_accounts = [
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
