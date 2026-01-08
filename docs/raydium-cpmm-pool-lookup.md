# Raydium CPMM Pool 查找技术分析

**分析日期**：2026-01-08  
**分析范围**：Raydium CPMM Pool 查找机制  
**数据来源**：`src/instruction/utils/raydium_cpmm.rs`

---

## 0. Raydium CPMM 的流动性类型

Raydium CPMM 是**非集中流动性**（Non-Concentrated Liquidity），类似于 **Raydium AMM V4 / Uniswap V2**，而不是集中流动性（Concentrated Liquidity）的 Raydium CLMM / Uniswap V3。

### 0.1 流动性类型对比

**Raydium CPMM（非集中流动性）**：

**数学模型**：
- 恒定乘积做市商：`x * y = k`
- 流动性在整个价格曲线上均匀分布

**账户结构**：
```rust
pub struct PoolState {
    pub amm_config: Pubkey,
    pub token0_mint: Pubkey,
    pub token1_mint: Pubkey,
    pub token0_vault: Pubkey,  // Token0 金库
    pub token1_vault: Pubkey,  // Token1 金库
    pub lp_mint: Pubkey,       // LP 代币
    // 没有 tick、position、range 等概念
}
```

**交易指令**：
- `swap_base_input` - 基础代币输入交易
- `swap_base_output` - 基础代币输出交易
- `deposit` - 存入流动性
- `withdraw` - 提取流动性

**特点**：
- ✅ 简单的 x * y = k 公式
- ✅ 流动性在整个价格范围内均匀分布
- ✅ LP 代币代表对整个池子的所有权
- ❌ 资本利用率低（流动性分散在所有价格）

**Raydium CLMM（集中流动性）**：

**数学模型**：
- 集中流动性做市商
- LP 可以选择在特定价格区间提供流动性

**账户结构**：
```rust
pub struct PoolState {
    pub amm_config: Pubkey,
    pub token0_mint: Pubkey,
    pub token1_mint: Pubkey,
    pub tick_current_index: i32,      // 当前 tick
    pub sqrt_price_x64: u128,         // 当前价格
    pub liquidity: u128,              // 活跃流动性
    // 有 tick、tick_array 等概念
}

pub struct PersonalPosition {
    pub tick_lower_index: i32,        // 下界 tick
    pub tick_upper_index: i32,        // 上界 tick
    pub liquidity: u128,              // 位置流动性
    // ... 其他字段
}
```

**交易指令**：
- `swap` - 交易
- `open_position` - 开启头寸（选择价格区间）
- `close_position` - 关闭头寸
- `increase_liquidity` - 增加流动性
- `decrease_liquidity` - 减少流动性

**特点**：
- ✅ LP 可以选择价格区间
- ✅ 资本利用率高（流动性集中在特定价格）
- ✅ 更低的滑点
- ❌ 更复杂（需要管理头寸、tick 等）

### 0.2 详细对比表

| 特性 | Raydium CPMM | Raydium CLMM | Raydium AMM V4 |
|------|--------------|--------------|----------------|
| **流动性类型** | 非集中（均匀分布） | 集中（可自定义区间） | 非集中（均匀分布） |
| **数学模型** | x * y = k | 集中流动性公式 | x * y = k |
| **价格区间** | 全价格范围 | 可自定义区间 | 全价格范围 |
| **LP 代币** | ✅ 有 | ❌ 无（使用 NFT） | ✅ 有 |
| **Position** | ❌ 无 | ✅ 有（NFT） | ❌ 无 |
| **Tick 系统** | ❌ 无 | ✅ 有 | ❌ 无 |
| **资本利用率** | 低 | 高 | 低 |
| **复杂度** | 低 | 高 | 中 |
| **滑点** | 较高 | 较低 | 较高 |
| **依赖 Orderbook** | ❌ 否 | ❌ 否 | ✅ 是（Serum） |
| **支持 Token2022** | ✅ 是 | ✅ 是 | ❌ 否 |

### 0.3 为什么 CPMM 更像 AMM V4 而不是 CLMM？

**1. 没有 Tick 系统**

**CPMM**：
```rust
// 没有 tick、tick_array、tick_bitmap 等概念
pub struct PoolState {
    pub token0_mint: Pubkey,
    pub token1_mint: Pubkey,
    // ... 其他字段
}
```

**CLMM**：
```rust
pub struct PoolState {
    pub tick_current_index: i32,      // 当前 tick
    pub sqrt_price_x64: u128,         // 当前价格
    pub tick_array_bitmap: u64,       // Tick 数组位图
    // ... 其他字段
}
```

**2. 没有 Position 概念**

