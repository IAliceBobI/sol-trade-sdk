//! Mock RPC 系统使用示例
//!
//! 这个测试展示了如何使用 MockRpcMode 进行录制-重放测试

use sol_trade_sdk::common::mock_rpc::{MockMode, MockRpcMode};

/// 测试：演示 Mock Rpc 的基本使用
#[test]
fn test_mock_rpc_mode_creation() {
    // 创建 Mock RPC 客户端
    let mock_rpc = MockRpcMode::new();

    // 默认模式是 Live
    assert_eq!(mock_rpc.mode(), MockMode::Live);

    println!("✅ MockRpcMode 创建成功");
    println!("   模式: {:?}", mock_rpc.mode());
    println!("   Mock 目录: {}", mock_rpc.mock_dir());
}

/// 测试：从环境变量读取模式
#[test]
fn test_mock_mode_from_env() {
    // 先清理环境变量
    std::env::remove_var("MOCK_MODE");

    // 默认模式是 Live
    let mock_rpc_default = MockRpcMode::new();
    assert_eq!(mock_rpc_default.mode(), MockMode::Live);

    // 设置环境变量
    std::env::set_var("MOCK_MODE", "record");
    let mock_rpc = MockRpcMode::new();
    assert_eq!(mock_rpc.mode(), MockMode::Record);

    std::env::set_var("MOCK_MODE", "replay");
    let mock_rpc = MockRpcMode::new();
    assert_eq!(mock_rpc.mode(), MockMode::Replay);

    std::env::set_var("MOCK_MODE", "live");
    let mock_rpc = MockRpcMode::new();
    assert_eq!(mock_rpc.mode(), MockMode::Live);

    // 清理
    std::env::remove_var("MOCK_MODE");

    println!("✅ 环境变量模式切换测试通过");
}

/// 测试：文件名生成
#[test]
fn test_file_name_generation() {
    let mock_rpc = MockRpcMode::new();

    let method = "getProgramAccounts";
    let params1 = serde_json::json!(["program123", {"offset": 1}]);
    let params2 = serde_json::json!(["program123", {"offset": 1}]);
    let params3 = serde_json::json!(["program456"]);

    let file1 = mock_rpc.generate_file_name(method, &params1);
    let file2 = mock_rpc.generate_file_name(method, &params2);
    let file3 = mock_rpc.generate_file_name(method, &params3);

    // 相同参数应该生成相同文件名
    assert_eq!(file1, file2);

    // 不同参数应该生成不同文件名
    assert_ne!(file1, file3);

    // 文件名格式: method_hash.json
    assert!(file1.starts_with("getProgramAccounts_"));
    assert!(file1.ends_with(".json"));

    println!("✅ 文件名生成测试通过");
    println!("   文件1: {}", file1);
    println!("   文件2: {}", file2);
    println!("   文件3: {}", file3);
}

/// 测试：录制和重放功能（使用临时目录）
#[test]
fn test_record_and_replay() {
    // 创建临时目录
    let temp_dir = std::env::temp_dir().join("mock_rpc_test");
    std::fs::create_dir_all(&temp_dir)
        .unwrap_or_else(|_| panic!("无法创建临时目录: {}", temp_dir.display()));

    // 创建 Mock RPC 客户端（Record 模式）
    let mut record_mock = MockRpcMode::new_with_mode(
        "http://127.0.0.1:8899".to_string(),
        MockMode::Record,
    );
    record_mock.mock_dir = temp_dir.as_path().to_str().unwrap().to_string();

    // 准备测试数据
    let method = "getAccountInfo";
    let params = serde_json::json!([
        "H7R2KBXrMhjTFmHwXYG6mCtEUAwq8Y5EYjV8YNJrz8L"
    ]);
    let response = serde_json::json!({
        "context": {"slot": 123456},
        "value": {
            "data": ["base64data", "base64"],
            "owner": "program123",
            "lamports": 1000000
        }
    });

    // 保存录制
    record_mock.save_recording(method, &params, &response);

    // 验证文件存在
    assert!(record_mock.has_mock_data(method, &params));

    // 创建 Replay 模式的 Mock RPC
    let mut replay_mock = MockRpcMode::new_with_mode(
        "http://127.0.0.1:8899".to_string(),
        MockMode::Replay,
    );
    replay_mock.mock_dir = temp_dir.as_path().to_str().unwrap().to_string();

    // 加载录制
    let loaded_response = replay_mock.load_recording(method, &params).unwrap();

    // 验证数据一致
    assert_eq!(loaded_response, response);

    // 清理
    std::fs::remove_dir_all(&temp_dir).ok();

    println!("✅ 录制和重放测试通过");
}

/// 示例：在真实测试中使用 Mock Rpc
///
/// 运行方式：
/// ```bash
/// # 1. 录制模式：从真实 RPC 获取数据并保存
/// MOCK_MODE=record cargo test --test mock_rpc_example -- --nocapture
///
/// # 2. 重放模式：从本地文件读取数据
/// MOCK_MODE=replay cargo test --test mock_rpc_example -- --nocapture
///
/// # 3. 直播模式：直接调用真实 RPC
/// MOCK_MODE=live cargo test --test mock_rpc_example -- --nocapture
/// # 或
/// cargo test --test mock_rpc_example -- --nocapture
/// ```
#[tokio::test]
#[ignore]  // 默认跳过，手动运行时需要去掉 #[ignore]
async fn example_mock_usage() {
    // 根据 MOCK_MODE 环境变量创建 Mock RPC
    let mock_rpc = MockRpcMode::new();

    println!("🎬 当前模式: {:?}", mock_rpc.mode());

    // 使用 mock_rpc 就像使用普通的 RpcClient 一样
    // （因为实现了 Deref trait）

    match mock_rpc.mode() {
        MockMode::Record => {
            println!("📼 录制模式：正在从真实 RPC 获取数据...");
            // 这里调用真实的 RPC 方法
            // MockRpcMode 会自动保存响应到文件
        }
        MockMode::Replay => {
            println!("▶️  重放模式：正在从本地文件读取数据...");
            // MockRpcMode 会自动从文件加载响应
        }
        MockMode::Live => {
            println!("📡 直播模式：直接调用真实 RPC");
            // 直接调用真实 RPC，不保存任何数据
        }
    }

    // 示例：获取账户信息
    // let account = mock_rpc.get_account(&pubkey).await.unwrap();
    // assert_eq!(account.owner, expected_owner);

    println!("✅ 测试完成");
}

/// 测试：清理 Mock 数据
#[test]
fn test_clear_mock_data() {
    // 创建临时目录
    let temp_dir = std::env::temp_dir().join("mock_rpc_clear_test");
    std::fs::create_dir_all(&temp_dir)
        .unwrap_or_else(|_| panic!("无法创建临时目录: {}", temp_dir.display()));

    let mut mock_rpc = MockRpcMode::new_with_mode(
        "http://127.0.0.1:8899".to_string(),
        MockMode::Record,
    );
    mock_rpc.mock_dir = temp_dir.to_str().unwrap().to_string();

    // 保存一些测试数据
    let method = "testMethod";
    let params = serde_json::json!({"test": "data"});
    let response = serde_json::json!({"result": "ok"});

    mock_rpc.save_recording(method, &params, &response);

    // 验证文件存在
    assert!(mock_rpc.has_mock_data(method, &params));

    // 清理数据
    mock_rpc.clear_mock_data();

    // 验证文件已删除
    assert!(!mock_rpc.has_mock_data(method, &params));

    println!("✅ 清理 Mock 数据测试通过");

    // 清理临时目录
    std::fs::remove_dir_all(&temp_dir).ok();
}
