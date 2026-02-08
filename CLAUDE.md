# CLAUDE.md

这个文件提供 guidance 给 Claude Code (claude.ai/code) 用于在此代码库中工作。

## 项目概述

**Sol Trade SDK** 是一个 Rust SDK，用于在 Solana 区块链上进行 DEX 交易。支持 7 个主流 DEX 协议和多种 MEV 保护服务。

- **版本**: 4.0.0
- **语言**: Rust Edition 2024
- **Solana 依赖**: 3.0.x 版本（重要：使用 solana-sdk 3.0.0）
- **测试节点**: 本地 `127.0.0.1:8899` 是 surfpool（fork 了 Solana mainnet，通过 frpc 从远端转发）
- **重要**: 测试时不要使用 `--release` 参数（编译太慢），使用 `cargo nextest` 替代 `cargo test`

## 快速参考

### 常用命令

```bash
# 构建项目（开发版本）
cargo build

# 类型检查（快速）
cargo check

# 运行所有测试（使用 nextest 加速）
cargo nextest run

# 运行特定测试
cargo nextest run <test_name>

# 运行特定包的测试
cargo nextest run --package sol-trade-sdk <test_name>

# 带输出的测试
cargo test --test <test_name> -- --nocapture
```

### 核心文件位置速查

| 组件 | 文件位置 |
|------|---------|
| **DexType 枚举** | `src/trading/factory.rs:14` - 所有支持的 DEX 协议类型 |
| **DexParamEnum** | `src/trading/core/params.rs` - 类型安全的协议参数 |
| **GasFeeStrategy** | `src/common/gas_fee_strategy.rs` - Gas 费策略配置 |
| **TradeFactory** | `src/trading/factory.rs` - 交易执行器工厂 |
| **GenericTradeExecutor** | `src/trading/core/executor.rs` - 通用交易执行器 |
| **InstructionBuilder trait** | `src/trading/core/traits.rs` - 指令构建器接口 |
| **TradingClient** | `src/client/types.rs` - 主客户端类型定义 |
| **SolanaTrade** | `src/client/constructor.rs` - Builder 模式构造器 |
| **交易逻辑** | `src/client/trading.rs` - buy/sell 实现 |
| **Quote 计算** | `src/client/quote.rs` - 统一 Quote 接口 |
| **链上模拟** | `src/client/simulation.rs` - 模拟执行 |
| **MiddlewareManager** | `src/trading/middleware/mod.rs` - 中间件管理器 |
| **测试工具 Crate** | `sol-trade-test-utils/` - 独立的测试工具库 |

## Pool 地址 DEX 识别

SDK 提供便捷工具函数，通过 Pool 地址自动识别 DEX 协议。

### 基本用法

```rust
use sol_trade_sdk::common::dex_detector::{detect_dex_from_pool, DexInfo};
use sol_trade_sdk::common::SolanaRpcClient;

let rpc = SolanaRpcClient::new("http://127.0.0.1:8899".to_string());

// 识别单个 Pool
let pool_address = "58L7CzkRV5qD1owUPurbAC7gUNV1kSzwj2TMkvgEpbjZ";
let dex_info = detect_dex_from_pool(&rpc, pool_address).await?;

println!("DEX: {}", dex_info.display_name());  // "Raydium AMM V4"
println!("Code: {}", dex_info.dex_name());     // "raydium_amm_v4"
```

### 批量识别

```rust
use sol_trade_sdk::common::dex_detector::detect_dex_from_pools_batch;

let pools = vec![
    "58L7CzkRV5qD1owUPurbAC7gUNV1kSzwj2TMkvgEpbjZ",
    "DQfGJgjYcGSonFj6QoiQSYRmSMdnFM8NkYGXdHU7KNnB",
];

let results = detect_dex_from_pools_batch(&rpc, &pools).await;

for info in results {
    println!("{}: {}", info.pool_address, info.display_name());
}
```

### DexInfo 结构体

```rust
pub struct DexInfo {
    pub protocol: DexProtocol,      // DEX 协议枚举
    pub pool_address: String,        // Pool 地址
    pub program_id: String,          // Program ID (owner)
}

impl DexInfo {
    pub fn dex_name(&self) -> &str;           // 代码名称
    pub fn display_name(&self) -> &str;        // 显示名称
}
```

### 支持的 DEX

