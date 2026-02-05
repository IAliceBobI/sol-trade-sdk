# Raydium AMM V4 Token-2022 支持分析报告

**分析日期**: 2025-02-05
**合约仓库**: `/opt/projects/sol-trade-sdk/temp/dex/raydium-amm`
**合约版本**: v0.3.1

---

## 📋 结论

**Raydium AMM V4 不支持 Token-2022 Program**，合约硬编码了必须使用原生的 `spl_token` Program。

---

## 🔍 技术证据

### 1. **初始化检查** (`processor.rs:873-878`)

```rust
pub fn process_initialize2(...) -> ProgramResult {
    // ...

    check_assert_eq!(
        *token_program_info.key,
        spl_token::id(),  // ← 硬编码检查，必须是 spl_token::id()
        "spl_token_program",
        AmmError::InvalidSplTokenProgram
    );

    // ...
}
```

**影响**:
- ❌ 无法使用 Token-2022 创建 Pool
- ❌ 如果传入 Token-2022 Program 地址，交易会失败并返回 `InvalidSplTokenProgram` 错误

---

### 2. **Swap 指令检查** (`processor.rs:2255-2260`)

```rust
pub fn process_swap_base_in(...) -> ProgramResult {
    // ...

    check_assert_eq!(
        *token_program_info.key,
        spl_token::id(),  // ← 硬编码检查
        "spl_token_program",
        AmmError::InvalidSplTokenProgram
    );

    let spl_token_program_id = token_program_info.key;

    // 后续所有 Token 操作都使用这个 ID
    let amm_coin_vault = Self::unpack_token_account(&amm_coin_vault_info, spl_token_program_id)?;
    let amm_pc_vault = Self::unpack_token_account(&amm_pc_vault_info, spl_token_program_id)?;
    let user_source = Self::unpack_token_account(&user_source_info, spl_token_program_id)?;
    let user_destination = Self::unpack_token_account(&user_destination_info, spl_token_program_id)?;

    // ...
}
```

**影响**:
- ❌ 所有 Token 账户解包操作都使用 `spl_token::state::Account`
- ❌ Token-2022 的扩展字段（如 Transfer Fee、Interest Bearing）无法识别

---

### 3. **Mint 账户检查** (`processor.rs:156-180`)

```rust
/// Unpacks a spl_token `Account`.
#[inline]
pub fn unpack_token_account(
    account_info: &AccountInfo,
    token_program_id: &Pubkey,
) -> Result<spl_token::state::Account, AmmError> {
    if account_info.owner != token_program_id {
        Err(AmmError::InvalidSplTokenProgram)
    } else {
        spl_token::state::Account::unpack(&account_info.data.borrow())
            .map_err(|_| AmmError::ExpectedAccount)
    }
}

/// Unpacks a spl_token `Mint`.
#[inline]
pub fn unpack_mint(
    account_info: &AccountInfo,
    token_program_id: &Pubkey,
) -> Result<spl_token::state::Mint, AmmError> {
    if account_info.owner != token_program_id {
        Err(AmmError::InvalidSplTokenProgram)
    } else {
        spl_token::state::Mint::unpack(&account_info.data.borrow())
            .map_err(|_| AmmError::ExpectedMint)
    }
}
```

**问题**:
- ❌ `spl_token::state::Account` 无法解析 Token-2022 的扩展数据结构
- ❌ `spl_token::state::Mint` 无法解析 Token-2022 Mint 的扩展字段
- ❌ 如果 Token Mint 的 owner 是 Token-2022 Program，会直接返回 `InvalidSplTokenProgram` 错误

---

### 4. **依赖配置** (`Cargo.toml`)

```toml
[dependencies]
solana-program = "=2.1.0"
spl-token = { version = "=7.0.0", features = ["no-entrypoint"] }
spl-associated-token-account = { version = "6.0.0", features = ["no-entrypoint"] }
```

**说明**:
- `spl-token = "=7.0.0"` 是老的 Token Program
- 没有引入 `spl-token-2022` 依赖
- 所有 Token 操作都基于老的数据结构

---

### 5. **Token 指令调用** (`invokers.rs`)

