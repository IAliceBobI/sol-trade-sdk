//! Auto Mock RPC 客户端
//!
//! 智能 Auto 模式：有缓存就用，没缓存就调用 RPC 并保存
//!
//! 用于 DEX Parser 和 Pool 查询测试

use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_account_decoder::UiAccount;
use solana_client::nonblocking::rpc_client::RpcClient as NonblockingRpcClient;
use solana_client::rpc_config::{RpcProgramAccountsConfig, RpcTransactionConfig};
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_response::UiTokenAmount;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::Signature,
};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;

/// Pool 查询 RPC 客户端 Trait
///
/// 统一 `RpcClient` 和 `AutoMockRpcClient` 的接口，让 Pool 查询函数可以接受两者。
#[async_trait::async_trait]
pub trait PoolRpcClient: Send + Sync {
    /// 获取账户信息
    async fn get_account(&self, pubkey: &Pubkey) -> Result<Account, String>;

    /// 获取程序账户列表
    async fn get_program_ui_accounts_with_config(
        &self,
        program_id: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(String, UiAccount)>, String>;

    /// 获取 Token 账户余额
    async fn get_token_account_balance(
        &self,
        pubkey: &Pubkey,
    ) -> Result<UiTokenAmount, String>;
}

/// 为标准的非阻塞 RpcClient 实现 PoolRpcClient
#[async_trait::async_trait]
impl PoolRpcClient for NonblockingRpcClient {
    async fn get_account(&self, pubkey: &Pubkey) -> Result<Account, String> {
        self.get_account(pubkey)
            .await
            .map_err(|e| format!("RPC 调用失败: {}", e))
    }

    async fn get_program_ui_accounts_with_config(
        &self,
        program_id: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(String, UiAccount)>, String> {
        let accounts = self
            .get_program_ui_accounts_with_config(program_id, config)
            .await
            .map_err(|e| format!("RPC 调用失败: {}", e))?;

        // 将 Pubkey 转换为 String 以保持一致性
        Ok(accounts
            .into_iter()
            .map(|(pubkey, account)| (pubkey.to_string(), account))
            .collect())
    }

    async fn get_token_account_balance(
        &self,
        pubkey: &Pubkey,
    ) -> Result<UiTokenAmount, String> {
        self.get_token_account_balance(pubkey)
            .await
            .map_err(|e| format!("RPC 调用失败: {}", e))
    }
}

/// Auto Mock RPC 客户端
///
/// 智能 Auto 模式：
/// - 有缓存数据 → 从文件加载
/// - 无缓存数据 → 调用 RPC 并保存
///
/// # Namespace（命名空间）
///
/// 通过设置 namespace，可以让不同测试使用独立的 mock 数据文件，避免冲突。
///
/// ## 设置方式
///
/// 1. **环境变量**（推荐）：
///    ```bash
///    MOCK_NAMESPACE=test1 cargo test --test xxx
///    ```
///
/// 2. **代码中指定**：
///    ```rust
///    let client = AutoMockRpcClient::new_with_namespace(
///        rpc_url.to_string(),
///        Some("test1".to_string())
///    );
///    ```
///
/// ## 文件命名规则
///
/// - 有 namespace: `{method}_{namespace}_{params_hash}.json`
/// - 无 namespace: `{method}_{params_hash}.json`
pub struct AutoMockRpcClient {
    /// 内部 RPC 客户端
    inner: Arc<RpcClient>,
    /// Mock 数据目录
    mock_dir: String,
    /// 命名空间（可选，用于隔离不同测试的 mock 数据）
    namespace: Option<String>,
}

impl AutoMockRpcClient {
    /// 创建新的 Auto Mock RPC 客户端
    ///
    /// # 参数
    /// - `rpc_url`: RPC 节点地址
    ///
    /// # 环境变量
    /// - `MOCK_DIR`: Mock 数据目录（默认: tests/mock_data）
    /// - `MOCK_NAMESPACE`: Mock 命名空间（可选，用于隔离不同测试的 mock 数据）
    pub fn new(rpc_url: String) -> Self {
        let mock_dir = std::env::var("MOCK_DIR")
            .unwrap_or_else(|_| "tests/mock_data".to_string());

        // 从环境变量读取 namespace
        let namespace = std::env::var("MOCK_NAMESPACE").ok();

        Self {
            inner: Arc::new(RpcClient::new(rpc_url)),
            mock_dir,
            namespace,
        }
    }

