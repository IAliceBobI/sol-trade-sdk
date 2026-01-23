//! Auto Mock RPC 客户端
//!
//! 智能 Auto 模式：有缓存就用，没缓存就调用 RPC 并保存
//!
//! 用于 DEX Parser 和 Pool 查询测试

use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

/// Auto Mock RPC 客户端
///
/// 智能 Auto 模式：
/// - 有缓存数据 → 从文件加载
/// - 无缓存数据 → 调用 RPC 并保存
pub struct AutoMockRpcClient {
    /// 内部 RPC 客户端
    inner: Arc<RpcClient>,
    /// Mock 数据目录
    mock_dir: String,
}

impl AutoMockRpcClient {
    /// 创建新的 Auto Mock RPC 客户端
    ///
    /// # 参数
    /// - `rpc_url`: RPC 节点地址
    ///
    /// # 环境变量
    /// - `MOCK_DIR`: Mock 数据目录（默认: tests/mock_data）
    pub fn new(rpc_url: String) -> Self {
        let mock_dir = std::env::var("MOCK_DIR")
            .unwrap_or_else(|_| "tests/mock_data".to_string());

        Self {
            inner: Arc::new(RpcClient::new(rpc_url)),
            mock_dir,
        }
    }

    /// 获取 Mock 数据目录
    pub fn mock_dir(&self) -> &str {
        &self.mock_dir
    }

    /// 生成文件名
    ///
    /// 格式: {method}_{params_hash}.json
    fn generate_file_name(&self, method: &str, params: &Value) -> String {
        let params_str = params.to_string();
        let mut hasher = DefaultHasher::new();
        params_str.hash(&mut hasher);
        let hash = hasher.finish();

        format!("{}_{:016x}.json", method, hash)
    }

    /// 检查 Mock 数据是否存在
    fn has_mock_data(&self, method: &str, params: &Value) -> bool {
        let file_name = self.generate_file_name(method, params);
        let file_path = Path::new(&self.mock_dir).join(&file_name);
        file_path.exists()
    }

    /// 保存 Mock 数据到文件
    fn save_mock_data(&self, method: &str, params: &Value, response: &Value) {
        // 确保目录存在
        fs::create_dir_all(&self.mock_dir).unwrap_or_else(|e| {
            eprintln!("⚠️  无法创建 Mock 数据目录: {}", e);
        });

        let file_name = self.generate_file_name(method, params);
        let file_path = Path::new(&self.mock_dir).join(&file_name);

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

    /// 从文件加载 Mock 数据
    fn load_mock_data<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: &Value,
    ) -> Result<T, String> {
        let file_name = self.generate_file_name(method, params);
        let file_path = Path::new(&self.mock_dir).join(&file_name);

        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("❌ Mock 数据文件不存在: {:?} ({})", file_path, e))?;

        let mock_data: Value = serde_json::from_str(&content)
            .map_err(|e| format!("❌ 解析 Mock 数据失败: {} (path: {:?})", e, file_path))?;

        mock_data
            .get("response")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "❌ Mock 数据格式错误: 缺少 response 字段".to_string())
    }

    /// Auto 模式调用核心逻辑
    ///
    /// 通用方法，处理所有 Auto 模式的 RPC 调用
    async fn auto_call<M, P, R>(&self, method: &str, params: &P, rpc_call: M) -> Result<R, String>
    where
        M: FnOnce() -> Result<R, Box<dyn std::error::Error>>,
        P: Serialize,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        let params_json = serde_json::json!(params);

        // 有缓存就用
        if self.has_mock_data(method, &params_json) {
            return self.load_mock_data(method, &params_json);
        }

        // 没缓存就调用 RPC 并保存
        let result = rpc_call().map_err(|e| e.to_string())?;

        // 保存到文件
        let result_json =
            serde_json::to_value(&result).map_err(|e| format!("序列化结果失败: {}", e))?;
        self.save_mock_data(method, &params_json, &result_json);

        Ok(result)
    }
}
