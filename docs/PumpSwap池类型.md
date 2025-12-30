# PumpSwap Pool 类型指南

本指南从架构师视角深入解析 PumpSwap 协议中的流动性池类型、账户结构设计以及 SDK 的池选择策略。

## 📋 概述

PumpSwap 是部署在 Solana 上的恒定乘积 AMM（Constant Product AMM），程序地址为 `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`。

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
- **代币创建者**：支持 `coin_creator` 字段，用于代币创建者费用分配

### PDA 计算流程

```rust
// 步骤 1: 计算 pool_authority（在 PumpFun 程序下）
let (pool_authority, _) =
    Pubkey::try_find_program_address(&[b"pool-authority", mint.as_ref()], &PUMPFUN)?;

// 步骤 2: 计算标准池地址（在 PumpSwap AMM 程序下）
let pool_index = [0u8, 0u8];
let wsol_mint = WSOL_TOKEN_ACCOUNT;

let (pool, _) = Pubkey::try_find_program_address(
    &[b"pool", &pool_index, pool_authority.as_ref(), mint.as_ref(), wsol_mint.as_ref()],
    &accounts::AMM_PROGRAM,
)?;
```

### 标准池创建流程

当 PumpFun 代币完成 bonding curve 并触发迁移时：

1. PumpFun 的 `migrate` 指令被调用
2. 系统在 PumpSwap AMM 程序下创建标准池
3. 初始流动性从 bonding curve 转移到 AMM 池
4. LP 代币被销毁（防止流动性提取）
5. 设置 `coin_creator` 为原始代币创建者

## 🎨 非标准池（Non-canonical Pool）

### 描述
非标准池是直接在 PumpSwap 上创建的，不经过 PumpFun 迁移。这些池具有自定义的 `pool_index` 值。

### 特征
- **池索引**：除 `[0, 0]` 以外的任何值（允许同一创建者为同一交易对创建多个池）
- **池权限**：自定义或从不同的种子派生
- **池地址**：使用不同的种子组合派生
- **用例**：自定义交易对、替代流动性来源、多池策略
- **优先级**：标准池之后的次选
- **代币创建者**：可能没有 `coin_creator` 或设置为默认值

### PDA 计算流程

```rust
// 非标准池使用自定义的 index 和 creator
let pool_index = [index_hi, index_lo];  // 自定义索引
let creator = user_pubkey;              // 池创建者

let (pool, _) = Pubkey::try_find_program_address(
    &[b"pool", &pool_index, creator.as_ref(), base_mint.as_ref(), quote_mint.as_ref()],
    &accounts::AMM_PROGRAM,
)?;
```

## 📊 Pool 账户结构详解

### Pool 账户字段