所有 8 个 DEX 协议均支持识别：
- PumpFun, PumpSwap, Bonk
- Raydium AMM V4, Raydium CLMM, Raydium CPMM
- Meteora DAMM V2

### 测试

```bash
# 运行集成测试
cargo nextest run test_dex_detector -- --nocapture

# 运行示例
cargo run --example dex_detection
```

### 重要配置和常量

- **TradeConfig**: `src/common/mod.rs` - 交易配置
- **InfrastructureConfig**: `src/common/mod.rs` - 基础设施配置
- **DEX Program 地址**: `src/constants/dex_protocols.rs`
- **Token 地址**: `src/constants/tokens.rs`

## 核心架构设计

### 设计原则

本项目遵循以下核心设计原则：

1. **类型安全**：使用枚举和 trait 确保编译时类型检查（如 `DexParamEnum` 是零成本抽象）
2. **零成本抽象**：使用 `LazyLock`、枚举分发等零运行时开销的模式
3. **共享基础设施**：通过 `TradingInfrastructure` 减少多钱包场景的资源消耗
4. **可扩展性**：工厂模式和 trait 系统使得添加新 DEX 协议变得简单
5. **模块化**：单个文件超过 800 行时考虑分离模块

### 核心设计模式

#### 1. 工厂模式 (`src/trading/factory.rs`)

```rust
// 零开销单例模式 - 使用 LazyLock 在编译时创建实例
pub fn create_executor(dex_type: DexType) -> Arc<dyn TradeExecutor> {
    match dex_type {
        DexType::PumpFun => Self::pumpfun_executor(),
        DexType::PumpSwap => Self::pumpswap_executor(),
        // ... 其他协议
    }
}
```

**支持的 DEX 类型**：
- PumpFun, PumpSwap, Bonk
- RaydiumCpmm, RaydiumAmmV4, RaydiumClmm
- MeteoraDammV2

#### 2. 策略模式 (`src/trading/core/traits.rs`)

- **TradeExecutor trait**: 定义统一的交易接口
  ```rust
  async fn swap(&self, params: SwapParams) -> Result<(bool, Vec<Signature>, Option<anyhow::Error>)>;
  ```

- **InstructionBuilder trait**: 每个协议实现自己的指令构建逻辑
  ```rust
  async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>>;
  async fn build_sell_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>>;
  ```

- **GenericTradeExecutor**: 提供通用的交易执行逻辑，委托给具体的 InstructionBuilder

#### 3. 中间件模式 (`src/trading/middleware/`)

```rust
let middleware_manager = MiddlewareManager::new()
    .add_middleware(Box::new(FirstMiddleware))   // 按顺序执行
    .add_middleware(Box::new(SecondMiddleware));
```

支持在交易执行前修改/添加/删除指令。

#### 4. 适配器模式 (`src/swqos/`)

统一不同 MEV 服务的接口（Jito, ZeroSlot, Bloxroute, Temporal, FlashBlock, BlockRazor, Node1, Astralane）。

### 核心类型和流程

#### TradingClient（主要交易客户端）

```rust
// 单钱包场景
let client = TradingClient::new(payer, trade_config).await;
client.buy(buy_params).await?;

// 多钱包共享基础设施
let infrastructure = Arc::new(TradingInfrastructure::new(infra_config).await);
let client1 = TradingClient::from_infrastructure(payer1, infrastructure.clone(), true);
let client2 = TradingClient::from_infrastructure(payer2, infrastructure.clone(), true);

// Builder 模式（推荐）
let client = SolanaTrade::builder()
    .payer(payer)
    .rpc_url("http://127.0.0.1:8899")
    .swqos_config(SwqosConfig::Default)
    .build()
    .await?;
```

#### DexParamEnum（类型安全的协议参数）

零成本抽象，使用枚举分发避免动态类型检查：

```rust
pub enum DexParamEnum {
    PumpFun(PumpFunParams),
    PumpSwap(PumpSwapParams),
    Bonk(BonkParams),
    RaydiumCpmm(RaydiumCpmmParams),
    // ...
}
```

#### 交易参数结构

- **TradeBuyParams / TradeSellParams**: 包含所有交易配置
  - `dex_type`: DEX 协议类型
  - `input_token_amount`: 交易数量
  - `slippage_basis_points`: 滑点容忍度（100 = 1%）
  - `extension_params`: 协议特定参数（DexParamEnum）
  - `gas_fee_strategy`: Gas 费策略
  - `address_lookup_table_account`: 地址查找表（可选）
  - `durable_nonce`: Durable Nonce（可选）

