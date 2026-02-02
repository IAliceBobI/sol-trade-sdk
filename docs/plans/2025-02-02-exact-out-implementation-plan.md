# Exact Out 交易功能实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现 sell_simulate 和 buy_simulate 的 exact_out 支持，覆盖 4 个主要 DEX（CLMM, CPMM, AMM V4, PumpSwap）

**Architecture:** 链上模拟优先，利用现有 fixed_output_amount 参数机制。各 DEX 独立实现 exact_out 计算函数，指令构建器统一处理 exact_in/exact_out 模式。

**Tech Stack:** Rust, Solana SDK 3.0.x, tokio, anyhow

---

## 重要发现与修正

### 原设计文档中的问题

1. **`simulate_transaction_internal` 不存在**
   - 实际使用：`crate::utils::simulation_based_calc::simulate_swap_transaction`
   - 位置：`src/lib.rs:713-723`

2. **SimulationResult 缺少 `amount_in` 字段**
   - 当前只有 `amount_out`
   - 需要添加 `amount_in: u64` 字段

3. **返回值构建**
   - `buy_simulate` 返回：`SimulationResult { amount_out: sim_result.actual_output_amount, ... }`
   - 需要同时设置 `amount_in`

### 关键依赖

- ✅ `simulate_swap_transaction` - 链上模拟函数
- ✅ CLMM 官方数学库 (`sqrt_price_math::get_next_sqrt_price_from_output`)
- ✅ PumpSwap 已有 `buy_base_input_internal`
- ❌ 需要新增：CPMM/AMM V4/CLMM 的 `quote_exact_out` 函数

---

## Task 1: 添加 SimulationResult.amount_in 字段

**Files:**
- Modify: `src/trading/results.rs`

**Step 1: 修改 SimulationResult 结构体**

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

**Step 2: 更新 buy_simulate 返回值构建**

在 `src/lib.rs:726-735` 修改：

```rust
Ok(SimulationResult {
    amount_out: sim_result.actual_output_amount,
    amount_in: params.input_amount.unwrap_or(0), // 添加此行
    fee_amount: 0,
    compute_units: sim_result.units_consumed.unwrap_or(0),
    transaction_fee: sim_result.transaction_fee,
    success: sim_result.success,
    error: sim_result.error,
    logs: sim_result.logs,
    dex_type: params.dex_type,
})
```

**Step 3: 编译检查**

Run: `cargo check --package sol-trade-sdk`
Expected: 编译成功，无错误

**Step 4: 提交**

```bash
git add src/trading/results.rs src/lib.rs
git commit -m "feat(results): 添加 SimulationResult.amount_in 字段支持 exact_out 模式"
```

---

## Task 2: 添加 TradeSellParams.fixed_output_token_amount 字段

**Files:**
- Modify: `src/lib.rs` (TradeSellParams 结构体)

**Step 1: 查找 TradeSellParams 定义**

Run: `grep -n "pub struct TradeSellParams" src/lib.rs`
Expected: 找到结构体定义位置（约在第 271 行）

**Step 2: 添加 fixed_output_token_amount 字段**

在 `mint` 字段后添加：

```rust
pub struct TradeSellParams {
    // Trading configuration
    /// The DEX protocol to use for the trade
    pub dex_type: DexType,
    /// Type of the token to sell
    pub output_token_type: TradeTokenType,
    /// Public key of the token to sell
    pub mint: Pubkey,
    /// Amount of tokens to sell (in smallest token units)
    pub input_token_amount: u64,

    /// 新增：固定输出金额（用于 exact_out 模式）
    /// If set, ignore input_token_amount and calculate required input
    pub fixed_output_token_amount: Option<u64>,

    /// Optional slippage tolerance in basis points (e.g., 100 = 1%)
    pub slippage_basis_points: Option<u64>,
    // ... 其他字段保持不变
}
```

**Step 3: 更新 TradeSellParams 的 Default 实现（如果存在）**

Run: `grep -A 20 "impl Default for TradeSellParams" src/lib.rs`

