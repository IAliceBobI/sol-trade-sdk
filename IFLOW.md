# Sol Trade SDK 项目文档

## 项目概述

Sol Trade SDK 是一个用 Rust 编写的综合性 Solana DEX（去中心化交易所）交易 SDK，为开发者提供统一、高效的交易接口。该项目支持多个 Solana 生态中的主流 DEX 协议，包括 PumpFun、PumpSwap、Bonk、Raydium（AMM V4、CPMM、CLMM）和 Meteora DAMM V2。

### 核心特性

- **多协议支持**：统一的交易接口支持 7 个主流 DEX 协议
- **MEV 保护**：集成 13 个 MEV 保护服务（Jito、ZeroSlot、Temporal、Bloxroute、FlashBlock、BlockRazor、Node1、Astralane、Stellium、Lightspeed、Soyas、NextBlock）
- **并发交易**：支持通过多个 MEV 服务同时发送交易，返回所有交易签名，最快成功的交易生效
- **中间件系统**：支持自定义指令中间件，在交易执行前修改、添加或删除指令
- **交易生命周期回调**：支持在交易签名后、发送前拦截交易，用于数据库入库、审计等场景
- **回调执行模式**：支持同步和异步两种回调执行模式，满足不同业务需求
- **性能优化**：采用零开销抽象、SIMD 优化、零拷贝 I/O 等技术实现超低延迟
- **地址查找表**：支持 ALT 优化交易大小和减少费用
- **Nonce 缓存**：支持 Durable Nonce 实现交易重放保护和优化
- **Seed 优化**：支持 Seed 优化减少交易大小和费用
- **WSOL 管理**：自动 WSOL ATA 创建和管理，支持包装/解包装操作
- **百分比交易**：支持按百分比卖出代币
- **自定义 URL**：每个 SWQOS 服务支持自定义端点 URL

### 技术栈

- **语言**：Rust 2021 Edition
- **框架**：Solana SDK 3.0.x
- **异步运行时**：Tokio (rt-multi-thread)
- **加密**：Rustls、Ring
- **序列化**：Borsh、Bincode、Serde
- **网络**：Reqwest、Isahc、gRPC (Tonic/Quinn)
- **性能优化**：crossbeam、memmap2、core_affinity、dashmap、clru、parking_lot

## 项目结构

