# Token Program 与 ATA 最佳实践

## 问题背景

在 Solana 开发中，Token Program 有两个版本：
- **旧版 Token Program**: `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`
- **Token-2022 Program**: `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`

不同 token 可能使用不同的 Token Program，例如：
- **WSOL**: 使用旧版 Token Program
- **PUMP token**: 使用 Token-2022 Program
- **USDC**: 使用旧版 Token Program
- **新发行的 token**: 大多使用 Token-2022 Program

## 常见错误

### ❌ 硬编码 Token Program

```rust
// 错误示例：硬编码
base_token_program: spl_token::id(),        // 假设所有 token 都用旧版
quote_token_program: spl_token::id(),       // 假设所有 token 都用旧版
```

**问题**：
1. 需要记忆每个 token 使用的 Token Program
2. 容易出错
3. 不支持新 token
4. 维护成本高

## ✅ 正确方案：自动检测

### 方法 1：从 Mint Account 获取（推荐）

**原理**：Mint 账户的 `owner` 字段就是该 Token 使用的 Token Program

```rust
/// 从 mint 地址获取 Token Program（mint 的 owner 就是 Token Program）
async fn get_token_program_for_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<Pubkey, String> {
    let account = rpc
        .get_account(mint)
        .await
        .map_err(|e| format!("RPC error: {}", e))?;

    // Mint 账户的 owner 就是该 Token 使用的 Token Program
    Ok(account.owner)
}

// 使用
let base_token_program = get_token_program_for_mint(&rpc, &base_mint).await?;
let quote_token_program = get_token_program_for_mint(&rpc, &quote_mint).await?;
```

### 方法 2：使用 SDK 的 `get_mint_info`

```rust
use sol_trade_sdk::utils::token::get_mint_info;

let mint_info = get_mint_info(&rpc, &mint).await?;
// mint_info.is_token2022 可以判断是否为 Token-2022
```

## ATA 地址计算

### ATA 地址依赖 Token Program

ATA 地址是通过 PDA (Program Derived Address) 计算的：

```rust
// ATA 地址计算公式
Pubkey::find_program_address(
    &[
        b"associated_token_account",
        owner.as_ref(),
        mint.as_ref(),
    ],
    token_program  // ← 不同的 Token Program 会计算出不同的 ATA 地址！
)
```

**重要**：如果使用错误的 Token Program，会计算出错误的 ATA 地址。

### 为什么有时写错了也能用？

**ATA 不需要按规则创建**，只要满足以下条件就能正常使用：

1. ✅ 账户地址在链上存在
2. ✅ 账户的 Owner 是正确的 Token Program
3. ✅ Mint 地址正确
4. ✅ 有正确的权限

**场景举例**：
- 你用错误的 Token Program 计算出地址 A
- 但链上已经存在地址 B（用正确的 Token Program 创建的）
- 如果你的代码最终使用了地址 B，交易仍然能成功
- 但这不是可靠的做法！

## 最佳实践

### 1. 始终自动检测 Token Program

```rust
// ✅ 正确
let token_program = get_token_program_for_mint(&rpc, &mint).await?;

// ❌ 错误
let token_program = spl_token::id();  // 硬编码
```

### 2. 使用 SDK 提供的工具函数

```rust
use sol_trade_sdk::utils::token::get_mint_info;

let mint_info = get_mint_info(&rpc, &mint).await?;
let is_token2022 = mint_info.is_token2022;
```

### 3. 测试时验证 Token Program

```rust
// 打印检测到的 Token Program，便于调试
println!("✅ 自动检测 {} Token Program: {}", mint, token_program);
```

## 常见 Token 的 Token Program

| Token | Mint Address | Token Program |
|-------|-------------|---------------|
| WSOL | `So11111111111111111111111111111111111111112` | 旧版 Token Program |
| PUMP | `pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn` | **Token-2022** |
| USDC | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | 旧版 Token Program |
| USDT | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` | 旧版 Token Program |

**注意**：这只是一个参考表，新 token 可能使用不同的 Token Program，始终应该自动检测。

## 相关代码示例

### PumpSwap 测试中的自动检测

文件：`tests/verify_pumpswap_with_simulation.rs`

```rust
/// 从 mint 地址获取 Token Program（mint 的 owner 就是 Token Program）
async fn get_token_program_for_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<Pubkey, String> {
    let account = rpc
        .get_account(mint)
        .await
        .map_err(|e| format!("RPC error: {}", e))?;

    // Mint 账户的 owner 就是该 Token 使用的 Token Program
    Ok(account.owner)
}

// 在测试中使用
let base_token_program = match get_token_program_for_mint(&rpc, &base_mint).await {
    Ok(program) => {
        println!("✅ 自动检测 base_mint ({}) Token Program: {}", base_mint, program);
        program
    },
    Err(e) => {
        println!("⚠️  无法获取 base_mint Token Program，使用默认值: {}", e);
        TOKEN_PROGRAM
    },
};
```

## 参考

- [Token-2022 Program 官方文档](https://www.solana-program.com/docs/token-2022)
- [Associated Token Account 地址推导](https://www.anchor-lang.com/docs/tokens/basics/create-token-account)
- SDK 实现：`src/utils/token.rs`
