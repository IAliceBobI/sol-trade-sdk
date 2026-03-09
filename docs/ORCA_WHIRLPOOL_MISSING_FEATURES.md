# Orca Whirlpool 功能缺失分析

> 对比基准：Raydium CLMM 实现
> 分析日期：2026-03-09

## 概述

本文档分析了 Orca Whirlpool (Orca 的集中流动性 AMM) 与当前已实现的 Raydium CLMM 的功能对比。Orca Whirlpool 是一个类似 Uniswap V3 的集中流动性 AMM，其实现复杂度与 Raydium CLMM 相当。

**当前状态**: 🔴 **完全未实现**

## Orca Whirlpool vs Raydium CLMM 对比

### 技术对比

| 特性 | Orca Whirlpool | Raydium CLMM |
|------|----------------|--------------|
| **类型** | 集中流动性 AMM (类似 Uniswap V3) | 集中流动性 AMM |
| **Program ID** | `7ZFam7zqEuFms1PvCxHyeXQGWhrwYpWkQ5jaM7iWpP1g` (current) | `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK` |
| **核心概念** | Whirlpool, TickArray, Position | PoolState, TickArray, Position |
| **费用等级** | 0.01%, 0.05%, 0.25%, 1% | 动态费用 |
| **官方 SDK** | whirlpools-client (Rust), @orca-so/kit (TS) | 无官方 Rust SDK |

### 参考代码位置

- **Orca 官方代码**: `./temp/orca/whirlpools/`
- **Autobahn Router 集成**: `./temp/common/aggregators/autobahn-router/lib/dex-orca/`
- **Raydium CLMM 实现**: `src/instruction/raydium_clmm.rs`, `src/utils/calc/clmm_math/`

---

## 需要实现的功能清单

### 1. 核心常量定义 - 🔴 必需

**文件**: `src/constants/dex_protocols.rs`

```rust
/// Orca Whirlpool 协议常量
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "7ZFam7zqEuFms1PvCxHyeXQGWhrwYpWkQ5jaM7iWpP1g";
pub const ORCA_WHIRLPOOL_NAME: &str = "orca_whirlpool";
pub const ORCA_WHIRLPOOL_DISPLAY_NAME: &str = "Orca Whirlpool";

pub const ORCA_WHIRLPOOL_PUBKEY: Pubkey = pubkey!("7ZFam7zqEuFms1PvCxHyeXQGWhrwYpWkQ5jaM7iWpP1g");

pub enum DexProtocol {
    // ... existing variants
    OrcaWhirlpool,
}
```

---

### 2. DexType 枚举扩展 - 🔴 必需

**文件**: `src/trading/factory.rs`

```rust
pub enum DexType {
    // ... existing variants
    OrcaWhirlpool,
}
```

---

### 3. Pool 类型定义 - 🔴 高优先级

**文件**: `src/instruction/utils/orca_whirlpool_types.rs` (新建)

参考 Raydium CLMM 的 PoolState 结构，需要定义：

```rust
/// Orca Whirlpool 状态
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Whirlpool {
    /// DEX Program ID
    pub program_id: Pubkey,
    /// DEX 协议名称
    pub dex_name: String,
    /// DEX 显示名称
    pub dex_display_name: String,
    /// Whirlpool 地址
    pub whirlpool: Pubkey,
    /// Token A mint
    pub token_a_mint: Pubkey,
    /// Token B mint
    pub token_b_mint: Pubkey,
    /// Token A vault
    pub token_a_vault: Pubkey,
    /// Token B vault
    pub token_b_vault: Pubkey,
    /// 当前 tick 索引
    pub tick_current_index: i32,
    /// Tick 间距
    pub tick_spacing: u16,
    /// 当前 sqrt 价格
    pub sqrt_price: u128,
    /// 费率 (基点)
    pub fee_rate: u16,
    /// 流动性
    pub liquidity: u128,
    /// 奖励信息
    pub reward_infos: [WhirlpoolRewardInfo; 3],
    // ... 其他字段
}

/// TickArray 结构
pub struct TickArray {
    pub start_tick_index: i32,
    pub ticks: [Tick; TICK_ARRAY_SIZE],
    pub whirlpool: Pubkey,
}

/// 单个 Tick
pub struct Tick {
    pub initialized: bool,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_a: u128,
    pub fee_growth_outside_b: u128,
    pub reward_growths_outside: [u128; 3],
}
```

---

### 4. 参数结构体 - 🔴 高优先级

**文件**: `src/trading/core/params/orca_whirlpool_params.rs` (新建)

