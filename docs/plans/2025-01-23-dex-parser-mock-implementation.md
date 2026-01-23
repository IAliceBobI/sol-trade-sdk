# DEX Parser Mock 系统实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 DEX Parser 和 Pool 查询测试提供智能 Mock 功能，将测试时间减少 96-97%

**Architecture:** 新增 `AutoMockRpcClient` 结构体，实现智能 Auto 模式（有缓存就用，没缓存就调用 RPC 并保存）。`DexParser` 新增 `new_with_mock()` 方法，使用 `AutoMockRpcClient` 替代标准 `RpcClient`。

**Tech Stack:** Rust (Edition 2021), solana-client 3.0.x, tokio, serde, serde_json

---

## 前置准备

### Task 0: 验证环境和依赖

**Files:**
- Check: `Cargo.toml`

**Step 1: 检查项目依赖**

运行：
```bash
cargo check
```

预期：编译通过，无错误

**Step 2: 运行现有测试**

运行：
```bash
cargo test --test dex_parser_comprehensive -- --nocapture
```

预期：测试通过，记录当前耗时（约 37s）

**Step 3: 记录基准数据**

记录：
- 当前测试耗时
- Mock 数据目录：`tests/mock_data/`

---

## 第一部分：实现 AutoMockRpcClient 核心功能

### Task 1: 创建 AutoMockRpcClient 基础结构

**Files:**
- Create: `src/common/auto_mock_rpc.rs`

**Step 1: 创建文件并添加基础结构体**

编辑：`src/common/auto_mock_rpc.rs`

```rust
//! Auto Mock RPC 客户端
//!
//! 智能 Auto 模式：有缓存就用，没缓存就调用 RPC 并保存
//!
//! 用于 DEX Parser 和 Pool 查询测试

use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{
    RpcAccountInfoConfig,
    RpcProgramAccountsConfig,
    RpcTransactionConfig,
};
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::Signature,
};
use solana_transaction_status::EncodedTransactionWithConfigMeta;
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::Path,
    sync::Arc,
};

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
}
```

**Step 2: 添加模块到 common/mod.rs**

编辑：`src/common/mod.rs`

在文件末尾添加：
```rust
pub mod auto_mock_rpc;
```

**Step 3: 验证编译**

运行：
```bash
cargo check
```

预期：编译通过，无错误

**Step 4: 提交**

```bash
git add src/common/auto_mock_rpc.rs src/common/mod.rs
git commit -m "✨ feat(mock): 添加 AutoMockRpcClient 基础结构

- 创建 AutoMockRpcClient 结构体
- 支持环境变量 MOCK_DIR 配置
- 为添加 Auto 模式做准备

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 2: 实现 Auto 模式核心逻辑

**Files:**
- Modify: `src/common/auto_mock_rpc.rs`

**Step 1: 添加辅助方法**

在 `impl AutoMockRpcClient` 块中添加：

```rust
impl AutoMockRpcClient {
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

        mock_data.get("response")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or_else(|| "❌ Mock 数据格式错误: 缺少 response 字段".to_string())
    }
}
```

**Step 2: 添加通用 auto_call 方法**

在 `impl AutoMockRpcClient` 块中添加：

```rust
impl AutoMockRpcClient {
    /// Auto 模式调用核心逻辑
    ///
    /// 通用方法，处理所有 Auto 模式的 RPC 调用
    async fn auto_call<M, P, R>(
        &self,
        method: &str,
        params: &P,
        rpc_call: M,
    ) -> Result<R, String>
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
        let result_json = serde_json::to_value(&result)
            .map_err(|e| format!("序列化结果失败: {}", e))?;
        self.save_mock_data(method, &params_json, &result_json);

        Ok(result)
    }
}
```

**Step 3: 验证编译**

运行：
```bash
cargo check
```

预期：编译通过，无错误

**Step 4: 提交**

```bash
git add src/common/auto_mock_rpc.rs
git commit -m "✨ feat(mock): 实现 Auto 模式核心逻辑

- 添加文件名生成方法（参数哈希）
- 添加 Mock 数据检查、加载、保存方法
- 实现 auto_call 通用 Auto 模式逻辑

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 3: 实现 get_transaction_with_config 方法

