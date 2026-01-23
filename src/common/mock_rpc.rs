//! Mock RPC 系统借鉴 httpmock 设计
//!
//! 支持三种模式：
//! - Record: 录制真实 RPC 调用
//! - Replay: 重放录制的响应
//! - Live: 直接调用真实 RPC
//!
//! ## 使用方法
//!
//! ```bash
//! # 录制模式：从真实 RPC 获取数据并保存
//! MOCK_MODE=record cargo test --test pool_tests
//!
//! # 重放模式：从本地文件读取数据
//! MOCK_MODE=replay cargo test --test pool_tests
//!
//! # 直播模式：直接调用真实 RPC（默认）
//! MOCK_MODE=live cargo test --test pool_tests
//! # 或不设置 MOCK_MODE
//! ```

use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use std::fs;
use std::ops::Deref;
use std::path::Path;

/// Mock 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockMode {
    /// 录制模式：调用真实 RPC 并保存响应
    Record,
    /// 重放模式：从本地文件读取响应
    Replay,
    /// 直播模式：直接调用真实 RPC
    Live,
}

impl MockMode {
    /// 从环境变量读取模式
    pub fn from_env() -> Self {
        match std::env::var("MOCK_MODE").as_deref() {
            Ok("record") => MockMode::Record,
            Ok("replay") => MockMode::Replay,
            Ok("live") | _ => MockMode::Live,
        }
    }
}

/// Mock RPC 客户端
///
/// 这个结构包装了标准的 `RpcClient`，并根据 `MockMode` 选择行为。
/// 它实现了 `Deref`，因此可以像 `RpcClient` 一样使用。
pub struct MockRpcMode {
    inner: RpcClient,
    pub mode: MockMode,
    pub mock_dir: String,
}

impl MockRpcMode {
    /// 创建新的 Mock RPC 客户端
    ///
    /// 从环境变量 `RPC_URL` 读取 RPC 地址（默认: http://127.0.0.1:8899）
    /// 从环境变量 `MOCK_MODE` 读取模式（默认: Live）
    pub fn new() -> Self {
        let mode = MockMode::from_env();
        let rpc_url = std::env::var("RPC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
        let mock_dir = std::env::var("MOCK_DIR")
            .unwrap_or_else(|_| "tests/mock_data".to_string());

        Self {
            inner: RpcClient::new(rpc_url),
            mode,
            mock_dir,
        }
    }

    /// 使用指定的 RPC URL 创建 Mock RPC 客户端
    pub fn new_with_url(rpc_url: String) -> Self {
        let mode = MockMode::from_env();
        let mock_dir = std::env::var("MOCK_DIR")
            .unwrap_or_else(|_| "tests/mock_data".to_string());

        Self {
            inner: RpcClient::new(rpc_url),
            mode,
            mock_dir,
        }
    }

    /// 使用指定的模式创建 Mock RPC 客户端
    pub fn new_with_mode(rpc_url: String, mode: MockMode) -> Self {
        let mock_dir = std::env::var("MOCK_DIR")
            .unwrap_or_else(|_| "tests/mock_data".to_string());

        Self {
            inner: RpcClient::new(rpc_url),
            mode,
            mock_dir,
        }
    }

    /// 获取当前模式
    pub fn mode(&self) -> MockMode {
        self.mode
    }

    /// 获取 Mock 数据目录
    pub fn mock_dir(&self) -> &str {
        &self.mock_dir
    }

    /// 调用 RPC 并根据模式处理
    ///
    /// 这是核心方法，根据模式选择：
    /// - Record: 调用真实 RPC → 保存响应 → 返回
    /// - Replay: 从文件加载响应 → 返回
    /// - Live: 直接调用真实 RPC
    pub async fn call_rpc(
        &self,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        match self.mode {
            MockMode::Record => {
                // 1. 调用真实 RPC
                let response = self.real_rpc_call(method, params.clone()).await?;

                // 2. 保存到文件
                self.save_recording(method, &params, &response);

                Ok(response)
            }
            MockMode::Replay => {
                // 从文件加载
                self.load_recording(method, &params)
            }
            MockMode::Live => {
                // 直接调用真实 RPC
                self.real_rpc_call(method, params).await
            }
        }
    }

    /// 调用真实 RPC
    async fn real_rpc_call(&self, _method: &str, _params: Value) -> Result<Value, String> {
        // 注意：这里需要使用真实的 RPC 调用
        // 由于 solana_client::rpc_client::RpcClient 没有通用的 call 方法，
        // 我们需要在调用处使用这个 Mock 客户端的真实 RPC
        // 这个方法主要用于 Record 模式
        Err("Use real RPC methods for Live/Record mode".to_string())
    }

    /// 保存录制到文件
    pub fn save_recording(&self, method: &str, params: &Value, response: &Value) {
        // 确保目录存在
        fs::create_dir_all(&self.mock_dir).unwrap_or_else(|e| {
            eprintln!("⚠️  无法创建 Mock 数据目录: {}", e);
        });

        // 生成文件名
        let file_name = self.generate_file_name(method, params);
        let file_path = Path::new(&self.mock_dir).join(&file_name);

        // 保存数据
        let mock_data = serde_json::json!({
            "method": method,
            "params": params,
            "response": response
        });

        let json = serde_json::to_string_pretty(&mock_data).unwrap_or_else(|e| {
            eprintln!("⚠️  序列化失败: {}", e);
            return String::new();
        });

        fs::write(&file_path, json).unwrap_or_else(|e| {
            eprintln!("⚠️  保存 Mock 数据失败: {} (path: {:?})", e, file_path);
        });
    }

    /// 从文件加载录制
    pub fn load_recording(&self, method: &str, params: &Value) -> Result<Value, String> {
        let file_name = self.generate_file_name(method, params);
        let file_path = Path::new(&self.mock_dir).join(&file_name);

        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("❌ Mock 数据文件不存在: {:?} ({})", file_path, e))?;

        let mock_data: Value = serde_json::from_str(&content)
            .map_err(|e| format!("❌ 解析 Mock 数据失败: {} (path: {:?})", e, file_path))?;

        mock_data.get("response")
            .cloned()
            .ok_or_else(|| "❌ Mock 数据格式错误: 缺少 response 字段".to_string())
    }

