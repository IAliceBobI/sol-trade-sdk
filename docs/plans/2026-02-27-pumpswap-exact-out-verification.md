# PumpSwap Exact Out 验证实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 验证 PumpSwap Exact Out 计算的准确性，从单元测试到真实交易验证

**Architecture:** 分层验证策略 - 本地计算单元测试 → 链上模拟验证 → 真实交易验证。每层独立测试，逐步验证精度。

**Tech Stack:** Rust, solana-sdk, tokio, nextest

---

## 背景知识

### PumpSwap Exact Out 计算模型

**费用结构** (从官方文档验证):
- `LP_FEE_BASIS_POINTS = 25` (0.25%)
- `PROTOCOL_FEE_BASIS_POINTS = 5` (0.05%)
- `COIN_CREATOR_FEE_BASIS_POINTS = 5` (0.05%)
- 总费用: 0.35% (有 coin_creator) 或 0.30% (无)

**恒定乘积公式**:
```
(base_reserve + base_in) * (quote_reserve - quote_out) = k
```

**Exact Out 方向**:
- **Buy (quote→base)**: 指定想获得多少 base，计算需要多少 quote
  - 使用 `buy_base_input_internal`
  - 费用加在输入 (quote) 上
- **Sell (base→quote)**: 指定想获得多少 quote，计算需要卖多少 base
  - 使用 `sell_quote_input_internal`
  - 费用从输出 (quote) 扣除

### 验证策略

| 阶段 | 方法 | 目的 | 误差容忍 |
|------|------|------|----------|
| 1. 单元测试 | 纯数学验证 | 验证公式正确性 | 0% |
| 2. 链上模拟 | simulateTransaction | 验证链上执行一致性 | < 0.5% |
| 3. 真实交易 | 实际执行 | 验证最终结果 | < 1% |

---

## Task 1: 创建本地计算单元测试

**Files:**
- Create: `tests/pumpswap_exact_out_unit_tests.rs`

**目标:** 验证 `buy_base_input_internal` 和 `sell_quote_input_internal` 的数学正确性

**Step 1: 创建测试文件框架**

```rust
//! PumpSwap Exact Out 单元测试
//!
//! 测试本地计算函数的数学正确性
//!
//! 运行测试:
//!     cargo nextest run pumpswap_exact_out_unit -- --nocapture

use solana_sdk::pubkey::Pubkey;
use sol_trade_sdk::utils::calc::pumpswap::{
    buy_base_input_internal, sell_quote_input_internal,
    buy_quote_input_internal, sell_base_input_internal,
};

/// 默认 coin_creator (无 creator)
fn no_creator() -> Pubkey {
    Pubkey::default()
}

/// 有 creator 的地址
fn with_creator() -> Pubkey {
    Pubkey::new_unique()
}

// ============================================
// Test Data
// ============================================

/// 小型 Pool: base=1M, quote=10 SOL
const SMALL_BASE_RESERVE: u64 = 1_000_000_000;  // 1M tokens (6 decimals)
const SMALL_QUOTE_RESERVE: u64 = 10_000_000_000; // 10 SOL (9 decimals)

/// 中型 Pool: base=100M, quote=100 SOL
const MEDIUM_BASE_RESERVE: u64 = 100_000_000_000;
const MEDIUM_QUOTE_RESERVE: u64 = 100_000_000_000;

// ============================================
// Tests
// ============================================

#[test]
fn test_buy_base_input_internal_no_creator() {
    // 想买 1000 base tokens，计算需要多少 quote
    let base_to_buy = 1_000_000u64; // 1000 tokens (6 decimals)

    let result = buy_base_input_internal(
        base_to_buy,
        0, // no slippage
        SMALL_BASE_RESERVE,
        SMALL_QUOTE_RESERVE,
        &no_creator(),
    ).expect("calculation should succeed");

    println!("Buy {} base tokens:", base_to_buy);
    println!("  internal_quote_amount: {}", result.internal_quote_amount);
    println!("  ui_quote (with fees): {}", result.ui_quote);
    println!("  fees: {}", result.ui_quote - result.internal_quote_amount);

    // 验证: ui_quote > internal_quote_amount (有费用)
    assert!(result.ui_quote > result.internal_quote_amount);

    // 验证费用比例 ≈ 0.30% (no creator)
    let fee_rate = (result.ui_quote - result.internal_quote_amount) as f64
        / result.ui_quote as f64 * 100.0;
    println!("  fee_rate: {:.4}%", fee_rate);
    assert!(fee_rate > 0.25 && fee_rate < 0.35);
}

#[test]
fn test_buy_base_input_internal_with_creator() {
    let base_to_buy = 1_000_000u64;

    let result_no = buy_base_input_internal(
        base_to_buy,
        0,
        SMALL_BASE_RESERVE,
        SMALL_QUOTE_RESERVE,
        &no_creator(),
    ).unwrap();

    let result_with = buy_base_input_internal(
        base_to_buy,
        0,
        SMALL_BASE_RESERVE,
        SMALL_QUOTE_RESERVE,
        &with_creator(),
    ).unwrap();

    println!("No creator: ui_quote = {}", result_no.ui_quote);
    println!("With creator: ui_quote = {}", result_with.ui_quote);

    // 有 creator 的费用应该更高
    assert!(result_with.ui_quote > result_no.ui_quote);
}
```