```rust
pub struct Pool {
    pub pool_bump: u8,                    // PDA bump 种子
    pub index: u16,                       // 池索引（0 = 标准池，其他 = 非标准池）
    pub creator: Pubkey,                  // 池创建者（用于 PDA 派生）
    pub base_mint: Pubkey,                // 基础代币 mint 地址
    pub quote_mint: Pubkey,               // 报价代币 mint 地址
    pub lp_mint: Pubkey,                  // LP 代币 mint 地址
    pub pool_base_token_account: Pubkey,  // 池基础代币 ATA
    pub pool_quote_token_account: Pubkey, // 池报价代币 ATA
    pub lp_supply: u64,                   // LP 供应量（真实流通量，不含销毁和锁定）
    pub coin_creator: Pubkey,             // 代币创建者（仅标准池有效）
    pub is_mayhem_mode: bool,             // 是否处于 Mayhem 模式
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `pool_bump` | u8 | PDA 派生时使用的 bump 种子，用于验证地址 |
| `index` | u16 | 池索引，0 表示标准池，其他值表示非标准池 |
| `creator` | Pubkey | 池创建者公钥，用于 PDA 派生 |
| `base_mint` | Pubkey | 基础代币的 mint 地址 |
| `quote_mint` | Pubkey | 报价代币的 mint 地址 |
| `lp_mint` | Pubkey | LP 代币的 mint 地址，可通过 `["pool_lp_mint", pool_key]` PDA 派生 |
| `pool_base_token_account` | Pubkey | 池的基础代币 ATA，可通过 PDA 派生 |
| `pool_quote_token_account` | Pubkey | 池的报价代币 ATA，可通过 PDA 派生 |
| `lp_supply` | u64 | LP 代币总供应量，不含销毁和锁定的代币 |
| `coin_creator` | Pubkey | 原始代币创建者，用于代币创建者费用分配 |
| `is_mayhem_mode` | bool | 是否启用 Mayhem 模式（特殊费用机制） |

### LP 供应量说明

`lp_supply` 字段表示**真实流通供应量**，这是一个重要的设计决策：

- 如果用户向池子存入流动性，然后直接销毁他们的 `lp_mint` 代币，`Pool::lp_supply` 仍会反映 `lp_mint` 的原始供应量
- 这样设计是为了区分用户直接销毁的 `lp_mint` 代币和通过 `withdraw` 指令销毁的代币
- 确保 `withdraw` 指令的正确性，防止流动性操纵

## 🔧 费用机制

### 费用结构

PumpSwap 使用三级费用结构：

```rust
pub struct Fees {
    pub lp_fee_bps: u64,          // LP 提供者费用基点
    pub protocol_fee_bps: u64,    // 协议费用基点
    pub creator_fee_bps: u64,     // 代币创建者费用基点
}
```

### 默认费用配置

| 费用类型 | 基点值 | 百分比 | 接收者 |
|----------|--------|--------|--------|
| LP 费用 | 25 bps | 0.25% | LP 代币持有者 |
| 协议费用 | 5 bps | 0.05% | 协议费用接收者（8 个地址） |
| 代币创建者费用 | 5 bps | 0.05% | 代币创建者（仅标准池） |
| **总计** | **35 bps** | **0.35%** | - |

### 费用分配流程

当用户执行 `buy` 或 `sell` 交易时：

1. **LP 费用**（25 bps）：添加到池子流动性中，由 LP 代币持有者共享
2. **协议费用**（5 bps）：发送到 `protocol_fee_recipient` 账户
3. **代币创建者费用**（5 bps）：发送到 `coin_creator_vault_ata` 账户

### 协议费用接收者

协议费用接收者是一个包含 8 个地址的数组：

```rust
pub struct GlobalConfig {
    // ...
    pub protocol_fee_recipients: [Pubkey; 8],
    // ...
}
```

**重要提示**：每次交易时，应从这 8 个地址中**随机选择**一个作为 `protocol_fee_recipient`，以提高程序交易吞吐量。

### FeeConfig 账户

PumpSwap 支持基于市值的分级费用结构：

```rust
pub struct FeeConfig {
    pub bump: u8,
    pub admin: Pubkey,
    pub flat_fees: Fees,           // 固定费用（默认）
    pub fee_tiers: Vec<FeeTier>,   // 分级费用（按市值）
}

pub struct FeeTier {
    pub market_cap_lamports_threshold: u128,  // 市值阈值
    pub fees: Fees,                           // 对应的费用
}
```

FeeConfig 是一个独立的 PDA 账户，位于 Fee Program（`pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ`）下。

## 🌪️ Mayhem 模式

### 概述

Mayhem 模式是 PumpSwap 的特殊费用机制，为高频交易和套利提供不同的费用激励。

### 启用条件

- 由管理员通过 `GlobalConfig::mayhem_mode_enabled` 全局启用
- 仅适用于 Pump 池（标准池）
- 需要在池创建时或后续通过管理员指令设置

### Mayhem 模式费用

在 Mayhem 模式下，费用接收者会变为 `MAYHEM_FEE_RECIPIENT`：

```rust
pub const MAYHEM_FEE_RECIPIENT: Pubkey =
    pubkey!("GesfTA3X2arioaHp8bbKdjG9vJtskViWACZoYvxp4twS");