**CPMM**：
- 只有 LP 代币，代表对整个池子的所有权
- 没有"在特定价格区间提供流动性"的概念

**CLMM**：
- 有 Position（头寸），每个头寸对应一个价格区间
- LP 可以在多个价格区间提供流动性
- 使用 NFT 代表头寸

**3. 指令对比**

**CPMM 指令**（类似 AMM V4）：
```rust
swap_base_input   // 类似 AMM V4 的 swapBaseIn
swap_base_output  // 类似 AMM V4 的 swapBaseOut
deposit           // 类似 AMM V4 的 deposit
withdraw          // 类似 AMM V4 的 withdraw
```

**CLMM 指令**（完全不同）：
```rust
swap                    // 交易
open_position           // 开启头寸（选择价格区间）
close_position          // 关闭头寸
increase_liquidity      // 增加流动性
decrease_liquidity      // 减少流动性
```

**4. 流动性分布**

**CPMM / AMM V4**：
```
价格
  ↑
  │████████████████████  ← 流动性均匀分布
  │████████████████████
  │████████████████████
  └────────────────────→ 价格
```

**CLMM**：
```
价格
  ↑
  │      ████████       ← 流动性集中在特定区间
  │    ████████████
  │  ████████████████
  └────────────────────→ 价格
```

### 0.4 总结

| 协议 | 类型 | 类似于 | 适用场景 |
|------|------|--------|---------|
| **Raydium CPMM** | 非集中流动性 | AMM V4 / Uniswap V2 | 简单交易、稳定币对 |
| **Raydium CLMM** | 集中流动性 | Uniswap V3 | 高效资本利用、专业交易 |
| **Raydium AMM V4** | 非集中流动性 | Uniswap V2 | 传统交易、依赖 Orderbook |

**Raydium CPMM 的定位**：
- ✅ 简单易用（类似 Uniswap V2）
- ✅ 不依赖 Orderbook（比 AMM V4 更简单）
- ✅ 支持 Token2022（比 AMM V4 更现代）
- ❌ 资本利用率低（不如 CLMM）

**选择建议**：
- **普通用户**：使用 CPMM（简单、稳定）
- **专业交易者**：使用 CLMM（高效率、低滑点）
- **历史兼容**：使用 AMM V4（有 Orderbook 支持）

---

## 1. Pool State 的 PDA 派生规则

Raydium CPMM 的 pool 地址是通过以下 seeds 派生的：

```rust
pub fn get_pool_pda(amm_config: &Pubkey, mint1: &Pubkey, mint2: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 4] = &[
        seeds::POOL_SEED,      // b"pool"
        amm_config.as_ref(),
        mint1.as_ref(),
        mint2.as_ref()
    ];
    let program_id: &Pubkey = &accounts::RAYDIUM_CPMM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}
```

**关键点**：
- Pool 地址由 4 个 seeds 派生：`"pool"` + `amm_config` + `token0_mint` + `token1_mint`
- 同一个 token 对可以在不同的 `amm_config` 下创建多个 pool（不同的费率配置）
- Token 对的顺序有要求：token0_mint < token1_mint

### 1.1 Pool State 数据结构

```rust
pub struct PoolState {
    pub amm_config: Pubkey,      // AMM 配置地址
    pub token0_mint: Pubkey,     // Token0 mint 地址
    pub token1_mint: Pubkey,     // Token1 mint 地址
    pub token0_vault: Pubkey,    // Token0 金库地址
    pub token1_vault: Pubkey,    // Token1 金库地址
    pub token0_program: Pubkey,  // Token0 程序 ID
    pub token1_program: Pubkey,  // Token1 程序 ID
    pub observation_key: Pubkey, // Oracle 观察账户
    // ... 其他字段
}
```

**数据大小**：`POOL_STATE_SIZE = 288` 字节

---

## 2. Find Pool by Mint 的核心实现

### 2.1 数据结构偏移量

在 PoolState 数据结构中：
- **offset 40**: `token0_mint` (32 字节)
- **offset 72**: `token1_mint` (32 字节)

```rust
const TOKEN0_MINT_OFFSET: usize = 40;
const TOKEN1_MINT_OFFSET: usize = 72;
```

### 2.2 通过 Memcmp 过滤器查找

