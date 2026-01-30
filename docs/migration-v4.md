# 迁移指南：从 v3.3.6 到 v4.0.0

## 📋 概述

v4.0.0 是一个重大版本更新，引入了统一的交易接口，支持三种执行模式：
- **本地计算** (`buy_quote`) - 快速估算
- **链上模拟** (`buy_simulate`) - 准确验证
- **真实执行** (`buy`) - 实际交易（保持不变）

**这是一个 Breaking Change**，请仔细阅读本指南进行迁移。

---

## 🆕 新功能

### 1. buy_quote - 本地快速估算

```rust
use sol_trade_sdk::{TradingClient, TradeBuyParams, QuoteResult};

let client = TradingClient::new(payer, config).await;
let params = TradeBuyParams::new(...);

// 快速本地估算，无需发送交易
let quote: QuoteResult = client.buy_quote(params).await?;

println!("预期输出: {}", quote.amount_out);
println!("手续费: {}", quote.fee_amount);
println!("计算耗时: {} ms", quote.calculation_time_ms);
```

**支持的 DEX**: Raydium CLMM, Raydium CPMM, Raydium AMM V4, PumpSwap

**适用场景**:
- 快速价格查询
- 多个 DEX 价格对比
- UI 实时显示

---

### 2. buy_simulate - 链上准确模拟

```rust
use sol_trade_sdk::{TradingClient, TradeBuyParams, SimulationResult};

let client = TradingClient::new(payer, config).await;
let params = TradeBuyParams::new(...);

// 链上模拟，准确验证但不发送真实交易
let sim: SimulationResult = client.buy_simulate(params).await?;

println!("模拟输出: {}", sim.amount_out);
println!("CU 消耗: {}", sim.compute_units);
println!("交易费用: {}", sim.transaction_fee);
println!("成功: {}", sim.success);

if let Some(error) = sim.error {
    println!("错误: {}", error);
}
```

**支持的 DEX**: 所有 DEX

**适用场景**:
- 交易前验证
- 估算 CU 消耗
- 测试交易参数

---

## 🔄 迁移步骤

### 步骤 1: 更新依赖

更新 `Cargo.toml` 中的版本号：

```toml
[dependencies]
sol-trade-sdk = "4.0.0"
```

---

### 步骤 2: 更新错误处理

旧的代码：

```rust
use sol_trade_sdk::TradingClient;

let client = TradingClient::new(payer, config).await;
match client.buy(params).await {
    Ok((success, sigs, error)) => {
        // 处理结果
    },
    Err(e) => {
        // 处理错误
    },
}
```

新代码需要区分错误类型：

```rust
use sol_trade_sdk::{
    TradingClient,
    UnifiedTradingError,  // 新增统一错误类型
};

// buy_quote 和 buy_simulate 使用 UnifiedTradingError
match client.buy_quote(params).await {
    Ok(quote) => {
        // 处理 QuoteResult
    },
    Err(UnifiedTradingError::UnsupportedDex(dex)) => {
        eprintln!("不支持的 DEX: {:?}", dex);
    },
    Err(UnifiedTradingError::InvalidParameters(msg)) => {
        eprintln!("参数错误: {}", msg);
    },
    Err(e) => {
        eprintln!("其他错误: {}", e);
    },
}

// buy 仍然使用 anyhow::Error（保持向后兼容）
match client.buy(params).await {
    Ok((success, sigs, error)) => {
        // 处理结果（与之前相同）
    },
    Err(e) => {
        // 处理错误（与之前相同）
    },
}
```

---

### 步骤 3: 采用渐进式工作流

推荐的交易流程：

```rust
use sol_trade_sdk::{TradingClient, TradeBuyParams};

let client = TradingClient::new(payer, config).await;
let params = TradeBuyParams::new(...);

// 步骤 1: 快速估算（可选）
let quick_quote = client.buy_quote(params.clone()).await?;
println!("预期输出: {}", quick_quote.amount_out);

// 步骤 2: 链上模拟验证（推荐）
let verified = client.buy_simulate(params.clone()).await?;
if !verified.success {
    eprintln!("模拟失败: {:?}", verified.error);
    return Err( ... );
}

println!("CU 消耗: {}", verified.compute_units);

// 步骤 3: 真实执行
let (success, sigs, error) = client.buy(params).await?;
```