所有 Token 指令都使用 `spl_token::instruction::*`：

```rust
use spl_token::instruction::burn;
use spl_token::instruction::mint_to;
use spl_token::instruction::transfer;
use spl_token::instruction::sync_native;
// ... 等等
```

**问题**:
- ❌ 这些指令生成的指令数据格式与 Token-2022 不兼容
- ❌ Token-2022 需要额外的扩展指令处理

---

## 🔐 Token Program ID 对比

| Program | 地址 | 说明 |
|---------|------|------|
| **Token Program** | `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` | 老版本，AMM V4 使用 |
| **Token-2022 Program** | `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb` | 新版本，AMM V4 不支持 |

Raydium AMM V4 硬编码检查 `*token_program_info.key == spl_token::id()`，即必须是 `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`。

---

## 💡 为什么不支持 Token-2022？

### 技术原因

1. **数据结构不兼容**
   - Token-2022 引入了扩展机制（Extensions）
   - 老的 `spl_token::state::Account` 和 `spl_token::state::Mint` 无法解析扩展数据
   - 需要使用 `spl_token_2022::extension::StateWithExtensions` 解析

2. **指令格式不兼容**
   - Token-2022 的某些指令需要处理扩展数据
   - AMM V4 的指令构建逻辑未考虑扩展字段

3. **安全考虑**
   - Token-2022 的 Transfer Fee 扩展会影响 Swap 计算
   - Interest Bearing Token 的余额会随时间变化
   - 硬编码检查可以避免不可预测的行为

### 设计权衡

Raydium AMM V4 选择：
- ✅ **简单性**: 使用固定的 Token Program，逻辑简单
- ✅ **安全性**: 避免扩展 Token 的复杂行为
- ❌ **灵活性**: 无法使用 Token-2022 的新特性

---

## 🆚 与其他 DEX 的对比

| DEX 协议 | Token-2022 支持 | 说明 |
|----------|----------------|------|
| **Raydium AMM V4** | ❌ 不支持 | 硬编码检查 |
| **Raydium CPMM** | ✅ 支持 | 检查 `token_program` 字段 |
| **Raydium CLMM** | ✅ 支持 | 使用 Token-2022 |
| **PumpSwap** | ✅ 支持 | 使用 Token-2022 |
| **Orca** | ✅ 支持 | 原生支持 Token-2022 |

---

## 📝 建议

### 如果需要使用 Token-2022

1. **使用 Raydium CPMM**
   - ✅ 原生支持 Token-2022
   - ✅ 可以混合使用 Token 和 Token-2022
   - ✅ Pool 状态中存储 `token_program` 字段

2. **使用 Raydium CLMM**
   - ✅ 支持 Token-2022
   - ✅ 更高效的资本利用率
   - ✅ 集流动性做市商

3. **等待 Raydium AMM V4 升级**
   - Raydium 可能会发布支持 Token-2022 的 AMM V4 版本
   - 需要合约升级和重新部署

### 开发注意事项

在使用 SDK 时：
- ✅ AMM V4 只能处理使用 Token Program 的 Token
- ❌ 不要尝试使用 Token-2022 Token 创建/交易 AMM V4 Pool
- ✅ 如果 Token 是 Token-2022，请使用 CPMM 或 CLMM

---

## 🔗 相关代码位置

| 文件 | 行数 | 说明 |
|------|------|------|
| `processor.rs` | 873-878 | 初始化检查 |
| `processor.rs` | 2255-2260 | Swap 检查 |
| `processor.rs` | 156-180 | Token 账户解包 |
| `processor.rs` | 934-936 | Mint 账户解包 |
| `Cargo.toml` | 26 | spl-token 依赖 |

---

## 结论

**Raydium AMM V4 完全不支持 Token-2022 Program**，这是合约层面的硬性限制，无法通过 SDK 绕过。

如果您需要交易 Token-2022 Token，请使用：
- **Raydium CPMM** ✅
- **Raydium CLMM** ✅
- **Orca** ✅
- **PumpSwap** ✅

**不要使用**：
- **Raydium AMM V4** ❌

---

**分析完成**: 2025-02-05
**分析人**: Claude Code