如果找到，添加：
```rust
fn default() -> Self {
    Self {
        dex_type: Default::default(),
        output_token_type: TradeTokenType::WSOL,
        mint: Pubkey::default(),
        input_token_amount: 0,
        fixed_output_token_amount: None, // 添加此行
        slippage_basis_points: None,
        // ...
    }
}
```

**Step 4: 编译检查**

Run: `cargo check --package sol-trade-sdk`
Expected: 编译成功

**Step 5: 提交**

```bash
git add src/lib.rs
git commit -m "feat(params): 添加 TradeSellParams.fixed_output_token_amount 支持 exact_out"
```

---

## Task 3: 实现 get_output_mint 辅助方法

**Files:**
- Modify: `src/lib.rs`

**Step 1: 在 TradingClient impl 中添加辅助方法**

在 `get_input_mint` 方法附近添加：

```rust
impl TradingClient {
    // ... 现有代码 ...

    /// 获取输出代币的 mint 地址
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

    // ... 其他代码 ...
}
```

**Step 2: 验证 SOL_MINT 常量存在**

Run: `grep "pub const SOL_MINT" src/constants.rs`
Expected: 找到常量定义

**Step 3: 编译检查**

Run: `cargo check --package sol-trade-sdk`
Expected: 编译成功

**Step 4: 提交**

```bash
git add src/lib.rs
git commit -m "feat(client): 添加 get_output_mint 辅助方法"
```

---

## Task 4: 实现 CPMM quote_exact_out 函数

**Files:**
- Create: `src/utils/calc/raydium_cpmm.rs` (新增函数)

**Step 1: 查看现有 quote_exact_in 实现**

Run: `grep -A 30 "pub fn quote_exact_in" src/utils/calc/raydium_cpmm.rs`

**Step 2: 添加 QuoteExactOutResult 结构体**

```rust
pub struct QuoteExactOutResult {
    pub amount_in: u64,
    pub fee_amount: u64,
    pub price_impact_bps: Option<u64>,
}
```

**Step 3: 实现 quote_exact_out 函数**

```rust
/// Quote an exact-out swap against a Raydium CPMM pool
///
/// # Arguments
///
/// * `pool_state` - Pool state containing reserves and fee rate
/// * `amount_out` - Desired output amount
/// * `is_token0_in` - true if token0 is the input, false if token1 is the input
///
/// # Returns
///
/// Returns `QuoteExactOutResult` containing the required input amount and fees
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

    // 流动性检查
    if amount_out >= reserve_out {
        return Err(format!(
            "Insufficient liquidity: requested={}, available={}",
            amount_out, reserve_out
        ));
    }

    // 恒定乘积公式: (reserve_in + amount_in) * (reserve_out - amount_out) = reserve_in * reserve_out
    // 反解: amount_in = (reserve_in * amount_out) / (reserve_out - amount_out)

    let numerator = (reserve_in as u128).checked_mul(amount_out as u128)
        .ok_or_else(|| "Calculation overflow in numerator".to_string())?;

    let denominator = (reserve_out as u128).checked_sub(amount_out as u128)
        .ok_or_else(|| "Invalid reserve calculation".to_string())?;

    let amount_in = numerator.checked_div(denominator)
        .ok_or_else(|| "Calculation overflow in division".to_string())?
        as u64;

    // 计算手续费 (使用现有的 compute_fee 函数)
    let fee_rate = pool_state.swap_fee_numerator as u64;
    let fee_amount = (amount_in as u128)
        .checked_mul(fee_rate as u128)
        .and_then(|p| p.checked_div(1_000_000u128))
        .ok_or_else(|| "Fee calculation overflow".to_string())?
        as u64;

    let total_amount_in = amount_in.checked_add(fee_amount)
        .ok_or_else(|| "Total amount calculation overflow".to_string())?;

    // 计算价格影响
    let price_impact_bps = if reserve_in > 0 {
        let impact = (amount_out as u128)
            .checked_mul(10_000u128)
            .and_then(|p| p.checked_div(reserve_out as u128))
            .unwrap_or(0);
        Some(impact as u64)
    } else {
        None
    };

    Ok(QuoteExactOutResult {
        amount_in: total_amount_in,
        fee_amount,
        price_impact_bps,
    })
}
```