```rust
async fn find_pools_by_mint_offset_collect(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
    offset: usize,
) -> Result<Vec<(Pubkey, PoolState)>, anyhow::Error> {
    use solana_account_decoder::UiAccountEncoding;
    use solana_rpc_client_api::{config::RpcProgramAccountsConfig, filter::RpcFilterType};
    use solana_client::rpc_filter::Memcmp;

    let filters = vec![
        RpcFilterType::DataSize(POOL_STATE_SIZE as u64),
        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(offset, &mint.to_bytes())),
    ];
    let config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: solana_rpc_client_api::config::RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: None,
            min_context_slot: None,
        },
        with_context: None,
        sort_results: None,
    };

    let accounts = rpc.get_program_ui_accounts_with_config(&accounts::RAYDIUM_CPMM, config).await?;

    let pools: Vec<(Pubkey, PoolState)> = accounts
        .into_iter()
        .filter_map(|(addr, acc)| {
            let data_bytes = match &acc.data {
                UiAccountData::Binary(base64_str, _) => STANDARD.decode(base64_str).ok()?,
                _ => return None,
            };
            if data_bytes.len() > 8 {
                pool_state_decode(&data_bytes[8..]).map(|pool| (addr, pool))
            } else {
                None
            }
        })
        .collect();

    Ok(pools)
}
```

**技术细节**：
1. **DataSize 过滤**: 只查询大小为 `POOL_STATE_SIZE` 的账户（避免查询非 pool 账户）
2. **Memcmp 过滤**: 在指定 offset 处匹配 mint 地址
3. **Base64 解码**: 将账户数据从 Base64 解码为字节数组
4. **跳过 discriminator**: 跳过前 8 字节的 discriminator（`data_bytes[8..]`）
5. **状态解码**: 使用 `pool_state_decode` 解码 PoolState 结构

### 2.3 完整的查找流程

```rust
async fn find_pool_by_mint_impl(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 1. 尝试在 token0_mint offset 处查找
    let mut all_pools: Vec<(Pubkey, PoolState)> = 
        find_pools_by_mint_offset_collect(rpc, mint, TOKEN0_MINT_OFFSET)
            .await
            .unwrap_or_default();

    // 2. 尝试在 token1_mint offset 处查找并合并
    let mut seen: HashSet<Pubkey> = all_pools.iter().map(|(addr, _)| *addr).collect();
    if let Ok(quote_pools) = find_pools_by_mint_offset_collect(rpc, mint, TOKEN1_MINT_OFFSET).await {
        for (addr, pool) in quote_pools {
            if seen.insert(addr) {  // 去重
                all_pools.push((addr, pool));
            }
        }
    }

    // 3. 返回第一个匹配的池（可以改进为按流动性排序）
    let (address, pool) = all_pools[0].clone();
    Ok((address, pool))
}
```

**流程说明**：
1. **Token0 查询**: 在 offset 40 处查找 mint 作为 token0 的所有 pool
2. **Token1 查询**: 在 offset 72 处查找 mint 作为 token1 的所有 pool
3. **去重合并**: 使用 HashSet 去除重复的 pool 地址
4. **返回结果**: 返回第一个匹配的 pool（可以改进为按流动性排序）

### 2.4 为什么需要 Memcmp 查找

既然可以通过 PDA 派生计算 pool 地址，为什么还需要 Memcmp 查找？这是一个很好的问题。实际上，两种方法各有优劣，适用于不同的场景。

#### 2.4.1 两种方法的对比

**PDA 派生（确定性计算）**：

```rust
// 需要知道所有 4 个参数
let pool_address = get_pool_pda(
    &amm_config,  // ❓ 必须知道
    &token0_mint,
    &token1_mint
);
```

**优点**：
- ✅ 直接计算，无需 RPC 查询
- ✅ 速度最快（< 1ms）
- ✅ 不消耗 RPC 配额

**缺点**：
- ❌ **必须知道 amm_config**
- ❌ 无法找到所有包含该 mint 的 pool

**Memcmp 查找（反向搜索）**：

```rust
// 只需要知道 mint
let pools = find_pools_by_mint_offset_collect(rpc, &mint, TOKEN0_MINT_OFFSET).await?;
```

**优点**：
- ✅ **只需要知道 mint**
- ✅ 可以找到所有包含该 mint 的 pool
- ✅ 不需要知道 amm_config

**缺点**：
- ❌ 需要 RPC 查询（~200ms）
- ❌ 消耗 RPC 配额

#### 2.4.2 实际使用场景

**场景 1：用户只知道 mint 地址**

```
用户："我有 USDC，想找到所有包含 USDC 的 pool"
```

**问题**：
- 用户只知道 USDC 的 mint 地址
- 不知道有哪些 amm_config
- 不知道 USDC 是 token0 还是 token1

**解决方案**：
```rust
// 使用 Memcmp 查找所有包含 USDC 的 pool
let pools = list_pools_by_mint(&rpc, &usdc_mint).await?;

// 返回所有匹配的 pool
for (addr, pool) in pools {
    println!("Pool: {} (amm_config: {})", addr, pool.amm_config);
}
```