```
src/
├── common/              # 通用功能和工具
│   ├── address_lookup.rs    # 地址查找表功能
│   ├── bonding_curve.rs     # Bonding Curve 相关
│   ├── fast_fn.rs           # 快速函数（性能优化）
│   ├── fast_timing.rs       # 快速时间处理
│   ├── gas_fee_strategy.rs  # Gas 费用策略
│   ├── global.rs            # 全局配置
│   ├── nonce_cache.rs       # Nonce 缓存
│   ├── seed.rs              # Seed 优化
│   ├── spl_associated_token_account.rs  # SPL 关联代币账户
│   ├── spl_token.rs         # SPL Token
│   ├── spl_token_2022.rs    # SPL Token 2022
│   ├── subscription_handle.rs  # 订阅处理
│   ├── types.rs             # 通用类型定义
│   └── wsol_manager.rs      # WSOL 管理
├── constants/           # 常量定义
│   ├── accounts.rs          # 账户地址常量
│   ├── decimals.rs          # 小数位常量
│   ├── swqos.rs             # SWQOS 常量
│   ├── tokens.rs            # 代币地址常量
│   ├── trade_platform.rs    # 交易平台常量
│   └── trade.rs             # 交易常量
├── instruction/          # 指令构建
│   ├── bonk.rs              # Bonk 协议指令
│   ├── meteora_damm_v2.rs   # Meteora DAMM V2 指令
│   ├── pumpfun.rs           # PumpFun 指令
│   ├── pumpswap.rs          # PumpSwap 指令
│   ├── raydium_amm_v4.rs    # Raydium AMM V4 指令
│   ├── raydium_clmm.rs      # Raydium CLMM 指令
│   ├── raydium_cpmm.rs      # Raydium CPMM 指令
│   └── utils/               # 指令工具和类型定义
├── perf/                 # 性能优化模块
│   ├── compiler_optimization.rs   # 编译器优化
│   ├── hardware_optimizations.rs  # 硬件优化
│   ├── kernel_bypass.rs           # 内核绕过
│   ├── protocol_optimization.rs   # 协议优化
│   ├── realtime_tuning.rs         # 实时调优
│   ├── simd.rs                    # SIMD 优化
│   ├── syscall_bypass.rs          # 系统调用绕过
│   ├── ultra_low_latency.rs       # 超低延迟
│   └── zero_copy_io.rs            # 零拷贝 I/O
├── swqos/                # MEV 服务客户端（13 个服务）
│   ├── jito.rs              # Jito 客户端
│   ├── zeroslot.rs          # ZeroSlot 客户端
│   ├── temporal.rs          # Temporal 客户端
│   ├── bloxroute.rs         # Bloxroute 客户端
│   ├── flashblock.rs        # FlashBlock 客户端
│   ├── blockrazor.rs        # BlockRazor 客户端
│   ├── node1.rs             # Node1 客户端
│   ├── astralane.rs         # Astralane 客户端
│   ├── stellium.rs          # Stellium 客户端
│   ├── lightspeed.rs        # Lightspeed 客户端
│   ├── soyas.rs             # Soyas 客户端
│   ├── nextblock.rs         # NextBlock 客户端（默认禁用）
│   ├── solana_rpc.rs        # Solana RPC 客户端
│   ├── common.rs            # SWQOS 通用功能
│   ├── serialization.rs     # 序列化工具
│   └── mod.rs               # SWQOS 模块导出
├── trading/              # 统一交易引擎
│   ├── factory.rs           # 交易工厂（创建不同协议执行器）
│   ├── lifecycle.rs         # 交易生命周期回调系统
│   ├── common/              # 通用交易工具
│   │   ├── compute_budget_manager.rs  # 计算预算管理
│   │   ├── nonce_manager.rs           # Nonce 管理
│   │   ├── transaction_builder.rs     # 交易构建
│   │   ├── utils.rs                  # 交易工具
│   │   └── wsol_manager.rs           # WSOL 管理
│   ├── core/                # 核心交易引擎
│   │   ├── async_executor.rs  # 异步执行器
│   │   ├── executor.rs       # 交易执行器
│   │   ├── params.rs         # 交易参数
│   │   └── execution.rs      # 执行逻辑
│   └── middleware/          # 中间件系统
├── utils/                # 工具函数
│   ├── quote.rs             # 报价工具
│   ├── token.rs             # 代币工具
│   ├── calc/                # 数量计算工具
│   └── price/               # 价格计算工具
└── lib.rs                # 主库文件（导出公共 API）

examples/              # 示例程序（19 个独立 workspace 成员）
├── trading_client/               # 创建 TradingClient 实例
├── pumpfun_sniper_trading/       # PumpFun 狙击交易
├── pumpfun_copy_trading/         # PumpFun 跟单交易
├── pumpfun_buy_test/             # PumpFun 买入测试
├── pumpswap_trading/             # PumpSwap 交易
├── pumpswap_direct_trading/      # PumpSwap 直接交易
├── raydium_cpmm_trading/         # Raydium CPMM 交易
├── raydium_amm_v4_trading/       # Raydium AMM V4 交易
├── meteora_damm_v2_direct_trading/  # Meteora DAMM V2 交易
├── bonk_sniper_trading/          # Bonk 狙击交易
├── bonk_copy_trading/            # Bonk 跟单交易
├── middleware_system/            # 中间件系统示例
├── address_lookup/               # 地址查找表示例
├── nonce_cache/                  # Nonce 缓存示例
├── wsol_wrapper/                 # WSOL 包装示例
├── seed_trading/                 # Seed 优化交易示例
├── gas_fee_strategy/             # Gas 费用策略示例
├── cli_trading/                  # CLI 交易工具
└── transaction_callback/         # 交易生命周期回调示例

docs/                  # 文档（包含中英文版本）
├── ADDRESS_LOOKUP_TABLE.md      # 地址查找表指南（英文）
├── ADDRESS_LOOKUP_TABLE_CN.md   # 地址查找表指南（中文）
├── GAS_FEE_STRATEGY.md          # Gas 费用策略指南（英文）
├── GAS_FEE_STRATEGY_CN.md       # Gas 费用策略指南（中文）
├── MIN_TIP_AMOUNT.md            # Node1 最小小费金额限制
├── NONCE_CACHE.md               # Nonce 缓存指南（英文）
├── NONCE_CACHE_CN.md            # Nonce 缓存指南（中文）
├── TRADING_PARAMETERS.md        # 交易参数参考（英文）
├── TRADING_PARAMETERS_CN.md     # 交易参数参考（中文）
└── TRANSACTION_CALLBACK.md      # 交易生命周期回调指南
```