```

### 相关错误码

| 错误码 | 名称 | 说明 |
|--------|------|------|
| 6041 | MayhemModeDisabled | Mayhem 模式已禁用 |
| 6042 | OnlyPumpPoolsMayhemMode | 只有 Pump 池可以使用 Mayhem 模式 |
| 6043 | MayhemModeInDesiredState | Mayhem 模式已处于目标状态 |

## 🎁 代币激励系统

### GlobalVolumeAccumulator

全局交易量累加器，用于跟踪所有用户的交易量并分配代币激励：

```rust
pub struct GlobalVolumeAccumulator {
    pub start_time: i64,                    // 激励开始时间
    pub end_time: i64,                      // 激励结束时间
    pub seconds_in_a_day: i64,              // 一天的秒数
    pub mint: Pubkey,                       // 激励代币 mint
    pub total_token_supply: [u64; 30],      // 每天的代币供应量（30 天）
    pub sol_volumes: [u64; 30],             // 每天的 SOL 交易量（30 天）
}
```

### UserVolumeAccumulator

用户交易量累加器，用于跟踪单个用户的交易量：

```rust
pub struct UserVolumeAccumulator {
    pub user: Pubkey,                       // 用户公钥
    pub volume: u64,                        // 用户累计交易量
    pub last_update_timestamp: i64,         // 最后更新时间
}
```

### PDA 计算

```rust
// 全局交易量累加器
let (global_volume_accumulator, _) = Pubkey::try_find_program_address(
    &[b"global_volume_accumulator"],
    &accounts::AMM_PROGRAM,
)?;

// 用户交易量累加器
let (user_volume_accumulator, _) = Pubkey::try_find_program_address(
    &[b"user_volume_accumulator", user.as_ref()],
    &accounts::AMM_PROGRAM,
)?;
```

### 激励领取

用户可以通过 `claim_token_incentives` 指令领取激励代币：

```rust
// 指令参数
spendable_quote_in: u64,      // 可花费的报价金额
min_base_amount_out: u64,     // 最小基础代币输出
track_volume: OptionBool,     // 是否跟踪交易量
```

**重要提示**：使用 `track_volume` 参数时，用户需要确保有足够的 SOL 来创建以下账户（如果尚未创建）：
- `protocol_fee_recipient_token_account`: `rent.minimum_balance(TokenAccount::LEN)`
- `coin_creator_vault_ata`: `rent.minimum_balance(TokenAccount::LEN)`
- `user_volume_accumulator`: `rent.minimum_balance(UserVolumeAccumulator::LEN)`

## 🗂️ GlobalConfig 账户

### 账户地址

```
ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw
```

PDA 种子：`["global_config"]`

### 账户结构

```rust
pub struct GlobalConfig {
    pub admin: Pubkey,                      // 管理员公钥
    pub lp_fee_basis_points: u64,           // LP 费用基点（默认 25）
    pub protocol_fee_basis_points: u64,     // 协议费用基点（默认 5）
    pub disable_flags: u8,                  // 禁用标志位
    pub protocol_fee_recipients: [Pubkey; 8], // 协议费用接收者
    pub coin_creator_fee_basis_points: u64, // 代币创建者费用基点（默认 5）
    pub admin_set_coin_creator_authority: Pubkey, // 设置代币创建者的管理员权限
    pub whitelist_pda: Pubkey,              // 白名单 PDA
    pub reserved_fee_recipient: Pubkey,     // 预留费用接收者
    pub mayhem_mode_enabled: bool,          // 是否启用 Mayhem 模式
    pub reserved_fee_recipients: [Pubkey; 7], // 预留费用接收者
}
```

### 禁用标志位

`disable_flags` 是一个位掩码，用于禁用特定功能：

| 位 | 功能 | 说明 |
|----|------|------|
| 0 | Disable create pool | 禁用创建池 |
| 1 | Disable deposit | 禁用存入流动性 |
| 2 | Disable withdraw | 禁用提取流动性 |
| 3 | Disable buy | 禁用买入 |
| 4 | Disable sell | 禁用卖出 |

## 🔍 SDK 池选择逻辑

SDK 在查找池时遵循基于优先级的方法：

### 1. 标准池查找（最高优先级）

```rust
// 优先查找标准池（PumpFun 迁移的 mint/WSOL 交易对）
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