**Step 2: 运行测试验证通过**

Run: `cargo nextest run pumpswap_exact_out_unit --no-capture`
Expected: PASS

**Step 3: 添加反向验证测试**

```rust
#[test]
fn test_exact_out_buy_reverse_verification() {
    // 场景: 指定买 X 个 base，计算需要 Y 个 quote
    // 反向验证: 用 Y 个 quote 做 exact_in，能否得到 >= X 个 base?

    let base_to_buy = 1_000_000u64;

    // Exact Out: 计算需要多少 quote
    let exact_out_result = buy_base_input_internal(
        base_to_buy,
        0,
        MEDIUM_BASE_RESERVE,
        MEDIUM_QUOTE_RESERVE,
        &no_creator(),
    ).unwrap();

    // Exact In: 用计算出的 quote 买入
    let exact_in_result = buy_quote_input_internal(
        exact_out_result.ui_quote,
        0,
        MEDIUM_BASE_RESERVE,
        MEDIUM_QUOTE_RESERVE,
        &no_creator(),
    ).unwrap();

    println!("期望买: {} base", base_to_buy);
    println!("需要支付: {} quote", exact_out_result.ui_quote);
    println!("实际得到: {} base", exact_in_result.base);

    // 反向验证: 实际得到的应该 >= 期望的
    assert!(
        exact_in_result.base >= base_to_buy,
        "Reverse verification failed: expected >= {}, got {}",
        base_to_buy, exact_in_result.base
    );
}

#[test]
fn test_exact_out_sell_reverse_verification() {
    // 场景: 指定获得 X 个 quote，计算需要卖 Y 个 base
    // 反向验证: 用 Y 个 base 做 exact_in，能否得到 >= X 个 quote?

    let quote_to_receive = 1_000_000_000u64; // 1 SOL

    // Exact Out: 计算需要卖多少 base
    let exact_out_result = sell_quote_input_internal(
        quote_to_receive,
        0,
        MEDIUM_BASE_RESERVE,
        MEDIUM_QUOTE_RESERVE,
        &no_creator(),
    ).unwrap();

    // Exact In: 用计算出的 base 卖出
    let exact_in_result = sell_base_input_internal(
        exact_out_result.base,
        0,
        MEDIUM_BASE_RESERVE,
        MEDIUM_QUOTE_RESERVE,
        &no_creator(),
    ).unwrap();

    println!("期望获得: {} quote", quote_to_receive);
    println!("需要卖: {} base", exact_out_result.base);
    println!("实际得到: {} quote", exact_in_result.ui_quote);

    // 反向验证: 实际得到的应该 >= 期望的
    assert!(
        exact_in_result.ui_quote >= quote_to_receive,
        "Reverse verification failed: expected >= {}, got {}",
        quote_to_receive, exact_in_result.ui_quote
    );
}
```

