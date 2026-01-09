# Pool 选择算法分析

**分析日期**：2026-01-09  
**分析范围**：Autobahn Router、Cobra Router、Solana DEX V1 Router  
**数据来源**：`/opt/projects/sol-trade-sdk/temp/autobahn-router`、`/opt/projects/sol-trade-sdk/temp/Cobra-router`

---

## 概述

本文档分析了三个 Solana DEX Router 项目中的 Pool 选择算法，为 sol-trade-sdk 的 Pool 评分和选择功能提供参考。

---

## 1. Autobahn Router（最完善的算法）

**项目位置**：`/opt/projects/sol-trade-sdk/temp/autobahn-router/bin/autobahn-router/src/routing.rs`

### 1.1 核心算法：`select_best_pools()`

```rust
fn select_best_pools(
    hot_mints: &HashSet<Pubkey>,
    max_count_for_hot: usize,      // 热门 mint 对的最大 pool 数量
    max_edge_per_cold_pair: usize, // 冷门 mint 对的最大 pool 数量
    path_warming_amounts: &Vec<u64>,
    all_edges: &Vec<Arc<Edge>>,
    mint_to_index: &HashMap<Pubkey, MintNodeIndex>,
    swap_mode: SwapMode,
) -> (i32, MintVec<Vec<EdgeWithNodes>>)
```

### 1.2 选择标准

#### 1.2.1 价格影响（Price Impact）- 主要指标

```rust
fn compute_price_impact(edge: &Arc<Edge>) -> Option<f64> {
    let state = edge.state.read().unwrap();
    if !state.is_valid() || state.cached_prices.len() < 2 {
        return None;
    }

    let first = state.cached_prices[0].1;  // 小额交易价格
    let last = state.cached_prices[state.cached_prices.len() - 1].1;  // 大额交易价格

    if first == 0.0 || last == 0.0 {
        return None;
    }

    if first < last {
        // 异常情况：大额交易价格更好，可能是 OpenBook 的 lot size 问题
        Some(last / first - 1.0)
    } else {
        Some(first / last - 1.0)
    }
}
```

**过滤规则**：
- 过滤掉价格影响 > 25% 的 pool
- 保留价格影响最小的 pool

#### 1.2.2 Hot/Cold Mint 区分

```rust
let is_hot = hot_mints.is_empty()
    || (hot_mints.contains(&edge.input_mint)
        && hot_mints.contains(&edge.output_mint));
let max_count = if is_hot {
    max_count_for_hot  // 默认 8
} else {
    max_edge_per_cold_pair  // 默认 3
};
```

**说明**：
- Hot Mint：两个 mint 都在热门集合中（如 WSOL、USDC、USDT）
- Cold Mint：至少一个 mint 不在热门集合中
- Hot Mint 对保留更多 pool（8 个），Cold Mint 对保留更少 pool（3 个）

#### 1.2.3 多金额测试

```rust
for (i, _amount) in path_warming_amounts.iter().enumerate() {
    let mut best = HashMap::<(Pubkey, Pubkey), Vec<(EdgeIndex, f64)>>::new();

    for (edge_index, edge) in all_edges.iter().enumerate() {
        // 获取不同金额下的价格
        let price = state.cached_prices[i].1;
        if price.is_nan() || price.is_sign_negative() || price <= 0.000000001 {
            continue;
        }

        // 保留每个金额下的最佳 pool
        let entry = best.entry((edge.input_mint, edge.output_mint));
        // ... 排序和截断逻辑
    }
}
```

**说明**：
- 对多个金额（如 0.001 SOL、0.01 SOL、0.1 SOL）进行测试
- 保留每个金额下的最佳 pool
- 合并所有金额的 pool 结果

#### 1.2.4 价格排序

```rust
vec.sort_unstable_by(|x, y| y.1.partial_cmp(&x.1).unwrap());
```

**说明**：
- 按价格从高到低排序
- 价格越高，滑点越小，pool 越好

### 1.3 路径搜索算法：`best_price_paths_depth_search()`