---

## 📊 API 变更总结

### 新增类型

```rust
/// 本地计算结果
pub struct QuoteResult {
    pub amount_out: u64,
    pub fee_amount: u64,
    pub price_impact_bps: Option<u64>,
    pub calculation_time_ms: u64,
    pub dex_type: DexType,
}

/// 链上模拟结果
pub struct SimulationResult {
    pub amount_out: u64,
    pub fee_amount: u64,
    pub compute_units: u64,
    pub transaction_fee: u64,
    pub success: bool,
    pub error: Option<String>,
    pub logs: Option<Vec<String>>,
    pub dex_type: DexType,
}

/// 统一错误类型
pub enum TradingError {
    UnsupportedDex(DexType),
    InvalidParameters(String),
    QuoteFailed(String),
    SimulationFailed(String),
    RpcError(reqwest::Error),
    SerializationError(bincode::Error),
    TransactionBuildError(String),
}
```

### 新增方法

```rust
impl TradingClient {
    /// 本地计算（新增）
    pub async fn buy_quote(
        &self,
        params: TradeBuyParams,
    ) -> Result<QuoteResult, UnifiedTradingError>;

    /// 链上模拟（新增）
    pub async fn buy_simulate(
        &self,
        params: TradeBuyParams,
    ) -> Result<SimulationResult, UnifiedTradingError>;

    /// 真实执行（保持不变）
    pub async fn buy(
        &self,
        params: TradeBuyParams,
    ) -> Result<(bool, Vec<Signature>, Option<TradeError>), anyhow::Error>;
}
```

---

## ⚠️ 破坏性变更

### 1. 新的导出结构

以下类型现在从 `sol_trade_sdk` 顶层导出：

```rust
use sol_trade_sdk::{
    // 新增导出
    QuoteResult,
    SimulationResult,
    UnifiedTradingError,
    UnifiedResult,

    // 现有导出（保持不变）
    TradingClient,
    TradeBuyParams,
    DexType,
    DexParamEnum,
    TradeTokenType,
};
```

### 2. 移除的导出

以下 DEX 特定函数不再导出（使用统一接口代替）：

- ~~`raydium_clmm::quote_exact_in`~~ → 使用 `client.buy_quote()`
- ~~`raydium_cpmm::quote_exact_in`~~ → 使用 `client.buy_quote()`
- ~~`raydium_amm_v4::quote_exact_in`~~ → 使用 `client.buy_quote()`
- ~~`pumpswap::quote_exact_in`~~ → 使用 `client.buy_quote()`

**注意**: 这些函数仍然存在于 `instruction::utils::*` 模块中，但不再从顶层导出。

---

## 🔧 迁移示例

### 示例 1: 简单价格查询

**旧代码** (v3.3.6):

```rust
// 无法快速查询价格，必须发送真实交易
let (success, sigs, error) = client.buy(params).await?;
```

**新代码** (v4.0.0):

```rust
// 快速本地估算，无需发送交易
let quote = client.buy_quote(params).await?;
println!("预期输出: {}", quote.amount_out);

// 如需更准确的验证，可使用链上模拟
let sim = client.buy_simulate(params).await?;
println!("准确输出: {}", sim.amount_out);
```

---

### 示例 2: DEX 价格对比

**旧代码** (v3.3.6):

```rust
// 无法方便地对比多个 DEX 的价格
```

**新代码** (v4.0.0):

