# DEX Parser Mock 系统设计

**日期**: 2025-01-23
**作者**: Claude
**状态**: 设计阶段

## 1. 概述

为 DEX Parser 和 Pool 查询测试提供智能 Mock 功能，加速测试并提高稳定性。

### 1.1 目标

- **加速测试**: 减少 `dex_parser_comprehensive.rs` 的 37.40s 运行时间
- **提高稳定性**: 避免因 RPC 节点故障导致的测试失败
- **智能缓存**: 有数据就用，没数据就调用 RPC 并保存
- **生产安全**: 确保生产环境不会误用 Mock 数据

### 1.2 使用场景

- DEX Parser 测试（`dex_parser_comprehensive.rs`, `dex_parser_real_tx.rs`）
- Pool 查询测试（`raydium_amm_v4_pool_tests.rs` 等）
- 任何需要调用 `get_transaction`, `get_program_accounts`, `get_account` 的测试

## 2. 架构设计

### 2.1 核心组件：AutoMockRpcClient

新增独立的 Mock 客户端，实现智能 Auto 模式。

```rust
pub struct AutoMockRpcClient {
    inner: Arc<RpcClient>,
    mock_dir: String,
}

impl AutoMockRpcClient {
    /// 创建新的 Auto Mock 客户端
    pub fn new(rpc_url: String) -> Self {
        Self {
            inner: Arc::new(RpcClient::new(rpc_url)),
            mock_dir: std::env::var("MOCK_DIR")
                .unwrap_or_else(|_| "tests/mock_data".to_string()),
        }
    }

    /// Auto 模式调用核心逻辑
    async fn auto_call<M, P, R>(
        &self,
        method: &str,
        params: &P,
        rpc_call: M,
    ) -> Result<R, String>
    where
        M: FnOnce() -> Result<R, Box<dyn std::error::Error>>,
        P: Serialize,
        R: Serialize + DeserializeOwned,
    {
        let params_json = json!(params);

        // 有缓存就用
        if self.has_mock_data(method, &params_json) {
            return self.load_mock_data(method, &params_json);
        }

        // 没缓存就调用 RPC 并保存
        let result = rpc_call().map_err(|e| e.to_string())?;
        self.save_mock_data(method, &params_json, &result);
        Ok(result)
    }
}
```

### 2.2 支持的 RPC 方法

```rust
impl AutoMockRpcClient {
    /// 获取交易（用于 DEX Parser）
    pub async fn get_transaction_with_config(
        &self,
        sig: &Signature,
        config: RpcTransactionConfig,
    ) -> Result<EncodedTransactionWithConfigMeta, String> {
        self.auto_call(
            "getTransaction",
            &(sig, config),
            || self.inner.get_transaction_with_config(sig, config),
        ).await
    }

    /// 获取程序账户（用于 Pool 列表）
    pub async fn get_program_accounts(
        &self,
        pubkey: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(Pubkey, Account)>, String> {
        self.auto_call(
            "getProgramAccounts",
            &(pubkey, config),
            || self.inner.get_program_accounts_with_config(pubkey, config),
        ).await
    }

    /// 获取账户（用于单个 Pool）
    pub async fn get_account(
        &self,
        pubkey: &Pubkey,
    ) -> Result<Account, String> {
        self.auto_call(
            "getAccountInfo",
            &(pubkey,),
            || self.inner.get_account(pubkey),
        ).await
    }
}
```

### 2.3 与现有 MockRpcMode 的关系

```
┌─────────────────────────────────────────┐
│     MockRpcMode (已有)                  │
│  - Record/Replay/Live 三模式             │
│  - 手动控制，用于 Pool 测试              │
│  - 环境变量: MOCK_MODE                   │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│     AutoMockRpcClient (新增)            │
│  - Auto 智能模式                        │
│  - 自动判断，用于 DEX Parser            │
│  - 显式 API 调用                         │
└─────────────────────────────────────────┘
```

**区别**：
- `MockRpcMode`: 手动切换模式，需要设置 `MOCK_MODE` 环境变量
- `AutoMockRpcClient`: 自动智能模式，无需环境变量控制

**并存原因**：
- Pool 测试习惯用手动控制（录制一批数据，反复重放）
- DEX Parser 测试更适合自动模式（交易签名众多，自动管理）

## 3. DexParser API 设计

### 3.1 构造函数

```rust
impl DexParser {
    /// 生产环境：使用标准 RpcClient
    pub fn new(config: ParserConfig) -> Self {
        let rpc_client = Arc::new(RpcClient::new(config.rpc_url.clone()));
        Self {
            config,
            rpc_client,
            parsers: ...,
        }
    }

    /// 测试环境：使用 Auto Mock RpcClient
    pub fn new_with_mock(config: ParserConfig) -> Self {
        let mock_client = AutoMockRpcClient::new(config.rpc_url);
        Self {
            config,
            rpc_client: Arc::new(mock_client),
            parsers: ...,
        }
    }
}
```

### 3.2 使用示例