```rust
fn best_price_paths_depth_search<F>(
    input_node: MintNodeIndex,
    amount: u64,
    max_path_length: usize,  // 默认 4 hops
    max_accounts: usize,     // 默认 40
    out_edges_per_node: &MintVec<Vec<EdgeWithNodes>>,
    best_paths_by_node_prealloc: &mut MintVec<Vec<(NotNan<f64>, Vec<EdgeWithNodes>)>>,
    best_by_node_prealloc: &mut Vec<BestVec3>,
    edge_price: F,
    hot_mints: &HashSet<MintNodeIndex>,
    avoid_cold_mints: bool,
    swap_mode: SwapMode,
) -> anyhow::Result<MintVec<Vec<(f64, Vec<EdgeWithNodes>)>>>
```

**算法特点**：
- 深度优先搜索（DFS）
- 最大路径长度：4 hops
- 最大账户数：40
- 保留每个节点的最佳 3 条路径
- 使用缓存机制（`PathDiscoveryCache`）

**避免冷 Mint 策略**：
```rust
// Stop depth search when encountering a cold mint
if avoid_cold_mints && hot_mints.len() > 0 && !hot_mints.contains(&out_edge.source_node) {
    stats[1] += 1;
    continue;
}
```

### 1.4 缓存机制

```rust
struct PathDiscoveryCache {
    cache: HashMap<(MintNodeIndex, MintNodeIndex, SwapMode), Vec<PathDiscoveryCacheEntry>>,
    last_expire_timestamp_millis: u64,
    max_age_millis: u64,
}

impl PathDiscoveryCache {
    fn insert(
        &mut self,
        from: MintNodeIndex,
        to: MintNodeIndex,
        swap_mode: SwapMode,
        in_amount: u64,
        max_accounts: usize,
        timestamp_millis: u64,
        mut edges: Vec<Vec<EdgeIndex>>,
    ) {
        // 使用二分查找插入，按金额排序
        let pos = entry
            .binary_search_by_key(&(in_amount, max_accounts_bucket), |x| {
                (x.in_amount.round() as u64, x.max_account)
            })
            .unwrap_or_else(|e| e);

        // 如果已存在，替换；否则插入
        if pos < entry.len()
            && entry[pos].max_account == max_accounts_bucket
            && entry[pos].in_amount.round() as u64 == in_amount
        {
            entry[pos] = new_elem;
        } else {
            entry.insert(pos, new_elem);
        }
    }
}
```

**缓存策略**：
- 键：`(from_mint, to_mint, swap_mode)`
- 值：按金额和账户数排序的路径列表
- 过期时间：可配置（默认 60 秒）
- 自动清理过期条目

---

## 2. Cobra Router（竞速模式）

**项目位置**：`/opt/projects/sol-trade-sdk/temp/Cobra-router/CobraRouter/CobraRouter/router/_main.py`

### 2.1 核心算法：`find_best_market_for_mint_race()`

```python
async def find_best_market_for_mint_race(
    self,
    mint: str,
    *,
    prefer_authority: bool = True,
    timeout: float | None = None,
    exclude_pools: list[str] = [],
    use_cache: bool = False
):
```

### 2.2 选择策略

#### 2.2.1 Authority Hint（快速路径）

```python
# 0. authority hint (fast, low RPC cost)
authority, info = await self.get_mint_authority(mint)
if authority == "INVALID":
    pass
elif authority is None and info is None:
    logging.error("find_best_market_for_mint_race: mint not found on-chain.")
    return (None, None)

if prefer_authority:
    if authority in SUPPORTED_DEXES.values():
        dex_name = ADDR_TO_DEX[authority]
        if dex_name == "PumpFun":
            return await self.check_route_pump(mint)
        elif dex_name == "Launchpad":
            return await self.check_route_launchpad(mint)
    elif "BLV" in mint:
        return await self.check_route_believe(mint)
```

**说明**：
- 通过 mint 的 authority 快速判断属于哪个 DEX
- 如果 authority 已知，直接返回对应的 DEX
- 适用于：PumpFun、Launchpad、Believe

#### 2.2.2 竞速模式（并发查询所有 DEX）