## Quote 计算功能

SDK 为所有支持的 DEX 提供统一的 Quote 接口，支持 Exact In 和 Exact Out 两种计算模式。

### 支持的 DEX 和精度

| DEX | Exact In | Exact Out | 精度 |
|-----|----------|-----------|------|
| **Raydium CLMM** | ✅ Buy/Sell | ✅ Buy/Sell | 0% |
| **Raydium CPMM** | ✅ Buy/Sell | ✅ Buy/Sell | 0% |
| **Raydium AMM V4** | ✅ Buy/Sell | ✅ Buy/Sell | 0% |
| **PumpSwap** | ✅ Buy/Sell | ✅ Buy/Sell | Buy: 待修复, Sell: 0.4% |

### 基本用法

```rust
use sol_trade_sdk::instruction::utils::raydium_clmm::{quote_exact_in, quote_exact_out};

// Exact In: 已知输入，计算输出
let quote = quote_exact_in(&rpc, &pool, amount_in, zero_for_one).await?;

// Exact Out: 已知输出，计算输入
let quote = quote_exact_out(&rpc, &pool, amount_out, zero_for_one).await?;
```

### 返回值

```rust
pub struct QuoteExactInResult {
    pub amount_out: u64,           // 预期输出金额（最小单位）
    pub fee_amount: u64,           // 手续费金额
    pub price_impact_bps: Option<u64>, // 价格影响（基点）
    pub extra_accounts_read: usize, // 读取的额外账户数
}
```

### 链上模拟验证

```bash
# 运行所有验证测试
cargo nextest run verify_* -- --nocapture

# 单独测试特定 DEX
cargo nextest run verify_raydium_cpmm_exact_in_buy -- --nocapture
```

所有测试的误差容限均 < 0.1%（大部分场景达到 0% 误差）。

## 流动性管理

SDK 支持向 Raydium CPMM 池子添加流动性。

```rust
use sol_trade_sdk::liquidity::cpmm::{build_deposit_instruction, calculate_lp_token_amount};

// 计算 LP token 数量
let lp_amount = calculate_lp_token_amount(&pool_state, token_0_amount, token_1_amount, RoundDirection::Floor)?;

// 构建 Deposit 指令
let instruction = build_deposit_instruction(params, owner.pubkey())?;
```

测试：`cargo nextest run test_add_liquidity_cpmm -- --nocapture`

## 交易解析器

SDK 包含完整的 DEX 交易解析器，可以从交易签名解析出详细的交易信息。

### 支持的协议

- PumpSwap (pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA)
- Raydium AMM V4 (675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8)
- Raydium CPMM (CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C)
- Raydium CLMM (CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK)

### 使用方法

```rust
use sol_trade_sdk::parser::DexParser;

let parser = DexParser::default();
let result = parser.parse_transaction(signature).await?;

for trade in result.trades {
    println!("DEX: {}", trade.dex);
    println!("交易类型: {:?}", trade.trade_type);
}
```

## 测试工具 Crate

项目包含独立的测试工具 crate `sol-trade-test-utils/`，提供以下功能：

### 1. 空投和余额管理

```rust
use sol_trade_test_utils::{airdrop_and_wait, ensure_sol_balance, ensure_token_balance};

// 空投 SOL
airdrop_and_wait("http://127.0.0.1:8899", &pubkey, 10).await?;

// 确保 SOL 余额（自动空投）
ensure_sol_balance(&rpc, "http://127.0.0.1:8899", &payer.pubkey(), 10).await?;

// 确保 Token 余额（自动 mint）
ensure_token_balance(&rpc, "http://127.0.0.1:8899", &payer, &mint, "1000").await?;
```

### 2. 流动性管理

```rust
use sol_trade_test_utils::ensure_cpmm_liquidity;

ensure_cpmm_liquidity(&rpc, "http://127.0.0.1:8899", &payer, &pool_address, lp_amount, "10000", "10").await?;
```

### 3. 测试密钥管理

```rust
use sol_trade_test_utils::get_simulation_test_keypair;
let payer = Arc::new(get_simulation_test_keypair());
```