**Step 4: 编译检查**

Run: `cargo check --package sol-trade-sdk`
Expected: 编译成功，可能需要修复一些小错误

**Step 5: 编写单元测试**

在 `tests/` 目录创建测试文件或添加到现有测试：

```rust
#[tokio::test]
async fn test_raydium_cpmm_quote_exact_out_basic() {
    use sol_trade_sdk::utils::calc::raydium_cpmm::{quote_exact_out, PoolState};

    let pool = PoolState {
        token0_reserve: 1_000_000_000,
        token1_reserve: 2_000_000_000,
        swap_fee_numerator: 2500, // 0.25%
        ..Default::default()
    };

    let result = quote_exact_out(&pool, 100_000, true).unwrap();

    assert!(result.amount_in > 0);
    assert!(result.fee_amount > 0);
    assert!(result.amount_in > result.fee_amount);
}

#[tokio::test]
async fn test_raydium_cpmm_quote_exact_out_insufficient_liquidity() {
    use sol_trade_sdk::utils::calc::raydium_cpmm::{quote_exact_out, PoolState};

    let pool = PoolState {
        token0_reserve: 1_000_000,
        token1_reserve: 1_000_000,
        ..Default::default()
    };

    let result = quote_exact_out(&pool, 2_000_000, true);

    assert!(result.is_err());
}
```

**Step 6: 运行测试**

Run: `cargo test --package sol-trade-sdk test_raydium_cpmm_quote_exact_out -- --nocapture`
Expected: 测试通过

**Step 7: 提交**

```bash
git add src/utils/calc/raydium_cpmm.rs tests/
git commit -m "feat(cpmm): 实现 quote_exact_out 函数支持 exact_out 计算"
```

---

## Task 5: 实现 AMM V4 quote_exact_out 函数

**Files:**
- Create: `src/utils/calc/raydium_amm_v4.rs` (新增函数)

**Step 1: 查看现有 PoolState 结构**

Run: `grep -A 20 "pub struct PoolState" src/instruction/utils/raydium_amm_v4_types.rs`

**Step 2: 添加 quote_exact_out 函数**

```rust
pub fn quote_exact_out(
    pool_state: &PoolState,
    amount_out: u64,
    is_coin_in: bool,
) -> Result<QuoteExactOutResult, String> {
    let (reserve_in, reserve_out) = if is_coin_in {
        (pool_state.coin_reserve, pool_state.pc_reserve)
    } else {
        (pool_state.pc_reserve, pool_state.coin_reserve)
    };

    // 与 CPMM 相同的逻辑
    if amount_out >= reserve_out {
        return Err(format!(
            "Insufficient liquidity: requested={}, available={}",
            amount_out, reserve_out
        ));
    }

    let numerator = (reserve_in as u128).checked_mul(amount_out as u128)
        .ok_or_else(|| "Calculation overflow".to_string())?;

    let denominator = (reserve_out as u128).checked_sub(amount_out as u128)
        .ok_or_else(|| "Invalid reserve".to_string())?;

    let amount_in = numerator.checked_div(denominator)
        .ok_or_else(|| "Division overflow".to_string())?
        as u64;

    // 计算手续费 (AMM V4 费率通常是 0.25% = 2500 / 1_000_000)
    let fee_amount = (amount_in as u128)
        .checked_mul(2500u128)
        .and_then(|p| p.checked_div(1_000_000u128))
        .ok_or_else(|| "Fee overflow".to_string())?
        as u64;

    let total_amount_in = amount_in.checked_add(fee_amount)
        .ok_or_else(|| "Total overflow".to_string())?;

    Ok(QuoteExactOutResult {
        amount_in: total_amount_in,
        fee_amount,
        price_impact_bps: None, // AMM V4 暂不计算价格影响
    })
}
```