```rust
/// Orca Whirlpool 交易参数
#[derive(Clone, Debug)]
pub struct OrcaWhirlpoolParams {
    /// Whirlpool 地址
    pub whirlpool: Pubkey,
    /// Token A mint
    pub token_a_mint: Pubkey,
    /// Token B mint
    pub token_b_mint: Pubkey,
    /// Token A vault
    pub token_a_vault: Pubkey,
    /// Token B vault
    pub token_b_vault: Pubkey,
    /// Token A program
    pub token_a_program: Pubkey,
    /// Token B program
    pub token_b_program: Pubkey,
    /// Tick arrays (需要 1-3 个)
    pub tick_array_0: Pubkey,
    pub tick_array_1: Option<Pubkey>,
    pub tick_array_2: Option<Pubkey>,
    /// 当前 tick 索引
    pub tick_current_index: i32,
    /// Tick 间距
    pub tick_spacing: u16,
    /// 当前 sqrt 价格
    pub sqrt_price: u128,
    /// Oracle 账户
    pub oracle: Pubkey,
}

impl OrcaWhirlpoolParams {
    pub async fn from_whirlpool_address_by_rpc(
        rpc: &SolanaRpcClient,
        whirlpool_address: &Pubkey,
    ) -> Result<Self, anyhow::Error>;
}
```

---

### 5. 指令构建器 - 🔴 高优先级

**文件**: `src/instruction/orca_whirlpool.rs` (新建)

```rust
/// Orca Whirlpool 指令构建器
pub struct OrcaWhirlpoolInstructionBuilder;

#[async_trait]
impl InstructionBuilder for OrcaWhirlpoolInstructionBuilder {
    async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>>;
    async fn build_sell_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>>;
}
```

**核心挑战**:
- 需要计算正确的 TickArray 地址
- 需要处理跨越多个 TickArray 的交易
- 需要设置正确的 sqrt_price_limit

---

### 6. 数学计算模块 - 🔴 高优先级

**文件**: `src/utils/calc/orca_whirlpool/` (新建目录)

```
src/utils/calc/orca_whirlpool/
├── mod.rs
├── tick_math.rs      # Tick 索引与 sqrt_price 转换
├── swap_math.rs      # Swap 计算逻辑
├── fee_math.rs       # 费用计算
└── price_math.rs     # 价格计算
```

参考 `temp/orca/whirlpools/whirlpools/programs/whirlpool/src/math/` 的实现：

```rust
/// sqrt_price <-> tick_index 转换
pub fn sqrt_price_from_tick_index(tick_index: i32) -> u128;
pub fn tick_index_from_sqrt_price(sqrt_price: u128) -> i32;

/// Swap 计算模拟
pub fn compute_swap(
    whirlpool: &Whirlpool,
    tick_arrays: &TickArrays,
    amount: u64,
    a_to_b: bool,
    amount_specified_is_input: bool,
) -> Result<SwapResult, SwapError>;

/// 价格计算
pub fn get_token_price_in_usd(...);
```

---

### 7. Pool 查询模块 - 🟡 中优先级

**文件**: `src/instruction/utils/orca_whirlpool/` (新建目录)

```
src/instruction/utils/orca_whirlpool/
├── mod.rs
├── pool_queries.rs   # Pool 查询
├── tick_array.rs     # TickArray 处理
└── quotes.rs         # Quote 报价
```

```rust
/// 获取 Whirlpool 数据
pub async fn get_whirlpool_by_address(
    rpc: &SolanaRpcClient,
    whirlpool_address: &Pubkey,
) -> Result<Whirlpool, anyhow::Error>;

/// 通过 mint 查找 Whirlpool
pub async fn find_whirlpool_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, Whirlpool), anyhow::Error>;

/// 计算 TickArray PDA
pub fn tick_array_pk(whirlpool: &Pubkey, program_id: &Pubkey, tick: i32) -> Pubkey;

/// 获取所需的 TickArray 地址
pub fn derive_tick_array_start_indexes(
    curr_tick: i32,
    tick_spacing: u16,
    a_to_b: bool,
) -> TickArrayStartIndexes;
```

---

### 8. Quote 报价模块 - 🟡 中优先级

```rust
/// Exact In 报价
pub async fn quote_exact_in(
    rpc: &SolanaRpcClient,
    params: QuoteExactInParams,
) -> Result<QuoteExactInResult, anyhow::Error>;

/// Exact Out 报价
pub async fn quote_exact_out(
    rpc: &SolanaRpcClient,
    params: QuoteExactOutParams,
) -> Result<QuoteExactOutResult, anyhow::Error>;

/// Swap 模拟 (本地计算)
pub fn simulate_swap(
    whirlpool: &Whirlpool,
    tick_arrays: &TickArrays,
    amount: u64,
    a_to_b: bool,
    amount_specified_is_input: bool,
) -> Result<PostSwapUpdate, SwapError>;
```