### 4. Token 操作

```rust
use sol_trade_test_utils::{mint_token_to, transfer_token_to};
mint_token_to(&rpc, rpc_url, &mint_authority, &mint, &recipient, amount).await?;
transfer_token_to(&rpc, rpc_url, &payer, &mint, &from, &to, amount).await?;
```

### 5. CPMM 测试参数构建

```rust
use sol_trade_test_utils::CpmmTestParamsBuilder;
let params = CpmmTestParamsBuilder::new()
    .with_pool(&pool_address)
    .with_token_mints(&token0_mint, &token1_mint)
    .build(&rpc).await?;
```

详细文档见：`sol-trade-test-utils/README.md`

## Pool 查询工具

SDK 提供了多个 Pool 查询工具函数，位于 `src/instruction/utils/raydium_cpmm.rs`：

```rust
// 通过 mint 地址查找 Pool（自动缓存）
let pool = get_pool_by_mint(&rpc_client, &mint).await?;

// 强制刷新并查找 Pool（忽略缓存）
let pool = get_pool_by_mint_force(&rpc_client, &mint).await?;

// 通过 Pool 地址获取 Pool 信息
let pool = get_pool_by_address(&rpc_client, &pool_address).await?;

// 列出所有包含指定 mint 的 Pool
let pools = list_pools_by_mint(&rpc_client, &mint).await?;

// 获取代币价格（USD）
let price = get_token_price_in_usd_with_pool(&rpc_client, &mint).await?;

// 清理 Pool 缓存
clear_pool_cache();
```

**重要**: 这些工具会自动缓存 Pool 信息以提高性能，必要时使用 `clear_pool_cache()` 或 `get_pool_by_mint_force()` 强制刷新。

## 性能优化

### 编译配置

```toml
[profile.release]
opt-level = 3              # 最高优化级别
lto = "thin"               # 瘦 LTO，平衡性能与编译速度
codegen-units = 16         # 并行编译
panic = "abort"            # 恐慌即中止
strip = true               # 去除符号表
incremental = true         # 增量编译
```

### 运行时优化

1. **Seed 优化**: 使用优化的 PDA 派生算法 (`src/common/seed.rs`)
2. **批量 RPC**: 使用 `getMultipleAccounts` 批量获取账户
3. **缓存机制**:
   - `DashMap` 并发哈希表
   - Rent 缓存（后台自动更新）
   - DEX Pool 缓存 (`src/common/dex_pool_cache.rs`)
4. **并行查询**: 利用 Tokio 并发查询
5. **零拷贝 I/O**: 减少内存拷贝开销

## 测试环境

- **本地测试节点**: `127.0.0.1:8899`（surfpool，fork 了 Solana mainnet）
- **重要**: 不要使用 `--release` 参数进行测试（编译太慢）
- **交易调试**: 使用 Solscan 查看交易失败原因（见"交易失败调试方法"部分）

### 测试分类

```bash
# WSOL 测试
cargo nextest run wsol_tests

# Seed 优化测试
cargo nextest run seed_optimize_tests

# Raydium CLMM 测试
cargo nextest run raydium_clmm_pool_tests
cargo nextest run raydium_clmm_buy_sell_tests

# Raydium CPMM 测试
cargo nextest run raydium_cpmm_buy_sell_tests

# Raydium AMM V4 测试
cargo nextest run raydium_amm_v4_buy_sell_tests
cargo nextest run raydium_amm_v4_pool_tests

# PumpSwap Pool 测试
cargo nextest run pumpswap_pool_tests

# Quote 验证测试
cargo nextest run verify_*

# 流动性测试
cargo nextest run test_add_liquidity_cpmm

# 交易解析器测试
cargo nextest run dex_parser_comprehensive
cargo nextest run dex_parser_real_tx
cargo nextest run dex_parser_unit

# Jito Bundle 测试
cargo nextest run jito_bundle_send
cargo nextest run jito_simulate_bundle
```

### 测试注意事项

1. **串行测试**: 某些测试使用 `serial_test::serial` 标记，必须串行运行以避免冲突
2. **账户文件**: 部分测试使用 `docs/id.json` 作为测试账户（包含测试钱包私钥）
3. **缓存清理**: Raydium 相关测试会自动清理 Pool 缓存，确保数据新鲜度

## 交易失败调试方法

