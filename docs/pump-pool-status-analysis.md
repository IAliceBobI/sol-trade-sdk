# Pump 协议池子状态管理分析

**分析日期**：2026-01-07  
**分析范围**：PumpFun、PumpSwap  
**数据来源**：`/opt/projects/sol-trade-sdk/temp/pump.fun`、`/opt/projects/sol-trade-sdk/temp/pumpfun-bonkfun-bot/idl/pump_swap_idl.json`

---

## 概述

Pump 协议（PumpFun 和 PumpSwap）的池子状态管理与 Raydium 等传统 DEX 有显著不同。本文档详细分析了 Pump 协议的池子状态机制。

---

## 1. PumpFun 池子状态

### 1.1 池子状态字段

PumpFun 的 `BondingCurveAccount` 结构包含一个 `complete` 字段（boolean）：

```typescript
export class BondingCurveAccount {
  public discriminator: bigint;
  public virtualTokenReserves: bigint;
  public virtualSolReserves: bigint;
  public realTokenReserves: bigint;
  public realSolReserves: bigint;
  public tokenTotalSupply: bigint;
  public complete: boolean;  // 池子状态字段
}
```

### 1.2 状态说明

| 状态 | 值 | 说明 |
|------|-----|------|
| **活跃** | `false` | 池子处于活跃状态，可以交易 |
| **完成** | `true` | 池子已完成，已迁移到 Raydium，不能再交易 |

### 1.3 状态检查

PumpFun SDK 在交易前会自动检查池子状态：

```typescript
// 买入价格计算
getBuyPrice(amount: bigint): bigint {
    if (this.complete) {
        throw new Error("Curve is complete");
    }
    // ...
}

// 卖出价格计算
getSellPrice(amount: bigint, feeBasisPoints: bigint): bigint {
    if (this.complete) {
        throw new Error("Curve is complete");
    }
    // ...
}
```

### 1.4 池子选择

PumpFun 使用 PDA 派生，**每个 mint 只有一个池子**，不需要从多个池子中选择：

```typescript
// Bonding Curve PDA
const bondingCurve = PublicKey.findProgramAddressSync(
    ["bonding-curve".as_bytes(), mint.toBuffer()],
    PUMPFUN_PROGRAM_ID
);
```

### 1.5 建议

✅ **当前代码已经正确**：
- PumpFun 使用 PDA 派生，不需要选择池子
- `complete` 字段会在交易时自动检查（抛出错误）
- 不需要额外的状态过滤逻辑

---

## 2. PumpSwap 池子状态

### 2.1 Pool 结构