```rust
// 列出该代币的所有池子
if let Ok(pools) = list_by_mint(rpc, mint).await {
    // 过滤出 WSOL 交易对
    let mut wsol_pools: Vec<_> = pools
        .iter()
        .filter(|(_, pool)| {
            pool.base_mint == WSOL_TOKEN_ACCOUNT || pool.quote_mint == WSOL_TOKEN_ACCOUNT
        })
        .collect();
    
    if !wsol_pools.is_empty() {
        // 按 LP 供应量排序（从高到低）
        wsol_pools.sort_by(|a, b| b.1.lp_supply.cmp(&a.1.lp_supply));
        // 返回流动性最好的 WSOL 池
        return Ok((*wsol_pools[0].0, wsol_pools[0].1.clone()));
    }
}
```

### 3. 通用池选择（低优先级）

如果没有找到 WSOL 交易对：

```rust
// 返回 LP 供应量最高的池
let mut all_pools: Vec<_> = pools.iter().collect();
all_pools.sort_by(|a, b| b.1.lp_supply.cmp(&a.1.lp_supply));
let (address, pool) = all_pools[0];
return Ok((*address, pool.clone()));
```

### 4. 回退方案（最后手段）

```rust
// 尝试单独的 find_by_base_mint 和 find_by_quote_mint 函数
// 用于向后兼容
if let Ok((address, pool)) = find_by_base_mint(rpc, mint).await {
    return Ok((address, pool));
}

if let Ok((address, pool)) = find_by_quote_mint(rpc, mint).await {
    return Ok((address, pool));
}
```

## 💻 使用示例

### SDK 自动池选择

```rust
use sol_trade_sdk::instruction::utils::pumpswap::PumpSwapParams;

// SDK 自动处理池选择（推荐）
let pump_swap_params = PumpSwapParams::from_mint_by_rpc(&client.rpc, &mint).await?;
```

### 手动池选择

```rust
use sol_trade_sdk::instruction::utils::pumpswap;

// 手动查找池
let (pool_address, pool) = pumpswap::find_by_mint(&client.rpc, &mint).await?;

// 列出所有池
let pools = pumpswap::list_by_mint(&client.rpc, &mint).await?;
```

### 显式计算标准池地址

```rust
use sol_trade_sdk::instruction::utils::pumpswap;

// 计算标准池地址
let (canonical_pool, pool_authority) =
    pumpswap::calculate_canonical_pool_pda(&mint).unwrap();

// 验证池是否存在
let pool = pumpswap::fetch_pool(&client.rpc, &canonical_pool).await?;
```

### 获取池余额

```rust
use sol_trade_sdk::instruction::utils::pumpswap;

// 获取池的基础代币和报价代币余额
let (base_balance, quote_balance) =
    pumpswap::get_token_balances(&pool, &client.rpc).await?;
```

### 交易报价

```rust
use sol_trade_sdk::instruction::utils::pumpswap;

// 报价精确输入的交易
// is_base_in = true: base -> quote
// is_base_in = false: quote -> base
let quote_result = pumpswap::quote_exact_in(
    &client.rpc,
    &pool_address,
    amount_in,
    is_base_in,
).await?;

println!("预计输出: {}", quote_result.amount_out);
println!("费用: {}", quote_result.fee_amount);
```

