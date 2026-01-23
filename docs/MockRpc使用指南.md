# Mock RPC 系统使用指南

## 📖 概述

`MockRpcMode` 是一个借鉴 **httpmock** 设计的 Solana RPC Mock 系统，支持三种模式：

- 📼 **Record（录制）**: 从真实 RPC 获取数据并保存到本地文件
- ▶️ **Replay（重放）**: 从本地文件读取数据，无需 RPC 调用
- 📡 **Live（直播）**: 直接调用真实 RPC（默认）

---

## 🚀 快速开始

### 1. 基本使用

```rust
use sol_trade_sdk::common::mock_rpc::MockRpcMode;

// 创建 Mock RPC 客户端
let mock_rpc = MockRpcMode::new();

// 使用 mock_rpc 就像使用普通的 RpcClient 一样
// （因为实现了 Deref trait）

// 示例：获取账户信息
let account = mock_rpc.get_account(&pubkey).await.unwrap();
```

### 2. 模式控制

通过环境变量 `MOCK_MODE` 控制模式：

```bash
# 录制模式：从真实 RPC 获取数据并保存
MOCK_MODE=record cargo test --test pool_tests

# 重放模式：从本地文件读取数据（推荐用于 CI）
MOCK_MODE=replay cargo test --test pool_tests

# 直播模式：直接调用真实 RPC（默认）
MOCK_MODE=live cargo test --test pool_tests
# 或
cargo test --test pool_tests
```

---

## 📁 Mock 数据存储

### 存储位置

默认：`tests/mock_data/`

可通过环境变量 `MOCK_DIR` 自定义：

```bash
MOCK_DIR=/path/to/mock_data cargo test
```

### 文件命名规则

格式：`{method}_{params_hash}.json`

示例：
```
getProgramAccounts_e71576df0f31c712.json
getAccountInfo_9fcaa456c18cbbb0.json
```

**优点**：
- ✅ 相同的方法名和参数总是生成相同的文件名
- ✅ 不同的参数生成不同的文件名
- ✅ 避免文件名冲突

### 文件内容格式

```json
{
  "method": "getProgramAccounts",
  "params": [
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
    {
      "dataSlice": {"offset": 1, "length": 2}
    }
  ],
  "response": {
    "context": {"slot": 123456},
    "value": [...]
  }
}
```

---

## 🎯 使用场景

### 场景 1: Pool 查询测试（慢测试优化）

**问题**：`list_pools_by_mint` 需要扫描数百个 Pool，非常慢

**解决方案**：使用 Mock 数据

```bash
# 1. 首次运行：录制模式
MOCK_MODE=record cargo test --test raydium_amm_v4_pool_tests -- --nocapture

# 2. 后续运行：重放模式（超快！）
MOCK_MODE=replay cargo test --test raydium_amm_v4_pool_tests -- --nocapture
```

**效果**：
- 录制：54s → 保存到文件
- 重放：54s → 2s（减少 96%！）

### 场景 2: 交易解析器测试

**问题**：每次测试都需要解析真实交易

**解决方案**：录制一次，重复使用

```bash
# 录制
MOCK_MODE=record cargo test --test dex_parser_comprehensive

# 重放
MOCK_MODE=replay cargo test --test dex_parser_comprehensive
```

### 场景 3: CI/CD 加速

**问题**：CI 中运行测试太慢

**解决方案**：使用 Mock 数据，无需 RPC 调用

```yaml
# .github/workflows/test.yml
- name: Run tests with Mock
  env:
    MOCK_MODE: replay
  run: cargo test --workspace
```

---

## 📝 实际测试示例

### 示例 1: 基本 Pool 查询测试

```rust
use sol_trade_sdk::common::mock_rpc::MockRpcMode;
use sol_trade_sdk::instruction::utils::raydium_amm_v4::get_pool_by_address;
use solana_sdk::pubkey::Pubkey;

#[tokio::test]
async fn test_pool_query_with_mock() {
    // 创建 Mock RPC（根据 MOCK_MODE 自动选择模式）
    let mock_rpc = MockRpcMode::new();

    // 正常调用（与生产代码相同）
    let pool_address = Pubkey::from_str("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2").unwrap();

    let pool_info = get_pool_by_address(&mock_rpc, &pool_address).await.unwrap();

    // 验证结果
    assert_eq!(pool_info.status, 6);
    assert_eq!(pool_info.coin_mint.to_string(), "So11111111111111111111111111111111111112");

    println!("✅ 测试通过");
}
```

### 示例 2: 手动保存 Mock 数据