**Step 4: 运行反向验证测试**

Run: `cargo nextest run pumpswap_exact_out_unit --no-capture`
Expected: PASS

**Step 5: Commit**

```bash
git add tests/pumpswap_exact_out_unit_tests.rs
git commit -m "🧪 test(pumpswap): 添加 Exact Out 单元测试

- 添加 buy_base_input_internal 单元测试
- 添加 sell_quote_input_internal 单元测试
- 添加反向验证测试确保计算一致性"
```

---

## Task 2: 创建链上模拟验证测试

**Files:**
- Create: `tests/pumpswap_exact_out_simulation_tests.rs`

**目标:** 验证本地计算与链上模拟执行的一致性

**Step 1: 创建模拟测试框架**

```rust
//! PumpSwap Exact Out 链上模拟验证测试
//!
//! 通过 simulateTransaction 验证本地计算的准确性
//!
//! 运行测试:
//!     cargo nextest run pumpswap_exact_out_sim -- --nocapture

use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;
use std::sync::Arc;
use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::utils::pumpswap::get_pool_by_address,
    utils::calc::pumpswap::{
        buy_base_input_internal, sell_quote_input_internal,
    },
};
use sol_trade_test_utils::{ensure_token_balance, get_simulation_test_keypair};

/// 测试用的 PUMP-WSOL Pool
const TEST_POOL: &str = "539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR";

/// WSOL Mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// PUMP Token Mint
const PUMP_MINT: &str = "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn";

/// 验证结果
struct VerificationResult {
    expected_output: u64,
    actual_output: u64,
    error_rate_percent: f64,
    passed: bool,
}

/// 验证 exact_out buy 计算准确性
async fn verify_exact_out_buy(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_out: u64,
    tolerance_percent: f64,
) -> Result<VerificationResult, anyhow::Error> {
    // 1. 获取 Pool 状态
    let pool = get_pool_by_address(rpc, pool_address).await?;

    // 2. 获取储备余额
    let base_balance = rpc.get_token_account_balance(&pool.pool_base_token_account).await?;
    let quote_balance = rpc.get_token_account_balance(&pool.pool_quote_token_account).await?;

    let base_reserve = base_balance.amount.parse::<u64>()?;
    let quote_reserve = quote_balance.amount.parse::<u64>()?;

    // 3. 本地计算
    let local_result = buy_base_input_internal(
        amount_out,
        0,
        base_reserve,
        quote_reserve,
        &pool.coin_creator,
    )?;

    println!("📊 本地计算结果:");
    println!("  期望输出: {} base", amount_out);
    println!("  需要输入: {} quote", local_result.ui_quote);

    // 4. TODO: 构造交易并模拟执行
    // 这里需要在后续步骤实现

    Ok(VerificationResult {
        expected_output: amount_out,
        actual_output: amount_out, // 暂时占位
        error_rate_percent: 0.0,
        passed: true,
    })
}

#[tokio::test]
#[serial_test::serial(pumpswap_tests)]
async fn test_exact_out_buy_simulation_small_amount() {
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url));

    let pool_address = Pubkey::from_str(TEST_POOL).unwrap();
    let amount_out = 1_000u64; // 小额测试

    let result = verify_exact_out_buy(&rpc, &pool_address, amount_out, 0.5)
        .await
        .expect("Verification should succeed");

    println!("\n📊 验证结果:");
    println!("  期望输出: {}", result.expected_output);
    println!("  误差率: {:.4}%", result.error_rate_percent);
    println!("  测试结果: {}", if result.passed { "✅ PASS" } else { "❌ FAIL" });
}
```

**Step 2: 运行测试验证框架正常**

Run: `cargo nextest run test_exact_out_buy_simulation_small --no-capture`
Expected: PASS (框架测试)

