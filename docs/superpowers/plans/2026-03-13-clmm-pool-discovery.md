# CLMM 自动池发现实现计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现基于 AmmConfig 遍历的 CLMM 池发现功能，根据两个 mint 地址快速找到最优流动性池。

**Architecture:** 预定义 16 个 Raydium CLMM AmmConfig 地址，通过 PDA 推导公式计算候选池地址，并发检查池子存在性和流动性，返回最佳池子。

**Tech Stack:** Rust, Solana SDK, tokio async, futures

---

## 文件结构

```
src/instruction/utils/raydium_clmm/
├── mod.rs                      # 添加新模块导出
├── pool_queries.rs             # 修改：添加新函数
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

/// Raydium CLMM 程序 ID
pub const RAYDIUM_CLMM_PROGRAM_ID: Pubkey = crate::constants::RAYDIUM_CLMM_PUBKEY;

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
    // Config #0 - 最低费率
    "9iFER3bpjf1PTTCQCfTRu17EJgvsxo9pVyA9QWwEuX4x",
    // Config #1
    "EdPxg8QaeFSrTYqdWJn6Kezwy9McWncTYueD9eMGCuzR",
    // Config #2
    "9EeWRCL8CJnikDFCDzG8rtmBs5KQR1jEYKCR5rRZ2NEi",
    // Config #3
    "3h2e43PunVA5K34vwKCLHWhZF4aZpyaC9RmxvshGAQpL",
    // Config #4
    "3XCQJQryqpDvvZBfGxR7CLAw5dpGJ9aa7kt1jRLdyxuZ",
    // Config #5
    "DrdecJVzkaRsf1TQu1g7iFncaokikVTHqpzPjenjRySY",
    // Config #6
    "J8u7HvA1g1p2CdhBFdsnTxDzGkekRpdw4GrL9MKU2D3U",
    // Config #7
    "RPxHtdN5V7ajwkoG6NnwSBAeaX5k9giY37dpp98xTjD",
    // Config #8
    "9WjDVMHWCirG9jkchbetHTnSzdXbAPnD9bsoGRcz1xUw",
    // Config #9
    "FMrUDGjEe1izXPbn8SZPNjMfB5JvvhVq5ymmpZDebB5R",
    // Config #10
    "E64NGkDLLCdQ2yFNPcavaKptrEgmiQaNykUuLC1Qgwyp",
    // Config #11
    "Y6YhgJbt9FRk3JVjwdZtsioVCJwCKhy1hum8HMDYyB1",
    // Config #12
    "47Nq74YtwjVeTQF6KFKRKU4cY1Vd5AXBHpYRkubkDLZi",
    // Config #13
    "DQeN7dZyQvXKT7YwmgqyuC7AYFkwMoP7RwtucsDEdfYZ",
    // Config #14
    "A1BBtTYJd4i3xU8D6Tc2FzU6ZN4oXZWXKZnCxwbHXr8x",
    // Config #15 - 最高费率
    "Gex2NJRS3jVLPfbzSFM5d5DRsNoL5ynnwT1TXoDEhanz",
];

/// 解析 AmmConfig 地址为 Pubkey 数组
///
/// # Panics
/// 如果任何地址解析失败会 panic（编译时常量，不会发生）
pub fn get_amm_config_pubkeys() -> Vec<Pubkey> {
    AMM_CONFIG_ADDRESSES
        .iter()
        .map(|s| Pubkey::from_str(s).expect("Invalid AmmConfig address"))
        .collect()
}

/// 对两个 mint 地址进行字典序排序
///
/// Raydium CLMM 池子 PDA 要求 mint 按字典序排序
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
///
/// # Formula
/// seeds = [b"pool", amm_config, sorted_mint0, sorted_mint1]
pub fn derive_pool_pda(
    amm_config: &Pubkey,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> (Pubkey, u8) {
    let (mint0, mint1) = sort_mints(mint_a, mint_b);

    Pubkey::find_program_address(
        &[POOL_SEED, mint0.as_ref(), mint1.as_ref(), amm_config.as_ref()],
        &RAYDIUM_CLMM_PROGRAM_ID,
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
        // 验证所有地址都不同
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

        // 两次调用应该返回相同结果
        let (pda1, bump1) = derive_pool_pda(&config, &mint_a, &mint_b);
        let (pda2, bump2) = derive_pool_pda(&config, &mint_a, &mint_b);

        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
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
    derive_pool_pda, get_amm_config_pubkeys, sort_mints, AMM_CONFIG_ADDRESSES,
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

use anyhow::{anyhow, Result};
use futures::stream::{self, StreamExt};
use solana_sdk::pubkey::Pubkey;

use crate::common::SolanaRpcClient;

use super::{
    amm_configs::{derive_pool_pda, get_amm_config_pubkeys},
    pool_queries::get_pool_by_address,
    raydium_clmm_types::PoolState,
};

/// 池发现结果：包含池地址、状态和流动性
#[derive(Debug, Clone)]
pub struct DiscoveredPool {
    pub address: Pubkey,
    pub pool: PoolState,
    pub amm_config_index: usize,
}

/// 通过遍历 AmmConfig 发现 CLMM 池子
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
/// 所有找到的池子列表，按流动性降序排序
///
/// # Performance
/// - 并发检查所有 16 个候选池
/// - 相比 RPC 扫描方式，减少了大量网络传输
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
                Err(_) => None, // 池子不存在
            }
        })
        .buffer_unordered(16) // 并发 16 个请求
        .collect()
        .await;

    // 收集有效结果并按流动性降序排序
    let mut valid_pools: Vec<_> = results.into_iter().flatten().collect();
    valid_pools.sort_by(|a, b| b.pool.liquidity.cmp(&a.pool.liquidity));

    Ok(valid_pools)
}

/// 发现最佳 CLMM 池子（流动性最高）
///
/// # Arguments
/// * `rpc` - Solana RPC 客户端
/// * `mint_a` - 第一个 token mint
/// * `mint_b` - 第二个 token mint
///
/// # Returns
/// 流动性最高的池子，如果存在的话
pub async fn find_best_pool_by_mints(
    rpc: &SolanaRpcClient,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> Result<Option<DiscoveredPool>> {
    let pools = discover_pools_by_mints(rpc, mint_a, mint_b).await?;
    Ok(pools.into_iter().next())
}

/// 发现 CLMM 池子（带最小流动性过滤）
///
/// # Arguments
/// * `rpc` - Solana RPC 客户端
/// * `mint_a` - 第一个 token mint
/// * `mint_b` - 第二个 token mint
/// * `min_liquidity` - 最小流动性阈值
///
/// # Returns
/// 所有流动性 >= min_liquidity 的池子，按流动性降序排序
pub async fn discover_pools_with_min_liquidity(
    rpc: &SolanaRpcClient,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
    min_liquidity: u128,
) -> Result<Vec<DiscoveredPool>> {
    let pools = discover_pools_by_mints(rpc, mint_a, mint_b).await?;
    let filtered: Vec<_> = pools
        .into_iter()
        .filter(|p| p.pool.liquidity >= min_liquidity)
        .collect();
    Ok(filtered)
}

#[cfg(test)]
mod tests {
    // 集成测试放在 tests/clmm_pool_discovery_tests.rs
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
    discover_pools_by_mints, discover_pools_with_min_liquidity, find_best_pool_by_mints,
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

use sol_trade_sdk::instruction::utils::raydium_clmm::{
    discover_pools_by_mints, find_best_pool_by_mints, DiscoveredPool,
};
use sol_trade_test_utils::get_test_rpc_client;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// SOL mint 地址
fn sol_mint() -> Pubkey {
    Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap()
}

/// USDC mint 地址
fn usdc_mint() -> Pubkey {
    Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap()
}

#[tokio::test]
async fn test_discover_sol_usdc_pools() {
    let rpc = get_test_rpc_client();

    let pools = discover_pools_by_mints(&rpc, &sol_mint(), &usdc_mint())
        .await
        .expect("Failed to discover pools");

    // SOL/USDC 是热门交易对，应该至少有一个池子
    assert!(!pools.is_empty(), "Should find at least one SOL/USDC pool");

    // 验证池子按流动性降序排序
    for i in 1..pools.len() {
        assert!(
            pools[i - 1].pool.liquidity >= pools[i].pool.liquidity,
            "Pools should be sorted by liquidity descending"
        );
    }

    println!("Found {} SOL/USDC pools:", pools.len());
    for (i, p) in pools.iter().enumerate() {
        println!(
            "  #{}: {} - liquidity: {}, tick_spacing: {}",
            i + 1,
            p.address,
            p.pool.liquidity,
            p.pool.tick_spacing
        );
    }
}

#[tokio::test]
async fn test_find_best_sol_usdc_pool() {
    let rpc = get_test_rpc_client();

    let best_pool = find_best_pool_by_mints(&rpc, &sol_mint(), &usdc_mint())
        .await
        .expect("Failed to find best pool");

    assert!(best_pool.is_some(), "Should find a best pool");

    let pool = best_pool.unwrap();
    println!(
        "Best SOL/USDC pool: {} (liquidity: {}, config: {})",
        pool.address, pool.pool.liquidity, pool.amm_config_index
    );

    // 验证池子的 mint 是 SOL 和 USDC
    let has_sol = pool.pool.token_mint0 == sol_mint() || pool.pool.token_mint1 == sol_mint();
    let has_usdc = pool.pool.token_mint0 == usdc_mint() || pool.pool.token_mint1 == usdc_mint();
    assert!(has_sol && has_usdc, "Pool should contain SOL and USDC");
}

#[tokio::test]
async fn test_discover_pools_same_mint() {
    let rpc = get_test_rpc_client();

    // 相同 mint 不应该找到任何池子
    let pools = discover_pools_by_mints(&rpc, &sol_mint(), &sol_mint())
        .await
        .expect("Failed to discover pools");

    // 可能返回空，也可能返回一些池子（取决于链上状态）
    // 这里只验证函数不会崩溃
    println!("Found {} pools for same mint", pools.len());
}

#[tokio::test]
async fn test_discover_nonexistent_pair() {
    let rpc = get_test_rpc_client();

    // 使用一个不存在的 mint 地址
    let fake_mint = Pubkey::new_unique();

    let pools = discover_pools_by_mints(&rpc, &fake_mint, &sol_mint())
        .await
        .expect("Failed to discover pools");

    // 不存在的 pair 应该返回空列表
    assert!(pools.is_empty(), "Should not find any pool for fake mint");
}
```