    /// 生成文件名
    ///
    /// 格式: {method}_{params_hash}.json
    /// 使用参数的 hash 确保不同的参数生成不同的文件
    pub fn generate_file_name(&self, method: &str, params: &Value) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // 计算 params 的 hash
        let params_str = params.to_string();
        let mut hasher = DefaultHasher::new();
        params_str.hash(&mut hasher);
        let hash = hasher.finish();

        format!("{}_{:016x}.json", method, hash)
    }

    /// 清理所有 Mock 数据
    pub fn clear_mock_data(&self) {
        if let Ok(_) = fs::remove_dir_all(&self.mock_dir) {
            println!("🗑️  已清理 Mock 数据目录: {}", self.mock_dir);
        }
    }

    /// 检查 Mock 数据是否存在
    pub fn has_mock_data(&self, method: &str, params: &Value) -> bool {
        let file_name = self.generate_file_name(method, params);
        let file_path = Path::new(&self.mock_dir).join(&file_name);
        file_path.exists()
    }
}

// 实现 Deref，使 MockRpcMode 可以像 RpcClient 一样使用
impl Deref for MockRpcMode {
    type Target = RpcClient;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// 从标准 RpcClient 创建 MockRpcMode
impl From<RpcClient> for MockRpcMode {
    fn from(rpc: RpcClient) -> Self {
        Self {
            inner: rpc,
            mode: MockMode::from_env(),
            mock_dir: "tests/mock_data".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_mode_from_env() {
        // 测试默认模式（Live）
        std::env::remove_var("MOCK_MODE");
        assert_eq!(MockMode::from_env(), MockMode::Live);

        // 测试 Record 模式
        std::env::set_var("MOCK_MODE", "record");
        assert_eq!(MockMode::from_env(), MockMode::Record);

        // 测试 Replay 模式
        std::env::set_var("MOCK_MODE", "replay");
        assert_eq!(MockMode::from_env(), MockMode::Replay);

        // 清理
        std::env::remove_var("MOCK_MODE");
    }

    #[test]
    fn test_generate_file_name() {
        let mock = MockRpcMode::new_with_url("http://127.0.0.1:8899".to_string());

        let method = "getProgramAccounts";
        let params = serde_json::json!([
            "program123",
            {"dataSlice": {"offset": 1, "length": 2}}
        ]);

        let file_name = mock.generate_file_name(method, &params);

        // 文件名应该包含方法名和参数的 hash
        assert!(file_name.starts_with("getProgramAccounts_"));
        assert!(file_name.ends_with(".json"));
        assert!(file_name.len() > "getProgramAccounts_.json".len());

        // 相同的参数应该生成相同的文件名
        let file_name2 = mock.generate_file_name(method, &params);
        assert_eq!(file_name, file_name2);

        // 不同的参数应该生成不同的文件名
        let params2 = serde_json::json!(["program456"]);
        let file_name3 = mock.generate_file_name(method, &params2);
        assert_ne!(file_name, file_name3);
    }

    #[test]
    fn test_save_and_load_recording() {
        use tempfile::TempDir;

        // 创建临时目录
        let temp_dir = TempDir::new().unwrap();
        let mock = MockRpcMode::new_with_mode(
            "http://127.0.0.1:8899".to_string(),
            MockMode::Record,
        );
        mock.mock_dir = temp_dir.path().to_str().unwrap().to_string();

        let method = "testMethod";
        let params = serde_json::json!({"param1": "value1"});
        let response = serde_json::json!({"result": "success"});

        // 保存录制
        mock.save_recording(method, &params, &response);

        // 验证文件存在
        assert!(mock.has_mock_data(method, &params));

        // 加载录制
        let loaded = mock.load_recording(method, &params).unwrap();
        assert_eq!(loaded, response);

        // 清理
        temp_dir.close().unwrap();
    }
}
