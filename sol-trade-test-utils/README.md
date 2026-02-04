# sol-trade-test-utils

Solana 交易测试工具库 - 提供便捷的测试辅助函数。

## 功能

### 1. 空投 SOL

```rust
use sol_trade_test_utils::airdrop_and_wait;

// 空投 10 SOL
airdrop_and_wait("http://127.0.0.1:8899", &payer.pubkey(), 10).await?;
```

### 2. 确保 Token 余额

```rust
use sol_trade_test_utils::ensure_token_balance;

// 确保至少有 1000 USDC
ensure_token_balance(&rpc, "http://127.0.0.1:8899", &payer, &usdc_mint, "1000").await?;
```

### 3. 确保 SOL 余额

```rust
use sol_trade_test_utils::ensure_sol_balance;

// 确保至少有 10 SOL
ensure_sol_balance(&rpc, "http://127.0.0.1:8899", &payer.pubkey(), 10).await?;
```

### 4. 确保 PIPE-WSOL Pool 流动性（推荐）

```rust
use sol_trade_test_utils::ensure_pipe_wsol_pool_liquidity;

// 确保 PIPE-WSOL pool 至少有 1000 SOL 的流动性
// 如果不足，会自动添加流动性
ensure_pipe_wsol_pool_liquidity(
    &rpc,
    "http://127.0.0.1:8899",
    &payer,
    1000,  // 1000 SOL
).await?;
```

这是最便捷的方式，特别适合测试场景。函数会：
1. 检查当前 PIPE-WSOL pool 的 WSOL vault 余额
2. 如果不足 1000 SOL，自动计算并添加所需流动性
3. 自动按比例添加 PIPE token

### 5. 确保 CPMM 流动性（通用）

```rust
use sol_trade_test_utils::ensure_cpmm_liquidity;

// 向任意 CPMM 池子添加流动性
ensure_cpmm_liquidity(
    &rpc,
    "http://127.0.0.1:8899",
    &payer,
    &pool_address,
    1_000_000_000,  // 10 亿 LP
    "10000",        // 10000 Token0
    "10",           // 10 Token1
).await?;
```

### 6. Mint Token

```rust
use sol_trade_test_utils::mint_token_to;

// Mint 1000 个 token 到指定账户
mint_token_to(&rpc, rpc_url, &mint_authority, &mint, &recipient, 1_000_000_000).await?;
```

### 7. 转移 Token

```rust
use sol_trade_test_utils::transfer_token_to;

// 转移 100 个 token
transfer_token_to(&rpc, rpc_url, &payer, &mint, &from, &to, 100_000_000).await?;
```

## 测试示例

```rust
use sol_trade_sdk::common::SolanaRpcClient;
use solana_sdk::signer::Signer;
use std::sync::Arc;

use sol_trade_test_utils::{
    ensure_sol_balance,
    ensure_token_balance,
    get_simulation_test_keypair,
};

#[tokio::test]
async fn my_test() {
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.to_string()));
    let payer = Arc::new(get_simulation_test_keypair());

    // 确保 SOL 余额
    ensure_sol_balance(&rpc, rpc_url, &payer.pubkey(), 10)
        .await
        .expect("SOL 余额不足");

    // 确保 Token 余额
    ensure_token_balance(&rpc, rpc_url, &payer, &usdc_mint, "1000")
        .await
        .expect("Token 余额不足");

    // 执行测试...
}
```

## 在现有测试中使用

现有的测试代码无需修改，`tests/common/mod.rs` 已经重新导出了所有功能：

```rust
// 依然可以使用旧的导入方式
mod common;
use common::{ensure_sol_balance, ensure_token_balance, get_simulation_test_keypair};
```

## 注意事项

1. **仅用于测试环境**：这些函数使用了 surfpool 特定的 RPC 方法
2. **串行测试**：建议使用 `#[serial_test::serial]` 避免测试冲突
3. **网络依赖**：需要连接到本地测试节点（127.0.0.1:8899）

## 文档

更多详细信息请参考源代码文档。
