# CLMM 池发现（AmmConfig 遍历方式）

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现基于 AmmConfig 遍历的 CLMM 池发现功能，快速找到指定 mint 对的所有池子。

**Architecture:** 预定义 16 个 Raydium CLMM AmmConfig 地址，通过 PDA 推导计算候选池地址，并发检查池子是否存在。

**Tech Stack:** Rust, Solana SDK, tokio async, futures

---

## 文件结构

```
src/instruction/utils/raydium_clmm/
├── mod.rs                      # 添加新模块导出
├── amm_configs.rs              # 新建：AmmConfig 常量和 PDA 推导
└── pool_discovery.rs           # 新建：池发现核心逻辑

tests/
└── clmm_pool_discovery_tests.rs  # 新建：测试文件
```

---

## Chunk 1: AmmConfig 常量和 PDA 推导

### Task 1: 创建 AmmConfig 常量模块

**Files:**
- Create: `src/instruction/utils/raydium_clmm/amm_configs.rs`
- Modify: `src/instruction/utils/raydium_clmm/mod.rs`

- [ ] **Step 1: 创建 amm_configs.rs 文件**

```rust
// src/instruction/utils/raydium_clmm/amm_configs.rs
//
// Raydium CLMM AmmConfig 常量定义
// 参考：Cobra Router 实现

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Pool PDA 种子
pub const POOL_SEED: &[u8] = b"pool";

/// Raydium CLMM 的 16 个 AmmConfig 地址
///
/// 每个 AmmConfig 对应不同的费率等级和 tick_spacing：
/// - 费率从 0.01% 到 2%
/// - tick_spacing 从 1 到 200
///
/// 池子 PDA 推导公式：
/// seeds = [b"pool", amm_config, mint0, mint1]
/// 其中 mint0 和 mint1 按字典序排序
pub const AMM_CONFIG_ADDRESSES: [&str; 16] = [
    "9iFER3bpjf1PTTCQCfTRu17EJgvsxo9pVyA9QWwEuX4x",
    "EdPxg8QaeFSrTYqdWJn6Kezwy9McWncTYueD9eMGCuzR",
    "3h2e43PunVA5K34vwKCLHWhZF4aZpyaC9RmxvshGAQpL",
    "3XCQJQryqpDvvZBfGxR7CLAw5dpGJ9aa7kt1jRLdyxuZ",
    "DrdecJVzkaRsf1TQu1g7iFncaokikVTHqpzPjenjRySY",
    "J8u7HvA1g1p2CdhBFdsnTxDzGkekRpdw4GrL9MKU2D3U",
    "RPxHtdN5V7ajwkoG6NnwSBAeaX5k9giY37dpp98xTjD",
    "9WjDVMHWCirG9jkchbetHTnSzdXbAPnD9bsoGRcz1xUw",
    "FMrUDGjEe1izXPbn8SZPNjMfB5JvvhVq5ymmpZDebB5R",
    "E64NGkDLLCdQ2yFNPcavaKptrEgmiQaNykUuLC1Qgwyp",
    "Y6YhgJbt9FRk3JVjwdZtsioVCJwCKhy1hum8HMDYyB1",
    "47Nq74YtwjVeTQF6KFKRKU4cY1Vd5AXBHpYRkubkDLZi",
    "DQeN7dZyQvXKT7YwmgqyuC7AYFkwMoP7RwtucsDEdfYZ",
    "A1BBtTYJd4i3xU8D6Tc2FzU6ZN4oXZWXKZnCxwbHXr8x",
    "Gex2NJRS3jVLPfbzSFM5d5DRsNoL5ynnwT1TXoDEhanz",
    "CDpiwv9eLsRvvuzZEJ8CBtK14wdvkSnkub4vmGtzzdK8",
];

/// 解析 AmmConfig 地址为 Pubkey 数组
pub fn get_amm_config_pubkeys() -> Vec<Pubkey> {
    AMM_CONFIG_ADDRESSES
        .iter()
        .map(|s| Pubkey::from_str(s).expect("Invalid AmmConfig address"))
        .collect()
}

/// 对两个 mint 地址进行字典序排序
pub fn sort_mints(mint_a: &Pubkey, mint_b: &Pubkey) -> (Pubkey, Pubkey) {
    if mint_a < mint_b {
        (*mint_a, *mint_b)
    } else {
        (*mint_b, *mint_a)
    }
}

/// 计算 CLMM 池子 PDA 地址
///
/// # Arguments
/// * `amm_config` - AmmConfig 账户地址
/// * `mint_a` - 第一个 token mint
/// * `mint_b` - 第二个 token mint
///
/// # Returns
/// (pool_pda, bump) 元组
pub fn derive_pool_pda(
    amm_config: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> (Pubkey, u8) {
    use crate::constants::RAYDIUM_CLMM_PUBKEY;
    let (mint0, mint1) = sort_mints(mint_a, mint_b);

    Pubkey::find_program_address(
        &[POOL_SEED, mint0.as_ref(), mint1.as_ref(), amm_config.as_ref()],
        &RAYDIUM_CLMM_PUBKEY,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amm_config_count() {
        assert_eq!(AMM_CONFIG_ADDRESSES.len(), 16);
    }

    #[test]
    fn test_all_configs_valid_pubkeys() {
        let pubkeys = get_amm_config_pubkeys();
        assert_eq!(pubkeys.len(), 16);
        let unique: std::collections::HashSet<_> = pubkeys.iter().collect();
        assert_eq!(unique.len(), 16);
    }

    #[test]
    fn test_sort_mints_order() {
        let mint_a = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let mint_b = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

        let (m0, m1) = sort_mints(&mint_a, &mint_b);
        assert!(m0 < m1);
    }

    #[test]
    fn test_derive_pool_pda_consistency() {
        let config = Pubkey::from_str(AMM_CONFIG_ADDRESSES[0]).unwrap();
        let mint_a = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let mint_b = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

        let (pda1, bump1) = derive_pool_pda(&config, &mint_a, &mint_b);
        let (pda2, bump2) = derive_pool_pda(&config, &mint_a, &mint_b);

        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
        println!("SOL/USDC pool PDA: {} (bump: {})", pda1, bump1);
    }
}
```

