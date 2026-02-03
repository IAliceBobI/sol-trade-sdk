# Raydium CPMM Quote 计算与交易构建 Bug 修复记录

## 问题概述

在测试 Raydium CPMM swap 功能时，发现本地计算与链上模拟结果存在巨大差异（误差 > 99.999%），导致验证测试失败。

**测试场景**: 卖出 1000 PIPE 换取 WSOL
- **本地计算**: 98,312 WSOL lamports
- **链上模拟**: 14,251,402,502,935,669 WSOL lamports (错误值)
- **误差**: 99.99999999931016%

## 调试过程

### 1. 初步排查

首先验证了以下方面：
- ✅ 池子储备金数据正确
- ✅ 费率配置正确
- ✅ 恒定乘积公式实现正确
- ✅ 测试参数正确（amount_in = 1,000,000,000 最小单位）

### 2. 关键线索

通过打印交易日志，发现了关键的 Program data：

```
Program data: QMbN6CYIceKgQApi1NNwif0gC7BVq6/tdVcJa0xGJ7i0fHkg1+sOjxY/XagflwMAlViNMRcAAAAAypo7AAAAABWAAQ...
```

解码后发现：
- **Offset 56**: 输入金额 = 1,000,000,000 ✓
- **Offset 64**: **98,325** ✓ (正确的输出金额！)
- **Offset 112**: 12,327,463,165,039,354,507 (错误数据，除以 865 后得到 14,251,402,502,935,670)

这揭示了问题：**解析器使用了错误的 offset**！

### 3. 根本原因

通过对比官方实现和我们的代码，发现了 3 个关键 bug：

#### Bug 1: 错误的 Program data 解析 offset
**文件**: `src/utils/simulation_based_calc.rs`

```rust
// ❌ 错误实现
let raw_out = u64::from_le_bytes(data[112..120].try_into().ok()?);
let amount_out = raw_out / 865;

// ✅ 正确实现
let amount_out = u64::from_le_bytes(data[64..72].try_into().ok()?);
```

**分析**:
- offset 112 存储的是其他元数据，不是输出金额
- offset 64 才是真正的输出金额（lamports）
- offset 64 的值不需要除法转换，直接使用

#### Bug 2: 费用计算逻辑错误
**文件**: `src/utils/calc/raydium_cpmm.rs`

```rust
// ❌ 错误实现
let trade_fee = compute_trading_fee(input_amount, trade_fee_rate);
let protocol_fee = compute_protocol_fund_fee(input_amount, protocol_fee_rate);
let fund_fee = compute_protocol_fund_fee(input_amount, fund_fee_rate);

// ✅ 正确实现（参考 Raydium 官方实现）
let trade_fee = compute_trading_fee(input_amount, trade_fee_rate);
let protocol_fee = compute_protocol_fund_fee(trade_fee, protocol_fee_rate);  // 从 trade_fee 计算
let fund_fee = compute_protocol_fund_fee(trade_fee, fund_fee_rate);       // 从 trade_fee 计算
```

**分析**:
- `protocol_fee` 和 `fund_fee` 应该从 `trade_fee` 中按比例提取
- 官方实现使用 `Fees::protocol_fee(trade_fee, rate)` 和 `Fees::fund_fee(trade_fee, rate)`
- 我们的错误实现直接从 `input_amount` 计算，导致费用计算过大

#### Bug 3: 交易构建中的硬编码账户
**文件**: `src/instruction/raydium_cpmm.rs`

```rust
// ❌ 错误实现
let output_vault_account = get_vault_account(
    &pool_state,
    if is_wsol {
        &crate::constants::WSOL_TOKEN_ACCOUNT  // 硬编码！
    } else {
        &crate::constants::USDC_TOKEN_ACCOUNT
    },
    protocol_params,
);

// 账户列表中也硬编码了 mint
AccountMeta::new_readonly(params.input_mint, false),
if is_wsol {
    crate::constants::WSOL_TOKEN_ACCOUNT_META  // 硬编码！
} else {
    crate::constants::USDC_TOKEN_ACCOUNT_META
}, // Output token mint

// ✅ 正确实现
let output_vault_account = get_vault_account(&pool_state, &params.output_mint, protocol_params);
let input_vault_account = get_vault_account(&pool_state, &params.input_mint, protocol_params);

// 账户列表使用动态 mint
AccountMeta::new_readonly(params.input_mint, false),    // Input token mint
AccountMeta::new_readonly(params.output_mint, false),   // Output token mint
```

**分析**:
- 原代码假设 pool 总是包含 WSOL 或 USDC，并硬编码了相应的 mint
- 对于 PIPE-WSOL 这样的 pool，硬编码逻辑会导致账户不匹配
- 应该根据实际的交易方向（input_mint 和 output_mint）动态构建账户

## 修复方案

### 修复 1: CPMM Program data 解析

**文件**: `src/utils/simulation_based_calc.rs`

