//! HTTP 代理工具库
//!
//! 提供统一的 HTTP 代理客户端，用于测试环境中通过代理访问外部 API
//!
//! ## 使用示例
//!
//! ```rust
//! use tests::common::proxy_http::{ProxyHttpClient, create_proxy_client};
//!
//! // 使用默认代理地址
//! let client = create_proxy_client().unwrap();
//!
//! // 使用自定义代理地址
//! let client = ProxyHttpClient::new("http://127.0.0.1:7891").unwrap();
//!
//! // 发送请求
//! let response = client.post("https://api.example.com")
//!     .json(&request_body)
//!     .send()
//!     .await?;
//! ```

use reqwest::Proxy;
use std::env;

use serde::Deserialize;

/// 默认代理地址（从环境变量 PROXY_URL 读取，默认 http://127.0.0.1:7891）
const DEFAULT_PROXY_URL: &str = "http://127.0.0.1:7891";

/// 代理 HTTP 客户端
///
/// 封装了带有代理配置的 reqwest::Client
#[derive(Clone, Debug)]
pub struct ProxyHttpClient {
    client: reqwest::Client,
    proxy_url: String,
}

impl ProxyHttpClient {
    /// 创建新的代理客户端
    ///
    /// ## 参数
    /// - `proxy_url`: 代理地址（例如 "http://127.0.0.1:7891"）
    ///
    /// ## 返回
    /// 返回配置好的 ProxyHttpClient 实例
    pub fn new(proxy_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let proxy = Proxy::all(proxy_url)?;
        let client = reqwest::Client::builder().proxy(proxy).build()?;

        Ok(Self { client, proxy_url: proxy_url.to_string() })
    }

    /// 从环境变量创建代理客户端
    ///
    /// 读取 `PROXY_URL` 环境变量，如果未设置则使用默认地址
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let proxy_url = env::var("PROXY_URL").unwrap_or(DEFAULT_PROXY_URL.to_string());
        Self::new(&proxy_url)
    }

    /// 获取内部 reqwest::Client
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }

    /// 获取代理地址
    pub fn proxy_url(&self) -> &str {
        &self.proxy_url
    }
}

/// 创建代理客户端（便捷函数）
///
/// 使用环境变量 `PROXY_URL` 或默认地址
pub fn create_proxy_client() -> Result<ProxyHttpClient, Box<dyn std::error::Error>> {
    ProxyHttpClient::from_env()
}

/// 创建代理客户端（指定代理地址）
pub fn create_proxy_client_with_url(
    proxy_url: &str,
) -> Result<ProxyHttpClient, Box<dyn std::error::Error>> {
    ProxyHttpClient::new(proxy_url)
}

// ============================================================================
// Solana RPC 辅助函数
// ============================================================================

/// 通过代理查询 Solana 账户余额
///
/// ## 参数
/// - `rpc_url`: Solana RPC 端点
/// - `proxy_url`: 代理地址（如果为 None，使用环境变量或默认值）
/// - `address`: 要查询的地址
///
/// ## 返回
/// 返回余额（单位：lamports）
pub async fn get_solana_balance_with_proxy(
    rpc_url: &str,
    proxy_url: Option<&str>,
    address: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let client = match proxy_url {
        Some(url) => create_proxy_client_with_url(url)?,
        None => create_proxy_client()?,
    };

    // 使用 serde_json::json! 宏直接构造请求
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getBalance",
        "params": [address]
    });

    let response = client
        .client()
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    let rpc_response: RpcResponseBalance = response.json().await?;

    if let Some(error) = rpc_response.error {
        Err(format!("RPC 错误: {}", error.message).into())
    } else {
        Ok(rpc_response.result.value)
    }
}

/// 通过代理获取最新 blockhash
///
/// ## 参数
/// - `rpc_url`: Solana RPC 端点
/// - `proxy_url`: 代理地址（如果为 None，使用环境变量或默认值）
///
/// ## 返回
/// 返回最新的 blockhash（base58 编码）
pub async fn get_latest_blockhash_with_proxy(
    rpc_url: &str,
    proxy_url: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = match proxy_url {
        Some(url) => create_proxy_client_with_url(url)?,
        None => create_proxy_client()?,
    };

    // 使用 serde_json::json! 宏直接构造请求
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getLatestBlockhash",
        "params": [{"commitment": "confirmed"}]
    });

    let response = client
        .client()
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    let rpc_response: RpcResponseBlockhash = response.json().await?;

    if let Some(error) = rpc_response.error {
        Err(format!("RPC 错误: {}", error.message).into())
    } else {
        Ok(rpc_response.result.value.blockhash)
    }
}

/// 通过代理发送任意 RPC 请求
///
/// ## 参数
/// - `rpc_url`: Solana RPC 端点
/// - `proxy_url`: 代理地址（如果为 None，使用环境变量或默认值）
/// - `method`: RPC 方法名
/// - `params`: RPC 参数（JSON 值）
///
/// ## 返回
/// 返回 RPC 响应的 JSON 值
#[allow(dead_code)]
pub async fn send_rpc_request_with_proxy(
    rpc_url: &str,
    proxy_url: Option<&str>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = match proxy_url {
        Some(url) => create_proxy_client_with_url(url)?,
        None => create_proxy_client()?,
    };

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    let response = client
        .client()
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    let rpc_response: serde_json::Value = response.json().await?;

    if let Some(_error) = rpc_response.get("error") {
        Err(format!("RPC 请求失败: {}", rpc_response).into())
    } else {
        Ok(rpc_response)
    }
}

// ============================================================================
// RPC 响应数据结构
// ============================================================================

/// RPC 响应结构（getBalance）
#[derive(Deserialize)]
struct RpcResponseBalance {
    result: BalanceResult,
    error: Option<RpcError>,
}

/// RPC 响应结构（getLatestBlockhash）
#[derive(Deserialize)]
struct RpcResponseBlockhash {
    result: BlockhashResult,
    error: Option<RpcError>,
}

/// 余额结果
#[derive(Deserialize)]
struct BalanceResult {
    #[allow(dead_code)]
    context: Option<Context>,
    value: u64,
}

/// Blockhash 结果
#[derive(Deserialize)]
struct BlockhashResult {
    #[allow(dead_code)]
    context: Option<Context>,
    value: BlockhashValue,
}

/// 上下文
#[derive(Deserialize)]
struct Context {
    #[serde(default, rename = "apiVersion")]
    #[allow(dead_code)]
    api_version: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    slot: Option<u64>,
}

/// Blockhash 值
#[derive(Deserialize)]
struct BlockhashValue {
    blockhash: String,
    #[serde(default, rename = "lastValidBlockHeight")]
    #[allow(dead_code)]
    last_valid_block_height: Option<u64>,
}

/// RPC 错误
#[derive(Deserialize)]
struct RpcError {
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_proxy_client() {
        // 测试创建代理客户端
        let client = ProxyHttpClient::new("http://127.0.0.1:7891");
        assert!(client.is_ok());

        let client = client.unwrap();
        assert_eq!(client.proxy_url(), "http://127.0.0.1:7891");
    }

    #[test]
    fn test_create_proxy_client_from_env() {
        // 测试从环境变量创建
        unsafe {
            env::set_var("PROXY_URL", "http://127.0.0.1:9999");
        }
        let client = ProxyHttpClient::from_env();
        assert!(client.is_ok());
        assert_eq!(client.unwrap().proxy_url(), "http://127.0.0.1:9999");

        // 清理环境变量
        unsafe {
            env::remove_var("PROXY_URL");
        }
    }
}
