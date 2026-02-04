//! 通过代理访问 Solana RPC 的工具函数
//!
//! 提供通过 HTTP 代理访问 Solana RPC 的辅助函数

use reqwest::Client;
use serde_json::{json, Value};

/// 通过代理获取最新的 blockhash
///
/// # 参数
/// - `rpc_url`: Solana RPC URL
/// - `proxy_url`: 代理 URL (可选)
///
/// # 返回
/// 返回最新的 blockhash 字符串
pub async fn get_latest_blockhash_with_proxy(
    rpc_url: &str,
    proxy_url: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = if let Some(proxy) = proxy_url {
        let proxy = reqwest::Proxy::all(proxy)?;
        Client::builder().proxy(proxy).build()?
    } else {
        Client::new()
    };

    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestBlockhash",
        "params": []
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let json: Value = response.json().await?;

    if let Some(result) = json.get("result") {
        if let Some(value) = result.get("value") {
            if let Some(blockhash) = value.get("blockhash").and_then(|v| v.as_str()) {
                return Ok(blockhash.to_string());
            }
        }
    }

    if let Some(error) = json.get("error") {
        return Err(format!("RPC error: {}", error).into());
    }

    Err("Failed to get blockhash".into())
}

/// 通过代理获取 SOL 余额
///
/// # 参数
/// - `rpc_url`: Solana RPC URL
/// - `proxy_url`: 代理 URL (可选)
/// - `pubkey`: 公钥字符串
///
/// # 返回
/// 返回余额 (lamports)
pub async fn get_solana_balance_with_proxy(
    rpc_url: &str,
    proxy_url: Option<&str>,
    pubkey: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let client = if let Some(proxy) = proxy_url {
        let proxy = reqwest::Proxy::all(proxy)?;
        Client::builder().proxy(proxy).build()?
    } else {
        Client::new()
    };

    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [pubkey]
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await?;

    let json: Value = response.json().await?;

    if let Some(result) = json.get("result") {
        if let Some(value) = result.get("value") {
            if let Some(lamports) = value.as_u64() {
                return Ok(lamports);
            }
        }
    }

    if let Some(error) = json.get("error") {
        return Err(format!("RPC error: {}", error).into());
    }

    Err("Failed to get balance".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_get_blockhash() {
        let result = get_latest_blockhash_with_proxy("https://api.testnet.solana.com", None).await;
        assert!(result.is_ok());
        println!("Blockhash: {}", result.unwrap());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_balance() {
        let pubkey = "8be6dbPmZH1URHXyFTbY876QuVunrD8wTZhHGXjEdrvj";
        let result = get_solana_balance_with_proxy("https://api.testnet.solana.com", None, pubkey).await;
        assert!(result.is_ok());
        println!("Balance: {} lamports", result.unwrap());
    }
}