```python
# 1. task runners
async def run_pump():
    return await self.check_route_pump(mint)

async def run_launchpad():
    return await self.check_route_launchpad(mint)

async def run_pumpswap():
    ok, pool = await self.check_route_pumpswap(mint)
    return (SUPPORTED_DEXES["PumpSwap"], pool) if ok and pool else (None, None)

async def run_ray_cpmm():
    ok, pool = await self.check_ray_cpmm_for_mint(mint)
    return (SUPPORTED_DEXES["RayCPMM"], pool) if ok and pool else (None, None)

# ... 其他 DEX

runners = {
    "pump": run_pump,
    "launchpad": run_launchpad,
    "pumpswap": run_pumpswap,
    "ray_cpmm": run_ray_cpmm,
    "ray_v4": run_ray_v4,
    "ray_clmm": run_ray_clmm,
    "dbc": run_dbc,
    "damm_v2": run_damm_v2,
    "damm_v1": run_damm_v1,
    "dlmm": run_dlmm,
}

tasks = {name: asyncio.create_task(fn(), name=name) for name, fn in runners.items()}

# 返回第一个成功的结果
for fut in asyncio.as_completed(tasks.values(), timeout=timeout):
    dex_addr, pool = await fut
    if dex_addr is not None and pool is not None:
        # 取消其他任务
        for t in tasks.values():
            if t is not fut and not t.done():
                t.cancel()
        return (dex_addr, pool)
```

**说明**：
- 并发查询所有 DEX（11 个 DEX）
- 返回第一个成功的结果
- 取消其他未完成的任务
- 适用于：快速找到可用的 pool

#### 2.2.3 本地缓存

```python
if use_cache and str(mint) in self.local_cache:
    return self.local_cache[str(mint)]

# ... 查找 pool

if use_cache and mint not in self.local_cache:
    logging.info("Caching %s -> %s", mint, (dex_addr, pool))
    self.local_cache[str(mint)] = (dex_addr, pool)
```

**说明**：
- 缓存 mint → (dex, pool) 映射
- 避免重复查询
- 无过期时间（手动清理）

### 2.3 CPMM Pool 选择：`find_suitable_pool()`

**项目位置**：`/opt/projects/sol-trade-sdk/temp/Cobra-router/CobraRouter/CobraRouter/router/raydiumswap/cpmm/cpmm_core.py`

```python
async def find_suitable_pool(self, mint: str | Pubkey, pools: list[str], sol_amount: float = 0.000001) -> str:
    try:
        best_pool = None
        keys = None
        mint_pk = mint if isinstance(mint, Pubkey) else Pubkey.from_string(mint)
        for pool in pools:
            keys = await self.async_fetch_pool_keys(pool)
            if keys:
                reserve_a, reserve_b = await self.async_get_pool_reserves(keys)
                token_a_mint = keys.mint_a
                token_b_mint = keys.mint_b

                logging.info(f"Pool: {pool}")
                
                if reserve_a <= 0 or reserve_b <= 0:
                    logging.info("Skipping pool with zero reserves")
                    continue
                
                if token_a_mint == mint_pk and token_b_mint == WSOL_MINT:
                    token_reserve = reserve_a
                    sol_reserve = reserve_b
                    price_per_sol = token_reserve / sol_reserve if sol_reserve > 0 else 0
                    required_tokens = sol_amount * price_per_sol
                    logging.info(f"Price per SOL: {price_per_sol:,.2f} tokens, Required tokens for {sol_amount} SOL: {required_tokens:,.6f}")
                    
                elif token_b_mint == mint_pk and token_a_mint == WSOL_MINT:
                    sol_reserve = reserve_a
                    token_reserve = reserve_b
                    price_per_sol = token_reserve / sol_reserve if sol_reserve > 0 else 0
                    required_tokens = sol_amount * price_per_sol
                    logging.info(f"Price per SOL: {price_per_sol:,.2f} tokens, Required tokens for {sol_amount} SOL: {required_tokens:,.6f}")
                else:
                    logging.info(f"Pool doesn't contain SOL, skipping")
                    continue
                
                if (
                    (required_tokens > 0 and price_per_sol > 0.0)
                    and token_reserve > required_tokens
                    and sol_reserve > sol_amount
                ):
                    logging.info(f"✅ Pool has sufficient liquidity!")
                    best_pool = pool
                    break
                else:
                    logging.info(f"❌ Insufficient liquidity (need {required_tokens:,.6f} tokens, {sol_amount} SOL)")
            await asyncio.sleep(0.1)
        
        return (best_pool, keys)
    except Exception as e:
        logging.info(f"Error in pool scanning: {e}")
        traceback.print_exc()
        return None
```