## 📊 对比表

### 标准池 vs 非标准池

| 特性 | 标准池 | 非标准池 |
|------|--------|----------|
| **池索引** | `[0, 0]` | 任何其他值 |
| **来源** | PumpFun 迁移 | 直接在 PumpSwap 创建 |
| **池权限** | PumpFun PDA | 自定义/不同的 PDA |
| **流动性** | 通常最高 | 各不相同 |
| **SDK 优先级** | 第一 | 第二 |
| **常见性** | 最常见 | 较少见 |
| **代币创建者** | 支持 | 可能不支持 |
| **LP 销毁** | 是 | 否 |
| **Mayhem 模式** | 支持 | 不支持 |

### 池选择策略对比

| 策略 | 优先级 | 适用场景 | 预期效果 |
|------|--------|----------|----------|
| 标准池查找 | 1 | PumpFun 迁移代币 | 最佳流动性，最低滑点 |
| WSOL 交易对选择 | 2 | 需要使用 WSOL | 良好流动性，兼容性好 |
| 通用池选择 | 3 | 无 WSOL 交易对 | 可用流动性，可能较高滑点 |
| 回退方案 | 4 | 向后兼容 | 基本功能，可能非最优 |

## 📚 相关账户 PDA

### Coin Creator Vault Authority

代币创建者金库权限，用于存储代币创建者费用：

```rust
pub(crate) fn coin_creator_vault_authority(coin_creator: Pubkey) -> Pubkey {
    let (pump_pool_authority, _) = Pubkey::find_program_address(
        &[b"creator_vault", &coin_creator.to_bytes()],
        &accounts::AMM_PROGRAM,
    );
    pump_pool_authority
}
```

### Coin Creator Vault ATA

代币创建者金库 ATA，用于接收代币创建者费用：

```rust
pub(crate) fn coin_creator_vault_ata(coin_creator: Pubkey, quote_mint: Pubkey) -> Pubkey {
    let creator_vault_authority = coin_creator_vault_authority(coin_creator);
    get_associated_token_address_with_program_id(
        &creator_vault_authority,
        &quote_mint,
        &TOKEN_PROGRAM,
    )
}
```

### Fee Recipient ATA

费用接收者 ATA，用于接收协议费用：

```rust
pub(crate) fn fee_recipient_ata(fee_recipient: Pubkey, quote_mint: Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(
        &fee_recipient,
        &quote_mint,
        &TOKEN_PROGRAM,
    )
}
```

## ⚠️ 重要注意事项

1. **自动选择**：SDK 自动选择最佳池，通常无需手动选择
2. **流动性优先**：SDK 优先选择 LP 供应量更高的池以获得更好的执行效果
3. **WSOL 偏好**：当可用时，WSOL 交易对优于其他报价代币
4. **池验证**：SDK 在选择前验证池的所有权和有效性
5. **向后兼容**：回退方法确保与较旧的池类型兼容
6. **费用随机化**：每次交易应从 8 个协议费用接收者中随机选择一个
7. **账户创建成本**：使用 `track_volume` 时需要额外 SOL 创建账户
8. **Mayhem 模式**：仅适用于 Pump 池，需要管理员启用
9. **LP 供应量含义**：`lp_supply` 是真实流通量，不含销毁和锁定
10. **标准池唯一性**：每个 mint 只能有一个标准池（index = 0）

## 🔗 相关文档