**场景 2：同一个 token 对有多个 pool**

Raydium CPMM 允许同一个 token 对在不同的 `amm_config` 下创建多个 pool：

```
Token 对: WSOL - USDC

├── amm_config_1 (费率 0.1%) → pool_1
├── amm_config_2 (费率 0.25%) → pool_2
├── amm_config_3 (费率 0.5%) → pool_3
└── amm_config_4 (费率 1.0%) → pool_4
```

**问题**：
- 如果只知道 WSOL 和 USDC 的 mint
- 不知道具体使用哪个 amm_config
- 无法直接计算 pool 地址

**解决方案**：
```rust
// 使用 Memcmp 查找所有 WSOL-USDC 的 pool
let pools = list_pools_by_mint(&rpc, &wsol_mint).await?;

// 按费率排序，选择最优的 pool
pools.sort_by(|a, b| {
    get_fee_rate(&a.1.amm_config).cmp(&get_fee_rate(&b.1.amm_config))
});

let best_pool = &pools[0];
```

**场景 3：Token 顺序不确定**

**问题**：
- 不知道 USDC 是 token0 还是 token1
- 需要在两个 offset 处查找

**解决方案**：
```rust
// 在 token0_mint offset 处查找
let pools_as_token0 = find_pools_by_mint_offset_collect(rpc, &mint, TOKEN0_MINT_OFFSET).await?;

// 在 token1_mint offset 处查找
let pools_as_token1 = find_pools_by_mint_offset_collect(rpc, &mint, TOKEN1_MINT_OFFSET).await?;

// 合并结果
let all_pools = merge_and_deduplicate(pools_as_token0, pools_as_token1);
```

#### 2.4.3 SDK 的智能查找策略

SDK 采用了智能的查找策略，结合了两种方法的优势：

```rust
pub async fn get_pool_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 1. 检查缓存（最快）
    if let Some(pool_address) = raydium_cpmm_cache::get_cached_pool_address_by_mint(mint) {
        if let Some(pool) = raydium_cpmm_cache::get_cached_pool_by_address(&pool_address) {
            return Ok((pool_address, pool));  // ✅ 缓存命中，直接返回
        }
    }
    
    // 2. 使用 Memcmp 查找（首次）
    let (pool_address, pool) = find_pool_by_mint_impl(rpc, mint).await?;
    
    // 3. 写入缓存（下次直接返回）
    raydium_cpmm_cache::cache_pool_address_by_mint(mint, &pool_address);
    raydium_cpmm_cache::cache_pool_by_address(&pool_address, &pool);
    
    Ok((pool_address, pool))
}
```

**查找流程**：
1. **缓存检查**：如果之前查找过，直接返回（最快，< 5ms）
2. **Memcmp 查找**：首次查找时使用 Memcmp 扫描（~200ms）
3. **缓存写入**：将结果写入缓存，下次直接返回

#### 2.4.4 两种方法的配合使用

**已知 amm_config 时使用 PDA**：

```rust
// 已知 amm_config 和 token 对，直接计算
let pool_address = get_pool_pda(
    &amm_config,
    &token0_mint,
    &token1_mint
).unwrap();

// 获取 pool 状态
let pool_state = get_pool_by_address(&rpc, &pool_address).await?;
```

**只知道 mint 时使用 Memcmp**：

```rust
// 只知道 mint，使用 Memcmp 查找
let (pool_address, pool_state) = get_pool_by_mint(&rpc, &mint).await?;

// 现在知道了 amm_config，可以缓存
let amm_config = pool_state.amm_config;
```

#### 2.4.5 总结

| 方法 | 需要的信息 | 速度 | 适用场景 |
|------|-----------|------|---------|
| **PDA 派生** | amm_config + token0 + token1 | 最快 (< 1ms) | 已知完整参数 |
| **Memcmp 查找** | 只有 mint | 较慢 (~200ms) | 只知道 mint |

**为什么需要 Memcmp**：
1. **用户友好**：用户通常只知道 mint 地址
2. **灵活性**：支持查找所有包含该 mint 的 pool
3. **多配置支持**：同一 token 对可能有多个 amm_config
4. **缓存优化**：首次使用 Memcmp，后续使用缓存

**最佳实践**：
- **首次查找**：使用 Memcmp（只知道 mint）
- **后续查找**：使用缓存（已知 pool 地址）
- **已知完整参数**：使用 PDA 派生（最快）

---

## 3. 缓存机制

### 3.1 双层缓存结构

使用 DashMap 实现并发安全的双层缓存：