**Files:**
- Modify: `src/common/auto_mock_rpc.rs`

**Step 1: 添加方法**

在 `impl AutoMockRpcClient` 块中添加：

```rust
impl AutoMockRpcClient {
    /// 获取交易（用于 DEX Parser）
    ///
    /// Auto 模式：有缓存就用，没缓存就调用 RPC 并保存
    pub async fn get_transaction_with_config(
        &self,
        sig: &Signature,
        config: RpcTransactionConfig,
    ) -> Result<EncodedTransactionWithConfigMeta, String> {
        // 在 spawn_blocking 中执行，因为 RPC 调用是同步的
        let inner = self.inner.clone();
        let sig = *sig;
        let method = "getTransaction";

        let params = (&sig, &config);
        let params_json = serde_json::json!(params);

        // 检查缓存
        if self.has_mock_data(method, &params_json) {
            return self.load_mock_data(method, &params_json);
        }

        // 调用 RPC
        tokio::task::spawn_blocking(move || {
            inner
                .get_transaction_with_config(&sig, config)
                .map_err(|e| format!("RPC 调用失败: {}", e))
        })
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
        .and_then(|tx| {
            // 保存到文件
            let tx_json = serde_json::to_value(&tx)
                .map_err(|e| format!("序列化失败: {}", e))?;
            self.save_mock_data(method, &params_json, &tx_json);
            Ok(tx)
        })
    }
}
```

**Step 2: 验证编译**

运行：
```bash
cargo check
```

预期：编译通过，无错误

**Step 3: 提交**