```rust
use sol_trade_sdk::{DexType, TradeTokenType};

// 创建不同 DEX 的参数
let mut params = base_params.clone();

// 对比多个 DEX 的价格
for dex_type in &[DexType::RaydiumClmm, DexType::RaydiumCpmm, DexType::PumpSwap] {
    params.dex_type = dex_type.clone();

    match client.buy_quote(params.clone()).await {
        Ok(quote) => {
            println!("{:?}: 预期输出 = {}", dex_type, quote.amount_out);
        },
        Err(e) => {
            eprintln!("{:?} 失败: {}", dex_type, e);
        },
    }
}
```

---

### 示例 3: 完整交易流程

**旧代码** (v3.3.6):

```rust
// 直接发送交易，无法提前验证
let (success, sigs, error) = client.buy(params).await?;

if !success {
    eprintln!("交易失败: {:?}", error);
}
```

**新代码** (v4.0.0):

```rust
// 步骤 1: 快速估算
let quick = client.buy_quote(params.clone()).await?;
println!("预期输出: {}", quick.amount_out);

// 步骤 2: 链上验证
let verified = client.buy_simulate(params.clone()).await?;
if !verified.success {
    eprintln!("模拟失败: {:?}", verified.error);
    return Err( ... );
}

println!("CU 消耗: {}, 交易费用: {}",
         verified.compute_units,
         verified.transaction_fee);

// 步骤 3: 真实执行（现在更有信心）
let (success, sigs, error) = client.buy(params).await?;

if !success {
    eprintln!("交易失败: {:?}", error);
}
```

---

## 🚀 性能优化建议

### 1. 使用 buy_quote 进行批量查询

```rust
// 批量查询多个代币的价格（快速、无成本）
let quotes = futures::future::join_all(
    tokens.iter().map(|mint| {
        let mut params = base_params.clone();
        params.mint = *mint;
        client.buy_quote(params)
    })
).await;
```

### 2. 使用 buy_simulate 进行风险控制

```rust
// 在大额交易前先模拟，避免失败的交易
let sim = client.buy_simulate(large_amount_params).await?;

if !sim.success {
    eprintln!("交易会失败，不要执行: {:?}", sim.error);
    return;
}

if sim.compute_units > 1_000_000 {
    eprintln!("CU 消耗过高，考虑调整参数");
    return;
}

// 验证通过后再执行真实交易
client.buy(large_amount_params).await?;
```

---

## ❓ 常见问题

### Q1: 我必须使用新的 API 吗？

**A**: `buy()` 方法保持不变，你可以继续使用。但推荐使用新的渐进式工作流以提高成功率。

### Q2: buy_quote 和 buy_simulate 会消耗 SOL 吗？

**A**: 不会。这两个方法只进行计算和模拟，不会发送真实的交易到链上。

### Q3: 哪些 DEX 支持 buy_quote？

**A**: 目前支持 Raydium CLMM, Raydium CPMM, Raydium AMM V4, 和 PumpSwap。其他 DEX 可以使用 `buy_simulate`。

### Q4: buy_quote 和 buy_simulate 的结果一致吗？

**A**: 不完全一致。`buy_quote` 是本地估算，速度快但可能有误差；`buy_simulate` 是链上模拟，更准确但速度较慢。误差通常 < 0.1%。

### Q5: 我如何处理 UnsupportedDex 错误？

**A**: 对于不支持 `buy_quote` 的 DEX，可以直接使用 `buy_simulate` 或 `buy`：

```rust
match client.buy_quote(params.clone()).await {
    Ok(quote) => {
        // 使用 quote 结果
    },
    Err(UnifiedTradingError::UnsupportedDex(_)) => {
        // fallback 到 simulate 或 buy
        let sim = client.buy_simulate(params).await?;
    },
    Err(e) => {
        return Err(e.into());
    },
}
```

---

## 📚 更多信息

- **完整 API 文档**: https://docs.rs/sol-trade-sdk
- **GitHub Issues**: https://github.com/0xfnzero/sol-trade-sdk/issues
- **Telegram 群组**: https://t.me/fnzero_group

---

**迁移完成后，请运行测试确保一切正常：**

```bash
cargo test --package sol-trade-sdk
```

如有任何问题，请随时联系我们！