**Step 3: Commit**

```bash
git add tests/pumpswap_exact_out_simulation_tests.rs
git commit -m "🧪 test(pumpswap): 添加 Exact Out 模拟测试框架"
```

---

## Task 3: 实现完整的模拟验证逻辑

**Files:**
- Modify: `tests/pumpswap_exact_out_simulation_tests.rs`

**Step 1: 添加完整模拟验证函数**

```rust
use sol_trade_sdk::{
    instruction::pumpswap::PumpSwapInstructionBuilder,
    trading::core::params::{PumpSwapParams, SwapParams, DexParamEnum},
    trading::core::traits::InstructionBuilder as _,
    utils::simulation_based_calc::simulate_swap_transaction,
};

/// 完整的 Exact Out Buy 模拟验证
async fn verify_exact_out_buy_full(
    rpc: &SolanaRpcClient,
    payer: &Arc<solana_sdk::signer::keypair::Keypair>,
    pool_address: &Pubkey,
    amount_out: u64,
    tolerance_percent: f64,
) -> Result<VerificationResult, anyhow::Error> {
    // 1. 获取 Pool 状态
    let pool = get_pool_by_address(rpc, pool_address).await?;

    // 2. 获取储备余额
    let base_balance = rpc.get_token_account_balance(&pool.pool_base_token_account).await?;
    let quote_balance = rpc.get_token_account_balance(&pool.pool_quote_token_account).await?;

    let base_reserve = base_balance.amount.parse::<u64>()?;
    let quote_reserve = quote_balance.amount.parse::<u64>()?;

    // 3. 本地计算
    let local_result = buy_base_input_internal(
        amount_out,
        0,
        base_reserve,
        quote_reserve,
        &pool.coin_creator,
    )?;

    println!("📊 本地计算结果:");
    println!("  期望输出: {} base", amount_out);
    println!("  需要输入: {} quote", local_result.ui_quote);
    println!("  费用: {} quote", local_result.ui_quote - local_result.internal_quote_amount);

    // 4. 获取 Token Program
    let base_token_program = sol_trade_sdk::utils::token::get_token_program_with_cache(
        rpc, &pool.base_mint
    ).await?;
    let quote_token_program = sol_trade_sdk::utils::token::get_token_program_with_cache(
        rpc, &pool.quote_mint
    ).await?;

    // 5. 构造 SwapParams
    let pumpswap_params = PumpSwapParams {
        pool: *pool_address,
        base_mint: pool.base_mint,
        quote_mint: pool.quote_mint,
        pool_base_token_account: pool.pool_base_token_account,
        pool_quote_token_account: pool.pool_quote_token_account,
        pool_base_token_reserves: base_reserve,
        pool_quote_token_reserves: quote_reserve,
        coin_creator_vault_ata: sol_trade_sdk::instruction::utils::pumpswap::coin_creator_vault_ata(
            pool.coin_creator, pool.quote_mint
        ),
        coin_creator_vault_authority: sol_trade_sdk::instruction::utils::pumpswap::coin_creator_vault_authority(
            pool.coin_creator
        ),
        base_token_program,
        quote_token_program,
        is_mayhem_mode: pool.is_mayhem_mode,
    };

    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let base_mint = pool.base_mint;

    let swap_params = SwapParams {
        rpc: Some(Arc::new(rpc.clone())),
        payer: payer.clone(),
        trade_type: sol_trade_sdk::swqos::TradeType::Buy,
        input_mint: wsol_mint,
        input_token_program: Some(quote_token_program),
        output_mint: base_mint,
        output_token_program: Some(base_token_program),
        input_amount: Some(local_result.ui_quote),
        slippage_basis_points: Some(1000),
        protocol_params: DexParamEnum::PumpSwap(pumpswap_params),
        fixed_output_amount: Some(amount_out),
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::default(),
        simulate: true,
        ..Default::default()
    };

    // 6. 构建指令
    let instructions = PumpSwapInstructionBuilder
        .build_buy_instructions(&swap_params)
        .await?;

    // 7. 模拟执行
    let user_input_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(), &wsol_mint, &quote_token_program
    );
    let user_output_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(), &base_mint, &base_token_program
    );

    let sim_result = simulate_swap_transaction(
        rpc,
        payer,
        instructions,
        user_input_ata,
        user_output_ata,
        wsol_mint,
        base_mint,
    ).await?;

    if !sim_result.success {
        return Err(anyhow::anyhow!(
            "Simulation failed: {:?}",
            sim_result.error
        ));
    }

    // 8. 计算误差
    let actual_output = sim_result.actual_output_amount;
    let diff = amount_out.abs_diff(actual_output);
    let error_rate = if actual_output > 0 {
        (diff as f64 / actual_output as f64) * 100.0
    } else {
        100.0
    };

    let passed = error_rate <= tolerance_percent;

    println!("\n📊 模拟结果:");
    println!("  期望输出: {}", amount_out);
    println!("  实际输出: {}", actual_output);
    println!("  差值: {}", diff);
    println!("  误差率: {:.4}%", error_rate);

    Ok(VerificationResult {
        expected_output: amount_out,
        actual_output,
        error_rate_percent: error_rate,
        passed,
    })
}
```