```bash
git add src/common/auto_mock_rpc.rs
git commit -m "✨ feat(mock): 添加 get_transaction_with_config 方法

- 支持 DEX Parser 的交易获取
- Auto 模式：有缓存就用，没缓存就调用 RPC
- 异步实现，使用 spawn_blocking

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 4: 实现 get_program_accounts 方法（可选，用于 Pool 测试）

**Files:**
- Modify: `src/common/auto_mock_rpc.rs`

**Step 1: 添加方法**

在 `impl AutoMockRpcClient` 块中添加：

```rust
impl AutoMockRpcClient {
    /// 获取程序账户列表（用于 Pool 查询）
    ///
    /// Auto 模式：有缓存就用，没缓存就调用 RPC 并保存
    pub async fn get_program_accounts(
        &self,
        pubkey: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(Pubkey, Account)>, String> {
        let inner = self.inner.clone();
        let pubkey = *pubkey;
        let method = "getProgramAccounts";

        let params = (&pubkey, &config);
        let params_json = serde_json::json!(params);

        // 检查缓存
        if self.has_mock_data(method, &params_json) {
            return self.load_mock_data(method, &params_json);
        }

        // 调用 RPC
        tokio::task::spawn_blocking(move || {
            inner
                .get_program_accounts_with_config(&pubkey, config)
                .map_err(|e| format!("RPC 调用失败: {}", e))
        })
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
        .and_then(|accounts| {
            // 保存到文件
            let accounts_json = serde_json::to_value(&accounts)
                .map_err(|e| format!("序列化失败: {}", e))?;
            self.save_mock_data(method, &params_json, &accounts_json);
            Ok(accounts)
        })
    }
}
```

**Step 2: 验证编译**

运行：
```bash
cargo check
```

预期：编译通过，无错误

**Step 3: 提交**

```bash
git add src/common/auto_mock_rpc.rs
git commit -m "✨ feat(mock): 添加 get_program_accounts 方法

- 支持 Pool 查询的账户列表获取
- Auto 模式：有缓存就用，没缓存就调用 RPC

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 5: 实现 get_account 方法（可选，用于单个 Pool）

**Files:**
- Modify: `src/common/auto_mock_rpc.rs`

**Step 1: 添加方法**

在 `impl AutoMockRpcClient` 块中添加：

```rust
impl AutoMockRpcClient {
    /// 获取账户信息（用于单个 Pool）
    ///
    /// Auto 模式：有缓存就用，没缓存就调用 RPC 并保存
    pub async fn get_account(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Account, String> {
        let inner = self.inner.clone();
        let pubkey = *pubkey;
        let method = "getAccountInfo";

        let params = (&pubkey,);
        let params_json = serde_json::json!(params);

        // 检查缓存
        if self.has_mock_data(method, &params_json) {
            return self.load_mock_data(method, &params_json);
        }

        // 调用 RPC
        tokio::task::spawn_blocking(move || {
            inner
                .get_account(&pubkey)
                .map_err(|e| format!("RPC 调用失败: {}", e))
        })
        .await
        .map_err(|e| format!("任务执行失败: {}", e))?
        .and_then(|account| {
            // 保存到文件
            let account_json = serde_json::to_value(&account)
                .map_err(|e| format!("序列化失败: {}", e))?;
            self.save_mock_data(method, &params_json, &account_json);
            Ok(account)
        })
    }
}
```

**Step 2: 验证编译**

运行：
```bash
cargo check
```

预期：编译通过，无错误

**Step 3: 提交**

```bash
git add src/common/auto_mock_rpc.rs
git commit -m "✨ feat(mock): 添加 get_account 方法

- 支持单个 Pool 的账户信息获取
- Auto 模式：有缓存就用，没缓存就调用 RPC

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 第二部分：修改 DexParser 以支持 Mock

### Task 6: 修改 DexParser 的 rpc_client 字段类型

**Files:**
- Modify: `src/parser/dex_parser.rs`

**Step 1: 添加类型抽象**

在 `src/parser/dex_parser.rs` 顶部添加 trait：

```rust
//! 在 use 语句后添加

/// RPC 客户端 trait，支持多种实现
pub trait RpcClientTrait: Send + Sync {
    fn get_transaction_with_config(
        &self,
        sig: &Signature,
        config: RpcTransactionConfig,
    ) -> Result<EncodedTransactionWithConfigMeta, Box<dyn std::error::Error + Send + Sync>>;
}

// 为标准 RpcClient 实现 trait
impl RpcClientTrait for RpcClient {
    fn get_transaction_with_config(
        &self,
        sig: &Signature,
        config: RpcTransactionConfig,
    ) -> Result<EncodedTransactionWithConfigMeta, Box<dyn std::error::Error + Send + Sync>> {
        self.get_transaction_with_config(sig, config)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}
```

**Step 2: 修改 DexParser 结构体**

修改 `rpc_client` 字段类型：

```rust
// 原来的
use solana_rpc_client::rpc_client::RpcClient;

// 改为
use crate::common::auto_mock_rpc::AutoMockRpcClient;

pub struct DexParser {
    pub config: ParserConfig,
    // 使用 Arc<dyn RpcClientTrait> 支持多种实现
    rpc_client: Arc<dyn RpcClientTrait>,
    pub parsers: HashMap<String, Arc<dyn DexParserTrait>>,
}
```

**Step 3: 修改构造函数**

```rust
impl DexParser {
    pub fn new(config: ParserConfig) -> Self {
        let rpc_client = Arc::new(RpcClient::new(config.rpc_url.clone())) as Arc<dyn RpcClientTrait>;

        let mut parsers: HashMap<String, Arc<dyn DexParserTrait>> = HashMap::new();
        // ... 解析器注册代码不变 ...

        Self {
            config,
            rpc_client,
            parsers,
        }
    }
}
```

**Step 4: 验证编译**

运行：
```bash
cargo check
```

预期：可能有编译错误（因为我们还没为 AutoMockRpcClient 实现 trait）

**Step 5: 暂不提交，继续下一个任务**

---

### Task 7: 为 AutoMockRpcClient 实现 RpcClientTrait

**Files:**
- Modify: `src/common/auto_mock_rpc.rs`

**Step 1: 添加 trait 实现**

在文件末尾添加：

```rust
//! 需要先导入 DexParser 相关类型
use crate::parser::dex_parser::RpcClientTrait;

impl RpcClientTrait for AutoMockRpcClient {
    fn get_transaction_with_config(
        &self,
        sig: &Signature,
        config: RpcTransactionConfig,
    ) -> Result<EncodedTransactionWithConfigMeta, Box<dyn std::error::Error + Send + Sync>> {
        // 这是一个同步方法签名，但内部需要异步调用
        // 使用 blocking_receive 或者创建 runtime
        use tokio::runtime::Runtime;

        let rt = Runtime::new()
            .map_err(|e| format!("创建 runtime 失败: {}", e))?;

        rt.block_on(self.get_transaction_with_config(sig, config))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}
```

**注意**：这会有问题，因为我们在同步方法中调用异步方法。需要重新设计。

**更好的方案**：修改 DexParser 的 `fetch_and_parse_transaction` 方法为异步，使用 `Arc<AutoMockRpcClient>` 而不是 trait 对象。

让我重新调整设计...

**Step 2: 回滚 trait 方案，使用泛型**

实际上，更简单的方案是保持 DexParser 使用 `Arc<RpcClient>`，新增 `new_with_mock` 返回使用 `AutoMockRpcClient` 的不同实例。

但这样需要修改 `fetch_and_parse_transaction` 的签名...

**最简单的方案**：使用 `Arc<dyn Any>` 和运行时类型检查，或者直接创建两个不同的 DexParser 构造函数，返回不同的内部实现。

**让我们采用最实用的方案**：创建 `DexParserWithMock` 新结构体，或者修改 `DexParser` 使用 `Option<Arc<AutoMockRpcClient>>`。

让我暂停这个任务，重新设计...

---

**重新设计后的方案**：

由于 trait 对象和异步方法的复杂性，我们采用更简单的方案：

### Task 6（重新设计）: 使用 enum 包装 RPC 客户端

**Files:**
- Create: `src/common/rpc_client_wrapper.rs`
- Modify: `src/parser/dex_parser.rs`
- Modify: `src/common/mod.rs`

**Step 1: 创建 RPC 客户端包装器**

创建：`src/common/rpc_client_wrapper.rs`

```rust
//! RPC 客户端包装器
//!
//! 支持标准 RpcClient 和 AutoMockRpcClient

use serde_json::Value;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::EncodedTransactionWithConfigMeta;
use std::{sync::Arc, pin::Pin};

use super::auto_mock_rpc::AutoMockRpcClient;

/// RPC 客户端包装器枚举
pub enum RpcClientWrapper {
    /// 标准 RPC 客户端（生产环境）
    Standard(Arc<solana_rpc_client::rpc_client::RpcClient>),
    /// Auto Mock RPC 客户端（测试环境）
    AutoMock(Arc<AutoMockRpcClient>),
}

impl RpcClientWrapper {
    /// 获取交易（异步）
    pub async fn get_transaction_with_config(
        &self,
        sig: &Signature,
        config: RpcTransactionConfig,
    ) -> Result<EncodedTransactionWithConfigMeta, String> {
        match self {
            RpcClientWrapper::Standard(client) => {
                let client = client.clone();
                let sig = *sig;

                tokio::task::spawn_blocking(move || {
                    client
                        .get_transaction_with_config(&sig, config)
                        .map_err(|e| format!("RPC 调用失败: {}", e))
                })
                .await
                .map_err(|e| format!("任务执行失败: {}", e))?
            }
            RpcClientWrapper::AutoMock(client) => {
                client.get_transaction_with_config(sig, config).await
            }
        }
    }
}
```

**Step 2: 修改 DexParser**

编辑：`src/parser/dex_parser.rs`

```rust
// 添加导入
use crate::common::rpc_client_wrapper::RpcClientWrapper;

pub struct DexParser {
    pub config: ParserConfig,
    /// RPC 客户端包装器
    rpc_client: RpcClientWrapper,
    pub parsers: HashMap<String, Arc<dyn DexParserTrait>>,
}

impl DexParser {
    pub fn new(config: ParserConfig) -> Self {
        let rpc_client = RpcClientWrapper::Standard(
            Arc::new(RpcClient::new(config.rpc_url.clone()))
        );

        let mut parsers: HashMap<String, Arc<dyn DexParserTrait>> = HashMap::new();
        // ... 解析器注册代码不变 ...

        Self {
            config,
            rpc_client,
            parsers,
        }
    }