## 构建和运行

### 构建项目

```bash
# 开发构建
cargo build
# 启用性能追踪特性（仅用于调试，生产环境应禁用）
cargo build --debug --features perf-trace
```

### 运行示例

项目通过运行示例程序进行测试，每个示例都是独立的 workspace 成员：

```bash
# 创建 TradingClient 实例
cargo run --package trading_client

# PumpFun 代币狙击交易
cargo run --package pumpfun_sniper_trading

# PumpFun 代币跟单交易
cargo run --package pumpfun_copy_trading

# PumpFun 买入测试
cargo run --package pumpfun_buy_test

# PumpSwap 交易
cargo run --package pumpswap_trading

# PumpSwap 直接交易
cargo run --package pumpswap_direct_trading

# Raydium CPMM 交易
cargo run --package raydium_cpmm_trading

# Raydium AMM V4 交易
cargo run --package raydium_amm_v4_trading

# Meteora DAMM V2 交易
cargo run --package meteora_damm_v2_direct_trading

# Bonk 代币狙击交易
cargo run --package bonk_sniper_trading

# Bonk 代币跟单交易
cargo run --package bonk_copy_trading

# 中间件系统示例
cargo run --package middleware_system

# 地址查找表示例
cargo run --package address_lookup

# Nonce 缓存示例
cargo run --package nonce_cache

# WSOL 包装示例
cargo run --package wsol_wrapper

# Seed 优化交易示例
cargo run --package seed_trading

# Gas 费用策略示例
cargo run --package gas_fee_strategy

# CLI 交易工具
cargo run --package cli_trading

# 交易生命周期回调示例
cargo run --package transaction_callback
```

### 测试方法

项目没有传统的单元测试，主要通过以下方式测试：

1. **运行示例程序**：每个示例演示特定功能的使用
2. **模拟交易**：在 `TradeBuyParams` 和 `TradeSellParams` 中设置 `simulate: true` 进行模拟
3. **测试网验证**：在主网使用前，先在测试网充分测试

### 安装依赖

```bash
# 克隆项目
git clone https://github.com/0xfnzero/sol-trade-sdk

# 或使用 crates.io
# 在 Cargo.toml 中添加：
sol-trade-sdk = "3.3.6"
```

## 开发规范

### 代码风格

- 使用 Rust 2021 Edition
- 遵循 Rust 标准代码风格（使用 `rustfmt` 格式化）
- 使用 `clippy` 进行代码质量检查
- 代码注释使用中文（部分英文技术术语）

### 命名约定

- **类型**：PascalCase（如 `TradingClient`、`TradeBuyParams`）
- **函数**：snake_case（如 `buy`、`sell`、`create_executor`）
- **常量**：SCREAMING_SNAKE_CASE（如 `SOL_TOKEN_ACCOUNT`、`WSOL_TOKEN_ACCOUNT`）
- **模块**：snake_case（如 `common`、`trading`、`instruction`）

### 架构模式

1. **工厂模式**：`TradeFactory` 用于创建不同 DEX 协议的交易执行器
2. **零开销抽象**：使用 `LazyLock` 实现编译期静态实例，无运行时开销
3. **中间件模式**：`MiddlewareManager` 支持链式中间件处理
4. **策略模式**：`GasFeeStrategy` 支持不同的 Gas 费用策略
5. **类型安全**：使用 `DexParamEnum` 枚举确保协议参数类型安全
6. **单例模式**：支持全局单例访问（`get_instance`）
7. **回调模式**：`TransactionLifecycleCallback` 支持交易生命周期拦截

### 性能优化配置

