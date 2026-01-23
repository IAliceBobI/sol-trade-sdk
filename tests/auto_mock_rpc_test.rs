//! Auto Mock RPC 客户端测试
//!
//! 测试 Auto 模式的核心功能

use sol_trade_sdk::common::auto_mock_rpc::AutoMockRpcClient;
use solana_sdk::signature::Signature;
use std::str::FromStr;

#[test]
fn test_auto_mock_client_creation() {
    let client = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());

    assert_eq!(client.mock_dir(), "tests/mock_data");
    println!("✅ AutoMockRpcClient 创建成功");
}

#[test]
fn test_generate_file_name() {
    let client = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());

    let sig = Signature::from_str("5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK").unwrap();
    let params = serde_json::json!([sig, {"encoding": "jsonParsed"}]);

    let file1 = client.generate_file_name("getTransaction", &params);
    let file2 = client.generate_file_name("getTransaction", &params);

    // 相同参数生成相同文件名
    assert_eq!(file1, file2);
    assert!(file1.starts_with("getTransaction_"));
    assert!(file1.ends_with(".json"));

    println!("✅ 文件名生成测试通过: {}", file1);
}

#[test]
fn test_has_mock_data() {
    let client = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());

    let sig = Signature::from_str("5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK").unwrap();
    let params = serde_json::json!([sig, {"encoding": "jsonParsed"}]);

    // 初始状态：没有 Mock 数据
    assert!(!client.has_mock_data("getTransaction", &params));

    println!("✅ Mock 数据检查测试通过");
}

#[tokio::test]
#[ignore]  // 需要 RPC 节点，手动运行
async fn test_auto_mode_first_call() {
    let client = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());

    let sig = Signature::from_str("5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK").unwrap();

    use solana_client::rpc_config::{RpcTransactionConfig, UiTransactionEncoding};
    use solana_commitment_config::CommitmentConfig;

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    // 首次调用：从 RPC 获取
    let result = client.get_transaction(&sig, config).await;

    match result {
        Ok(tx) => {
            println!("✅ 首次调用成功，从 RPC 获取");
            println!("   Slot: {}", tx.slot);

            // 验证 Mock 文件已创建
            let params = serde_json::json!([sig, {
                "encoding": "jsonParsed",
                "commitment": "confirmed",
                "maxSupportedTransactionVersion": 0
            }]);
            assert!(client.has_mock_data("getTransaction", &params));
        }
        Err(e) => {
            eprintln!("❌ 调用失败: {}", e);
            panic!("测试失败: {}", e);
        }
    }
}