```rust
// 生产环境
let parser = DexParser::new(config);

// 测试环境
let parser = DexParser::new_with_mock(config);

// 解析交易（自动使用 Mock）
let result = parser.parse_transaction(signature).await?;
```

**关键**：`parse_transaction` 方法**无需修改**，因为 `AutoMockRpcClient` 实现了与 `RpcClient` 相同的接口。

## 4. 文件存储格式

### 4.1 目录结构

```
tests/mock_data/
├── getTransaction_e71576df0f31c712.json       # DEX Parser
├── getProgramAccounts_a1b2c3d4e5f6g7h8.json  # Pool 列表
├── getAccountInfo_i9j0k1l2m3n4o5p6.json      # 单个 Pool
└── ...
```

**共用目录**：与 Pool 测试的 Mock 数据共用 `tests/mock_data/`

### 4.2 文件命名规则

**格式**: `{method}_{params_hash}.json`

**生成方式**：
```rust
fn generate_file_name(&self, method: &str, params: &Value) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let params_str = params.to_string();
    let mut hasher = DefaultHasher::new();
    params_str.hash(&mut hasher);
    let hash = hasher.finish();

    format!("{}_{:016x}.json", method, hash)
}
```

**优点**：
- 相同的方法和参数总是生成相同的文件名
- 不同的参数生成不同的文件名
- 避免文件名冲突

### 4.3 文件内容格式

```json
{
  "method": "getTransaction",
  "params": [
    "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK",
    {
      "encoding": "jsonParsed",
      "commitment": "confirmed",
      "maxSupportedTransactionVersion": 0
    }
  ],
  "response": {
    "slot": 123456789,
    "blockTime": 1234567890,
    "transaction": { ... }
  }
}
```

## 5. 数据流

### 5.1 首次调用（无缓存）

```
┌──────────────────────────────────────────────┐
│ DexParser::parse_transaction(signature)      │
└──────────────┬───────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────┐
│ AutoMockRpcClient::get_transaction()        │
│  1. 检查缓存: has_mock_data() → false       │
│  2. 调用 RPC: real_rpc_call()               │
│  3. 保存缓存: save_mock_data()              │
└──────────────┬───────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────┐
│ 返回交易数据给 DexParser                    │
└──────────────────────────────────────────────┘
```

### 5.2 后续调用（有缓存）

```
┌──────────────────────────────────────────────┐
│ DexParser::parse_transaction(signature)      │
└──────────────┬───────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────┐
│ AutoMockRpcClient::get_transaction()        │
│  1. 检查缓存: has_mock_data() → true        │
│  2. 加载缓存: load_mock_data()              │
│  (无需 RPC 调用)                            │
└──────────────┬───────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────┐
│ 返回缓存数据给 DexParser                    │
└──────────────────────────────────────────────┘
```

### 5.3 RPC 失败（错误处理）

```
┌──────────────────────────────────────────────┐
│ AutoMockRpcClient::get_transaction()        │
│  1. 检查缓存: has_mock_data() → false       │
│  2. 调用 RPC: real_rpc_call() → Err(e)      │
│  3. 不保存数据，直接返回错误                 │
└──────────────┬───────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────┐
│ 测试失败，显示错误信息                       │
│ ❌ RPC 调用失败: timeout                    │
└──────────────────────────────────────────────┘
```

**策略**：失败不保存，避免保存错误数据

## 6. 测试策略

### 6.1 DEX Parser 测试

**修改前**（37.40s）:
```rust
#[tokio::test]
async fn test_all_dex_parsing() {
    let parser = DexParser::default();  // 每次调用 RPC

    for (dex_name, signature, _) in test_cases {
        let result = parser.parse_transaction(signature).await;
        // 验证...
    }
}
```

**修改后**（预计 1-2s）:
```rust
#[tokio::test]
async fn test_all_dex_parsing() {
    let parser = DexParser::new_with_mock(config);  // 使用 Mock

    for (dex_name, signature, _) in test_cases {
        let result = parser.parse_transaction(signature).await;
        // 首次：调用 RPC 并保存
        // 后续：直接从文件读取
        // 验证...
    }
}
```

### 6.2 Pool 查询测试（可选）

也可以使用 `AutoMockRpcClient`，但现有的 `MockRpcMode` 已经够用。

```rust
// 方式 1：使用现有 MockRpcMode
MOCK_MODE=record cargo test  # 录制
MOCK_MODE=replay cargo test  # 重放

// 方式 2：使用 AutoMockRpcClient（新增）
let mock_client = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());
let pools = mock_client.get_program_accounts(&program_id, config).await?;
```

### 6.3 测试工作流

```bash
# 1. 首次运行：自动从 RPC 获取并保存
cargo test --test dex_parser_comprehensive

# 2. 后续运行：使用缓存数据（快速）
cargo test --test dex_parser_comprehensive

# 3. 清理缓存：重新录制
rm -rf tests/mock_data/getTransaction_*.json
cargo test --test dex_parser_comprehensive

# 4. 临时查看真实 RPC 数据
rm tests/mock_data/getTransaction_XXX.json
cargo test --test dex_parser_comprehensive  # 自动重新获取
```