项目在 `Cargo.toml` 中配置了高性能 Release profile：

```toml
[profile.release]
opt-level = 3              # 最高优化级别（不影响编译速度）
lto = "thin"               # 瘦 LTO - 平衡性能与编译速度（比 fat 快 5-10 倍）
codegen-units = 16         # 16 个代码生成单元 - 并行编译（比 1 快 10 倍）
panic = "abort"            # 恐慌即中止
overflow-checks = false    # 禁用溢出检查
debug = false              # 禁用调试信息
debug-assertions = false   # 禁用调试断言
strip = true               # 去除符号表
incremental = true         # 增量编译 - 大幅加速重新编译

[profile.dev]
overflow-checks = true     # 开发时启用溢出检查
opt-level = 0
debug = 1
codegen-units = 256        # 保持高并行度
```

### 编译特性

- **default**：默认特性，无额外依赖
- **perf-trace**：性能追踪特性，用于调试和性能分析，生产环境应禁用以获得最佳性能

### 关键设计原则

1. **并发优先**：支持多 SWQOS 服务并发交易，提高成功率
2. **类型安全**：使用 Rust 类型系统确保交易参数正确性
3. **零拷贝**：尽可能使用引用和智能指针避免数据拷贝
4. **异步优先**：使用 Tokio 异步运行时处理所有 I/O 操作
5. **错误处理**：使用 `anyhow::Result` 统一错误处理
6. **黑名单机制**：支持 SWQOS 服务黑名单配置（如 NextBlock 默认禁用）
7. **智能检测**：自动检测并调整 Node1 最小小费限制
8. **回调优先**：支持交易签名后的回调处理，满足入库和审计需求

## 核心 API 使用

### 创建 TradingClient

```rust
use sol_trade_sdk::{common::TradeConfig, swqos::{SwqosConfig, SwqosRegion}, SolanaTrade};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;
use std::sync::Arc;

// 配置钱包和 RPC
let payer = Arc::new(Keypair::from_base58_string("your_keypair_here"));
let rpc_url = "https://mainnet.helius-rpc.com/?api-key=xxxxxx".to_string();
let commitment = CommitmentConfig::processed();

// 配置多个 SWQOS 服务
let swqos_configs: Vec<SwqosConfig> = vec![
    SwqosConfig::Default(rpc_url.clone()),
    SwqosConfig::Jito("your_uuid".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::Bloxroute("your_api_token".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::ZeroSlot("your_api_token".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::Temporal("your_api_token".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::FlashBlock("your_api_token".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::BlockRazor("your_api_token".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::Node1("your_api_token".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::Astralane("your_api_token".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::Stellium("your_api_token".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::Lightspeed("your_api_token".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::Soyas("your_api_token".to_string(), SwqosRegion::Frankfurt, None),
];

// 创建交易配置
let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment);

// 可选：自定义 WSOL ATA、Seed 优化和回调执行模式设置
// let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment)
//     .with_wsol_ata_config(
//         true,  // create_wsol_ata_on_startup: 启动时检查并创建 WSOL ATA（默认：true）
//         true   // use_seed_optimize: 全局启用 seed 优化（默认：true）
//     )
//     .with_callback_execution_mode(CallbackExecutionMode::Sync); // 全局回调执行模式

// 创建 TradingClient
let client = SolanaTrade::new(payer, trade_config).await;
```

### 使用自定义 URL

每个 SWQOS 服务现在支持可选的自定义 URL 参数：

```rust
// 使用自定义 URL（第三个参数）
let jito_config = SwqosConfig::Jito(
    "your_uuid".to_string(),
    SwqosRegion::Frankfurt, // 此参数仍然需要但会被忽略
    Some("https://custom-jito-endpoint.com".to_string()) // 自定义 URL
);

// 使用默认区域端点（第三个参数为 None）
let bloxroute_config = SwqosConfig::Bloxroute(
    "your_api_token".to_string(),
    SwqosRegion::NewYork, // 将使用此区域的默认端点
    None // 无自定义 URL，使用 SwqosRegion
);
```

### 执行买入交易