```rust
use sol_trade_sdk::common::mock_rpc::MockRpcMode;
use serde_json::json;

#[tokio::test]
async fn test_manual_save_mock_data() {
    let mut mock_rpc = MockRpcMode::new_with_mode(
        "http://127.0.0.1:8899".to_string(),
        MockMode::Record,
    );

    // 手动保存 Mock 数据
    let method = "getAccountInfo";
    let params = json!(["H7R2KBXrMhjTFmHwXYG6mCtEUAwq8Y5EYjV8YNJrz8L"]);
    let response = json!({
        "context": {"slot": 123456},
        "value": {
            "data": ["base64data", "base64"],
            "owner": "program123",
            "lamports": 1000000
        }
    });

    mock_rpc.save_recording(method, &params, &response);

    println!("✅ Mock 数据已保存");
}
```

### 示例 3: 检查 Mock 数据是否存在

```rust
use sol_trade_sdk::common::mock_rpc::MockRpcMode;

#[test]
fn test_check_mock_data_exists() {
    let mock_rpc = MockRpcMode::new();

    let method = "getAccountInfo";
    let params = serde_json::json!(["pubkey123"]);

    if mock_rpc.has_mock_data(method, &params) {
        println!("✅ Mock 数据存在");
    } else {
        println!("❌ Mock 数据不存在");
    }
}
```

---

## 🔧 高级用法

### 1. 自定义 Mock 数据目录

```rust
use sol_trade_sdk::common::mock_rpc::MockRpcMode;

let mock_rpc = MockRpcMode::new_with_url(
    "http://127.0.0.1:8899".to_string(),
);
mock_rpc.mock_dir = "/custom/mock/path".to_string();
```

### 2. 使用特定模式

```rust
use sol_trade_sdk::common::mock_rpc::{MockMode, MockRpcMode};

// 强制使用录制模式
let mock_rpc = MockRpcMode::new_with_mode(
    "http://127.0.0.1:8899".to_string(),
    MockMode::Record,
);

// 强制使用重放模式
let mock_rpc = MockRpcMode::new_with_mode(
    "http://127.0.0.1:8899".to_string(),
    MockMode::Replay,
);
```

### 3. 清理 Mock 数据

```rust
use sol_trade_sdk::common::mock_rpc::MockRpcMode;

let mock_rpc = MockRpcMode::new();

// 清理所有 Mock 数据
mock_rpc.clear_mock_data();
```

---

## ⚙️ 配置选项

### 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `MOCK_MODE` | Mock 模式（record/replay/live） | live |
| `RPC_URL` | RPC 节点地址 | http://127.0.0.1:8899 |
| `MOCK_DIR` | Mock 数据目录 | tests/mock_data |

### Cargo.toml 配置

无需额外配置！MockRpcMode 已经集成在 `sol-trade-sdk` 中。

---

## 📊 性能对比

### raydium_amm_v4_pool_tests

| 模式 | 耗时 | RPC 调用 |
|------|------|---------|
| **Live** | 53.93s | ~100 次 |
| **Record** | 54.12s | ~100 次 + 保存文件 |
| **Replay** | 2.15s | 0 次（从文件读取） |

**提升**: 53.93s → 2.15s（**减少 96%**）🚀

### dex_parser_comprehensive

| 模式 | 耗时 | RPC 调用 |
|------|------|---------|
| **Live** | 37.40s | ~20 次 |
| **Record** | 37.55s | ~20 次 + 保存文件 |
| **Replay** | 1.02s | 0 次（从文件读取） |

**提升**: 37.40s → 1.02s（**减少 97%**）🚀

---

## 🎓 最佳实践

### 1. 开发工作流

```bash
# 1. 开发阶段：使用 Live 模式
cargo test --test my_test

# 2. 录制 Mock 数据（只需一次）
MOCK_MODE=record cargo test --test my_test -- --nocapture

# 3. 后续测试：使用 Replay 模式
MOCK_MODE=replay cargo test --test my_test
```

### 2. CI/CD 配置

```yaml
# .github/workflows/test.yml
name: Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Run tests with Mock
        env:
          MOCK_MODE: replay
        run: cargo test --workspace
```

### 3. 调试技巧

```bash
# 查看 Mock 模式
cargo test --test my_test -- --nocapture | grep "当前模式"

# 查看 Mock 文件位置
ls tests/mock_data/

# 清理并重新录制
rm -rf tests/mock_data/
MOCK_MODE=record cargo test --test my_test -- --nocapture
```

---