**Step 3: 导出函数**

确保函数在 `src/utils/calc/raydium_amm_v4.rs` 中公开导出。

**Step 4: 编译和测试**

Run: `cargo check --package sol-trade-sdk`
Run: `cargo test --package sol-trade-sdk test_raydium_amm_v4_quote_exact_out`

**Step 5: 提交**

```bash
git add src/utils/calc/raydium_amm_v4.rs
git commit -m "feat(amm_v4): 实现 quote_exact_out 函数"
```

---

## Task 6: 修改 CPMM build_buy_instructions 支持 exact_out

**Files:**
- Modify: `src/instruction/raydium_cpmm.rs`

**Step 1: 查找 build_buy_instructions 方法**

Run: `grep -n "async fn build_buy_instructions" src/instruction/raydium_cpmm.rs`
Expected: 找到方法定义（约在第 31 行）

**Step 2: 修改参数验证逻辑**

在现有验证代码后添加 exact_out 处理：

```rust
async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
    // ========================================
    // 新增：exact_out 模式处理
    // ========================================
    let (input_amount, is_exact_out) = if let Some(fixed_output) = params.fixed_output_amount {
        // exact_out 模式：计算所需输入
        let pool_state = get_pool_state(
            params.rpc.as_ref().ok_or_else(|| anyhow!("RPC required"))?,
            &protocol_params.amm_config,
            &protocol_params.base_mint,
            &protocol_params.quote_mint,
        ).await?;

        let is_token0_in = protocol_params.base_mint == params.input_mint;
        let quote_result = crate::utils::calc::raydium_cpmm::quote_exact_out(
            &pool_state,
            fixed_output,
            is_token0_in,
        ).map_err(|e| anyhow!("quote_exact_out failed: {}", e))?;

        (quote_result.amount_in, true)
    } else {
        // exact_in 模式：现有逻辑
        let input_amount = params.input_amount.ok_or_else(|| anyhow!("Input amount required"))?;
        if input_amount == 0 {
            return Err(anyhow!("Amount cannot be zero"));
        }
        (input_amount, false)
    };

    // ========================================
    // 继续现有的指令构建逻辑
    // ========================================
    // 使用计算出的 input_amount 继续现有代码...
}
```

**Step 3: 编译检查**

Run: `cargo check --package sol-trade-sdk`
Expected: 编译成功

**Step 4: 提交**

```bash
git add src/instruction/raydium_cpmm.rs
git commit -m "feat(cpmm): build_buy_instructions 支持 exact_out 模式"
```

---

## Task 7: 修改 CPMM build_sell_instructions 支持 exact_out

**Files:**
- Modify: `src/instruction/raydium_cpmm.rs`

**Step 1: 查找 build_sell_instructions 方法**

Run: `grep -n "async fn build_sell_instructions" src/instruction/raydium_cpmm.rs`

**Step 2: 添加 exact_out 处理（与 buy 对称）**

```rust
async fn build_sell_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>> {
    // ... 现有的 protocol_params 获取代码 ...

    // ========================================
    // 新增：exact_out 模式处理
    // ========================================
    let (input_amount, is_exact_out) = if let Some(fixed_output) = params.fixed_output_amount {
        let pool_state = get_pool_state(
            params.rpc.as_ref().ok_or_else(|| anyhow!("RPC required"))?,
            &protocol_params.amm_config,
            &protocol_params.base_mint,
            &protocol_params.quote_mint,
        ).await?;

        // 卖出：input_mint 是目标代币
        let is_token0_in = params.input_mint == pool_state.base_mint;
        let quote_result = crate::utils::calc::raydium_cpmm::quote_exact_out(
            &pool_state,
            fixed_output,
            is_token0_in,
        ).map_err(|e| anyhow!("quote_exact_out failed: {}", e))?;

        (quote_result.amount_in, true)
    } else {
        let input_amount = params.input_amount.ok_or_else(|| anyhow!("Input amount required"))?;
        if input_amount == 0 {
            return Err(anyhow!("Amount cannot be zero"));
        }
        (input_amount, false)
    };

    // ... 继续现有逻辑 ...
}
```