```rust
use sol_trade_sdk::{TradeBuyParams, TradeTokenType, factory::DexType, trading::core::params::DexParamEnum, common::GasFeeStrategy};

// 配置 Gas 费用策略
let gas_fee_strategy = GasFeeStrategy::new();
gas_fee_strategy.set_global_fee_strategy(150000, 150000, 500000, 500000, 0.001, 0.001, 256 * 1024, 0);

// 构建买入参数
let buy_params = TradeBuyParams {
    dex_type: DexType::PumpSwap,
    input_token_type: TradeTokenType::WSOL,
    mint: token_mint_pubkey,
    input_token_amount: buy_sol_amount,
    slippage_basis_points: Some(100),  // 1% 滑点
    recent_blockhash: Some(blockhash),
    extension_params: DexParamEnum::PumpSwap(params),
    address_lookup_table_account: None,
    wait_transaction_confirmed: true,
    create_input_token_ata: true,
    close_input_token_ata: true,
    create_mint_ata: true,
    durable_nonce: None,
    fixed_output_token_amount: None,  // 可选：指定精确的输出金额
    gas_fee_strategy: gas_fee_strategy.clone(),
    on_transaction_signed: None,     // 可选：交易签名后回调
    callback_execution_mode: None,   // 可选：回调执行模式（覆盖全局配置）
    simulate: false,
};

// 执行买入
// 返回：(是否至少有一个交易成功, 所有交易签名, 最后一个错误（如果全部失败）)
let (success, signatures, error) = client.buy(buy_params).await?;
```

### 执行卖出交易

```rust
use sol_trade_sdk::{TradeSellParams, TradeTokenType};

let sell_params = TradeSellParams {
    dex_type: DexType::PumpSwap,
    output_token_type: TradeTokenType::WSOL,
    mint: token_mint_pubkey,
    input_token_amount: sell_token_amount,
    slippage_basis_points: Some(100),
    recent_blockhash: Some(blockhash),
    with_tip: true,
    extension_params: DexParamEnum::PumpSwap(params),
    // ... 其他参数
    on_transaction_signed: None,     // 可选：交易签名后回调
    callback_execution_mode: None,   // 可选：回调执行模式（覆盖全局配置）
    simulate: false,
};

let (success, signatures, error) = client.sell(sell_params).await?;
```

### 按百分比卖出

```rust
// 按百分比卖出代币
// percent: 1-100，其中 100 = 100%
let (success, signatures, error) = client.sell_by_percent(
    sell_params,
    total_token_amount,  // 总代币数量
    50  // 卖出 50%
).await?;
```

### WSOL 管理

```rust
// 包装 SOL 为 WSOL
let signature = client.wrap_sol_to_wsol(amount_lamports).await?;

// 关闭 WSOL 账户并解包为 SOL
let signature = client.close_wsol().await?;

// 创建 WSOL ATA（不包装 SOL）
let signature = client.create_wsol_ata().await?;
```

### 使用中间件

```rust
use sol_trade_sdk::trading::MiddlewareManager;

struct CustomMiddleware;

impl Middleware for CustomMiddleware {
    fn process(&self, instructions: Vec<Instruction>) -> Vec<Instruction> {
        // 自定义处理逻辑
        instructions
    }
}

let middleware_manager = MiddlewareManager::new()
    .add_middleware(Box::new(CustomMiddleware));

let client = SolanaTrade::new(payer, trade_config)
    .await
    .with_middleware_manager(middleware_manager);
```

### 使用交易生命周期回调

交易生命周期回调系统允许在交易签名后、发送前拦截交易，用于数据库入库、审计等场景：

```rust
use sol_trade_sdk::{
    CallbackContext, TransactionLifecycleCallback,
    CallbackExecutionMode, CallbackRef,
};
use std::sync::Arc;
use futures::future::BoxFuture;

// 自定义回调实现
#[derive(Clone)]
struct DatabaseCallback;

impl TransactionLifecycleCallback for DatabaseCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, anyhow::Result<()>> {
        let context_clone = context.clone();
        Box::pin(async move {
            // 在这里实现数据库入库逻辑
            println!("Saving transaction: {}", context_clone.signature);
            Ok(())
        })
    }
}

// 使用回调
let callback: CallbackRef = Arc::new(DatabaseCallback);

let buy_params = TradeBuyParams {
    // ... 其他参数
    on_transaction_signed: Some(callback),
    callback_execution_mode: Some(CallbackExecutionMode::Sync), // 同步模式：先入库再发送
};
```