    /// 使用 Auto Mock 模式创建解析器（测试环境）
    pub fn new_with_mock(config: ParserConfig) -> Self {
        use crate::common::auto_mock_rpc::AutoMockRpcClient;

        let rpc_client = RpcClientWrapper::AutoMock(
            Arc::new(AutoMockRpcClient::new(config.rpc_url))
        );

        let mut parsers: HashMap<String, Arc<dyn DexParserTrait>> = HashMap::new();
        // ... 解析器注册代码不变 ...

        Self {
            config,
            rpc_client,
            parsers,
        }
    }
}
```

**Step 3: 修改 fetch_and_parse_transaction 方法**

编辑：`src/parser/dex_parser.rs`

找到 `fetch_and_parse_transaction` 方法，修改 RPC 调用部分：

```rust
async fn fetch_and_parse_transaction(
    &self,
    signature: &str,
) -> Result<Vec<super::types::ParsedTradeInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let signature = signature.to_string();

    let sig = Signature::from_str(&signature)
        .map_err(|e| format!("无效签名: {}", e))?;

    // 使用 rpc_client 获取交易
    let tx = self.rpc_client.get_transaction_with_config(
        &sig,
        RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        },
    ).await
    .map_err(|e| format!("获取交易失败: {}", e))?;

    let slot = tx.slot;
    let block_time = tx.block_time;