```rust
fn parse_raydium_cpmm_data(program_data_base64: &str) -> Option<(u64, u64)> {
    use base64::Engine;

    // 解码 base64
    let data = base64::engine::general_purpose::STANDARD.decode(program_data_base64).ok()?;

    // 检查数据长度（至少需要 72 字节到 offset 64）
    if data.len() < 72 {
        return None;
    }

    // 解析输入金额（offset 56）
    let raw_in = u64::from_le_bytes(data[56..64].try_into().ok()?);
    let amount_in = raw_in / 1000;

    // 解析输出金额（offset 64）- 直接就是实际输出金额，不需要除法
    let amount_out = u64::from_le_bytes(data[64..72].try_into().ok()?);

    Some((amount_in, amount_out))
}
```

### 修复 2: 费用计算逻辑

**文件**: `src/utils/calc/raydium_cpmm.rs`

```rust
fn swap_base_input(
    input_amount: u64,
    input_vault_amount: u64,
    output_vault_amount: u64,
    trade_fee_rate: u64,
    creator_fee_rate: u64,
    protocol_fee_rate: u64,
    fund_fee_rate: u64,
    is_creator_fee_on_input: bool,
) -> SwapResult {
    let mut creator_fee = 0u64;

    // ✅ 修复：protocol_fee 和 fund_fee 从 trade_fee 计算
    let trade_fee = compute_trading_fee(input_amount, trade_fee_rate);
    let protocol_fee = compute_protocol_fund_fee(trade_fee, protocol_fee_rate);
    let fund_fee = compute_protocol_fund_fee(trade_fee, fund_fee_rate);

    // ... 其余逻辑保持不变
}
```

### 修复 3: 交易构建账户动态获取

**文件**: `src/instruction/raydium_cpmm.rs`

#### build_buy_instructions 修复
```rust
// ✅ 移除硬编码，使用动态 mint
let accounts: [AccountMeta; 13] = [
    // ... 其他账户
    AccountMeta::new_readonly(
        if is_base_in {
            protocol_params.quote_token_program
        } else {
            protocol_params.base_token_program
        },
        false,
    ), // Output Token Program (readonly)
    AccountMeta::new_readonly(params.input_mint, false),    // Input token mint
    AccountMeta::new_readonly(params.output_mint, false),   // Output token mint
    // ...
];
```

#### build_sell_instructions 修复
```rust
// ✅ 移除硬编码的 vault 获取逻辑
let output_vault_account = get_vault_account(&pool_state, &params.output_mint, protocol_params);
let input_vault_account = get_vault_account(&pool_state, &params.input_mint, protocol_params);

// ✅ 账户列表使用动态 mint
AccountMeta::new_readonly(params.input_mint, false),    // Input token mint
AccountMeta::new_readonly(params.output_mint, false),   // Output token mint
```

## 验证结果

### 测试场景
- **Pool**: PIPE-WSOL (BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp)
- **输入**: 1000 PIPE (1,000,000,000 最小单位)
- **输出**: WSOL

### 对比结果

| 指标 | 数值 |
|------|------|
| 本地计算 | 98,312 WSOL lamports |
| 链上模拟 | 98,325 WSOL lamports |
| **差值** | **13 lamports** |
| **误差率** | **0.0132%** ✅ |

### 测试通过
```bash
✅ 验证通过：误差 < 1.0%

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured
```

## 经验教训

### 1. 数据结构验证很重要
- 当解析链上返回的数据时，必须仔细验证每个 offset 的含义
- 不要假设某个 offset 存储的是什么数据，要通过实际解码验证

### 2. 参考官方实现
- 在实现复杂的链上逻辑时，应该参考官方代码
- 特别是费用计算、数据解析等关键逻辑

### 3. 避免硬编码
- 账户、mint 等应该动态获取，而不是硬编码
- 硬编码会导致代码无法适配不同场景

### 4. 调试技巧
- 打印交易日志可以帮助快速定位问题
- 解码 base64 数据可以验证数据结构
- 对比本地计算和链上模拟的差异是有效的调试方法

## 相关文件

- `src/utils/calc/raydium_cpmm.rs` - CPMM 费用计算
- `src/utils/simulation_based_calc.rs` - 链上模拟结果解析
- `src/instruction/raydium_cpmm.rs` - CPMM 交易指令构建
- `tests/verify_raydium_cpmm_exact_in_sell.rs` - 验证测试

## 参考资源

- Raydium CPMM 官方实现: `temp/dex/raydium-cp-swap/programs/cp-swap/src/`
- 恒定乘积公式: `x * y = k`
- CPMM swap 指令 discriminator: `[242, 35, 198, 137, 82, 225, 242, 182]`

## 附录：CPMM Program data 结构

根据实际解码，Raydium CPMM swap 的 Program data 结构（前 72 字节）：

| Offset | 大小 | 含义 | 示例值 |
|--------|------|------|--------|
| 40-47 | 8 bytes | 标志位 | 8 |
| 48-55 | 8 bytes | 元数据/价格信息 | varies |
| 56-63 | 8 bytes | 输入金额 (×1000) | 1,000,000,000 |
| **64-71** | **8 bytes** | **输出金额 (lamports)** | **98,325** |

**注意**:
- 输入金额需要除以 1000 转换
- 输出金额直接就是 lamports，不需要除法
- offset 112 存储的是其他数据，不是输出金额