**选择标准**：
1. 检查 pool 余额是否 > 0
2. 检查 pool 是否包含 WSOL
3. 计算价格和所需代币数量
4. 检查流动性是否充足（token_reserve > required_tokens && sol_reserve > sol_amount）
5. 返回第一个满足条件的 pool

---

## 3. Solana DEX V1 Router

**项目位置**：`/opt/projects/sol-trade-sdk/temp/solana-dex-v1-router`

**分析结果**：❌ 没有找到 pool 选择算法

---

## 4. 对比分析

| 特性 | Autobahn Router | Cobra Router | Solana DEX V1 Router |
|------|----------------|--------------|---------------------|
| **算法类型** | 价格影响 + 排序 | 竞速模式 | 无 |
| **主要指标** | Price Impact | 第一个可用的 pool | - |
| **Hot/Cold 区分** | ✅ 是 | ❌ 否 | - |
| **多金额测试** | ✅ 是 | ❌ 否 | - |
| **缓存机制** | ✅ PathDiscoveryCache | ✅ Local Cache | - |
| **并发查询** | ✅ 是 | ✅ 是 | - |
| **价格影响过滤** | ✅ 是 (25%) | ❌ 否 | - |
| **路径搜索** | ✅ DFS + 多路径 | ❌ 否 | - |
| **Authority Hint** | ❌ 否 | ✅ 是 | - |
| **超时控制** | ✅ 是 | ✅ 是 | - |

---

## 5. 建议的实现方案

基于这两个项目的经验，建议为 sol-trade-sdk 实现以下 pool 选择算法：

### 5.1 价格影响评分

```rust
pub fn compute_price_impact(
    pool: &PoolInfo,
    amount_in: u64,
) -> Option<f64> {
    // 模拟小额交易
    let price_small = simulate_swap(pool, 1000)?;
    
    // 模拟大额交易
    let price_large = simulate_swap(pool, amount_in)?;
    
    if price_small == 0.0 {
        return None;
    }
    
    // 计算价格影响
    Some((price_small - price_large) / price_small)
}
```

### 5.2 流动性评分

```rust
pub fn compute_liquidity_score(
    vault_a_balance: u64,
    vault_b_balance: u64,
    amount_in: u64,
) -> f64 {
    let min_balance = vault_a_balance.min(vault_b_balance);
    
    // 流动性充足度评分
    if amount_in * 10 > min_balance {
        0.3  // 流动性不足
    } else if amount_in * 2 > min_balance {
        0.7  // 流动性一般
    } else {
        1.0  // 流动性充足
    }
}
```

### 5.3 综合评分

```rust
pub fn score_pool(
    pool: &PoolInfo,
    amount_in: u64,
) -> f64 {
    let price_impact = compute_price_impact(pool, amount_in).unwrap_or(1.0);
    let liquidity_score = compute_liquidity_score(
        pool.vault_a_balance,
        pool.vault_b_balance,
        amount_in
    );
    
    // 加权评分
    0.6 * (1.0 - price_impact.min(0.25) / 0.25) +  // 价格影响 (60%)
    0.4 * liquidity_score                           // 流动性 (40%)
}
```

### 5.4 Pool 选择流程