    /// 创建新的 Auto Mock RPC 客户端（指定命名空间）
    ///
    /// # 参数
    /// - `rpc_url`: RPC 节点地址
    /// - `namespace`: 命名空间标识符，None 表示不使用命名空间（共享 mock 数据）
    ///
    /// # 示例
    /// ```rust
    /// // 使用独立命名空间
    /// let client = AutoMockRpcClient::new_with_namespace(
    ///     rpc_url.to_string(),
    ///     Some("my_test".to_string())
    /// );
    ///
    /// // 共享 mock 数据（不使用命名空间）
    /// let client = AutoMockRpcClient::new_with_namespace(
    ///     rpc_url.to_string(),
    ///     None
    /// );
    /// ```
    pub fn new_with_namespace(rpc_url: String, namespace: Option<String>) -> Self {
        let mock_dir = std::env::var("MOCK_DIR")
            .unwrap_or_else(|_| "tests/mock_data".to_string());

        Self {
            inner: Arc::new(RpcClient::new(rpc_url)),
            mock_dir,
            namespace,
        }
    }

    /// 获取当前命名空间
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// 获取 Mock 数据目录
    pub fn mock_dir(&self) -> &str {
        &self.mock_dir
    }

    /// 生成文件名
    ///
    /// 格式:
    /// - 有 namespace: `{method}_{namespace}_{params_hash}.json`
    /// - 无 namespace: `{method}_{params_hash}.json`
    pub fn generate_file_name(&self, method: &str, params: &Value) -> String {
        let params_str = params.to_string();
        let mut hasher = DefaultHasher::new();

        // 如果有 namespace，将其纳入 hash 计算，确保不同 namespace 的数据不会冲突
        if let Some(ns) = &self.namespace {
            ns.hash(&mut hasher);
        }
        params_str.hash(&mut hasher);
        let hash = hasher.finish();

        // 文件名格式：method_namespace_hash 或 method_hash
        match &self.namespace {
            Some(ns) => format!("{}_{}_{:016x}.json", method, ns, hash),
            None => format!("{}_{:016x}.json", method, hash),
        }
    }