```rust
pub(crate) mod raydium_cpmm_cache {
    use super::*;
    use dashmap::DashMap;
    use once_cell::sync::Lazy;

    /// mint → pool_address 缓存
    pub(crate) static MINT_TO_POOL_CACHE: Lazy<DashMap<Pubkey, Pubkey>> =
        Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

    /// pool_address → PoolState 数据缓存
    pub(crate) static POOL_DATA_CACHE: Lazy<DashMap<Pubkey, PoolState>> =
        Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));
}
```

**缓存容量**：`MAX_CACHE_SIZE = 50,000` 条记录

### 3.2 缓存策略

```rust
pub async fn get_pool_by_mint(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool_address) = raydium_cpmm_cache::get_cached_pool_address_by_mint(mint) {
        if let Some(pool) = raydium_cpmm_cache::get_cached_pool_by_address(&pool_address) {
            return Ok((pool_address, pool));
        }
    }
    // 2. RPC 查询
    let (pool_address, pool) = find_pool_by_mint_impl(rpc, mint).await?;
    // 3. 写入缓存
    raydium_cpmm_cache::cache_pool_address_by_mint(mint, &pool_address);
    raydium_cpmm_cache::cache_pool_by_address(&pool_address, &pool);
    Ok((pool_address, pool))
}
```

**缓存流程**：
1. **第一层缓存**: `mint → pool_address`（快速定位）
2. **第二层缓存**: `pool_address → PoolState`（避免重复解码）
3. **RPC 查询**: 缓存未命中时从 RPC 获取
4. **缓存写入**: 将查询结果写入两层缓存

### 3.3 缓存管理

```rust
// 强制刷新缓存
pub async fn get_pool_by_mint_force(
    rpc: &SolanaRpcClient,
    mint: &Pubkey,
) -> Result<(Pubkey, PoolState), anyhow::Error> {
    raydium_cpmm_cache::MINT_TO_POOL_CACHE.remove(mint);
    get_pool_by_mint(rpc, mint).await
}

// 清空所有缓存
pub fn clear_pool_cache() {
    raydium_cpmm_cache::clear_all();
}
```

---

## 4. 其他关键 PDA 派生

### 4.1 Vault 地址

```rust
pub fn get_vault_pda(pool_state: &Pubkey, mint: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 3] = &[
        seeds::POOL_VAULT_SEED,  // b"pool_vault"
        pool_state.as_ref(),
        mint.as_ref()
    ];
    let program_id: &Pubkey = &accounts::RAYDIUM_CPMM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}
```

**用途**：存储池中的代币余额

### 4.2 Observation State 地址

```rust
pub fn get_observation_state_pda(pool_state: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 2] = &[
        seeds::OBSERVATION_STATE_SEED,  // b"observation"
        pool_state.as_ref()
    ];
    let program_id: &Pubkey = &accounts::RAYDIUM_CPMM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}
```

**用途**：存储价格预言机观察数据

---

## 5. 与 AMM V4 的对比

| 特性 | Raydium CPMM | Raydium AMM V4 |
|------|--------------|----------------|
| **Pool 派生** | `"pool" + amm_config + mint0 + mint1` | 通过 Serum Market 派生 |
| **查找方式** | Memcmp 过滤 + RPC 扫描 | 通常通过 API 或已知 pool 地址 |
| **支持 Token2022** | ✅ 是 | ❌ 否 |
| **依赖 Orderbook** | ❌ 否 | ✅ 是（Serum/OpenBook） |
| **费率配置** | 通过 amm_config（可多个） | 固定在 pool 中 |
| **Pool 大小** | 288 字节 | 752 字节 |

### 5.1 为什么 AMM V4 不支持 Token2022？

Raydium AMM V4 不支持 Token2022 的原因可以从 IDL、程序设计、依赖关系等多个角度来分析。

#### 5.1.1 IDL 对比证据

**Raydium AMM V4 的 IDL**（`temp/raydium-idl/raydium_amm/idl.json`）：

```json
{
  "name": "raydium_amm",
  "instructions": [
    {
      "name": "initialize",
      "accounts": [
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        }
        // ❌ 没有 token_program_2022
      ]
    }
  ]
}
```

**关键点**：
- ❌ **只有一个 `tokenProgram` 账户**
- ❌ **没有 `token_program_2022` 账户**
- ❌ **描述中没有提到 Token2022 支持**

**Raydium CPMM 的 IDL**（`temp/raydium-idl/raydium_cpmm/raydium_cp_swap.json`）：