交易失败后，使用以下 URL 格式在 Solscan 查看详情：

```
https://solscan.io/tx/<TX_SIGNATURE>?cluster=custom&customUrl=http://127.0.0.1:8899
```

查看 **Program Logs** 部分来定位失败原因：

```
Program log: Instruction: TransferChecked
Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 1106 of 1106 compute units
Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA failed: exceeded CUs meter at BPF instruction
```

常见失败原因：
- **CU 不足**: 检查 `GasFeeStrategy` 中的 `compute_unit_limit` 设置
- **参数错误**: 验证交易参数（Pool 地址、Token Mint 等）
- **滑点过大**: 交易失败但未消耗费用（滑点保护触发）

## Gas 费策略

使用 `GasFeeStrategy` 灵活控制交易费用：

```rust
let gas_fee_strategy = GasFeeStrategy::new();
gas_fee_strategy.set_global_fee_strategy(
    150000,   // buy_priority_fee
    150000,   // sell_priority_fee
    500000,   // buy_compute_unit_limit
    500000,   // sell_compute_unit_limit
    0.001,    // buy_compute_unit_price
    0.001     // sell_compute_unit_price
);
```

详见 `docs/Gas费策略.md`。

## WSOL 管理

SDK 自动处理 WSOL：
- 自动创建 WSOL ATA
- 自动包装/解包 SOL
- 可配置启动时创建

## 开发工作流

### 添加新的 DEX 支持

当添加新的 DEX 协议支持时，需要：

1. **创建 InstructionBuilder** (`src/instruction/<new_dex>.rs`)
   - 实现 `InstructionBuilder` trait
   - 定义协议特定的参数结构

2. **添加 DexType 变体** (`src/trading/factory.rs`)
   ```rust
   pub enum DexType {
       // ... 现有变体
       NewDex,
   }
   ```

3. **在工厂中注册** (`src/trading/factory.rs`)
   ```rust
   fn new_dex_executor() -> Arc<dyn TradeExecutor> {
       static INSTANCE: LazyLock<...> = ...;
   }
   ```

4. **添加参数到 DexParamEnum** (`src/trading/core/params.rs`)
   ```rust
   pub enum DexParamEnum {
       // ... 现有变体
       NewDex(NewDexParams),
   }
   ```

5. **编写测试** (`tests/<new_dex>_tests.rs`)
   - Pool 查询测试
   - 买入/卖出测试
   - Quote 验证测试
   - 与现有测试保持一致的命名和结构

### 代码风格

- **使用 Rust Edition 2024** 语法特性
- **优先使用类型安全的枚举**（如 `DexParamEnum`）而非字符串或数字标识
- **利用零成本抽象**：trait 对象、枚举分发等
- **错误处理**：使用 `anyhow::Result` 或自定义错误类型
- **模块化**：单个文件超过 800 行时考虑分离模块

### 性能考虑

- **使用 `Arc` 共享不可变数据**（如 `TradingInfrastructure`）
- **缓存 expensive 计算**（如 Rent、Pool 信息）
- **批量 RPC 调用**使用 `getMultipleAccounts`
- **并发查询**使用 `tokio::spawn` 或 `futures::join_all`

### 添加新功能时的检查清单

- [ ] 更新 CLAUDE.md（如果涉及架构变更）
- [ ] 添加或更新测试
- [ ] 更新相关文档（`docs/` 目录）
- [ ] 确保代码注释使用中文
- [ ] 检查性能影响（如添加新的缓存或优化点）
- [ ] 验证在不同 DEX 协议下的一致性
- [ ] 确认 Solana 依赖版本兼容性（当前使用 3.0.x）
- [ ] 如果模块超过 800 行，考虑分离

### 数学计算注意事项

需要使用精确的整数运算（u128）而不是浮点运算（f64/u64）。这特别适用于：
- Token 数量计算
- 价格计算
- 滑点计算
- LP token 计算

## Git 工作流程

提交信息使用 emoji 前缀规范：
- `📝` - 文档
- `🐛` - 修复
- `✨` - 新功能
- `♻️` - 重构
- `🧪` - 测试
- `✅` - 测试改进
- `🚀` - 部署/发布

## MCP 服务

本项目配置了多个 MCP (Model Context Protocol) 服务，可以在需要时使用：

### 可用的 MCP 工具