- [ ] **Step 2: 运行测试验证编译**

Run: `cargo test --test clmm_pool_discovery_tests --no-run`
Expected: 编译成功

- [ ] **Step 3: 运行测试**

Run: `cargo nextest run clmm_pool_discovery_tests`
Expected: 测试通过（可能需要本地 surfpool 节点运行）

- [ ] **Step 4: Commit**

```bash
git add tests/clmm_pool_discovery_tests.rs
git commit -m "test(clmm): add pool discovery integration tests"
```

---

## Chunk 4: 更新 pool_queries.rs 添加便捷函数

### Task 4: 在现有 pool_queries.rs 中添加便捷函数

**Files:**
- Modify: `src/instruction/utils/raydium_clmm/pool_queries.rs`

- [ ] **Step 1: 在 pool_queries.rs 中添加新函数**

在文件末尾添加：

```rust
/// 通过 AmmConfig 遍历发现指定 mint 对的池子（快速方法）
///
/// 相比 `list_pools_by_mint` 的 RPC 扫描方式，此方法：
/// - 只需要 16 次 RPC 调用（并发）
/// - 不需要扫描整个程序账户
/// - 更快更轻量
///
/// # Arguments
/// * `rpc` - Solana RPC 客户端
/// * `mint_a` - 第一个 token mint
/// * `mint_b` - 第二个 token mint
///
/// # Returns
/// 流动性最高的池子
pub async fn get_best_pool_by_mint_pair(
    rpc: &SolanaRpcClient,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    use super::pool_discovery::find_best_pool_by_mints;

    let result = find_best_pool_by_mints(rpc, mint_a, mint_b)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No CLMM pool found for pair {} - {}", mint_a, mint_b))?;

    Ok((result.address, result.pool))
}

/// 通过 AmmConfig 遍历列出指定 mint 对的所有池子
///
/// # Arguments
/// * `rpc` - Solana RPC 客户端
/// * `mint_a` - 第一个 token mint
/// * `mint_b` - 第二个 token mint
///
/// # Returns
/// 所有存在的池子，按流动性降序排序
pub async fn list_pools_by_mint_pair(
    rpc: &SolanaRpcClient,
    mint_a: &Pubkey,
    mint_b: &Pubkey,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    use super::pool_discovery::discover_pools_by_mints;

    let results = discover_pools_by_mints(rpc, mint_a, mint_b).await?;

    Ok(results
        .into_iter()
        .map(|p| (p.address, p.pool))
        .collect())
}
```