PumpSwap 的 `Pool` 结构**没有 status 或 deactivate 字段**：

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct Pool {
    pub pool_bump: u8,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub lp_supply: u64,
    pub coin_creator: Pubkey,
    pub is_mayhem_mode: bool,  // 特殊模式，不是池子状态
}
```

### 2.2 注意事项

- `is_mayhem_mode` 字段表示特殊模式，不是池子的活跃/非活跃状态
- **没有单个池子的禁用状态**
- 池子状态通过 `GlobalConfig` 的 `disable_flags` 全局控制

### 2.3 全局禁用设置

PumpSwap 通过 `GlobalConfig` 的 `disable_flags` 字段（u8 位掩码）全局控制操作：

```json
{
  "name": "disable_flags",
  "docs": [
    "Flags to disable certain functionality",
    "bit 0 - Disable create pool",
    "bit 1 - Disable deposit",
    "bit 2 - Disable withdraw",
    "bit 3 - Disable buy",
    "bit 4 - Disable sell"
  ],
  "type": "u8"
}
```

### 2.4 禁用标志定义

```rust
pub const DISABLE_CREATE_POOL: u8 = 1 << 0;  // 0b00000001
pub const DISABLE_DEPOSIT: u8 = 1 << 1;     // 0b00000010
pub const DISABLE_WITHDRAW: u8 = 1 << 2;    // 0b00000100
pub const DISABLE_BUY: u8 = 1 << 3;         // 0b00001000
pub const DISABLE_SELL: u8 = 1 << 4;        // 0b00010000
```

### 2.5 禁用指令

PumpSwap 提供 `disable` 指令用于设置这些标志：

```json
{
  "name": "disable",
  "args": [
    {
      "name": "disable_create_pool",
      "type": "bool"
    },
    {
      "name": "disable_deposit",
      "type": "bool"
    },
    {
      "name": "disable_withdraw",
      "type": "bool"
    },
    {
      "name": "disable_buy",
      "type": "bool"
    },
    {
      "name": "disable_sell",
      "type": "bool"
    }
  ]
}
```

### 2.6 谁可以操作？

只有 **admin** 可以操作 `disable_flags`：

```json
{
  "name": "GlobalConfig",
  "type": {
    "kind": "struct",
    "fields": [
      {
        "name": "admin",
        "docs": ["The admin pubkey"],
        "type": "pubkey"
      },
      {
        "name": "disable_flags",
        "type": "u8"
      },
      ...
    ]
  }
}
```

### 2.7 影响范围

`disable_flags` 是**全局配置，影响所有 pool**：

- 所有 pool 都受控制
- 不是单个 pool 的状态
- 修改的是 `GlobalConfig` 账户，不是单个 pool 账户

### 2.8 池子选择

PumpSwap 使用 PDA 派生，**每个 mint 对只有一个 canonical 池子**，不需要从多个池子中选择：

```rust
// Canonical Pool PDA
let (pool, _) = Pubkey::find_program_address_sync(
    &["pool", &[0, 0], pool_authority, mint, wsol_mint],
    PUMPSWAP_AMM_PROGRAM
);
```

### 2.9 建议

⚠️ **需要添加全局禁用检查**：

1. 添加 `GlobalConfig` 结构定义
2. 在交易前检查 `disable_flags`
3. 过滤掉被禁用的操作

```rust
// 建议添加的代码

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct GlobalConfig {
    pub admin: Pubkey,
    pub lp_fee_basis_points: u64,
    pub protocol_fee_basis_points: u64,
    pub disable_flags: u8,
    pub protocol_fee_recipients: [Pubkey; 8],
    pub coin_creator_fee_basis_points: u64,
    pub admin_set_coin_creator_authority: Pubkey,
    pub whitelist_pda: Pubkey,
    pub reserved_fee_recipient: Pubkey,
    pub mayhem_mode_enabled: bool,
    pub reserved_fee_recipients: [Pubkey; 7],
}

// 禁用标志检查函数
pub fn is_create_pool_disabled(disable_flags: u8) -> bool {
    disable_flags & DISABLE_CREATE_POOL != 0
}

pub fn is_deposit_disabled(disable_flags: u8) -> bool {
    disable_flags & DISABLE_DEPOSIT != 0
}

pub fn is_withdraw_disabled(disable_flags: u8) -> bool {
    disable_flags & DISABLE_WITHDRAW != 0
}

pub fn is_buy_disabled(disable_flags: u8) -> bool {
    disable_flags & DISABLE_BUY != 0
}

pub fn is_sell_disabled(disable_flags: u8) -> bool {
    disable_flags & DISABLE_SELL != 0
}