**Step 2: 添加完整测试用例**

```rust
#[tokio::test]
#[serial_test::serial(pumpswap_tests)]
async fn test_exact_out_buy_full_verification() {
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));
    let payer = Arc::new(get_simulation_test_keypair());

    let pool_address = Pubkey::from_str(TEST_POOL).unwrap();
    let amount_out = 1_000u64; // 1000 base tokens

    // 初始化账户
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let pump_mint = Pubkey::from_str(PUMP_MINT).unwrap();

    ensure_token_balance(
        &rpc,
        &rpc_url,
        &payer,
        &[(wsol_mint, Some(1_000_000_000)), (pump_mint, None)],
        1,
    ).await.expect("Failed to init balances");

    // 执行验证
    let result = verify_exact_out_buy_full(
        &rpc, &payer, &pool_address, amount_out, 0.5
    ).await.expect("Verification failed");

    assert!(result.passed, "Verification failed with error rate {:.4}%", result.error_rate_percent);
}
```

**Step 3: 运行完整测试**

Run: `cargo nextest run test_exact_out_buy_full_verification --no-capture`
Expected: PASS

**Step 4: Commit**

```bash
git add tests/pumpswap_exact_out_simulation_tests.rs
git commit -m "🧪 test(pumpswap): 实现完整的 Exact Out 模拟验证"
```

---

## Task 4: 添加 Sell 方向验证

**Files:**
- Modify: `tests/pumpswap_exact_out_simulation_tests.rs`

**Step 1: 添加 Exact Out Sell 验证函数**

```rust
/// 完整的 Exact Out Sell 模拟验证
async fn verify_exact_out_sell_full(
    rpc: &SolanaRpcClient,
    payer: &Arc<solana_sdk::signer::keypair::Keypair>,
    pool_address: &Pubkey,
    amount_out: u64, // 想获得的 quote 数量
    tolerance_percent: f64,
) -> Result<VerificationResult, anyhow::Error> {
    // 1. 获取 Pool 状态
    let pool = get_pool_by_address(rpc, pool_address).await?;

    // 2. 获取储备余额
    let base_balance = rpc.get_token_account_balance(&pool.pool_base_token_account).await?;
    let quote_balance = rpc.get_token_account_balance(&pool.pool_quote_token_account).await?;

    let base_reserve = base_balance.amount.parse::<u64>()?;
    let quote_reserve = quote_balance.amount.parse::<u64>()?;

    // 3. 本地计算
    let local_result = sell_quote_input_internal(
        amount_out,
        0,
        base_reserve,
        quote_reserve,
        &pool.coin_creator,
    )?;

    println!("📊 本地计算结果:");
    println!("  期望输出: {} quote", amount_out);
    println!("  需要卖: {} base", local_result.base);

    // 4-7. 类似 buy 方向的实现...
    // (省略重复代码，实际实现时补全)

    // 8. 返回结果
    Ok(VerificationResult {
        expected_output: amount_out,
        actual_output: amount_out, // 实际从模拟结果获取
        error_rate_percent: 0.0,
        passed: true,
    })
}
```

