//! Jito Bundle 状态查询工具
//!
//! ## 使用方法
//!
//! ### 方式 1: 使用环境变量指定 Bundle ID
//! ```bash
//! export BUNDLE_ID="3d74badb78eb2c39080233892f442063f1c6b9f8f8b8bc9036c976a9699449db"
//! export JITO_NETWORK="testnet"  # 或 "mainnet"
//! cargo nextest run --test jito_query_bundle_status -- test_query_bundle_status --exact --nocapture --ignored
//! ```
//!
//! ### 方式 2: 直接修改测试函数中的 bundle_id 参数
//! ```bash
//! cargo nextest run --test jito_query_bundle_status -- test_query_bundle_status --exact --nocapture --ignored
//! ```
//!
//! ## 状态值说明
//!
//! - `Pending`: Bundle 正在处理中
//! - `Landed`: Bundle 已成功上链
//! - `Failed`: Bundle 处理失败
//! - `Invalid`: Bundle 无效（可能已过期）
//!
//! ## API 说明
//!
//! - `getInflightBundleStatuses`: 查询正在处理中的 Bundle 状态
//! - `getBundleStatuses`: 查询已处理的 Bundle 最终状态
//!
//! ## 📚 相关资源
//! - [Jito Bundle 状态查询文档](../docs/Jito_Bundle_状态查询.md)

use std::env;

/// Jito Bundle 状态查询工具
#[tokio::test]
#[serial_test::serial]
#[ignore] // 默认忽略，需要手动运行
async fn test_query_bundle_status() -> Result<(), Box<dyn std::error::Error>> {
    use reqwest::Client;

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
        "testnet" => ("https://dallas.testnet.block-engine.jito.wtf/api/v1", "Jito Testnet"),
        _ => ("https://dallas.testnet.block-engine.jito.wtf/api/v1", "Jito Testnet"),
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
        "   cargo nextest run --test jito_query_bundle_status -- test_query_bundle_status --exact --nocapture --ignored"
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