**Step 3: 编译检查**

Run: `cargo check --package sol-trade-sdk`

**Step 4: 提交**

```bash
git add src/instruction/raydium_cpmm.rs
git commit -m "feat(cpmm): build_sell_instructions 支持 exact_out 模式"
```

---

## Task 8: 修改 AMM V4 build_buy_instructions 支持 exact_out

**Files:**
- Modify: `src/instruction/raydium_amm_v4.rs`

**Step 1: 与 CPMM 类似修改**

参考 Task 6 的实现模式，使用 `raydium_amm_v4::quote_exact_out`。

**Step 2: 编译和提交**

```bash
cargo check --package sol-trade-sdk
git add src/instruction/raydium_amm_v4.rs
git commit -m "feat(amm_v4): build_buy_instructions 支持 exact_out"
```

---

## Task 9: 修改 AMM V4 build_sell_instructions 支持 exact_out

**Files:**
- Modify: `src/instruction/raydium_amm_v4.rs`

**Step 1: 与 CPMM 类似修改**

参考 Task 7 的实现模式。

**Step 2: 编译和提交**

```bash
cargo check --package sol-trade-sdk
git add src/instruction/raydium_amm_v4.rs
git commit -m "feat(amm_v4): build_sell_instructions 支持 exact_out"
```

---

## Task 10: 检查 PumpSwap build_sell_instructions 是否支持 exact_out

**Files:**
- Check: `src/instruction/pumpswap.rs`

**Step 1: 检查现有实现**

Run: `grep -A 50 "async fn build_sell_instructions" src/instruction/pumpswap.rs | head -60`

**Step 2: 判断是否需要修改**

如果已有 `fixed_output_amount` 处理（类似 build_buy_instructions 第 49-52 行），则无需修改。

如果没有，添加与 buy 对称的 exact_out 处理逻辑。

**Step 3: 如果需要修改，提交**

```bash
git add src/instruction/pumpswap.rs
git commit -m "feat(pumpswap): build_sell_instructions 支持 exact_out（如果需要）"
```

---

## Task 11: 实现 sell_simulate 方法

**Files:**
- Modify: `src/lib.rs`

**Step 1: 在 TradingClient impl 中添加方法**