- [ ] **Step 2: 运行测试验证编译通过**

Run: `cargo test --lib amm_configs --no-run`
Expected: 编译成功，无错误

- [ ] **Step 3: 运行单元测试**

Run: `cargo nextest run amm_configs`
Expected: 4 个测试全部通过

- [ ] **Step 4: 修改 mod.rs 导出新模块**

```rust
// src/instruction/utils/raydium_clmm/mod.rs
// 在文件末尾添加：

pub mod amm_configs;
pub use amm_configs::{
    derive_pool_pda, get_amm_config_pubkeys, sort_mints, AMM_CONFIG_ADDRESSES, POOL_SEED,
};
```

- [ ] **Step 5: 验证模块导出**

Run: `cargo check`
Expected: 无错误

- [ ] **Step 6: Commit**

```bash
git add src/instruction/utils/raydium_clmm/amm_configs.rs src/instruction/utils/raydium_clmm/mod.rs
git commit -m "feat(clmm): add AmmConfig constants and PDA derivation"
```

---

## Chunk 2: 池发现核心逻辑

### Task 2: 创建池发现模块

**Files:**
- Create: `src/instruction/utils/raydium_clmm/pool_discovery.rs`
- Modify: `src/instruction/utils/raydium_clmm/mod.rs`

- [ ] **Step 1: 创建 pool_discovery.rs 文件**

