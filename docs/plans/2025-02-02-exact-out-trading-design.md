# Exact Out 交易功能设计文档

**日期**: 2025-02-02
**版本**: 1.0
**状态**: 设计阶段

---

## 目录

- [概述](#概述)
- [核心目标](#核心目标)
- [实现范围](#实现范围)
- [技术架构](#技术架构)
- [数据结构](#数据结构)
- [API 设计](#api-设计)
- [各 DEX 实现](#各-dex-实现)
- [错误处理](#错误处理)
- [测试策略](#测试策略)
- [实现步骤](#实现步骤)
- [文件清单](#文件清单)

---

## 概述

本文档描述了为 Sol Trade SDK 添加 **exact_out 交易支持**的设计方案。Exact out 模式允许用户指定"我想要获得多少代币"，系统自动计算需要支付的金额，这对于目标金额交易（如止盈、精准建仓）非常重要。

### 当前状态

**已有功能：**
- ✅ `buy_quote` - 本地计算（exact_in）
- ✅ `buy_simulate` - 链上模拟（exact_in）
- ✅ `buy` - 真实交易（exact_in + exact_out，通过 `fixed_output_amount`）
- ✅ `sell` - 真实交易（exact_in）

**缺失功能：**
- ❌ `sell_quote` - 本地计算（exact_in）
- ❌ `sell_simulate` - 链上模拟（exact_in + exact_out）
- ❌ `buy_simulate` - 链上模拟（exact_out）

---

## 核心目标

1. **新增 `sell_simulate` 方法**，支持 exact_in 和 exact_out 模式
2. **扩展 `buy_simulate` 方法**，添加 exact_out 支持
3. **覆盖 4 个主要 DEX**：Raydium CLMM, Raydium CPMM, Raydium AMM V4, PumpSwap
4. **保持 API 一致性**，与现有 exact_in 模式对称

**暂不实现：**
- ❌ `sell_quote`（本地计算）
- ❌ `buy_quote`/`sell_quote` 的 exact_out（本地计算）

这些功能将在后续版本中添加，当前专注于链上模拟方案以快速交付。

---

## 实现范围

### 新增方法

```rust
impl TradingClient {
    /// 卖出模拟（支持 exact_in 和 exact_out）
    pub async fn sell_simulate(&self, params: TradeSellParams) -> UnifiedResult<SimulationResult>;
}
```

### 修改方法

```rust
impl TradingClient {
    /// 买入模拟（添加 exact_out 支持）
    pub async fn buy_simulate(&self, params: TradeBuyParams) -> UnifiedResult<SimulationResult>;
}
```

### 支持的 DEX

- ✅ **PumpSwap** - 已支持 exact_out，需验证 `build_sell_instructions`
- 🔄 **Raydium CLMM** - 需添加 exact_out 支持
- 🔄 **Raydium CPMM** - 需添加 exact_out 支持
- 🔄 **Raydium AMM V4** - 需添加 exact_out 支持

---

## 技术架构

### 整体策略

采用**链上模拟优先**策略，利用各 DEX 指令构建器已有的 `fixed_output_amount` 参数支持：

1. **指令构建器检测** `fixed_output_amount` 参数
2. **调用 DEX 特定计算函数**得出所需的 `input_amount`
3. **构建交易指令**（使用计算出的 input_amount）
4. **执行链上模拟**验证并返回结果

### 架构图

```
┌─────────────────────────────────────────────────────────┐
│                    TradingClient                         │
├─────────────────────────────────────────────────────────┤
│  buy_simulate(params)                                   │
│  sell_simulate(params)  [NEW]                           │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│              SwapParams (fixed_output_amount)            │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│           InstructionBuilder (per DEX)                   │
├─────────────────────────────────────────────────────────┤
│  build_buy_instructions()  │  build_sell_instructions() │
│  - 检测 fixed_output_amount                              │
│  - 调用 quote_exact_out()                                 │
│  - 计算 input_amount                                      │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│              quote_exact_out() (per DEX)                 │
├─────────────────────────────────────────────────────────┤
│  CLMM   → 遍历 tick array + get_next_sqrt_price_from_output  │
│  CPMM   → 恒定乘积公式反解                                     │
│  AMM V4 → 恒定乘积公式反解                                     │
│  PumpSwap → buy_base_input_internal [已有]                   │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│               SimulationResult                            │
├─────────────────────────────────────────────────────────┤
│  amount_in  (exact_in: 用户输入; exact_out: 计算值)         │
│  amount_out (exact_in: 计算值; exact_out: 用户请求)         │
│  compute_units, transaction_fee, ...                      │
└─────────────────────────────────────────────────────────┘
```

---

## 数据结构

### SimulationResult 扩展

**添加 `amount_in` 字段以支持 exact_out 模式：**

```rust
pub struct SimulationResult {
    /// 输出金额
    /// - exact_in 模式：计算得到的输出
    /// - exact_out 模式：用户请求的输出
    pub amount_out: u64,

    /// 新增：输入金额
    /// - exact_in 模式：用户输入
    /// - exact_out 模式：计算得到的输入
    pub amount_in: u64,

    /// 手续费金额
    pub fee_amount: u64,

    /// 计算单元消耗
    pub compute_units: u64,

    /// 交易费用
    pub transaction_fee: u64,

    /// 是否成功
    pub success: bool,

    /// 错误信息
    pub error: Option<String>,

    /// 程序日志
    pub logs: Option<Vec<String>>,

    /// DEX 类型
    pub dex_type: DexType,
}
```

### TradeSellParams 添加 fixed_output_token_amount

```rust
pub struct TradeSellParams {
    // ... 现有字段 ...

    /// 固定输出金额（用于 exact_out 模式）
    /// If set, ignore input_token_amount and calculate required input
    pub fixed_output_token_amount: Option<u64>,

    // ... 其他字段 ...
}
```

### 语义说明

**exact_in 模式：**
- `amount_in` = 用户输入的金额
- `amount_out` = 计算得到的输出金额

**exact_out 模式：**
- `amount_in` = 计算得到的输入金额
- `amount_out` = 用户请求的输出金额

---

## API 设计

### buy_simulate 参数处理

```rust
pub async fn buy_simulate(&self, params: TradeBuyParams) -> UnifiedResult<SimulationResult> {
    // 检查模式：exact_in 还是 exact_out
    let is_exact_out = params.fixed_output_token_amount.is_some();

    if is_exact_out {
        // exact_out 模式：忽略 input_token_amount 的验证
        if params.fixed_output_token_amount.unwrap() == 0 {
            return Err(UnifiedTradingError::InvalidParameters(
                "fixed_output_token_amount must be > 0".into()
            ));
        }
    } else {
        // exact_in 模式：现有验证逻辑
        if params.input_token_amount == 0 {
            return Err(UnifiedTradingError::InvalidParameters(
                "amount must be > 0".into()
            ));
        }
    }

    // 继续现有的模拟逻辑...
}
```

### sell_simulate 实现

```rust
/// 卖出模拟（exact_in 和 exact_out）
///
/// # 参数
///
/// * `params` - 卖出参数
///   - `input_token_amount`: 要卖出的代币数量（exact_in 模式）
///   - `fixed_output_token_amount`: 期望获得的输出代币数量（exact_out 模式，可选）
///
/// # 返回
///
/// 返回 `SimulationResult` 包含：
/// - `amount_in`: 实际卖出的数量
/// - `amount_out`: 获得的输出数量
/// - `compute_units`: 计算单元消耗
/// - `transaction_fee`: 交易费用
pub async fn sell_simulate(&self, params: TradeSellParams) -> UnifiedResult<SimulationResult> {
    // 1. 参数验证（与 buy_simulate 对称）
    let is_exact_out = params.fixed_output_token_amount.is_some();

    if is_exact_out {
        if params.fixed_output_token_amount.unwrap() == 0 {
            return Err(UnifiedTradingError::InvalidParameters(
                "fixed_output_token_amount must be > 0".into()
            ));
        }
    } else {
        if params.input_token_amount == 0 {
            return Err(UnifiedTradingError::InvalidParameters(
                "amount must be > 0".into()
            ));
        }
    }

    // 2. 获取 output_mint（卖出获得的代币）
    let output_mint = Self::get_output_mint(&params.output_token_type);

    // 3. 构建 SwapParams（完全复用 sell 中的逻辑）
    let swap_params = SwapParams {
        rpc: Some(self.rpc.clone()),
        payer: self.payer.clone(),
        trade_type: TradeType::Sell,
        input_mint: params.mint,
        output_mint,
        input_token_program: None,
        output_token_program: None,
        input_amount: Some(params.input_token_amount),
        slippage_basis_points: params.slippage_basis_points,
        address_lookup_table_account: params.address_lookup_table_account,
        recent_blockhash: params.recent_blockhash,
        wait_transaction_confirmed: false,
        protocol_params: params.extension_params.clone(),
        open_seed_optimize: self.use_seed_optimize,
        swqos_clients: self.swqos_clients.clone(),
        middleware_manager: self.middleware_manager.clone(),
        durable_nonce: params.durable_nonce,
        with_tip: true,
        create_input_mint_ata: false,
        close_input_mint_ata: params.close_input_token_ata,
        create_output_mint_ata: params.create_output_ata,
        close_output_mint_ata: params.close_output_ata,
        fixed_output_amount: params.fixed_output_token_amount,  // 传递 exact_out 参数
        gas_fee_strategy: params.gas_fee_strategy,
        simulate: true,  // 关键：模拟模式
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    // 4. 构建指令（根据 DEX 类型分发）
    use crate::trading::core::traits::InstructionBuilder;
    let instructions = match params.dex_type {
        DexType::RaydiumClmm => {
            crate::instruction::raydium_clmm::RaydiumClmmInstructionBuilder
                .build_sell_instructions(&swap_params).await
        },
        DexType::RaydiumCpmm => {
            crate::instruction::raydium_cpmm::RaydiumCpmmInstructionBuilder
                .build_sell_instructions(&swap_params).await
        },
        DexType::RaydiumAmmV4 => {
            crate::instruction::raydium_amm_v4::RaydiumAmmV4InstructionBuilder
                .build_sell_instructions(&swap_params).await
        },
        DexType::PumpSwap => {
            crate::instruction::pumpswap::PumpSwapInstructionBuilder
                .build_sell_instructions(&swap_params).await
        },
        _ => {
            return Err(UnifiedTradingError::UnsupportedDex(params.dex_type));
        }
    }.map_err(|e| UnifiedTradingError::TransactionBuildError(e.to_string()))?;

    // 5. 执行链上模拟
    let simulation_result = self.simulate_transaction_internal(
        &swap_params,
        &instructions,
    ).await?;

    Ok(simulation_result)
}
```

### 辅助方法：get_output_mint

```rust
impl TradingClient {
    fn get_output_mint(output_type: &TradeTokenType) -> Pubkey {
        match output_type {
            TradeTokenType::SOL => crate::constants::SOL_MINT,
            TradeTokenType::WSOL => crate::constants::WSOL_TOKEN_ACCOUNT,
            TradeTokenType::USDC => crate::constants::USDC_MINT,
            TradeTokenType::USDT => crate::constants::USDT_MINT,
            TradeTokenType::USD1 => crate::constants::USD1_MINT,
            TradeTokenType::Token(mint) => *mint,
        }
    }
}
```

### buy_simulate 与 sell_simulate 的对称性

| 方面 | buy_simulate | sell_simulate |
|------|-------------|---------------|
| 输入代币 | SOL/USDC/USDT | 目标代币（params.mint）|
| 输出代币 | 目标代币（params.mint）| SOL/USDC/USDT |
| TradeType | Buy | Sell |
| InstructionBuilder | `build_buy_instructions` | `build_sell_instructions` |
| 其他逻辑 | 完全相同 | 完全相同 |

---

## 各 DEX 实现

### Raydium CLMM

**使用官方数学库的 `get_next_sqrt_price_from_output` 函数：**

```rust
// src/utils/calc/raydium_clmm.rs

pub struct QuoteExactOutResult {
    pub amount_in: u64,
    pub fee_amount: u64,
    pub price_impact_bps: Option<u64>,
}

pub fn quote_exact_out(
    pool_state: &PoolState,
    amount_out: u64,
    zero_for_one: bool,  // true=token0->token1, false=token1->token0
) -> Result<QuoteExactOutResult, String> {
    let mut sqrt_price_x64 = pool_state.sqrt_price_x64;
    let mut amount_in = 0u64;
    let mut remaining_amount_out = amount_out;
    let mut fee_amount = 0u64;

    // 遍历 tick array 直到满足输出金额
    while remaining_amount_out > 0 {
        // 使用官方库的 exact_out 函数
        let next_price = sqrt_price_math::get_next_sqrt_price_from_output(
            sqrt_price_x64,
            current_liquidity,
            min(remaining_amount_out, tick_array_capacity),
            zero_for_one,
        );

        // 计算本步骤的输入和手续费
        // ... (详细实现见后续代码)

        sqrt_price_x64 = next_price;
    }

    Ok(QuoteExactOutResult { amount_in, fee_amount, .. })
}
```

**关键点：**
- 利用已有的 `sqrt_price_math` 库
- 需要处理 tick array 边界
- 计算累计的手续费

### Raydium CPMM

**恒定乘积公式反解：**

```rust
// src/utils/calc/raydium_cpmm.rs

pub fn quote_exact_out(
    pool_state: &PoolState,
    amount_out: u64,
    is_token0_in: bool,
) -> Result<QuoteExactOutResult, String> {
    let (reserve_in, reserve_out) = if is_token0_in {
        (pool_state.token0_reserve, pool_state.token1_reserve)
    } else {
        (pool_state.token1_reserve, pool_state.token0_reserve)
    };

    // 恒定乘积公式: (reserve_in + amount_in) * (reserve_out - amount_out) = reserve_in * reserve_out
    // 反解: amount_in = (reserve_in * amount_out) / (reserve_out - amount_out)

    if amount_out >= reserve_out {
        return Err("Insufficient liquidity".to_string());
    }

    // 使用 checked arithmetic 防止溢出
    let amount_in = (reserve_in as u128)
        .checked_mul(amount_out as u128)
        .and_then(|p| p.checked_div((reserve_out - amount_out) as u128))
        .ok_or_else(|| "Calculation overflow".to_string())?
        as u64;

    // 计算手续费
    let fee_amount = compute_fee(amount_in, pool_state.fee_rate);

    Ok(QuoteExactOutResult {
        amount_in: amount_in + fee_amount,
        fee_amount,
        price_impact_bps: calculate_price_impact(amount_in, reserve_in),
    })
}
```

### Raydium AMM V4

**与 CPMM 相同的恒定乘积逻辑：**

```rust
// src/utils/calc/raydium_amm_v4.rs

pub fn quote_exact_out(
    pool_state: &PoolState,
    amount_out: u64,
    is_coin_in: bool,
) -> Result<QuoteExactOutResult, String> {
    // 与 CPMM 相同的公式
    // 唯一区别：字段名是 coin_reserve 和 pc_reserve
    let (reserve_in, reserve_out) = if is_coin_in {
        (pool_state.coin_reserve, pool_state.pc_reserve)
    } else {
        (pool_state.pc_reserve, pool_state.coin_reserve)
    };

    // 相同的计算逻辑...
}
```

### PumpSwap

**✅ 已支持，无需修改**

已有 `buy_base_input_internal` 和 `sell_quote_input_internal` 函数。

---

## 指令构建器修改

### 通用修改模式（适用于 CLMM/CPMM/AMM V4）

**修改各 DEX 的 `build_buy_instructions` 和 `build_sell_instructions` 方法：**

```rust
// src/instruction/raydium_clmm.rs (示例)

async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
    let protocol_params = params.protocol_params
        .as_any()
        .downcast_ref::<RaydiumClmmParams>()
        .ok_or_else(|| anyhow!("Invalid protocol params"))?;

    // ========================================
    // 新增：exact_out 模式处理
    // ========================================
    let (input_amount, is_exact_out) = if let Some(fixed_output) = params.fixed_output_amount {
        // exact_out 模式：计算所需输入
        let pool_state = get_pool_by_address(
            params.rpc.as_ref().unwrap(),
            &protocol_params.pool_state
        ).await?;

        // zero_for_one: token0->token1
        let zero_for_one = determine_zero_for_one(&params, &pool_state)?;
        let quote_result = crate::utils::calc::raydium_clmm::quote_exact_out(
            &pool_state,
            fixed_output,
            zero_for_one,
        )?;

        (quote_result.amount_in, true)
    } else {
        // exact_in 模式：现有逻辑
        let input_amount = params.input_amount.ok_or_else(|| anyhow!("Input amount required"))?;
        (input_amount, false)
    };

    // ========================================
    // 现有验证逻辑（修改为允许 exact_out）
    // ========================================
    // 移除原来的 input_amount == 0 检查
    // 因为 exact_out 模式下 input_amount 是计算出来的

    // ========================================
    // 继续现有的指令构建逻辑
    // ========================================
    // ... 使用计算出的 input_amount 构建指令 ...
}
```

### 关键修改点

**1. 参数验证逻辑修改**

**旧代码：**
```rust
let input_amount = params.input_amount.ok_or_else(|| anyhow!("Input amount is required"))?;
if input_amount == 0 {
    return Err(anyhow!("Amount cannot be zero"));
}
```

**新代码：**
```rust
let has_fixed_output = params.fixed_output_amount.is_some();
if !has_fixed_output {
    // exact_in 模式：需要 input_amount
    let input_amount = params.input_amount.ok_or_else(|| anyhow!("Input amount required"))?;
    if input_amount == 0 {
        return Err(anyhow!("Amount cannot be zero"));
    }
}
// exact_out 模式：input_amount 稍后计算
```

**2. Pool 查询时机**

- **exact_in**：Pool 查询可能在后面进行（或已缓存）
- **exact_out**：需要先查询 Pool 才能计算 input_amount

**3. 辅助函数：`determine_zero_for_one`**

```rust
// 根据交易方向确定 zero_for_one 参数
fn determine_zero_for_one(params: &SwapParams, pool_state: &PoolState) -> Result<bool, anyhow::Error> {
    let input_is_token0 = params.input_mint == pool_state.token0_mint;
    Ok(input_is_token0)
}

// 卖出版本
fn determine_zero_for_one_sell(params: &SwapParams, pool_state: &PoolState) -> Result<bool, anyhow::Error> {
    // 卖出：input_mint 是目标代币，output_mint 是 SOL/USDC
    // 如果 input_mint == token0_mint，则是 token0 -> token1 (zero_for_one = true)
    Ok(params.input_mint == pool_state.token0_mint)
}
```

### PumpSwap 检查

**需要验证 PumpSwap 的 `build_sell_instructions` 是否已支持 exact_out：**

```rust
// 检查 src/instruction/pumpswap.rs 中的 build_sell_instructions
// 如果没有 fixed_output_amount 处理，需要添加（与 buy 对称）
```

---

## 错误处理

### 错误类型定义

```rust
pub enum QuoteError {
    InsufficientLiquidity {
        requested: u64,
        available: u64,
        pool: Pubkey,
    },
    CalculationOverflow,
    InvalidPoolState,
    TickArrayExhausted, // CLMM 特有
}
```

### 错误处理策略

**1. 流动性不足错误**

```rust
// 当 exact_out 请求超过池子流动性时
if amount_out >= reserve_out {
    return Err(QuoteError::InsufficientLiquidity {
        requested: amount_out,
        available: reserve_out,
        pool: pool_address,
    }.into());
}
```

**2. 参数验证错误**

```rust
pub async fn buy_simulate(&self, params: TradeBuyParams) -> UnifiedResult<SimulationResult> {
    // 检查 exact_in 和 exact_out 不能同时为空
    let has_input = params.input_token_amount > 0;
    let has_output = params.fixed_output_token_amount.is_some();

    if !has_input && !has_output {
        return Err(UnifiedTradingError::InvalidParameters(
            "Either input_token_amount or fixed_output_token_amount must be specified".into()
        ));
    }

    // 如果两者都设置了，优先使用 exact_out
    if has_input && has_output {
        log::warn!("Both input_token_amount and fixed_output_token_amount set, using exact_out mode");
    }

    // ...
}
```

**3. Pool 查询失败**

```rust
let pool_state = match get_pool_by_address(rpc, pool_address).await {
    Ok(pool) => pool,
    Err(e) => {
        return Err(UnifiedTradingError::RpcError(
            format!("Failed to fetch pool state: {}", e)
        ));
    }
};
```

**4. 模拟失败处理**

```rust
let simulation_result = match self.simulate_transaction_internal(...).await {
    Ok(result) => result,
    Err(e) => {
        // 区分不同类型的模拟失败
        if e.to_string().contains("Insufficient") {
            return Ok(SimulationResult {
                success: false,
                error: Some(e.to_string()),
                amount_in: 0,
                amount_out: 0,
                ..
            });
        }
        return Err(e.into());
    }
};
```

### 边界情况处理

**1. 零值边界**

```rust
// exact_out 模式下的零值检查
if let Some(fixed_output) = params.fixed_output_token_amount {
    if fixed_output == 0 {
        return Err(UnifiedTradingError::InvalidParameters(
            "fixed_output_token_amount cannot be zero".into()
        ));
    }
}
```

**2. 极端滑点保护**

```rust
// 在模拟结果中检查滑点是否过大
if let Some(ref logs) = simulation_result.logs {
    if is_slippage_too_high(logs) {
        return Ok(SimulationResult {
            success: false,
            error: Some("Slippage too high, transaction would fail".into()),
            ..
        });
    }
}
```

**3. Tick Array 边界（CLMM 特有）**

```rust
// 在 CLMM exact_out 计算中
while remaining_amount_out > 0 && tick_array_index < MAX_TICK_ARRAYS {
    // 计算当前 tick array 的输出
    // ...

    tick_array_index += 1;
}

if remaining_amount_out > 0 {
    return Err("Insufficient tick arrays for this swap size".into());
}
```

**4. 数值溢出保护**

```rust
// 在计算中使用 checked arithmetic
let amount_in = (reserve_in as u128)
    .checked_mul(amount_out as u128)
    .and_then(|p| p.checked_div((reserve_out - amount_out) as u128))
    .ok_or_else(|| anyhow::anyhow!("Calculation overflow"))?
    as u64;
```

---

## 测试策略

### 单元测试

**1. quote_exact_out 函数测试**

```rust
// tests/quote_exact_out_tests.rs

#[tokio::test]
async fn test_raydium_clmm_quote_exact_out() {
    let rpc = SolanaRpcClient::new("http://127.0.0.1:8899");
    let pool_address = Pubkey::from_str("POOL_ADDRESS").unwrap();

    let pool_state = get_pool_by_address(&rpc, &pool_address).await.unwrap();

    // 测试 small amount
    let result = quote_exact_out(&pool_state, 1_000_000, true).unwrap();
    assert!(result.amount_in > 0);
    assert!(result.fee_amount > 0);

    // 测试 large amount
    let result = quote_exact_out(&pool_state, 100_000_000_000, true).unwrap();
    assert!(result.amount_in > result.fee_amount);

    // 测试边界情况：超过流动性
    let result = quote_exact_out(&pool_state, u64::MAX, true);
    assert!(result.is_err());
}
```

**2. 各 DEX 的 exact_out 测试**

```rust
// 测试 CPMM
#[tokio::test]
async fn test_raydium_cpmm_quote_exact_out() {
    // 类似的测试结构
}

// 测试 AMM V4
#[tokio::test]
async fn test_raydium_amm_v4_quote_exact_out() {
    // 类似的测试结构
}

// 测试 PumpSwap（已有函数，验证正确性）
#[tokio::test]
async fn test_pumpswap_quote_exact_out() {
    // 验证已有函数的正确性
}
```

### 集成测试

**1. buy_simulate exact_out 测试**

```rust
// tests/buy_simulate_exact_out_tests.rs

#[tokio::test]
#[serial] // 使用 serial_test 避免冲突
async fn test_buy_simulate_exact_out_clmm() {
    let (client, payer) = setup_test_client().await;

    let params = TradeBuyParams {
        input_token_amount: 0,  // exact_out 模式下被忽略
        fixed_output_token_amount: Some(1_000_000),  // 期望获得 1M token
        input_token_type: TradeTokenType::WSOL,
        mint: TEST_TOKEN_MINT,
        dex_type: DexType::RaydiumClmm,
        extension_params: DexParamEnum::RaydiumClmm(...),
        ..Default::default()
    };

    let result = client.buy_simulate(params).await.unwrap();

    // 验证结果
    assert!(result.success);
    assert_eq!(result.amount_out, 1_000_000);
    assert!(result.amount_in > 0);
    assert!(result.compute_units > 0);
}

#[tokio::test]
async fn test_buy_simulate_exact_out_cpmm() {
    // 类似结构
}

#[tokio::test]
async fn test_buy_simulate_exact_out_amm_v4() {
    // 类似结构
}
```

**2. sell_simulate exact_in 和 exact_out 测试**

```rust
// tests/sell_simulate_tests.rs

#[tokio::test]
async fn test_sell_simulate_exact_in_clmm() {
    let params = TradeSellParams {
        input_token_amount: 1_000_000,  // 卖出 1M token
        fixed_output_token_amount: None,  // exact_in 模式
        ..Default::default()
    };

    let result = client.sell_simulate(params).await.unwrap();

    assert!(result.success);
    assert_eq!(result.amount_in, 1_000_000);
    assert!(result.amount_out > 0);
}

#[tokio::test]
async fn test_sell_simulate_exact_out_clmm() {
    let params = TradeSellParams {
        input_token_amount: 0,
        fixed_output_token_amount: Some(1_000_000),  // 期望获得 1M WSOL
        ..Default::default()
    };

    let result = client.sell_simulate(params).await.unwrap();

    assert!(result.success);
    assert_eq!(result.amount_out, 1_000_000);
    assert!(result.amount_in > 0);
}
```

### 链上模拟验证测试

**验证 exact_out 的准确性：**

```rust
// 对比 exact_out 和 exact_in 的结果
#[tokio::test]
async fn test_exact_out_simulation_accuracy() {
    let (client, _) = setup_test_client().await;

    // exact_out 模式
    let exact_out_params = TradeBuyParams {
        fixed_output_token_amount: Some(1_000_000),
        ..Default::default()
    };

    let exact_out_result = client.buy_simulate(exact_out_params).await.unwrap();

    // 使用 exact_out 的 input 作为 exact_in 的 input
    let exact_in_params = TradeBuyParams {
        input_token_amount: exact_out_result.amount_in,
        fixed_output_token_amount: None,
        ..Default::default()
    };

    let exact_in_result = client.buy_simulate(exact_in_params).await.unwrap();

    // 验证结果一致性（允许 < 0.1% 误差）
    let error_rate = (exact_in_result.amount_out as f64 - exact_out_result.amount_out as f64)
        / exact_out_result.amount_out as f64;
    assert!(error_rate.abs() < 0.001);
}
```

---

## 实现步骤

### 阶段 1：基础计算函数（优先级：高）

**任务列表：**

1. **CLMM exact_out 计算**
   - 文件：`src/utils/calc/raydium_clmm.rs`
   - 新增函数：`quote_exact_out()`
   - 使用官方数学库的 `get_next_sqrt_price_from_output`
   - 处理 tick array 遍历

2. **CPMM exact_out 计算**
   - 文件：`src/utils/calc/raydium_cpmm.rs`
   - 新增函数：`quote_exact_out()`
   - 恒定乘积公式反解

3. **AMM V4 exact_out 计算**
   - 文件：`src/utils/calc/raydium_amm_v4.rs`
   - 新增函数：`quote_exact_out()`
   - 与 CPMM 相同逻辑

### 阶段 2：指令构建器修改（优先级：高）

**任务列表：**

4. **CLMM build_buy_instructions 修改**
   - 文件：`src/instruction/raydium_clmm.rs`
   - 添加 `fixed_output_amount` 检测
   - 调用 `quote_exact_out` 计算 input_amount

5. **CLMM build_sell_instructions 修改**
   - 文件：`src/instruction/raydium_clmm.rs`
   - 与 buy 对称

6. **CPMM build_buy_instructions 修改**
   - 文件：`src/instruction/raydium_cpmm.rs`

7. **CPMM build_sell_instructions 修改**
   - 文件：`src/instruction/raydium_cpmm.rs`

8. **AMM V4 build_buy_instructions 修改**
   - 文件：`src/instruction/raydium_amm_v4.rs`

9. **AMM V4 build_sell_instructions 修改**
   - 文件：`src/instruction/raydium_amm_v4.rs`

10. **PumpSwap build_sell_instructions 检查**
    - 文件：`src/instruction/pumpswap.rs`
    - 如果未支持，添加 exact_out 处理

### 阶段 3：TradingClient API 实现（优先级：高）

**任务列表：**

11. **添加 SimulationResult.amount_in 字段**
    - 文件：`src/trading/results.rs`

12. **实现 sell_simulate 方法**
    - 文件：`src/lib.rs`
    - 添加 `get_output_mint` 辅助方法

13. **修改 buy_simulate 参数验证**
    - 文件：`src/lib.rs`
    - 支持 exact_out 模式

### 阶段 4：测试（优先级：中）

**任务列表：**

14. **单元测试**
    - `tests/quote_exact_out_tests.rs`

15. **集成测试**
    - `tests/buy_simulate_exact_out_tests.rs`
    - `tests/sell_simulate_tests.rs`

16. **准确性验证测试**
    - `tests/verify_exact_out_simulation.rs`

### 阶段 5：文档（优先级：低）

**任务列表：**

17. **更新 migration-v4.md**
    - 添加 exact_out 示例

18. **API 文档注释**
    - 为新增方法添加文档

---

## 文件清单

### 需要修改的文件

```
src/
├── utils/
│   └── calc/
│       ├── raydium_clmm.rs          → 新增 quote_exact_out
│       ├── raydium_cpmm.rs          → 新增 quote_exact_out
│       └── raydium_amm_v4.rs        → 新增 quote_exact_out
├── instruction/
│   ├── raydium_clmm.rs              → 修改 build_*_instructions
│   ├── raydium_cpmm.rs              → 修改 build_*_instructions
│   ├── raydium_amm_v4.rs            → 修改 build_*_instructions
│   └── pumpswap.rs                  → 检查 build_sell_instructions
├── trading/
│   └── results.rs                   → 添加 amount_in 字段
└── lib.rs                           → 实现 sell_simulate，修改 buy_simulate

tests/
├── quote_exact_out_tests.rs         → 新增
├── buy_simulate_exact_out_tests.rs  → 新增
├── sell_simulate_tests.rs           → 新增
└── verify_exact_out_simulation.rs   → 新增
```

### 估计工作量

- **阶段 1-3**（核心实现）：2-3 天
- **阶段 4**（测试）：1 天
- **阶段 5**（文档）：0.5 天

---

## 总结

### 核心决策

1. **实现范围**：`sell_simulate` 新增，`buy_simulate` 添加 exact_out 支持
2. **技术方案**：链上模拟优先，暂不实现本地计算 exact_out
3. **参数处理**：exact_out 模式下忽略 `input_token_amount`
4. **返回值**：添加 `amount_in` 字段保持语义清晰
5. **支持的 DEX**：CLMM, CPMM, AMM V4, PumpSwap

### 关键设计点

- 利用现有 `fixed_output_amount` 参数机制
- 各 DEX 独立实现 exact_out 计算函数
- 指令构建器统一处理 exact_in/exact_out 模式
- 完整的错误处理和边界情况检查
- 全面的测试覆盖

### 后续工作

**未来版本可以考虑：**
- `sell_quote` 本地计算实现
- `buy_quote`/`sell_quote` 的 exact_out 本地计算
- 性能优化（CLMM tick array 预加载、计算缓存等）

---

**文档版本**: 1.0
**最后更新**: 2025-02-02