pub fn is_tradeable(disable_flags: u8) -> bool {
    !is_buy_disabled(disable_flags) && !is_sell_disabled(disable_flags)
}
```

---

## 3. 对比 Raydium AMM V4

| 协议 | 池子状态 | 池子选择 | 需要过滤 |
|------|---------|---------|---------|
| **PumpFun** | `complete` (boolean) | PDA 派生（唯一） | ✅ 自动检查（交易时抛错） |
| **PumpSwap** | 无（全局 `disable_flags`） | PDA 派生（唯一） | ⚠️ 需要添加全局禁用检查 |
| **Raydium AMM V4** | `status` (u64) | 多个池子（需选择） | ✅ 已实现状态过滤 |

### Raydium AMM V4 池子状态

```rust
pub mod pool_status {
    pub const UNINITIALIZED: u64 = 0;
    pub const INITIALIZED: u64 = 1;
    pub const DISABLED: u64 = 2;
    pub const WITHDRAW_ONLY: u64 = 3;
    pub const ORDER_BOOK_ONLY: u64 = 4;
    pub const SWAP_ONLY: u64 = 5;
    pub const ACTIVE: u64 = 6;
}
```

---

## 4. 协议账户地址

### PumpFun

```typescript
const PROGRAM_ID = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
const GLOBAL_ACCOUNT_SEED = "global";
const MINT_AUTHORITY_SEED = "mint-authority";
const BONDING_CURVE_SEED = "bonding-curve";
const METADATA_SEED = "metadata";
```

### PumpSwap

```rust
pub const AMM_PROGRAM: Pubkey = pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
pub const GLOBAL_ACCOUNT: Pubkey = pubkey!("ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw");
pub const FEE_RECIPIENT: Pubkey = pubkey!("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV");
```

---

## 5. 最佳实践

### 对于 PumpFun

✅ **无需额外处理**：
- SDK 已自动检查 `complete` 字段
- 交易时抛出错误，避免失败
- 使用 PDA 派生，无需选择池子

### 对于 PumpSwap

⚠️ **建议添加全局禁用检查**：

1. **获取 GlobalConfig**：
```rust
pub async fn fetch_global_config(
    rpc: &SolanaRpcClient
) -> Result<GlobalConfig> {
    let account = rpc.get_account(&GLOBAL_ACCOUNT).await?;
    let data = account.data.as_slice();
    Ok(GlobalConfig::try_from_slice(data)?)
}
```

2. **交易前检查**：
```rust
let global_config = fetch_global_config(&rpc).await?;

if is_buy_disabled(global_config.disable_flags) {
    return Err(anyhow!("Buy is disabled by admin"));
}

if is_sell_disabled(global_config.disable_flags) {
    return Err(anyhow!("Sell is disabled by admin"));
}
```

3. **提供友好的错误信息**：
```rust
match global_config.disable_flags {
    flags if is_buy_disabled(flags) => {
        eprintln!("⚠️ PumpSwap buy is disabled by admin");
        eprintln!("Contact admin for more information");
        return Err(anyhow!("Buy disabled"));
    }
    _ => {}
}
```

---

## 6. 总结

### PumpFun

- ✅ 池子状态：`complete` 字段（boolean）
- ✅ 状态检查：SDK 自动检查，交易时抛错
- ✅ 池子选择：PDA 派生，无需选择
- ✅ 无需额外处理

### PumpSwap

- ⚠️ 池子状态：无单个池子状态
- ⚠️ 全局控制：`GlobalConfig.disable_flags`（u8 位掩码）
- ⚠️ 操作权限：只有 admin 可以修改
- ⚠️ 影响范围：所有 pool
- ⚠️ 池子选择：PDA 派生，无需选择
- ⚠️ **需要添加全局禁用检查**

### Raydium AMM V4

- ✅ 池子状态：`status` 字段（u64）
- ✅ 状态检查：已实现状态过滤
- ✅ 池子选择：多个池子，需要选择
- ✅ 已实现完整功能

---

## 7. 相关文档

- PumpFun SDK：`/opt/projects/sol-trade-sdk/temp/pump.fun/src/`
- PumpSwap IDL：`/opt/projects/sol-trade-sdk/temp/pumpfun-bonkfun-bot/idl/pump_swap_idl.json`
- Raydium Pool 状态管理：`/opt/projects/sol-trade-sdk/docs/raydium-pool-close-analysis.md`
- Pool 查询方法：`/opt/projects/sol-trade-sdk/docs/Pool查询方法.md`

---

**创建日期**：2026-01-07  
**最后更新**：2026-01-07  
**作者**：iFlow CLI