```rust
// src/instruction/utils/raydium_clmm/pool_discovery.rs
//
// CLMM 池发现模块 - 基于 AmmConfig 遍历的快速池发现
// 参考：Cobra Router 实现

use anyhow::Result;
use futures::stream::{self, StreamExt};
use solana_sdk::pubkey::Pubkey;

use crate::common::SolanaRpcClient;

use super::{
    amm_configs::{derive_pool_pda, get_amm_config_pubkeys},
    pool_queries::get_pool_by_address,
    raydium_clmm_types::PoolState,
};

/// 池发现结果
#[derive(Debug, Clone)]
pub struct DiscoveredPool {
    pub address: Pubkey,
    pub pool: PoolState,
    pub amm_config_index: usize,
}

/// 通过遍历 AmmConfig 发现指定 mint 对的所有 CLMM 池子
///
/// 此方法通过预定义的 16 个 AmmConfig 地址计算候选池 PDA，
/// 然后并发检查哪些池子实际存在。
///
/// # Arguments
/// * `rpc` - Solana RPC 客户端
/// * `mint_a` - 第一个 token mint
/// * `mint_b` - 第二个 token mint
///
/// # Returns
/// 所有存在的池子列表（流动性 > 0），按流动性降序排序
///
/// # Performance
/// - 16 次并发 RPC 调用
/// - 相比 `getProgramAccounts` 方式更快
///
/// # Example
/// ```ignore
/// let pools = discover_pools_by_mints(&rpc, &sol_mint, &usdc_mint).await?;
/// for pool in pools {
///     println!("Pool: {} (liquidity: {})", pool.address, pool.pool.liquidity);
/// }
/// ```
pub async fn discover_pools_by_mints(
    rpc: &SolanaRpcClient,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> Result<Vec<DiscoveredPool>> {
    let configs = get_amm_config_pubkeys();

    // 并发检查所有 16 个候选池
    let results: Vec<_> = stream::iter(configs.iter().enumerate())
        .map(|(index, config)| async move {
            let (pool_pda, _bump) = derive_pool_pda(config, mint_a, mint_b);

            // 尝试获取池子状态
            match get_pool_by_address(rpc, &pool_pda).await {
                Ok(pool) => {
                    // 过滤掉无流动性的池子
                    if pool.liquidity > 0 {
                        Some(DiscoveredPool {
                            address: pool_pda,
                            pool,
                            amm_config_index: index,
                        })
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        })
        .buffer_unordered(16)
        .collect()
        .await;

    // 收集有效结果并按流动性降序排序
    let mut valid_pools: Vec<_> = results.into_iter().flatten().collect();
    valid_pools.sort_by(|a, b| b.pool.liquidity.cmp(&a.pool.liquidity));

    Ok(valid_pools)
}

/// 发现指定 mint 对的所有 CLMM 池子（包含流动性为 0 的）
///
/// 与 `discover_pools_by_mints` 不同，此函数返回所有存在的池子，
/// 包括流动性为 0 的池子。
pub async fn discover_all_pools_by_mints(
    rpc: &SolanaRpcClient,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> Result<Vec<DiscoveredPool>> {
    let configs = get_amm_config_pubkeys();

    let results: Vec<_> = stream::iter(configs.iter().enumerate())
        .map(|(index, config)| async move {
            let (pool_pda, _bump) = derive_pool_pda(config, mint_a, mint_b);

            match get_pool_by_address(rpc, &pool_pda).await {
                Ok(pool) => Some(DiscoveredPool {
                    address: pool_pda,
                    pool,
                    amm_config_index: index,
                }),
                Err(_) => None,
            }
        })
        .buffer_unordered(16)
        .collect()
        .await;

    let mut valid_pools: Vec<_> = results.into_iter().flatten().collect();
    valid_pools.sort_by(|a, b| b.pool.liquidity.cmp(&a.pool.liquidity));

    Ok(valid_pools)
}

/// 发现指定 mint 对的第一个 CLMM 池子（任意流动性）
///
/// 遍历 AmmConfig，返回第一个存在的池子。
/// 适用于快速检查是否存在池子，不关心流动性。
pub async fn find_first_pool_by_mints(
    rpc: &SolanaRpcClient,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> Result<Option<DiscoveredPool>> {
    let configs = get_amm_config_pubkeys();

    for (index, config) in configs.iter().enumerate() {
        let (pool_pda, _bump) = derive_pool_pda(config, mint_a, mint_b);

        if let Ok(pool) = get_pool_by_address(rpc, &pool_pda).await {
            return Ok(Some(DiscoveredPool {
                address: pool_pda,
                pool,
                amm_config_index: index,
            }));
        }
    }

    Ok(None)
}
```

- [ ] **Step 2: 验证编译通过**

Run: `cargo check`
Expected: 无错误

- [ ] **Step 3: 修改 mod.rs 导出新模块**

```rust
// src/instruction/utils/raydium_clmm/mod.rs
// 添加：

pub mod pool_discovery;
pub use pool_discovery::{
    discover_all_pools_by_mints, discover_pools_by_mints, find_first_pool_by_mints,
    DiscoveredPool,
};
```

- [ ] **Step 4: Commit**

```bash
git add src/instruction/utils/raydium_clmm/pool_discovery.rs src/instruction/utils/raydium_clmm/mod.rs
git commit -m "feat(clmm): add pool discovery by iterating AmmConfigs"
```

---

## Chunk 3: 集成测试

### Task 3: 创建集成测试

**Files:**
- Create: `tests/clmm_pool_discovery_tests.rs`

- [ ] **Step 1: 创建测试文件**

```rust
// tests/clmm_pool_discovery_tests.rs
//
// CLMM 池发现集成测试

use solana_sdk::pubkey::Pubkey;
use sol_trade_sdk::instruction::utils::raydium_clmm::{
    discover_all_pools_by_mints, discover_pools_by_mints, find_first_pool_by_mints,
};
use sol_trade_test_utils::get_test_rpc_client;
use std::str::FromStr;

fn sol_mint() -> Pubkey {
    Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap()
}

fn usdc_mint() -> Pubkey {
    Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap()
}

fn usdt_mint() -> Pubkey {
    Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap()
}

#[tokio::test]
async fn test_discover_sol_usdc_pools() {
    let rpc = get_test_rpc_client();

    let pools = discover_pools_by_mints(&rpc, &sol_mint(), &usdc_mint())
        .await
        .expect("Failed to discover pools");

    // SOL/USDC 是热门交易对，应该有多个池子
    assert!(!pools.is_empty(), "Should find at least one SOL/USDC pool");

    // 验证池子按流动性降序排序
    for i in 1..pools.len() {
        assert!(
            pools[i - 1].pool.liquidity >= pools[i].pool.liquidity,
            "Pools should be sorted by liquidity descending"
        );
    }

    println!("Found {} SOL/USDC pools (liquidity > 0):", pools.len());
    for (i, p) in pools.iter().enumerate() {
        println!(
            "  #{}: config={} tick_spacing={} liquidity={}",
            i + 1, p.amm_config_index, p.pool.tick_spacing, p.pool.liquidity
        );
    }
}

#[tokio::test]
async fn test_discover_all_pools_includes_zero_liquidity() {
    let rpc = get_test_rpc_client();

    let all_pools = discover_all_pools_by_mints(&rpc, &sol_mint(), &usdc_mint())
        .await
        .expect("Failed to discover all pools");

    let active_pools = discover_pools_by_mints(&rpc, &sol_mint(), &usdc_mint())
        .await
        .expect("Failed to discover active pools");

    // all_pools 应该 >= active_pools
    assert!(
        all_pools.len() >= active_pools.len(),
        "All pools should be >= active pools"
    );

    println!(
        "All pools: {}, Active pools: {}",
        all_pools.len(),
        active_pools.len()
    );
}

#[tokio::test]
async fn test_find_first_pool() {
    let rpc = get_test_rpc_client();

    let pool = find_first_pool_by_mints(&rpc, &sol_mint(), &usdc_mint())
        .await
        .expect("Failed to find first pool");

    assert!(pool.is_some(), "Should find at least one SOL/USDC pool");

    let p = pool.unwrap();
    println!(
        "First pool found: config={} liquidity={}",
        p.amm_config_index, p.pool.liquidity
    );
}

#[tokio::test]
async fn test_discover_nonexistent_pair() {
    let rpc = get_test_rpc_client();

    let fake_mint = Pubkey::new_unique();

    let pools = discover_pools_by_mints(&rpc, &fake_mint, &sol_mint())
        .await
        .expect("Should not error");

    assert!(pools.is_empty(), "Should not find any pool for fake mint");
}

#[tokio::test]
async fn test_discover_sol_usdt_pools() {
    let rpc = get_test_rpc_client();

    let pools = discover_pools_by_mints(&rpc, &sol_mint(), &usdt_mint())
        .await
        .expect("Failed to discover pools");

    println!("Found {} SOL/USDT pools", pools.len());
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo test --test clmm_pool_discovery_tests --no-run`
Expected: 编译成功

- [ ] **Step 3: 运行测试**

Run: `cargo nextest run clmm_pool_discovery_tests`
Expected: 测试通过

- [ ] **Step 4: Commit**

```bash
git add tests/clmm_pool_discovery_tests.rs
git commit -m "test(clmm): add pool discovery integration tests"
```

---

## Chunk 4: 最终验证

### Task 4: 代码质量检查

- [ ] **Step 1: 运行 clippy**

Run: `cargo clippy -- -D warnings`
Expected: 无警告

- [ ] **Step 2: 格式化代码**

Run: `cargo fmt`

- [ ] **Step 3: 运行所有 CLMM 测试**

Run: `cargo nextest run clmm`
Expected: 所有测试通过

- [ ] **Step 4: Final Commit**

```bash
git add -A
git commit -m "feat(clmm): add AmmConfig-based pool discovery

- Add 16 predefined AmmConfig addresses (Cobra style)
- Implement PDA derivation for CLMM pools
- Concurrent pool discovery (16 parallel RPC calls)
- Faster than getProgramAccounts scanning"
```

---

## API 对比

| 函数 | 用途 | 返回 |
|------|------|------|
| `discover_pools_by_mints` | 发现有流动性的池 | 按流动性排序 |
| `discover_all_pools_by_mints` | 发现所有池（含 0 流动性） | 按流动性排序 |
| `find_first_pool_by_mints` | 快速检查池是否存在 | 第一个找到的池 |

## 使用示例

```rust
use sol_trade_sdk::instruction::utils::raydium_clmm::discover_pools_by_mints;
use solana_sdk::pubkey::Pubkey;

let sol = Pubkey::from_str("So11111111111111111111111111111111111111112")?;
let usdc = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;

// 发现所有有流动性的 SOL/USDC 池子
let pools = discover_pools_by_mints(&rpc, &sol, &usdc).await?;

// 选择最佳池（已有流动性排序）
if let Some(best) = pools.first() {
    println!("Best pool: {}", best.address);
    // 使用现有的选池策略或直接使用
}
```
