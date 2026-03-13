//! 空投相关功能

use reqwest::Client;
use serde_json::Value;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};

/// 为测试账户空投 SOL 并循环等待到账
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `payer` - 账户公钥
/// * `amount_sol` - 空投的 SOL 数量
///
/// # 返回
/// * `Ok(())` - 空投成功
/// * `Err(String)` - 空投失败
pub async fn airdrop_and_wait(
    rpc_url: &str,
    payer: &Pubkey,
    amount_sol: u64,
) -> Result<(), String> {
    let client = RpcClient::new(rpc_url.to_string());
    let amount_lamports = amount_sol * LAMPORTS_PER_SOL;

    // 尝试空投
    println!("💰 空投 {} SOL 到测试账户...", amount_sol);
    match client.request_airdrop(payer, amount_lamports).await {
        Ok(sig) => {
            println!("✅ 空投成功，签名: {}", sig);
            // 循环等待余额到账
            println!("⏳ 等待余额到账...");
            let mut retries = 0;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                match client.get_balance(payer).await {
                    Ok(balance) => {
                        if balance >= amount_lamports {
                            println!(
                                "✅ 余额已到账: {} lamports ({:.2} SOL)\n",
                                balance,
                                balance as f64 / 1_000_000_000.0
                            );
                            return Ok(());
                        }
                        retries += 1;
                        if retries > 20 {
                            return Err(format!("等待超时，当前余额: {} lamports", balance));
                        }
                    },
                    Err(e) => {
                        return Err(format!("查询余额失败: {}", e));
                    },
                }
            }
        },
        Err(e) => Err(format!("空投失败: {}", e)),
    }
}

/// 直接设置账户的 SOL 余额（使用 surfnet_setAccount）
///
/// 比空投更快，无需等待交易确认。仅适用于 surfpool 测试环境。
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `pubkey` - 账户公钥
/// * `lamports` - 目标余额（lamports）
///
/// # 返回
/// * `Ok(())` - 设置成功
/// * `Err(String)` - 设置失败
///
/// # 示例
/// ```ignore
/// // 设置账户余额为 10 SOL
/// set_sol_balance("http://127.0.0.1:8899", &pubkey, 10_000_000_000).await?;
/// ```
pub async fn set_sol_balance(rpc_url: &str, pubkey: &Pubkey, lamports: u64) -> Result<(), String> {
    let http_client = Client::new();

    // 构造 surfnet_setAccount RPC 请求
    let request_body = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"surfnet_setAccount","params":["{}",{{"lamports":{} }}]}}"#,
        pubkey, lamports
    );

    println!(
        "💰 直接设置 SOL 余额: {} lamports ({:.2} SOL)",
        lamports,
        lamports as f64 / 1_000_000_000.0
    );

    let response = http_client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .body(request_body)
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败: {}", e))?;

    let response_text = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;
    let response_json: Value =
        serde_json::from_str(&response_text).map_err(|e| format!("解析 JSON 失败: {}", e))?;

    if let Some(error) = response_json.get("error") {
        return Err(format!("RPC 错误: {}", error));
    }

    println!("   ✅ SOL 余额设置成功\n");
    Ok(())
}