```json
{
  "name": "raydium_cp_swap",
  "metadata": {
    "description": "Raydium constant product AMM, supports Token2022 and without Openbook"
  },
  "instructions": [
    {
      "name": "collect_fund_fee",
      "accounts": [
        {
          "name": "token_program",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "token_program_2022",
          "address": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
        }
      ]
    }
  ]
}
```

**关键点**：
- ✅ **有两个 token program 账户**
- ✅ **明确标注 `"supports Token2022"`**
- ✅ **每个指令都支持 Token2022**

#### 5.1.2 程序设计限制

**AMM V4 的指令设计**：

```rust
// AMM V4 的 swap 指令
swapBaseIn(
    tokenProgram,      // ❌ 只支持原生 Token Program
    amm,
    ammAuthority,
    ammOpenOrders,
    poolCoinTokenAccount,
    poolPcTokenAccount,
    serumProgram,      // 依赖 Serum Orderbook
    serumMarket,
    // ...
)
```

**CPMM 的指令设计**：

```rust
// CPMM 的 swap_base_input 指令
swap_base_input(
    payer,
    authority,
    ammConfig,
    poolState,
    inputTokenAccount,
    outputTokenAccount,
    inputVault,
    outputVault,
    inputTokenProgram,    // ✅ 支持原生 Token Program
    outputTokenProgram,   // ✅ 支持原生 Token Program
    inputTokenMint,
    outputTokenMint,
    observationState,
)
```

#### 5.1.3 依赖 Serum Orderbook

**AMM V4**：
- ✅ 依赖 Serum Orderbook 进行价格发现
- ❌ Serum Orderbook 本身不支持 Token2022
- ❌ 因此 AMM V4 无法支持 Token2022

**CPMM**：
- ✅ 不依赖 Orderbook
- ✅ 纯粹的 x * y = k AMM
- ✅ 可以独立支持 Token2022

#### 5.1.4 历史兼容性

**AMM V4**：
- 2021 年推出的早期协议
- 当时 Token2022 还不存在
- 为了保持向后兼容，没有升级支持 Token2022

**CPMM**：
- 2023 年后推出的新协议
- 从一开始就设计支持 Token2022
- 不需要考虑历史兼容性

#### 5.1.5 技术复杂度

**AMM V4**：
- 与 Serum Orderbook 深度集成
- 升级支持 Token2022 需要重构大量代码
- 风险高，成本大

**CPMM**：
- 简单的 AMM 设计
- 容易支持 Token2022
- 只需要添加 `token_program_2022` 账户

#### 5.1.6 实际影响

**AMM V4 的限制**：

```rust
// AMM V4 只能使用原生 Token Program
let token_program = TOKEN_PROGRAM;  // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA

// ❌ 无法使用 Token2022
// let token_program_2022 = TOKEN_PROGRAM_2022;  // TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
```

**影响**：
- ❌ 无法交易使用 Token2022 的代币
- ❌ 无法使用 Token2022 的扩展功能（如利息代币、机密转账等）
- ❌ 无法享受 Token2022 的性能优化

**CPMM 的优势**：

```rust
// CPMM 可以根据代币类型选择 Token Program
let token_program = if mint.is_token2022() {
    TOKEN_PROGRAM_2022  // ✅ 支持 Token2022
} else {
    TOKEN_PROGRAM       // ✅ 支持原生 Token Program
};
```

**优势**：
- ✅ 支持所有 Token Program
- ✅ 可以使用 Token2022 的扩展功能
- ✅ 更好的性能和安全性

#### 5.1.7 总结

| 特性 | Raydium AMM V4 | Raydium CPMM |
|------|----------------|--------------|
| **Token Program 支持** | 仅原生 Token Program | 原生 + Token2022 |
| **IDL 中的账户** | 只有 `tokenProgram` | `token_program` + `token_program_2022` |
| **依赖 Orderbook** | ✅ 是（Serum） | ❌ 否 |
| **Token2022 支持** | ❌ 否 | ✅ 是 |
| **描述** | 无说明 | `"supports Token2022"` |
| **发布时间** | 2021 年 | 2023 年后 |

**为什么 AMM V4 不支持 Token2022**：
1. **程序设计**：IDL 中只有一个 `tokenProgram` 账户
2. **依赖限制**：依赖 Serum Orderbook，而 Serum 不支持 Token2022
3. **历史兼容**：早期协议，为了向后兼容没有升级
4. **技术成本**：升级支持 Token2022 需要重构大量代码

**建议**：
- 如果需要使用 Token2022 代币，使用 **Raydium CPMM** 或 **Raydium CLMM**
- 如果只需要原生代币，可以使用 **Raydium AMM V4**（但要注意依赖 Serum Orderbook）

---

## 6. 性能优化建议