## 7. 安全性考虑

### 7.1 生产环境保护

**显式 API**：
```rust
// 生产代码
let parser = DexParser::new(config);  // 不会使用 Mock

// 测试代码
let parser = DexParser::new_with_mock(config);  // 显式启用 Mock
```

**无环境变量依赖**：
- `AutoMockRpcClient` 不依赖 `MOCK_MODE` 环境变量
- 必须显式调用 `new_with_mock()` 才启用
- 避免意外在生产环境启用

### 7.2 数据完整性

**参数哈希**：
- 不同的参数（如 encoding、commitment）生成不同的缓存文件
- 避免参数混淆导致的数据错误

**版本控制**：
- Mock 数据提交到 Git 仓库
- 确保测试数据的一致性和可追溯性

## 8. 性能预期

### 8.1 DEX Parser 测试

| 场景 | 预期耗时 | RPC 调用 |
|------|---------|---------|
| 首次（无缓存） | ~37s | ~20 次 |
| 后续（有缓存） | ~1-2s | 0 次 |
| 提升 | **97%** | - |

### 8.2 Pool 测试（可选）

| 场景 | 预期耗时 | RPC 调用 |
|------|---------|---------|
| 首次（无缓存） | ~54s | ~100 次 |
| 后续（有缓存） | ~2s | 0 次 |
| 提升 | **96%** | - |

## 9. 实施计划

### 9.1 实现步骤

1. **实现 AutoMockRpcClient**
   - 核心结构体
   - `auto_call` 通用方法
   - 文件管理方法（has/load/save/generate）

2. **实现 RPC 方法包装**
   - `get_transaction_with_config`
   - `get_program_accounts`
   - `get_account`

3. **修改 DexParser**
   - 添加 `new_with_mock()` 方法
   - 支持 `Arc<AutoMockRpcClient>` 作为 rpc_client

4. **更新测试**
   - 修改 `dex_parser_comprehensive.rs`
   - 修改 `dex_parser_real_tx.rs`
   - 验证测试通过

5. **文档更新**
   - 更新 `docs/MockRpc使用指南.md`
   - 添加 `AutoMockRpcClient` 使用说明

### 9.2 文件清单

**新增文件**：
- `src/common/auto_mock_rpc.rs` - AutoMockRpcClient 实现

**修改文件**：
- `src/common/mod.rs` - 添加 `pub mod auto_mock_rpc;`
- `src/parser/dex_parser.rs` - 添加 `new_with_mock()` 方法
- `src/parser/types.rs` - 修改 `ParserConfig`（如果需要）
- `tests/dex_parser_comprehensive.rs` - 使用 Mock
- `tests/dex_parser_real_tx.rs` - 使用 Mock
- `docs/MockRpc使用指南.md` - 添加 AutoMockRpcClient 说明

**测试文件**：
- `tests/auto_mock_rpc_example.rs` - AutoMockRpcClient 测试（可选）

## 10. 风险和注意事项

### 10.1 潜在风险

1. **参数序列化兼容性**
   - 确保 `(sig, config)` 的序列化结果稳定
   - 建议：使用 `serde_json` 的稳定格式

2. **文件系统权限**
   - 确保 `tests/mock_data/` 目录可写
   - CI 环境需要配置写权限

3. **并发测试**
   - 多个测试同时写入相同文件可能冲突
   - 建议：使用 `serial_test` 保护

### 10.2 注意事项

1. **数据清理**
   - 定期清理过期的 Mock 数据
   - 建议命令：`rm -rf tests/mock_data/getTransaction_*.json`

2. **CI/CD 集成**
   - Mock 数据提交到 Git，CI 中直接使用
   - 避免每次运行都调用 RPC

3. **调试**
   - 想看真实数据时，删除对应的 Mock 文件
   - 无需修改代码，自动重新获取

## 11. 总结

### 11.1 设计原则

- ✅ **零侵入**：生产代码无需修改
- ✅ **显式控制**：`new_with_mock()` 清晰表达意图
- ✅ **自动化**：无需环境变量或配置文件
- ✅ **可扩展**：易于添加新的 RPC 方法
- ✅ **类型安全**：编译期检查

### 11.2 预期收益

- 🚀 **性能提升**：测试时间减少 96-97%
- 🛡️ **稳定性**：避免 RPC 节点故障影响测试
- 💰 **成本降低**：减少 RPC 调用次数
- 📦 **可维护性**：Mock 数据版本可控

### 11.3 后续优化

- [ ] 支持更多 RPC 方法（如 `get_block`）
- [ ] 添加 Mock 数据过期机制
- [ ] 提供 Mock 数据管理工具（列出、清理、统计）
- [ ] 支持部分匹配（如通配符签名）

---

**参考文档**：
- [MockRpc使用指南](../MockRpc使用指南.md)
- [httpmock调研报告](../httpmock调研报告.md)
- [测试优化建议](../测试优化建议.md)
