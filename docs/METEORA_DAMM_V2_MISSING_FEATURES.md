# Meteora DAMM V2 功能缺失分析

> 对比基准：PumpSwap 实现
> 分析日期：2026-03-09

## 概述

本文档对比了 Meteora DAMM V2 与 PumpSwap 的实现差异，列出了 Meteora DAMM V2 当前缺失的功能模块。

## 当前已实现功能

### ✅ 核心功能

| 模块 | 文件位置 | 说明 |
|------|----------|------|
| 指令构建器 | `src/instruction/meteora_damm_v2.rs` | 实现 `InstructionBuilder` trait |
| 参数结构体 | `src/trading/core/params/meteora_params.rs` | `MeteoraDammV2Params` |
| Pool 类型定义 | `src/instruction/utils/meteora_damm_v2_types.rs` | Pool、PoolFeesStruct、RewardInfo 等 |
| Pool 数据获取 | `src/instruction/utils/meteora_damm_v2.rs` | `get_pool_by_address()` |
| DexType 枚举 | `src/trading/factory.rs` | `DexType::MeteoraDammV2` |
| DEX 协议常量 | `src/constants/dex_protocols.rs` | `METEORA_DAMM_V2_PROGRAM_ID` |
| Pool 缓存 | `src/instruction/utils/meteora_damm_v2.rs` | `meteora_cache` 模块 |
| DEX 检测 | `src/common/dex_detector.rs` | 支持从 Pool 地址识别 |

### ✅ 交易功能

- Buy（买入）：用 WSOL 或 USDC 购买代币
- Sell（卖出）：卖出代币换取 WSOL 或 USDC
- WSOL/USDC 基准代币支持

### ✅ 示例代码

- `examples/meteora_damm_v2_direct_trading/src/main.rs`

---

## 缺失功能清单

### 1. 计算模块 - 🔴 高优先级

**文件**: `src/utils/calc/meteora_damm_v2.rs` (需新建)

参考 PumpSwap 实现 (`src/utils/calc/pumpswap.rs`)，需要实现：

```rust
// 需要实现的结构体
pub struct BuyBaseInputResult {
    pub internal_quote_amount: u64,
    pub ui_quote: u64,
    pub max_quote: u64,
}

pub struct BuyQuoteInputResult {
    pub base: u64,
    pub internal_quote_without_fees: u64,
    pub max_quote: u64,
}

pub struct SellBaseInputResult {
    pub ui_quote: u64,
    pub min_quote: u64,
    pub internal_quote_amount_out: u64,
}

pub struct SellQuoteInputResult {
    pub internal_raw_quote: u64,
    pub base: u64,
    pub min_quote: u64,
}

// 需要实现的函数
pub fn buy_exact_out_base_internal(...) -> Result<BuyBaseInputResult, String>;
pub fn buy_exact_in_quote_internal(...) -> Result<BuyQuoteInputResult, String>;
pub fn sell_exact_in_base_internal(...) -> Result<SellBaseInputResult, String>;
pub fn sell_exact_out_quote_internal(...) -> Result<SellQuoteInputResult, String>;
```

**用途**:
- 支持自动计算交易输出/输入金额
- 支持滑点保护计算
- 支持费用计算（Meteora DAMM V2 有 base_fee 和 dynamic_fee）

---

### 2. Quote 报价模块 - 🔴 高优先级

**文件**: `src/instruction/utils/meteora_damm_v2/quotes.rs` (需新建)

参考 PumpSwap 实现 (`src/instruction/utils/pumpswap/quotes.rs`)，需要实现：

```rust
/// Exact In 报价
pub(crate) async fn quote_exact_in(
    rpc: &SolanaRpcClient,
    params: QuoteExactInParams,
) -> Result<QuoteExactInResult, anyhow::Error>;

/// Exact Out 报价
pub(crate) async fn quote_exact_out(
    rpc: &SolanaRpcClient,
    params: QuoteExactOutParams,
) -> Result<QuoteExactOutResult, anyhow::Error>;

/// 获取 Token 的 USD 价格
pub async fn get_token_price_in_usd<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    token_mint: &Pubkey,
    wsol_usd_clmm_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error>;

/// 通过已知 Pool 获取 Token 价格
pub async fn get_token_price_in_usd_with_pool<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    token_mint: &Pubkey,
    x_wsol_pool_address: &Pubkey,
    wsol_usd_clmm_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error>;
```