- [ ] **Step 2: 验证编译**

Run: `cargo check`
Expected: 无错误

- [ ] **Step 3: Commit**

```bash
git add src/instruction/utils/raydium_clmm/pool_queries.rs
git commit -m "feat(clmm): add get_best_pool_by_mint_pair convenience function"
```

---

## Chunk 5: 文档和最终验证

### Task 5: 更新文档和运行完整测试

**Files:**
- Modify: `docs/DEX_AND_POOL_REFERENCE.md` (如存在)

- [ ] **Step 1: 运行所有 CLMM 测试**

Run: `cargo nextest run clmm`
Expected: 所有测试通过

- [ ] **Step 2: 运行 clippy 检查**

Run: `cargo clippy -- -D warnings`
Expected: 无警告

- [ ] **Step 3: 格式化代码**

Run: `cargo fmt`

- [ ] **Step 4: Final Commit**

```bash
git add -A
git commit -m "feat(clmm): add AmmConfig-based pool discovery

- Add 16 predefined AmmConfig addresses
- Implement PDA derivation for CLMM pools
- Add concurrent pool discovery (16 parallel RPC calls)
- Support liquidity-based filtering and sorting
- Much faster than getProgramAccounts scanning"
```

---

## 使用示例

完成后的使用方式：

```rust
use sol_trade_sdk::instruction::utils::raydium_clmm::{
    get_best_pool_by_mint_pair,
    list_pools_by_mint_pair,
};
use solana_sdk::pubkey::Pubkey;

async fn example() {
    let rpc = get_test_rpc_client();
    let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

    // 方式 1: 获取最佳池子
    let (pool_addr, pool) = get_best_pool_by_mint_pair(&rpc, &sol_mint, &usdc_mint).await.unwrap();
    println!("Best pool: {} (liquidity: {})", pool_addr, pool.liquidity);

    // 方式 2: 列出所有池子（按流动性排序）
    let all_pools = list_pools_by_mint_pair(&rpc, &sol_mint, &usdc_mint).await.unwrap();
    for (addr, pool) in all_pools {
        println!("Pool: {} - tick_spacing: {}", addr, pool.tick_spacing);
    }
}
```

---

## 性能对比

| 方法 | RPC 调用 | 速度 | 适用场景 |
|------|----------|------|----------|
| `list_pools_by_mint` | 1 次 `getProgramAccounts` | 慢（大量数据传输） | 需要所有池子 |
| `get_best_pool_by_mint_pair` | 16 次 `getAccount`（并发） | 快 | 已知 mint 对 |