---

### 9. 交易解析器 - 🟢 低优先级

**文件**: `src/parser/orca_whirlpool/` (新建目录)

```
src/parser/orca_whirlpool/
├── mod.rs
└── events.rs
```

```rust
pub struct OrcaWhirlpoolParser;

impl DexParserTrait for OrcaWhirlpoolParser {
    async fn parse(&self, adapter: &TransactionAdapter) -> Result<Vec<ParsedTradeInfo>, ParseError>;
    fn protocol(&self) -> DexProtocol;
}
```

---

### 10. 测试覆盖 - 🟡 中优先级

**需要新建的测试文件**:

| 文件 | 说明 |
|------|------|
| `tests/orca_whirlpool_pool_tests.rs` | Pool 查询测试 |
| `tests/orca_whirlpool_swap_tests.rs` | Swap 计算测试 |
| `tests/orca_whirlpool_quote_tests.rs` | Quote 报价测试 |
| `sol-trade-test-utils/src/test_params/orca_params.rs` | 测试参数构建器 |

---

## 实现复杂度分析

### 与 Raydium CLMM 对比

| 复杂度维度 | Orca Whirlpool | Raydium CLMM | 说明 |
|-----------|---------------|--------------|------|
| **数学计算** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | 相当，都基于 Uniswap V3 |
| **TickArray 处理** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 类似的 PDA 计算 |
| **指令构建** | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | 需要多个账户 |
| **SDK 支持** | ⭐⭐⭐ (有官方 Rust SDK) | ⭐⭐ (无官方 SDK) | Orca 有更好的 SDK 支持 |

### 开发工作量估算

| 模块 | 预计工作量 | 优先级 |
|------|-----------|--------|
| 常量和枚举 | 0.5 天 | P0 |
| Pool 类型定义 | 1 天 | P0 |
| 参数结构体 | 1 天 | P0 |
| 数学计算模块 | 3-5 天 | P0 |
| 指令构建器 | 2-3 天 | P0 |
| Pool 查询 | 1-2 天 | P1 |
| Quote 报价 | 1-2 天 | P1 |
| 测试覆盖 | 2-3 天 | P1 |
| 交易解析器 | 1-2 天 | P2 |

**总计**: 约 12-20 个工作日

---

## 实现建议

### 第一阶段 (P0) - 核心交易功能

1. 添加常量定义和 DexType 枚举
2. 实现 Whirlpool 和 TickArray 类型
3. 实现数学计算模块（可参考 `whirlpools-client` SDK）
4. 实现基础指令构建器
5. 集成到 TradingClient

### 第二阶段 (P1) - Quote 和查询

6. 实现 Pool 查询功能
7. 实现 Quote 报价功能
8. 添加测试覆盖

### 第三阶段 (P2) - 解析器

9. 实现交易解析器
10. 文档和示例

---

## 参考资源

### 官方文档
- 🌐 官方网站: https://orca.so
- 📚 开发者文档: https://dev.orca.so/
- 📖 API 文档: https://api.orca.so/docs

### SDK
- **Rust SDK**: `whirlpools-client` (在 `temp/orca/whirlpools/whirlpools/`)
- **TypeScript SDK**: `@orca-so/kit`

### 本地参考代码
- `./temp/orca/whirlpools/whirlpools/` - 核心 Whirlpool 程序和 SDK
- `./temp/common/aggregators/autobahn-router/lib/dex-orca/` - Router 集成示例
- `./temp/raydium/parser/solana-dex-parser/src/parsers/orca/` - 解析器参考

### Program IDs (Mainnet)
- **Whirlpool (current)**: `7ZFam7zqEuFms1PvCxHyeXQGWhrwYpWkQ5jaM7iWpP1g`
- **Whirlpool (deprecated)**: `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`

---

## 注意事项

1. **SDK 可用性**: Orca 提供了官方的 Rust SDK (`whirlpools-client`)，可以大幅降低开发难度
2. **数学复杂性**: CLMM 类型的 DEX 数学计算较复杂，需要仔细测试边界条件
3. **TickArray 管理**: 需要正确处理跨 TickArray 的交易
4. **费用结构**: Orca 有多个费用等级，需要正确处理

---

最后更新: 2026-03-09
