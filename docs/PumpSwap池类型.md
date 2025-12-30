# PumpSwap Pool 类型指南

本指南解释 PumpSwap 协议中的两种流动性池类型以及 SDK 如何处理它们。

## 📋 概述

PumpSwap 支持两种类型的流动性池：

1. **标准池（Canonical Pool）** - 由 PumpFun 迁移创建的标准池
2. **非标准池（Non-canonical Pool）** - 直接在 PumpSwap 上创建的自定义池

## 🏗️ 标准池（Canonical Pool）

### 描述
标准池是当代币从 PumpFun 毕业到 PumpSwap 时，由 PumpFun 的 `migrate` 指令创建的。这些是最常见和流动性最好的池子。

### 特征
- **池索引**：`[0, 0]`（CANONICAL_POOL_INDEX）
- **池权限**：PumpFun 程序下的 `PDA("pool-authority", mint)`
- **池地址**：PumpSwap AMM 程序下的 `PDA("pool", [0, 0], pool_authority, mint, wsol_mint)`
- **流动性**：通常具有最高的流动性
- **优先级**：SDK 池选择的首选

### PDA 计算
```rust
let (pool_authority, _) =
    Pubkey::try_find_program_address(&[b"pool-authority", mint.as_ref()], &PUMPFUN)?;

let pool_index = [0u8, 0u8];
let wsol_mint = WSOL_TOKEN_ACCOUNT;

let (pool, _) = Pubkey::try_find_program_address(
    &[b"pool", &pool_index, pool_authority.as_ref(), mint.as_ref(), wsol_mint.as_ref()],
    &accounts::AMM_PROGRAM,
)?;
```

## 🎨 非标准池（Non-canonical Pool）

### 描述
非标准池是直接在 PumpSwap 上创建的，不经过 PumpFun 迁移。这些池具有自定义的 `pool_index` 值。

### 特征
- **池索引**：除 `[0, 0]` 以外的任何值
- **池权限**：自定义或从不同的种子派生
- **池地址**：使用不同的种子组合派生
- **用例**：自定义交易对、替代流动性来源
- **优先级**：标准池之后的次选

## 🔍 SDK 池选择逻辑

SDK 在查找池时遵循基于优先级的方法：

### 1. 标准池查找（最高优先级）
```rust
if let Some((pool_address, _)) = calculate_canonical_pool_pda(mint) {
    if let Ok(pool) = fetch_pool(rpc, &pool_address).await {
        // 验证它是一个 mint/WSOL 交易对
        if (pool.base_mint == *mint && pool.quote_mint == WSOL_TOKEN_ACCOUNT) ||
           (pool.base_mint == WSOL_TOKEN_ACCOUNT && pool.quote_mint == *mint) {
            return Ok((pool_address, pool));
        }
    }
}
```

### 2. WSOL 交易对选择（中等优先级）
如果找不到标准池或标准池无效：
- 列出该代币的所有池子
- 过滤出 WSOL 交易对
- 按 LP 供应量排序（从高到低）
- 返回流动性最好的 WSOL 池

### 3. 通用池选择（低优先级）
如果没有找到 WSOL 交易对：
- 返回 LP 供应量最高的池

### 4. 回退方案（最后手段）
- 尝试单独的 `find_by_base_mint` 和 `find_by_quote_mint` 函数
- 用于向后兼容

## 💻 使用示例

```rust
// SDK 自动处理池选择
let pump_swap_params = PumpSwapParams::from_mint_by_rpc(&client.rpc, &mint).await?;

// 手动池选择
let (pool_address, pool) = crate::instruction::utils::pumpswap::find_by_mint(&client.rpc, &mint).await?;

// 显式计算标准池地址
let (canonical_pool, _) = crate::instruction::utils::pumpswap::calculate_canonical_pool_pda(&mint).unwrap();
```

## 📊 对比表

| 特性 | 标准池 | 非标准池 |
|------|--------|----------|
| **池索引** | `[0, 0]` | 任何其他值 |
| **来源** | PumpFun 迁移 | 直接在 PumpSwap 创建 |
| **池权限** | PumpFun PDA | 自定义/不同的 PDA |
| **流动性** | 通常最高 | 各不相同 |
| **SDK 优先级** | 第一 | 第二 |
| **常见性** | 最常见 | 较少见 |

## ⚠️ 重要注意事项

1. **自动选择**：SDK 自动选择最佳池，通常无需手动选择
2. **流动性优先**：SDK 优先选择 LP 供应量更高的池以获得更好的执行效果
3. **WSOL 偏好**：当可用时，WSOL 交易对优于其他报价代币
4. **池验证**：SDK 在选择前验证池的所有权和有效性
5. **向后兼容**：回退方法确保与较旧的池类型兼容

## 🔗 相关文档

- [交易参数参考](TRADING_PARAMETERS_CN.md)
- [PumpSwap 直接交易示例](../examples/pumpswap_direct_trading/)
- [PumpSwap 交易示例](../examples/pumpswap_trading/)

## 📚 Pool 结构

```rust
pub struct Pool {
    pub pool_bump: u8,
    pub index: u16,              // 池索引（0 表示标准池）
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub lp_supply: u64,
    pub coin_creator: Pubkey,
    pub is_mayhem_mode: bool,
}
```