**回调执行模式**：

- **Async（异步）**：不阻塞交易发送，适合监控、日志场景
- **Sync（同步）**：等待回调完成后再发送，适合入库、审计场景

详见文档：`docs/TRANSACTION_CALLBACK.md`

### 获取全局实例

```rust
// 获取全局单例实例
let client = SolanaTrade::get_instance();
```

### 获取 RPC 客户端

```rust
// 获取 RPC 客户端进行直接区块链交互
let rpc = client.get_rpc();
```

## SWQOS 服务配置

### 支持的 MEV 保护服务

SDK 支持 13 个 MEV 保护服务，可通过 `SwqosConfig` 配置：

| 服务 | 状态 | 说明 | 最小小费 |
|------|------|------|----------|
| Default | ✅ 可用 | 默认 Solana RPC | - |
| Jito | ✅ 可用 | Jito MEV 保护 | - |
| ZeroSlot | ✅ 可用 | ZeroSlot MEV 保护 | - |
| Temporal | ✅ 可用 | Temporal MEV 保护 | - |
| Bloxroute | ✅ 可用 | Bloxroute MEV 保护 | - |
| FlashBlock | ✅ 可用 | FlashBlock MEV 保护 | - |
| BlockRazor | ✅ 可用 | BlockRazor MEV 保护 | - |
| Node1 | ✅ 可用 | Node1 MEV 保护 | 0.002 SOL |
| Astralane | ✅ 可用 | Astralane MEV 保护 | - |
| Stellium | ✅ 可用 | Stellium MEV 保护 | - |
| Lightspeed | ✅ 可用 | Lightspeed MEV 保护 | 0.001 SOL |
| Soyas | ✅ 可用 | Soyas MEV 保护 | - |
| NextBlock | ⚠️ 默认禁用 | NextBlock MEV 保护（在黑名单中） | - |

### Node1 最小小费限制

Node1 节点要求最小小费金额为 **0.002 SOL**，低于此金额的交易可能会被拒绝。

SDK 已自动添加智能检测，只对 Node1 的 tip_account 应用最小小费金额检查。其他 SWQOS 服务不受影响。

详见文档：`docs/MIN_TIP_AMOUNT.md`

### 区域配置

支持多个区域配置：

```rust
pub enum SwqosRegion {
    NewYork,
    Frankfurt,
    Amsterdam,
    SLC,
    Tokyo,
    London,
    LosAngeles,
    Default,
}
```

## 重要注意事项

1. **充分测试**：在主网使用前务必在测试网充分测试
2. **私钥安全**：妥善保管私钥，不要提交到版本控制系统
3. **API 令牌**：正确配置 SWQOS 服务的 API 令牌
4. **滑点设置**：合理设置滑点避免交易失败
5. **余额监控**：监控余额和交易费用
6. **合规性**：遵守相关法律法规
7. **Node1 小费**：使用 Node1 时注意最小小费限制（0.002 SOL）
8. **性能特性**：生产环境应禁用 `perf-trace` 特性以获得最佳性能
9. **并发交易**：使用多个 SWQOS 服务时会返回多个交易签名，需要正确处理
10. **WSOL 管理**：SDK 会自动管理 WSOL ATA，但也可以手动控制
11. **回调执行**：同步模式下回调失败会阻止交易发送，异步模式下不影响
12. **Token 类型**：PumpSwap 等协议使用 WSOL 而非原生 SOL，请使用 `TradeTokenType::WSOL`

## 相关资源

- **GitHub 仓库**：https://github.com/0xfnzero/sol-trade-sdk
- **官方文档**：https://fnzero.dev/
- **Telegram 群组**：https://t.me/fnzero_group
- **Discord**：https://discord.gg/vuazbGkqQE
- **Crates.io**：https://crates.io/crates/sol-trade-sdk
- **API 文档**：https://docs.rs/sol-trade-sdk

## 版本信息

- **当前版本**：3.3.6
- **Rust Edition**：2021
- **Solana SDK**：3.0.x
- **许可证**：MIT