    /// 检查 Mock 数据是否存在
    pub fn has_mock_data(&self, method: &str, params: &Value) -> bool {
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
    #[allow(dead_code)]
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

    /// 获取交易（Auto 模式）
    ///
    /// 智能模式：有缓存就用，没缓存就调用 RPC 并保存
    pub async fn get_transaction(
        &self,
        signature: &Signature,
        config: RpcTransactionConfig,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, String> {
        // 在 async 函数内部调用 auto_call，但它需要一个同步闭包
        // 所以我们使用一个特殊的 wrapper
        let params_json = serde_json::json!((
            signature.to_string(),
            RpcTransactionConfig {
                encoding: config.encoding.clone(),
                commitment: config.commitment.clone(),
                max_supported_transaction_version: config.max_supported_transaction_version,
            }
        ));

        // 有缓存就用
        if self.has_mock_data("get_transaction_with_config", &params_json) {
            return self.load_mock_data("get_transaction_with_config", &params_json);
        }

        // 没缓存就调用 RPC 并保存（使用 tokio::task::spawn_blocking）
        let inner = self.inner.clone();
        let sig = *signature;

        let tx = tokio::task::spawn_blocking(move || {
            inner.get_transaction_with_config(&sig, config)
                .map_err(|e| format!("RPC 调用失败: {}", e))
        })
        .await
        .map_err(|e| format!("任务执行失败: {}", e))??;

        // 保存到文件
        let result_json = serde_json::to_value(&tx)
            .map_err(|e| format!("序列化结果失败: {}", e))?;
        self.save_mock_data("get_transaction_with_config", &params_json, &result_json);

        Ok(tx)
    }

    /// 获取账户信息（Auto 模式）
    ///
    /// 智能 Auto 模式：有缓存就用，没缓存就调用 RPC 并保存
    pub async fn get_account(&self, pubkey: &Pubkey) -> Result<Account, String> {
        let params_json = serde_json::json!((pubkey.to_string(),));

        // 有缓存就用
        if self.has_mock_data("get_account", &params_json) {
            return self.load_mock_data("get_account", &params_json);
        }

        // 没缓存就调用 RPC 并保存
        let inner = self.inner.clone();
        let pk = *pubkey;

        let account = tokio::task::spawn_blocking(move || {
            inner.get_account(&pk)
                .map_err(|e| format!("RPC 调用失败: {}", e))
        })
        .await
        .map_err(|e| format!("任务执行失败: {}", e))??;

        // 保存到文件
        let result_json = serde_json::to_value(&account)
            .map_err(|e| format!("序列化结果失败: {}", e))?;
        self.save_mock_data("get_account", &params_json, &result_json);

        Ok(account)
    }

    /// 获取程序账户列表（Auto 模式）
    ///
    /// 智能 Auto 模式：有缓存就用，没缓存就调用 RPC 并保存
    pub async fn get_program_ui_accounts_with_config(
        &self,
        program_id: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(String, UiAccount)>, String> {
        // 序列化 config 用于缓存键（需要先克隆，因为后面还要用）
        let config_for_json = serde_json::to_value(&config)
            .map_err(|e| format!("序列化 config 失败: {}", e))?;
        let params_json = serde_json::json!((
            program_id.to_string(),
            config_for_json
        ));

        // 有缓存就用
        if self.has_mock_data("get_program_ui_accounts_with_config", &params_json) {
            return self.load_mock_data("get_program_ui_accounts_with_config", &params_json);
        }

        // 没缓存就调用 RPC 并保存
        let inner = self.inner.clone();
        let pid = *program_id;

        let accounts = tokio::task::spawn_blocking(move || {
            inner.get_program_ui_accounts_with_config(&pid, config)
                .map_err(|e| format!("RPC 调用失败: {}", e))
        })
        .await
        .map_err(|e| format!("任务执行失败: {}", e))??;

        // 将 Pubkey 转换为 String
        let accounts: Vec<(String, UiAccount)> = accounts
            .into_iter()
            .map(|(pubkey, account)| (pubkey.to_string(), account))
            .collect();

        // 保存到文件（保存原始格式，Pubkey 转为 String）
        let result_json = serde_json::to_value(&accounts)
            .map_err(|e| format!("序列化结果失败: {}", e))?;
        self.save_mock_data("get_program_ui_accounts_with_config", &params_json, &result_json);

        Ok(accounts)
    }

    /// 获取 Token 账户余额（Auto 模式）
    ///
    /// 智能 Auto 模式：有缓存就用，没缓存就调用 RPC 并保存
    pub async fn get_token_account_balance(
        &self,
        pubkey: &Pubkey,
    ) -> Result<UiTokenAmount, String> {
        let params_json = serde_json::json!((pubkey.to_string(),));

        // 有缓存就用
        if self.has_mock_data("get_token_account_balance", &params_json) {
            return self.load_mock_data("get_token_account_balance", &params_json);
        }

        // 没缓存就调用 RPC 并保存
        let inner = self.inner.clone();
        let pk = *pubkey;

        let balance = tokio::task::spawn_blocking(move || {
            inner.get_token_account_balance(&pk)
                .map_err(|e| format!("RPC 调用失败: {}", e))
        })
        .await
        .map_err(|e| format!("任务执行失败: {}", e))??;

        // 保存到文件
        let result_json = serde_json::to_value(&balance)
            .map_err(|e| format!("序列化结果失败: {}", e))?;
        self.save_mock_data("get_token_account_balance", &params_json, &result_json);

        Ok(balance)
    }
}

/// 为 AutoMockRpcClient 实现 PoolRpcClient
#[async_trait::async_trait]
impl PoolRpcClient for AutoMockRpcClient {
    async fn get_account(&self, pubkey: &Pubkey) -> Result<Account, String> {
        self.get_account(pubkey).await
    }

    async fn get_program_ui_accounts_with_config(
        &self,
        program_id: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(String, UiAccount)>, String> {
        self.get_program_ui_accounts_with_config(program_id, config).await
    }

    async fn get_token_account_balance(
        &self,
        pubkey: &Pubkey,
    ) -> Result<UiTokenAmount, String> {
        self.get_token_account_balance(pubkey).await
    }
}
