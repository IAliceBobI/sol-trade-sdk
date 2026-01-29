//! Testnet 密钥和余额查询测试
//!
//! 测试是否能正确读取 SOLANA_TEST_KEY_PATH 环境变量指向的密钥文件
//! 并通过代理查询该地址在 testnet 上的余额

use serde::{Deserialize, Serialize};
use solana_sdk::signature::{EncodableKey, Keypair, Signer};
use std::env;

#[tokio::test]
#[ignore]
async fn test_read_key_and_check_balance() {
    println!("\n========== Testnet 密钥和余额查询测试 ==========\n");

    // 读取环境变量
    let key_path = env::var("SOLANA_TEST_KEY_PATH").expect("SOLANA_TEST_KEY_PATH 环境变量未设置");

    println!("📁 密钥路径: {}", key_path);

    // 读取密钥
    let keypair = Keypair::read_from_file(&key_path).expect("无法读取密钥文件");

    println!("📍 地址: {}", keypair.pubkey());

    // 连接到 testnet RPC（通过代理）
    let rpc_url = "https://api.testnet.solana.com";
    let proxy_url = "http://127.0.0.1:7891";

    println!("\n🌐 RPC 端点: {}", rpc_url);
    println!("🔌 代理地址: {}", proxy_url);

    // 查询余额
    println!("\n💰 查询余额...");

    match get_balance_with_proxy(rpc_url, proxy_url, &keypair.pubkey().to_string()).await {
        Ok(balance) => {
            let sol = balance as f64 / 1_000_000_000.0;
            println!("✅ 余额查询成功!");
            println!("  - Lamports: {}", balance);
            println!("  - SOL: {:.9} SOL", sol);

            if balance == 0 {
                println!("\n⚠️  警告: 余额为 0");
                println!("💡 建议从 faucet 获取测试 SOL:");
                println!("   https://faucet.solana.com/");
            } else {
                println!("✅ 账户有余额，可以进行测试交易");
            }
        },
        Err(e) => {
            println!("⚠️  查询余额失败: {}", e);
            println!("💡 可能的原因:");
            println!("   - 代理服务器未启动或配置错误");
            println!("   - RPC 节点不可用");
            println!("   - 网络连接问题");
            println!("   - 地址在 testnet 上不存在");
            println!("\n💡 请确保代理服务器正在运行:");
            println!("   curl -x http://127.0.0.1:7891 https://api.testnet.solana.com");
        },
    }

    println!("\n=====================================================\n");
}

/// 通过代理查询余额
///
/// 使用 reqwest 通过代理调用 Solana RPC API
async fn get_balance_with_proxy(
    rpc_url: &str,
    proxy_url: &str,
    address: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    use reqwest::Proxy;

    // 创建代理
    let proxy = Proxy::all(proxy_url)?;

    // 创建带有代理的 HTTP 客户端
    let client = reqwest::Client::builder().proxy(proxy).build()?;

    // 构造 RPC 请求
    let request = RpcRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "getBalance".to_string(),
        params: RpcParams(address.to_string(), None),
    };

    // 发送请求
    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    // 解析响应
    let rpc_response: RpcResponse = response.json().await?;

    if let Some(error) = rpc_response.error {
        Err(format!("RPC 错误: {}", error.message).into())
    } else {
        Ok(rpc_response.result.value)
    }
}

/// RPC 请求结构
#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: RpcParams,
}

/// RPC 参数（序列化为数组）
#[derive(Serialize)]
struct RpcParams(
    String,                                                              // pubkey
    #[serde(skip_serializing_if = "Option::is_none")] Option<RpcConfig>, // config (可选)
);

/// RPC 配置
#[derive(Serialize)]
struct RpcConfig {
    encoding: String,
}

/// RPC 响应结构
#[derive(Deserialize)]
struct RpcResponse {
    result: RpcResult,
    error: Option<RpcError>,
}

/// RPC 结果
#[derive(Deserialize)]
struct RpcResult {
    value: u64,
}

/// RPC 错误
#[derive(Deserialize)]
struct RpcError {
    message: String,
}