    // ... 后续代码不变 ...
}
```

**Step 4: 添加模块到 mod.rs**

编辑：`src/common/mod.rs`

```rust
pub mod rpc_client_wrapper;
```

**Step 5: 验证编译**

运行：
```bash
cargo check
```

预期：编译通过

**Step 6: 提交**

```bash
git add src/common/rpc_client_wrapper.rs src/parser/dex_parser.rs src/common/mod.rs
git commit -m "✨ feat(parser): DexParser 支持 Auto Mock 模式

- 创建 RpcClientWrapper 包装器
- 支持 Standard 和 AutoMock 两种模式
- 添加 DexParser::new_with_mock() 方法
- 修改 fetch_and_parse_transaction 使用异步 RPC

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 第三部分：编写 AutoMockRpcClient 测试

### Task 7: 创建 AutoMockRpcClient 单元测试

**Files:**
- Create: `tests/auto_mock_rpc_test.rs`

**Step 1: 编写测试文件**

创建：`tests/auto_mock_rpc_test.rs`

```rust
//! Auto Mock RPC 客户端测试
//!
//! 测试 Auto 模式的核心功能

use sol_trade_sdk::common::auto_mock_rpc::AutoMockRpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
};
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
    let result = client.get_transaction_with_config(&sig, config).await;

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
            panic!("测试失败");
        }
    }
}

#[tokio::test]
#[ignore]  // 需要 RPC 节点，手动运行
async fn test_auto_mode_second_call() {
    let client = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());

    let sig = Signature::from_str("5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK").unwrap();

    use solana_client::rpc_config::{RpcTransactionConfig, UiTransactionEncoding};
    use solana_commitment_config::CommitmentConfig;

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    // 第二次调用：从 Mock 文件读取
    let result = client.get_transaction_with_config(&sig, config).await;

    match result {
        Ok(tx) => {
            println!("✅ 第二次调用成功，从 Mock 文件读取");
            println!("   Slot: {}", tx.slot);
        }
        Err(e) => {
            eprintln!("❌ 调用失败: {}", e);
            panic!("测试失败");
        }
    }
}
```

**Step 2: 验证编译**

运行：
```bash
cargo check --test auto_mock_rpc_test
```

预期：编译通过

**Step 3: 运行基础测试**

运行：
```bash
cargo test --test auto_mock_rpc_test test_auto_mock_client_creation -- --nocapture
cargo test --test auto_mock_rpc_test test_generate_file_name -- --nocapture
```

预期：两个基础测试通过

**Step 4: 提交**

```bash
git add tests/auto_mock_rpc_test.rs
git commit -m "🧪 test(mock): 添加 AutoMockRpcClient 单元测试

- 测试客户端创建
- 测试文件名生成
- 测试 Auto 模式的首次和第二次调用

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 第四部分：更新 DEX Parser 测试使用 Mock

### Task 8: 修改 dex_parser_comprehensive.rs 使用 Mock

**Files:**
- Modify: `tests/dex_parser_comprehensive.rs`

**Step 1: 修改测试创建 DexParser 的方式**

编辑：`tests/dex_parser_comprehensive.rs`

找到 `test_all_dex_parsing` 函数，修改 parser 创建：

```rust
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_all_dex_parsing() {
    // 修改前：let parser = DexParser::default();

    // 修改后：
    use sol_trade_sdk::parser::types::ParserConfig;

    let config = ParserConfig {
        rpc_url: "http://127.0.0.1:8899".to_string(),
        verbose: false,
    };
    let parser = DexParser::new_with_mock(config);  // 使用 Mock 模式

    // ... 后续测试代码不变 ...
}
```

**Step 2: 运行测试（首次，录制模式）**

运行：
```bash
cargo test --test dex_parser_comprehensive -- --nocapture
```

预期：
- 测试通过
- 创建 Mock 数据文件到 `tests/mock_data/`
- 首次运行耗时约 37s

**Step 3: 验证 Mock 数据文件**

运行：
```bash
ls -lh tests/mock_data/getTransaction_*.json
```

预期：显示多个 Mock 文件

**Step 4: 运行测试（第二次，应该使用缓存）**

运行：
```bash
cargo test --test dex_parser_comprehensive -- --nocapture
```

预期：
- 测试通过
- 耗时减少到 1-2s
- 不再调用 RPC

**Step 5: 提交**

```bash
git add tests/dex_parser_comprehensive.rs
git commit -m "✅ test(parser): dex_parser_comprehensive 使用 Auto Mock

- 修改测试使用 DexParser::new_with_mock()
- 首次运行：录制 Mock 数据（~37s）
- 后续运行：使用 Mock 数据（~1-2s）
- 性能提升：97%

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

### Task 9: 修改 dex_parser_real_tx.rs 使用 Mock

**Files:**
- Modify: `tests/dex_parser_real_tx.rs`

**Step 1: 修改所有测试函数**

类似 Task 8，修改所有测试创建 parser 的方式：

```rust
async fn test_parse_pumpswap_buy_transaction() {
    use sol_trade_sdk::parser::types::ParserConfig;

    let config = ParserConfig {
        rpc_url: "http://127.0.0.1:8899".to_string(),
        verbose: false,
    };
    let parser = DexParser::new_with_mock(config);

    // ... 测试代码不变 ...
}
```

对所有测试函数重复此修改。

**Step 2: 运行测试验证**

运行：
```bash
TEST_REAL_TRANSACTIONS=1 cargo test --test dex_parser_real_tx -- --nocapture
```

预期：测试通过

**Step 3: 提交**

```bash
git add tests/dex_parser_real_tx.rs
git commit -m "✅ test(parser): dex_parser_real_tx 使用 Auto Mock

- 修改所有测试使用 DexParser::new_with_mock()
- 统一测试工作流
- 提高测试速度和稳定性

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 第五部分：文档更新

### Task 10: 更新 MockRpc 使用指南

**Files:**
- Modify: `docs/MockRpc使用指南.md`

**Step 1: 添加 AutoMockRpcClient 章节**

在文件末尾添加：

```markdown
---

## 🚀 AutoMockRpcClient（智能模式）

### 概述

`AutoMockRpcClient` 是一个智能 Mock RPC 客户端，专为 DEX Parser 和 Pool 查询测试设计。

**特点**：
- 🤖 **智能模式**：有缓存就用，没有就调用 RPC 并保存
- 🎯 **零配置**：无需环境变量，无需手动控制模式
- 🔒 **生产安全**：必须显式调用 `new_with_mock()` 才启用

### 使用方法

```rust
use sol_trade_sdk::parser::{DexParser, types::ParserConfig};

// 测试环境
let config = ParserConfig {
    rpc_url: "http://127.0.0.1:8899".to_string(),
    verbose: false,
};
let parser = DexParser::new_with_mock(config);

// 解析交易（自动使用 Mock）
let result = parser.parse_transaction(signature).await?;
```

### 工作流程

1. **首次运行**：调用 RPC，保存响应到 `tests/mock_data/`
2. **后续运行**：从文件加载，无需 RPC 调用
3. **清理数据**：删除 Mock 文件，自动重新录制

### 与 MockRpcMode 的区别

| 特性 | MockRpcMode | AutoMockRpcClient |
|------|-------------|-------------------|
| 模式 | Record/Replay/Live | Auto（智能） |
| 控制 | 环境变量 `MOCK_MODE` | 显式 API 调用 |
| 用途 | Pool 测试 | DEX Parser、Pool 测试 |
| 工作流 | 手动切换模式 | 自动判断 |

### 文件命名

与 MockRpcMode 共用同一套命名规则：
- 格式：`{method}_{params_hash}.json`
- 目录：`tests/mock_data/`

### 性能提升

| 测试 | 无 Mock | 有 Mock | 提升 |
|------|---------|---------|------|
| dex_parser_comprehensive | 37s | 1-2s | 97% |
| raydium_amm_v4_pool_tests | 54s | 2s | 96% |

### API 参考

#### DexParser

```rust
impl DexParser {
    // 生产环境
    pub fn new(config: ParserConfig) -> Self;

    // 测试环境
    pub fn new_with_mock(config: ParserConfig) -> Self;
}
```

#### AutoMockRpcClient

```rust
impl AutoMockRpcClient {
    pub fn new(rpc_url: String) -> Self;
    pub fn mock_dir(&self) -> &str;

    // RPC 方法
    pub async fn get_transaction_with_config(...) -> Result<...>;
    pub async fn get_program_accounts(...) -> Result<...>;
    pub async fn get_account(...) -> Result<...>;
}
```
```

**Step 2: 提交**

```bash
git add docs/MockRpc使用指南.md
git commit -m "📝 docs(mock): 添加 AutoMockRpcClient 使用说明

- 添加 AutoMockRpcClient 概述和特点
- 说明与 MockRpcMode 的区别
- 添加使用示例和性能对比
- 更新 API 参考

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 第六部分：最终验证和清理

### Task 11: 最终验证

**Step 1: 运行所有相关测试**

运行：
```bash
cargo test --test auto_mock_rpc_test
cargo test --test dex_parser_comprehensive -- --nocapture
TEST_REAL_TRANSACTIONS=1 cargo test --test dex_parser_real_tx -- --nocapture
```

预期：所有测试通过

**Step 2: 检查 Mock 数据文件**

运行：
```bash
ls -lh tests/mock_data/ | wc -l
```

预期：显示 Mock 文件数量

**Step 3: 清理并重新运行测试**

运行：
```bash
rm -rf tests/mock_data/getTransaction_*.json
cargo test --test dex_parser_comprehensive -- --nocapture
```

预期：重新录制 Mock 数据，测试通过

**Step 4: 提交最终版本**

```bash
git add .
git commit -m "✅ feat(mock): DEX Parser Mock 系统实现完成

完成功能：
- AutoMockRpcClient 智能 Mock 客户端
- DexParser 支持 new_with_mock() API
- 所有 DEX Parser 测试使用 Mock
- 完整的单元测试和文档

性能提升：
- dex_parser_comprehensive: 37s → 1-2s (97%)
- 测试稳定性显著提高

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 总结

### 完成的功能

✅ **AutoMockRpcClient 核心**
- 智能 Auto 模式实现
- 支持多个 RPC 方法
- 文件管理和哈希命名

✅ **DexParser 集成**
- RpcClientWrapper 包装器
- new_with_mock() API
- 异步 RPC 调用支持

✅ **测试覆盖**
- AutoMockRpcClient 单元测试
- DEX Parser 集成测试
- 性能验证

✅ **文档**
- 设计文档
- 使用指南
- API 参考

### 性能提升

| 测试 | 原耗时 | 新耗时 | 提升 |
|------|--------|--------|------|
| dex_parser_comprehensive | 37s | 1-2s | 97% |
| dex_parser_real_tx | 20s+ | <1s | 95%+ |

### 后续优化建议

- [ ] 支持更多 RPC 方法
- [ ] 添加 Mock 数据过期机制
- [ ] 提供 Mock 数据管理工具
- [ ] 支持部分匹配（通配符）

---

**实施者请注意**：
- 遵循 TDD 原则：先写测试，再写代码
- 每个任务提交一次 Git
- 遇到问题及时记录和调整
- 保持代码简洁，YAGNI

**相关文档**：
- [设计文档](./2025-01-23-dex-parser-mock-design.md)
- [MockRpc使用指南](../MockRpc使用指南.md)
- [httpmock调研报告](../httpmock调研报告.md)