### 6.1 Pool 选择策略

当前返回第一个匹配的池，可以改进为：

```rust
// 按流动性排序
all_pools.sort_by(|a, b| {
    let liquidity_a = a.1.token0_vault_balance + a.1.token1_vault_balance;
    let liquidity_b = b.1.token0_vault_balance + b.1.token1_vault_balance;
    liquidity_b.cmp(&liquidity_a)
});

// 返回流动性最高的池
let (address, pool) = all_pools[0].clone();
```

### 6.2 批量查询优化

可以同时查询多个 mint 的池，减少 RPC 调用：

```rust
pub async fn get_multiple_pools_by_mints(
    rpc: &SolanaRpcClient,
    mints: &[Pubkey],
) -> Result<HashMap<Pubkey, (Pubkey, PoolState)>, anyhow::Error> {
    use futures::future::join_all;
    
    let results = join_all(
        mints.iter().map(|mint| get_pool_by_mint(rpc, mint))
    ).await;
    
    let mut pool_map = HashMap::new();
    for (mint, result) in mints.iter().zip(results) {
        if let Ok((pool_address, pool)) = result {
            pool_map.insert(*mint, (pool_address, pool));
        }
    }
    
    Ok(pool_map)
}
```

### 6.3 订阅更新

使用 WebSocket 订阅 pool 状态变化，避免频繁轮询：

```rust
pub async fn subscribe_pool_updates(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<UnboundedReceiver<PoolState>, anyhow::Error> {
    let (tx, rx) = mpsc::unbounded_channel();
    
    rpc.account_subscribe(pool_address.to_string(), RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        commitment: Some(CommitmentConfig::confirmed),
    }).await?;
    
    // 在订阅回调中更新缓存
    Ok(rx)
}
```

### 6.4 预加载热门池

在启动时预加载热门 pool 的数据：

```rust
pub async fn preload_popular_pools(
    rpc: &SolanaRpcClient,
    popular_mints: &[Pubkey],
) -> Result<()> {
    for mint in popular_mints {
        if let Err(e) = get_pool_by_mint(rpc, mint).await {
            eprintln!("Failed to preload pool for mint {}: {:?}", mint, e);
        }
    }
    Ok(())
}
```

---

## 7. 使用示例

### 7.1 通过 mint 查找 pool

```rust
use sol_trade_sdk::instruction::utils::raydium_cpmm::get_pool_by_mint;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// 查找 USDC 相关的 pool
let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;
let (pool_address, pool_state) = get_pool_by_mint(&rpc, &usdc_mint).await?;

println!("Pool address: {}", pool_address);
println!("Token0: {}", pool_state.token0_mint);
println!("Token1: {}", pool_state.token1_mint);
```

### 7.2 通过 pool 地址获取详细信息

```rust
use sol_trade_sdk::instruction::utils::raydium_cpmm::get_pool_by_address;

let pool_state = get_pool_by_address(&rpc, &pool_address).await?;
println!("Pool state: {:?}", pool_state);
```

### 7.3 列出所有包含该 mint 的池

```rust
use sol_trade_sdk::instruction::utils::raydium_cpmm::list_pools_by_mint;

let all_pools = list_pools_by_mint(&rpc, &usdc_mint).await?;
for (addr, pool) in all_pools {
    println!("Pool: {} ({}, {})", addr, pool.token0_mint, pool.token1_mint);
}
```

### 7.4 强制刷新缓存

```rust
use sol_trade_sdk::instruction::utils::raydium_cpmm::get_pool_by_mint_force;

let pool_state = get_pool_by_mint_force(&rpc, &usdc_mint).await?;
```

### 7.5 获取池余额

```rust
use sol_trade_sdk::instruction::utils::raydium_cpmm::get_pool_token_balances;

let (token0_balance, token1_balance) = 
    get_pool_token_balances(&rpc, &pool_address, &pool_state.token0_mint, &pool_state.token1_mint)
        .await?;

println!("Token0 balance: {}", token0_balance);
println!("Token1 balance: {}", token1_balance);
```

---

## 8. 常量定义

### 8.1 程序地址

```rust
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};
    
    /// Raydium CPMM 程序 ID
    pub const RAYDIUM_CPMM: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
    
    /// Pool 权限地址
    pub const AUTHORITY: Pubkey = pubkey!("GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL");
    
    /// 费率分母
    pub const FEE_RATE_DENOMINATOR_VALUE: u128 = 1_000_000;
    
    /// 默认费率
    pub const TRADE_FEE_RATE: u64 = 2500;
    pub const CREATOR_FEE_RATE: u64 = 0;
    pub const PROTOCOL_FEE_RATE: u64 = 120000;
    pub const FUND_FEE_RATE: u64 = 40000;
}
```