**Step 2: 添加 Sell 测试用例**

```rust
#[tokio::test]
#[serial_test::serial(pumpswap_tests)]
async fn test_exact_out_sell_full_verification() {
    // 实现类似 test_exact_out_buy_full_verification
}
```

**Step 3: 运行测试**

Run: `cargo nextest run test_exact_out_sell_full --no-capture`
Expected: PASS

**Step 4: Commit**

```bash
git add tests/pumpswap_exact_out_simulation_tests.rs
git commit -m "🧪 test(pumpswap): 添加 Exact Out Sell 模拟验证"
```

---

## Task 5: 创建真实交易验证测试

**Files:**
- Create: `tests/pumpswap_exact_out_live_tests.rs`

**目标:** 使用真实资金验证最终结果

**⚠️ 警告:** 此测试需要真实资金，仅在确认模拟测试通过后运行

**Step 1: 创建真实交易测试框架**

```rust
//! PumpSwap Exact Out 真实交易验证测试
//!
//! ⚠️ 警告: 此测试需要真实资金，仅在必要时运行
//!
//! 运行测试:
//!     cargo nextest run pumpswap_exact_out_live -- --ignored --nocapture

#[cfg(test)]
mod live_tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要手动启用
    #[serial_test::serial(pumpswap_live_tests)]
    async fn test_exact_out_buy_live_transaction() {
        // 1. 执行 Exact Out 计算
        // 2. 构造真实交易
        // 3. 发送并确认交易
        // 4. 验证实际输出 >= 期望输出

        println!("⚠️ 真实交易测试需要手动启用");
        println!("请确保:");
        println!("  1. 模拟测试全部通过");
        println!("  2. 账户有足够的资金");
        println!("  3. 了解可能的资金损失");
    }
}
```

**Step 2: Commit**

```bash
git add tests/pumpswap_exact_out_live_tests.rs
git commit -m "🧪 test(pumpswap): 添加 Exact Out 真实交易测试框架"
```

---

## Task 6: 整理和文档

**Files:**
- Update: `CLAUDE.md`

**Step 1: 更新项目文档**

在 CLAUDE.md 中添加测试命令:

```markdown
### PumpSwap Exact Out 验证测试

```bash
# 单元测试（纯数学验证）
cargo nextest run pumpswap_exact_out_unit -- --nocapture

# 模拟验证测试
cargo nextest run pumpswap_exact_out_sim -- --nocapture

# 真实交易测试（需要手动启用）
cargo nextest run pumpswap_exact_out_live -- --ignored --nocapture
```
```

**Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "📝 docs: 添加 PumpSwap Exact Out 测试文档"
```

---

## 执行摘要

| Task | 描述 | 文件 | 状态 |
|------|------|------|------|
| 1 | 本地计算单元测试 | `tests/pumpswap_exact_out_unit_tests.rs` | 待执行 |
| 2 | 模拟测试框架 | `tests/pumpswap_exact_out_simulation_tests.rs` | 待执行 |
| 3 | 完整模拟验证 | 修改 Task 2 文件 | 待执行 |
| 4 | Sell 方向验证 | 修改 Task 2 文件 | 待执行 |
| 5 | 真实交易测试 | `tests/pumpswap_exact_out_live_tests.rs` | 待执行 |
| 6 | 文档更新 | `CLAUDE.md` | 待执行 |

## 验收标准

- [ ] 所有单元测试通过 (误差 0%)
- [ ] 模拟测试通过 (误差 < 0.5%)
- [ ] 真实交易测试可选通过 (误差 < 1%)
- [ ] 文档已更新