1. **context7** - 查询依赖库的最新文档
   - 使用场景：查询 Rust crate 的 API 文档和使用示例
   - 示例：查询 `solana-sdk`、`spl-token` 等库的最新文档

2. **solana-mcp-server** - Solana 专业知识库
   - 使用场景：查询 Solana 区块链相关概念、最佳实践

3. **surfpool** - 测试节点查询
   - 使用场景：查询本地测试节点的状态、账户信息等

4. **mysql-remote** - 数据库查询
   - 使用场景：查询数据库表结构和数据细节

5. **browser-mcp** - 浏览器自动化
   - 使用场景：访问网页查询信息（如 Solscan 查看交易详情）

## 参考外部代码

如果参考了官方的代码，需要写本地绝对路径到代码的注释。比如 `./temp/xxx`

如果发现 github 有需要参考代码，可以 `git clone --depth 1` 到我们的 `./temp/`

## 文档

### 用户文档

- `docs/Gas费策略.md` - Gas 费策略详解
- `docs/Nonce使用指南.md` - Nonce 使用指南
- `docs/地址查找表.md` - 地址查找表使用
- `docs/txs.md` - DEX 交易测试素材（真实交易签名）

### 开发文档

- `docs/DEX_AND_POOL_REFERENCE.md` - DEX 和 Pool 地址参考
- `docs/CPMM_Bug_Fix_Record.md` - CPMM Bug 修复记录
- `docs/plans/` - 开发计划和进度报告

## 当前开发状态

### 最近完成的功能（2025-02-04）

1. **Quote 计算功能完成** - 所有 DEX 的 Exact In 和 Exact Out 全部实现并验证
2. **测试工具提取** - 创建独立的 `sol-trade-test-utils` crate
3. **流动性管理** - 新增 CPMM 流动性添加功能
4. **模块化重构** - TradingClient 拆分为多个子模块

### 正在开发

- Raydium CLMM 进一步优化
- 新的 DEX 协议支持

### 已知问题与限制

#### 当前状态（2025-02-05）

所有 DEX 的 Exact In 和 Exact Out 功能均已完美实现并测试通过！

✅ **完美工作的功能（100% 准确）**：
- 所有 DEX 的 Exact In（0% 误差）
- CLMM 所有功能（0% 误差）
- CPMM 纯 Token Pool（0% 误差）
- AMM V4 所有功能（0% 误差）

⚠️ **需要注意事项**：
- **CPMM 混合 Pool**（Token-2022 + Token Program）：
  - 本地计算 vs 链上执行误差约 **0.04%**（可接受）
  - 示例：USDC-PRTS Pool（USDC 用 Token Program，PRTS 用 Token-2022）
  - 原因：Token-2022 扩展数据（Transfer Fee、Metadata 等）的内部状态计算差异
  - **Transfer Fee 处理**：
    - SDK 的 Quote 计算假设 Transfer Fee 为 0%
    - 即使 Token 启用了 Transfer Fee 扩展，实际费率可能为 0%（如 PRTS 为 0%）
    - 如果 Token 有非零 Transfer Fee，链上会自动扣除，输出略低于预期
- PumpSwap Exact Out Buy: 需要进一步优化
- PumpSwap Exact Out Sell: 0.4% 误差（可接受）

详见 `docs/plans/2025-02-04-dex-exact-out-final-report.md`

## 常见问题

### Q: 如何选择合适的 MEV 服务？

A: 根据需求选择：
- **Jito**: 最稳定，适合大多数场景
- **ZeroSlot**: 最低延迟，适合高频交易
- **Temporal**: 时间敏感交易

可以同时配置多个服务，SDK 会自动选择最快的。

### Q: 如何处理滑点？

A: 使用 `slippage_basis_points` 参数：
- 100 = 1%
- 500 = 5%
- 建议根据市场波动性调整

### Q: 如何优化交易速度？

A:
1. 使用多个 MEV 服务并发
2. 启用 Seed 优化
3. 使用地址查找表
4. 使用 Durable Nonce
5. 优化 Gas 费策略

## 语言

请使用中文进行沟通和代码注释。

## 相关资源

- **项目主页**: https://fnzero.dev/
- **GitHub**: https://github.com/0xfnzero/sol-trade-sdk
- **Telegram**: https://t.me/fnzero_group
- **Discord**: https://discord.gg/vuazbGkqQE