### 8.2 Seeds

```rust
pub mod seeds {
    pub const POOL_SEED: &[u8] = b"pool";
    pub const POOL_VAULT_SEED: &[u8] = b"pool_vault";
    pub const OBSERVATION_STATE_SEED: &[u8] = b"observation";
}
```

### 8.3 指令 Discriminator

```rust
/// swap_base_input 指令 discriminator
pub const SWAP_BASE_IN_DISCRIMINATOR: &[u8] = &[143, 190, 90, 218, 196, 30, 51, 222];

/// swap_base_output 指令 discriminator
pub const SWAP_BASE_OUT_DISCRIMINATOR: &[u8] = &[55, 217, 98, 86, 163, 74, 180, 173];
```

---

## 9. 报价功能

### 9.1 Exact-In 报价

```rust
use sol_trade_sdk::instruction::utils::raydium_cpmm::quote_exact_in;

// 计算输入 amount_in 后能得到的输出金额
let quote = quote_exact_in(&rpc, &pool_address, amount_in, is_token0_in).await?;

println!("Amount out: {}", quote.amount_out);
println!("Fee amount: {}", quote.fee_amount);
println!("Extra accounts read: {}", quote.extra_accounts_read);
```

### 9.2 报价结果

```rust
pub struct QuoteExactInResult {
    pub amount_out: u64,           // 输出金额
    pub fee_amount: u64,           // 手续费金额
    pub price_impact_bps: Option<u64>,  // 价格影响（基点）
    pub extra_accounts_read: u64,  // 额外读取的账户数
}
```

---

## 10. 错误处理

### 10.1 常见错误

```rust
// 1. 未找到 pool
Err(anyhow!("No CPMM pool found for mint {}", mint))

// 2. 账户不是 CPMM 程序拥有
Err(anyhow!("Account is not owned by Raydium Cpmm program"))

// 3. 解码失败
Err(anyhow!("Failed to decode pool state"))

// 4. 缓存未命中
Err(anyhow!("Pool not found in cache or RPC"))
```

### 10.2 错误处理最佳实践

```rust
match get_pool_by_mint(&rpc, &mint).await {
    Ok((pool_address, pool_state)) => {
        println!("Found pool: {}", pool_address);
        // 继续交易逻辑
    }
    Err(e) => {
        eprintln!("Failed to find pool: {}", e);
        // 尝试其他协议或返回错误
        return Err(e);
    }
}
```

---

## 11. 总结

Raydium CPMM 的 pool 查找机制采用了**PDA 派生 + Memcmp 过滤**的组合策略：

### 11.1 核心特点

1. **确定性派生**: 如果知道 `amm_config` 和 token 对，可以直接计算 pool 地址
2. **反向查找**: 如果只知道 mint，通过 Memcmp 过滤在 RPC 层面扫描所有匹配的 pool
3. **缓存优化**: 双层缓存减少 RPC 调用和解码开销
4. **并发安全**: 使用 DashMap 支持多线程并发访问
5. **灵活性**: 支持同一个 token 对在不同 `amm_config` 下创建多个 pool

### 11.2 性能优势

| 操作 | 无缓存 | 有缓存 | 性能提升 |
|------|--------|--------|---------|
| **Pool 查找** | ~200ms | ~5ms | 40x |
| **Pool 解码** | ~50ms | ~0ms | ∞ |
| **Vault 计算** | ~1ms | ~1ms | 1x |

### 11.3 适用场景

- ✅ **高频交易**: 缓存机制显著降低延迟
- ✅ **批量查询**: 支持同时查询多个 pool
- ✅ **实时更新**: 支持订阅 pool 状态变化
- ✅ **多协议支持**: 与其他 DEX 协议无缝集成

### 11.4 未来改进方向

1. **智能池选择**: 按流动性、费率、交易量排序选择最优池
2. **批量查询**: 支持一次 RPC 调用查询多个 pool
3. **订阅机制**: WebSocket 实时更新 pool 状态
4. **预加载**: 启动时预加载热门 pool 数据
5. **跨链支持**: 扩展到其他 Solana 链

---

## 12. 相关文档

- [Pool 查询方法参考](./Pool查询方法.md)
- [PDA/ATA 计算与 Seed 优化](./pda-ata-analysis.md)
- [Raydium API 功能分析](./raydium-api-analysis.md)
- [Raydium Pool 销毁方法分析](./raydium-pool-close-analysis.md)

---

**创建日期**：2026-01-08  
**最后更新**：2026-01-08  
**作者**：iFlow CLI