## 🐛 故障排除

### 问题 1: Mock 数据不存在

**错误信息**：
```
❌ Mock 数据文件不存在: tests/mock_data/getProgramAccounts_abc123.json
```

**解决方案**：
```bash
# 重新录制 Mock 数据
MOCK_MODE=record cargo test --test my_test -- --nocapture
```

### 问题 2: Mock 数据格式错误

**错误信息**：
```
❌ 解析 Mock 数据失败: expected value at line 1 column 1
```

**解决方案**：
```bash
# 删除损坏的 Mock 文件
rm tests/mock_data/*.json

# 重新录制
MOCK_MODE=record cargo test --test my_test -- --nocapture
```

### 问题 3: 参数不匹配

**错误信息**：
```
❌ Mock 数据文件不存在
```

**原因**：参数已更改，需要重新录制

**解决方案**：
```bash
# 清理旧的 Mock 数据并重新录制
rm tests/mock_data/*.json
MOCK_MODE=record cargo test --test my_test -- --nocapture
```

---

## 📚 API 参考

### MockMode 枚举

```rust
pub enum MockMode {
    Record,  // 录制模式
    Replay,  // 重放模式
    Live,    // 直播模式
}
```

### MockRpcMode 结构

```rust
pub struct MockRpcMode {
    pub mode: MockMode,
    pub mock_dir: String,
    // inner: RpcClient (私有)
}

impl MockRpcMode {
    pub fn new() -> Self;
    pub fn new_with_url(rpc_url: String) -> Self;
    pub fn new_with_mode(rpc_url: String, mode: MockMode) -> Self;

    pub fn mode(&self) -> MockMode;
    pub fn mock_dir(&self) -> &str;

    pub fn has_mock_data(&self, method: &str, params: &Value) -> bool;
    pub fn save_recording(&self, method: &str, params: &Value, response: &Value);
    pub fn load_recording(&self, method: &str, params: &Value) -> Result<Value, String>;
    pub fn clear_mock_data(&self);

    pub fn generate_file_name(&self, method: &str, params: &Value) -> String;
}

impl Deref for MockRpcMode {
    type Target = RpcClient;
    // 可以像 RpcClient 一样使用
}
```

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
| dex_parser_comprehensive | 14.51s | 0.18s | 98.76% |
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
}
```

### 实际测试示例

```rust
use sol_trade_sdk::parser::{DexParser, types::ParserConfig};

#[tokio::test]
async fn test_dex_parser_with_mock() {
    let config = ParserConfig {
        rpc_url: "http://127.0.0.1:8899".to_string(),
        verbose: false,
    };

    // 使用 Mock 模式
    let parser = DexParser::new_with_mock(config);

    // 首次运行：从 RPC 获取并保存（14.51s）
    // 后续运行：从文件读取（0.18s）
    let result = parser.parse_transaction(signature).await.unwrap();

    assert!(result.success);
    assert!(!result.trades.is_empty());
}
```

### 测试工作流

```bash
# 1. 首次运行：自动录制 Mock 数据
cargo test --test dex_parser_comprehensive

# 2. 后续运行：使用缓存数据（快速）
cargo test --test dex_parser_comprehensive

# 3. 清理缓存：重新录制
rm -rf tests/mock_data/getTransaction*.json
cargo test --test dex_parser_comprehensive

# 4. 临时查看真实 RPC 数据
rm tests/mock_data/getTransaction_XXX.json
cargo test --test dex_parser_comprehensive  # 自动重新获取
```

---

## ✅ 总结

### 优点

- ✅ **零侵入式修改**：测试代码不需要修改
- ✅ **简单易用**：一个环境变量即可控制
- ✅ **性能提升巨大**：减少 96-97% 的测试时间
- ✅ **借鉴成熟设计**：参考 httpmock 的 Recording & Playback
- ✅ **智能自动化**：AutoMockRpcClient 自动判断使用缓存或 RPC

### 下一步

1. ✅ 基础功能已完成
2. ✅ AutoMockRpcClient 智能模式已实现
3. ⏭️ 开始在实际测试中使用
4. ⏭️ 录制常用的 Pool 数据
5. ⏭️ 优化 CI/CD 流程

**参考文档**：
- [httpmock 调研报告](/opt/projects/sol-trade-sdk/docs/httpmock调研报告.md)
- [侵入式修改分析](/opt/projects/sol-trade-sdk/docs/Mock数据侵入式修改分析.md)
- [测试优化建议](/opt/projects/sol-trade-sdk/docs/测试优化建议.md)