在 `buy_simulate` 方法后添加：

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
    // 1. 参数验证
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
                "input_token_amount must be > 0".into()
            ));
        }
    }

    // 2. 获取 output_mint
    let output_mint = Self::get_output_mint(&params.output_token_type);

    // 3. 构建 SwapParams
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
        fixed_output_amount: params.fixed_output_token_amount,
        gas_fee_strategy: params.gas_fee_strategy,
        simulate: true,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    // 4. 构建指令
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

    // 5. 获取用户 ATA
    let user_input_ata = spl_associated_token_account::get_associated_token_address(
        &self.payer.pubkey(),
        &params.mint,
    );
    let user_output_ata = spl_associated_token_account::get_associated_token_address(
        &self.payer.pubkey(),
        &output_mint,
    );

    // 6. 调用链上模拟
    let sim_result = crate::utils::simulation_based_calc::simulate_swap_transaction(
        &self.rpc,
        &self.payer,
        instructions,
        user_input_ata,
        user_output_ata,
        params.mint,
        output_mint,
    )
    .await
    .map_err(|e| UnifiedTradingError::SimulationFailed(e.to_string()))?;

    // 7. 转换返回值
    Ok(SimulationResult {
        amount_out: sim_result.actual_output_amount,
        amount_in: if is_exact_out {
            // exact_out 模式：从模拟结果提取实际输入（如果有）或使用计算的值
            // 简化处理：暂时使用 params.input_token_amount
            params.input_token_amount
        } else {
            params.input_token_amount
        },
        fee_amount: 0,
        compute_units: sim_result.units_consumed.unwrap_or(0),
        transaction_fee: sim_result.transaction_fee,
        success: sim_result.success,
        error: sim_result.error,
        logs: sim_result.logs,
        dex_type: params.dex_type,
    })
}
```

**Step 2: 编译检查**

Run: `cargo check --package sol-trade-sdk`
Expected: 编译成功

**Step 3: 提交**

```bash
git add src/lib.rs
git commit -m "feat(client): 实现 sell_simulate 方法支持 exact_in 和 exact_out"
```

---

## Task 12: 修改 buy_simulate 参数验证支持 exact_out

**Files:**
- Modify: `src/lib.rs`

**Step 1: 修改 buy_simulate 参数验证**

在现有的参数验证后添加 exact_out 检查：

```rust
pub async fn buy_simulate(&self, params: TradeBuyParams) -> UnifiedResult<SimulationResult> {
    // 1. 参数验证（支持 exact_in 和 exact_out）
    let is_exact_out = params.fixed_output_token_amount.is_some();

    if is_exact_out {
        // exact_out 模式验证
        if params.fixed_output_token_amount.unwrap() == 0 {
            return Err(UnifiedTradingError::InvalidParameters(
                "fixed_output_token_amount must be > 0".into()
            ));
        }
    } else {
        // exact_in 模式验证（现有逻辑）
        if params.input_token_amount == 0 {
            return Err(UnifiedTradingError::InvalidParameters("amount must be > 0".into()));
        }
    }

    // 2. 检查 USD1 只支持 Bonk
    if params.input_token_type == TradeTokenType::USD1 && params.dex_type != DexType::Bonk {
        return Err(UnifiedTradingError::InvalidParameters(
            "USD1 only supported on Bonk".into(),
        ));
    }

    // ... 继续现有逻辑 ...
}
```

**Step 2: 修改返回值构建**

在返回值处添加 `amount_in`：

```rust
Ok(SimulationResult {
    amount_out: sim_result.actual_output_amount,
    amount_in: if is_exact_out {
        // TODO: 从 SwapParams 中提取计算的 input_amount
        // 简化处理：暂时使用 0，后续需要从指令构建器传递
        0
    } else {
        params.input_token_amount
    },
    fee_amount: 0,
    compute_units: sim_result.units_consumed.unwrap_or(0),
    transaction_fee: sim_result.transaction_fee,
    success: sim_result.success,
    error: sim_result.error,
    logs: sim_result.logs,
    dex_type: params.dex_type,
})
```

**Step 3: 编译检查**

Run: `cargo check --package sol-trade-sdk`

**Step 4: 提交**

```bash
git add src/lib.rs
git commit -m "feat(client): buy_simulate 添加 exact_out 参数验证"
```

---

## Task 13-18: 测试和文档（后续实施）

### Task 13: 编写 CPMM exact_out 单元测试

### Task 14: 编写 AMM V4 exact_out 单元测试

### Task 15: 编写 buy_simulate exact_out 集成测试

### Task 16: 编写 sell_simulate 集成测试

### Task 17: 编写准确性验证测试

### Task 18: 更新文档

**详细步骤待定，根据前面任务的实施情况调整。**

---

## 实施顺序建议

1. **先完成 Task 1-3**（数据结构和辅助方法）
2. **再完成 Task 4-5**（基础计算函数）
3. **然后完成 Task 6-10**（指令构建器修改）
4. **最后完成 Task 11-12**（API 实现）
5. **Task 13-18**（测试和文档）根据前面情况调整

## 重要提醒

1. **每步都要编译验证** - 不要跳过 `cargo check`
2. **遇到错误立即停止** - 不要猜测，询问用户
3. **频繁提交** - 每个任务完成后立即 commit
4. **CLMM 暂时跳过** - CLMM 的 exact_out 实现较复杂，留到后续版本
5. **使用真实测试数据** - 参考现有的测试用例

---

**准备开始实施了吗？我们从 Task 1 开始。**