**用途**:
- 支持交易前预览输出金额
- 支持价格查询
- 支持 aggregator 集成

---

### 3. Pool 查询增强 - 🟡 中优先级

**文件**: `src/instruction/utils/meteora_damm_v2/pool_queries.rs` (需新建)

参考 PumpSwap 实现，需要添加：

```rust
/// 通过 mint 地址查找 Pool
pub async fn get_pool_by_mint<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<(Pubkey, Pool), anyhow::Error>;

/// 查找最优 Pool
pub async fn find_pool<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<Pubkey, anyhow::Error>;

/// 获取 Pool 中的代币余额
pub async fn get_token_balances<T: PoolRpcClient + ?Sized>(
    pool: &Pool,
    rpc: &T,
) -> Result<(u64, u64), anyhow::Error>;
```

**用途**:
- 简化调用流程（只需提供 mint 即可交易）
- 获取实时池子余额用于计算

---

### 4. 交易解析器 - 🟡 中优先级

**文件**: `src/parser/meteora_damm_v2/` (需新建目录)

```
src/parser/meteora_damm_v2/
├── mod.rs      # 主解析器
└── events.rs   # 事件结构定义
```

参考 PumpSwap 实现 (`src/parser/pumpswap/`)，需要实现：

```rust
// mod.rs
pub struct MeteoraDammV2Parser;

impl DexParserTrait for MeteoraDammV2Parser {
    async fn parse(&self, adapter: &TransactionAdapter) -> Result<Vec<ParsedTradeInfo>, ParseError>;
    fn protocol(&self) -> DexProtocol;
}

// events.rs
pub struct MeteoraDammV2BuyEvent { ... }
pub struct MeteoraDammV2SellEvent { ... }
pub enum MeteoraDammV2EventType { Buy, Sell, ... }
pub fn parse_meteora_damm_v2_event(data: &[u8]) -> Option<(MeteoraDammV2EventType, EventData)>;
```

**用途**:
- 解析链上交易日志
- 提取交易详情用于监控/分析

---

### 5. 测试覆盖 - 🟡 中优先级

**需要新建的测试文件**:

| 文件 | 说明 |
|------|------|
| `tests/meteora_damm_v2_pool_tests.rs` | Pool 查询测试 |
| `tests/meteora_damm_v2_exact_out_tests.rs` | Exact Out 计算测试 |
| `tests/meteora_damm_v2_simulation_tests.rs` | 模拟交易测试 |

**测试参数构建器**: `sol-trade-test-utils/src/test_params/meteora_params.rs`

```rust
pub struct MeteoraWsolBuyParamsBuilder { ... }
pub struct MeteoraWsolSellParamsBuilder { ... }
```

---

### 6. 辅助模块 - 🟢 低优先级

**文件结构**:
```
src/instruction/utils/meteora_damm_v2/
├── mod.rs
├── cache.rs
├── constants.rs     # 需新建
├── helpers.rs       # 需新建
├── pool_queries.rs  # 需新建
└── quotes.rs        # 需新建
```

**constants.rs** 需要定义:
```rust
pub const SWAP_DISCRIMINATOR: &[u8];
pub const BUY_DISCRIMINATOR: &[u8];
pub const SELL_DISCRIMINATOR: &[u8];
// 费用相关常量
```

**helpers.rs** 需要实现:
```rust
pub fn identify_quote_mint(pool: &Pool) -> Pubkey;
```

**价格模块**: `src/utils/price/meteora_damm_v2.rs`

---

## 优先级建议

### 第一阶段 (P0)
1. 计算模块 (`src/utils/calc/meteora_damm_v2.rs`)
2. Quote 报价模块

### 第二阶段 (P1)
3. Pool 查询增强 (pool_queries.rs)
4. 测试覆盖

### 第三阶段 (P2)
5. 交易解析器
6. 辅助模块完善

---

## 参考

- PumpSwap 实现: `src/instruction/pumpswap.rs`, `src/utils/calc/pumpswap.rs`
- Meteora DAMM V2 IDL: `./temp/meteora/dlmm/` 或官方文档
- 测试 Pool 地址: `4C3JRBp4Bycs3jQTuJVEL6kVAWJMhNUshaD5GmwcEaMu` (USDC-WSOL)