```rust
pub async fn select_best_pool(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
    amount_in: u64,
) -> Result<(Pubkey, PoolInfo)> {
    // 1. 获取所有包含该 mint 的 pool
    let pools = list_pools_by_mint(rpc, mint).await?;
    
    // 2. 过滤掉余额为 0 的 pool
    let valid_pools: Vec<_> = pools
        .into_iter()
        .filter(|(_, pool)| {
            pool.vault_a_balance > 0 && pool.vault_b_balance > 0
        })
        .collect();
    
    // 3. 计算每个 pool 的评分
    let mut scored_pools: Vec<_> = valid_pools
        .into_iter()
        .map(|(addr, pool)| {
            let score = score_pool(&pool, amount_in);
            (addr, pool, score)
        })
        .collect();
    
    // 4. 按评分排序（从高到低）
    scored_pools.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    
    // 5. 返回评分最高的 pool
    let (addr, pool, score) = scored_pools
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No valid pool found"))?;
    
    log::info!("Selected pool {} with score {:.2}", addr, score);
    
    Ok((addr, pool))
}
```

### 5.5 缓存机制

```rust
use dashmap::DashMap;

pub struct PoolSelectionCache {
    cache: DashMap<(Pubkey, u64), (Pubkey, PoolInfo, f64)>,
    max_age: Duration,
}

impl PoolSelectionCache {
    pub fn new() -> Self {
        Self {
            cache: DashMap::with_capacity(10_000),
            max_age: Duration::from_secs(60),
        }
    }
    
    pub fn get(&self, mint: &Pubkey, amount: u64) -> Option<(Pubkey, PoolInfo)> {
        self.cache.get(&(mint, amount))
            .and_then(|entry| {
                let (addr, pool, score) = entry.value();
                Some((*addr, pool.clone()))
            })
    }
    
    pub fn set(&self, mint: &Pubkey, amount: u64, addr: Pubkey, pool: PoolInfo, score: f64) {
        self.cache.insert((mint, amount), (addr, pool, score));
    }
    
    pub fn cleanup_expired(&self) {
        let now = std::time::Instant::now();
        self.cache.retain(|_, (_, _, _)| true);  // TODO: 实现过期清理
    }
}
```

---

## 6. 最佳实践

### 6.1 Hot Mint 定义

```rust
pub fn get_hot_mints() -> HashSet<Pubkey> {
    vec![
        WSOL_MINT,
        USDC_MINT,
        USDT_MINT,
        // ... 其他热门代币
    ]
    .into_iter()
    .collect()
}
```

### 6.2 多金额测试

```rust
pub const PATH_WARMING_AMOUNTS: &[u64] = &[
    1_000_000,      // 0.001 SOL
    10_000_000,     // 0.01 SOL
    100_000_000,    // 0.1 SOL
    1_000_000_000,  // 1 SOL
];
```

### 6.3 价格影响阈值

```rust
pub const MAX_PRICE_IMPACT: f64 = 0.25;  // 25%
```

### 6.4 错误处理

```rust
match select_best_pool(&rpc, &mint, amount_in).await {
    Ok((pool_address, pool_info)) => {
        log::info!("Selected pool: {}", pool_address);
        // 继续交易逻辑
    }
    Err(e) => {
        log::error!("Failed to select pool: {}", e);
        // 返回错误或尝试其他协议
    }
}
```

---

## 7. 总结

### Autobahn Router 的优势

- ✅ 最完善的算法
- ✅ 价格影响评分
- ✅ Hot/Cold Mint 区分
- ✅ 多金额测试
- ✅ 路径搜索算法
- ✅ 缓存机制

### Cobra Router 的优势

- ✅ 竞速模式（快速）
- ✅ Authority Hint（快速路径）
- ✅ 简单易用
- ✅ 适用于快速找到可用 pool

### 建议的实现策略

1. **基础版本**：参考 Cobra Router 的竞速模式
2. **进阶版本**：参考 Autobahn Router 的价格影响评分
3. **高级版本**：结合两者的优势，实现智能选择

---

## 8. 相关文档

- [Pool 查询方法参考](./Pool查询方法.md)
- [Raydium CPMM Pool 查找技术分析](./raydium-cpmm-pool-lookup.md)
- [Raydium API 功能分析](./raydium-api-analysis.md)

---

**创建日期**：2026-01-09  
**最后更新**：2026-01-09  
**作者**：iFlow CLI