- [交易参数参考](交易参数参考.md)
- [Gas费策略](Gas费策略.md)
- [PumpSwap 直接交易示例](../examples/pumpswap_direct_trading/)
- [PumpSwap 交易示例](../examples/pumpswap_trading/)
- [PumpSwap 官方文档](https://github.com/pump-fun/pump-fun-dex-public-docs)

## 🔗 相关常量

| 常量 | 值 | 说明 |
|------|-----|------|
| `AMM_PROGRAM` | `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA` | PumpSwap AMM 程序地址 |
| `FEE_PROGRAM` | `pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ` | Fee Program 地址 |
| `GLOBAL_ACCOUNT` | `ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw` | Global Config 账户地址 |
| `GLOBAL_VOLUME_ACCUMULATOR` | `C2aFPdENg4A2HQsmrd5rTw5TaYBX5Ku887cWjbFKtZpw` | 全局交易量累加器 |
| `FEE_CONFIG` | `5PHirr8joyTMp9JMm6nW7hNDVyEYdkzDqazxPD7RaTjx` | Fee Config 账户 |
| `LP_FEE_BASIS_POINTS` | 25 | LP 费用基点 |
| `PROTOCOL_FEE_BASIS_POINTS` | 5 | 协议费用基点 |
| `COIN_CREATOR_FEE_BASIS_POINTS` | 5 | 代币创建者费用基点 |
| `MAYHEM_FEE_RECIPIENT` | `GesfTA3X2arioaHp8bbKdjG9vJtskViWACZoYvxp4twS` | Mayhem 模式费用接收者 |
| `DEFAULT_COIN_CREATOR_VAULT_AUTHORITY` | `8N3GDaZ2iwN65oxVatKTLPNooAVUJTbfiVJ1ahyqwjSk` | 默认代币创建者金库权限 |

## 📚 Pool 结构（完整版）

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct Pool {
    pub pool_bump: u8,                    // PDA bump 种子
    pub index: u16,                       // 池索引（0 = 标准池）
    pub creator: Pubkey,                  // 池创建者
    pub base_mint: Pubkey,                // 基础代币 mint
    pub quote_mint: Pubkey,               // 报价代币 mint
    pub lp_mint: Pubkey,                  // LP 代币 mint
    pub pool_base_token_account: Pubkey,  // 池基础代币 ATA
    pub pool_quote_token_account: Pubkey, // 池报价代币 ATA
    pub lp_supply: u64,                   // LP 供应量（真实流通量）
    pub coin_creator: Pubkey,             // 代币创建者
    pub is_mayhem_mode: bool,             // Mayhem 模式标志
}

pub const POOL_SIZE: usize = 1 + 2 + 32 * 6 + 8 + 32 + 1;  // = 236
```

## 📚 GlobalConfig 结构（完整版）

```rust
pub struct GlobalConfig {
    pub admin: Pubkey,                      // 管理员公钥
    pub lp_fee_basis_points: u64,           // LP 费用基点
    pub protocol_fee_basis_points: u64,     // 协议费用基点
    pub disable_flags: u8,                  // 禁用标志位
    pub protocol_fee_recipients: [Pubkey; 8], // 协议费用接收者
    pub coin_creator_fee_basis_points: u64, // 代币创建者费用基点
    pub admin_set_coin_creator_authority: Pubkey, // 设置代币创建者的管理员权限
    pub whitelist_pda: Pubkey,              // 白名单 PDA
    pub reserved_fee_recipient: Pubkey,     // 预留费用接收者
    pub mayhem_mode_enabled: bool,          // Mayhem 模式启用标志
    pub reserved_fee_recipients: [Pubkey; 7], // 预留费用接收者
}
```

## 📚 FeeConfig 结构（完整版）

```rust
pub struct FeeConfig {
    pub bump: u8,                          // PDA bump
    pub admin: Pubkey,                     // 管理员
    pub flat_fees: Fees,                   // 固定费用
    pub fee_tiers: Vec<FeeTier>,           // 分级费用
}

pub struct Fees {
    pub lp_fee_bps: u64,                   // LP 费用基点
    pub protocol_fee_bps: u64,             // 协议费用基点
    pub creator_fee_bps: u64,              // 创建者费用基点
}

pub struct FeeTier {
    pub market_cap_lamports_threshold: u128,  // 市值阈值
    pub fees: Fees,                           // 对应的费用
}
```