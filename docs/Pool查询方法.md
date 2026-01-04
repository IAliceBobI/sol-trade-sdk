# Pool 查询方法参考

本文档总结了所有协议查询 Pool 的方法及其参数。

## 1. PumpSwap

| 方法 | 文件位置 | 参数 | 返回 | 说明 |
|------|----------|------|------|------|
| `fetch_pool` | `src/instruction/utils/pumpswap.rs:194` | `rpc`, `pool_address: &Pubkey` | `Result<Pool>` | 根据地址获取 pool |
| `find_by_mint` | `src/instruction/utils/pumpswap.rs:356` | `rpc`, `mint: &Pubkey` | `Result<(Pubkey, Pool)>` | 根据 mint 查找（优先 canonical WSOL 池） |
| `find_by_base_mint` | `src/instruction/utils/pumpswap.rs:207` | `rpc`, `base_mint: &Pubkey` | `Result<(Pubkey, Pool)>` | 根据 base_mint 查找 |
| `find_by_quote_mint` | `src/instruction/utils/pumpswap.rs:269` | `rpc`, `quote_mint: &Pubkey` | `Result<(Pubkey, Pool)>` | 根据 quote_mint 查找 |
| `list_by_mint` | `src/instruction/utils/pumpswap.rs:415` | `rpc`, `mint: &Pubkey` | `Result<Vec<(Pubkey, Pool)>>` | 列出所有相关 pool |
| `calculate_canonical_pool_pda` | `src/instruction/utils/pumpswap.rs:337` | `mint: &Pubkey` | `Option<(Pubkey, Pubkey)>` | 计算 canonical pool PDA（迁移池） |
| `get_token_balances` | `src/instruction/utils/pumpswap.rs:510` | `pool: &Pool`, `rpc` | `Result<(u64, u64)>` | 获取池余额 (base, quote) |
| `quote_exact_in` | `src/instruction/utils/pumpswap.rs:527` | `rpc`, `pool_address`, `amount_in`, `is_base_in` | `Result<QuoteExactInResult>` | 报价（Exact-In） |

### PumpSwap 账户地址

```rust
pub const AMM_PROGRAM: Pubkey = pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA");
pub const FEE_RECIPIENT: Pubkey = pubkey!("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV");
pub const GLOBAL_ACCOUNT: Pubkey = pubkey!("ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw");
```

---

## 2. Raydium CPMM

| 方法 | 文件位置 | 参数 | 返回 | 说明 |
|------|----------|------|------|------|
| `fetch_pool_state` | `src/instruction/utils/raydium_cpmm.rs:41` | `rpc`, `pool_address: &Pubkey` | `Result<PoolState>` | 根据地址获取 pool state |
| `find_pool_by_mint` | `src/instruction/utils/raydium_cpmm.rs:137` | `rpc`, `mint: &Pubkey` | `Result<(Pubkey, PoolState)>` | 根据 mint 查找（返回第一个匹配） |
| `list_pools_by_mint` | `src/instruction/utils/raydium_cpmm.rs:246` | `rpc`, `mint: &Pubkey` | `Result<Vec<(Pubkey, PoolState)>>` | 列出所有相关 pool |
| `get_pool_token_balances` | `src/instruction/utils/raydium_cpmm.rs:80` | `rpc`, `pool_state: &Pubkey`, `token0_mint`, `token1_mint` | `Result<(u64, u64)>` | 获取池余额 |
| `get_pool_pda` | `src/instruction/utils/raydium_cpmm.rs:54` | `amm_config`, `mint1`, `mint2: &Pubkey` | `Option<Pubkey>` | 计算 pool PDA |
| `get_vault_pda` | `src/instruction/utils/raydium_cpmm.rs:62` | `pool_state`, `mint: &Pubkey` | `Option<Pubkey>` | 计算 vault PDA |
| `quote_exact_in` | `src/instruction/utils/raydium_cpmm.rs:109` | `rpc`, `pool_address`, `amount_in`, `is_token0_in` | `Result<QuoteExactInResult>` | 报价（Exact-In） |

### Raydium CPMM 账户地址

```rust
pub const RAYDIUM_CPMM: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
pub const AUTHORITY: Pubkey = pubkey!("GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL");
```

---

## 3. Raydium CLMM

| 方法 | 文件位置 | 参数 | 返回 | 说明 |
|------|----------|------|------|------|
| `fetch_pool_state` | `src/instruction/utils/raydium_clmm.rs:127` | `rpc`, `pool_address: &Pubkey` | `Result<PoolState>` | 根据地址获取 pool state |
| `list_pools_by_mint` | `src/instruction/utils/raydium_clmm.rs:145` | `rpc`, `mint: &Pubkey` | `Result<Vec<(Pubkey, PoolState)>>` | 列出所有相关 pool |
| `get_tick_array_pda` | `src/instruction/utils/raydium_clmm.rs:27` | `pool_id`, `start_tick_index: i32` | `Result<(Pubkey, u8)>` | 计算 tick array PDA |
| `quote_exact_in` | `src/instruction/utils/raydium_clmm.rs:254` | `rpc`, `pool_address`, `amount_in`, `zero_for_one` | `Result<QuoteExactInResult>` | 报价（Exact-In） |

### Raydium CLMM 账户地址

```rust
pub const RAYDIUM_CLMM: Pubkey = pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
```

---

## 4. Raydium AMM V4

| 方法 | 文件位置 | 参数 | 返回 | 说明 |
|------|----------|------|------|------|
| `fetch_amm_info` | `src/instruction/utils/raydium_amm_v4.rs:37` | `rpc`, `amm: Pubkey` | `Result<AmmInfo>` | 根据地址获取 AmmInfo |

### Raydium AMM V4 账户地址

```rust
pub const RAYDIUM_AMM_V4: Pubkey = pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
pub const AUTHORITY: Pubkey = pubkey!("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
```

---

## 5. Bonk

| 方法 | 文件位置 | 参数 | 返回 | 说明 |
|------|----------|------|------|------|
| `fetch_pool_state` | `src/instruction/utils/bonk.rs:62` | `rpc`, `pool_address: &Pubkey` | `Result<PoolState>` | 根据地址获取 pool state |
| `get_pool_pda` | `src/instruction/utils/bonk.rs:159` | `base_mint`, `quote_mint: &Pubkey` | `Option<Pubkey>` | 计算 pool PDA |
| `get_vault_pda` | `src/instruction/utils/bonk.rs:171` | `pool_state`, `mint: &Pubkey` | `Option<Pubkey>` | 计算 vault PDA |
| `get_amount_out` | `src/instruction/utils/bonk.rs:127` | 见下方 | `u64` | 计算输出金额 |
| `get_amount_in` | `src/instruction/utils/bonk.rs:95` | 见下方 | `u64` | 计算输入金额 |

### Bonk 账户地址

```rust
pub const BONK: Pubkey = pubkey!("BSwp6bEBihVLdqJRKGgzjcGLHkcTuzmSo1TQkHepzH8p");
pub const AUTHORITY: Pubkey = pubkey!("WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh");
pub const GLOBAL_CONFIG: Pubkey = pubkey!("6s1xP3hpbAfFoNtUNF8mfHsjr2Bd97JxFJRWLbL6aHuX");
```

---

## 6. Meteora DAMM V2

| 方法 | 文件位置 | 参数 | 返回 | 说明 |
|------|----------|------|------|------|
| `fetch_pool` | `src/instruction/utils/meteora_damm_v2.rs:39` | `rpc`, `pool_address: &Pubkey` | `Result<Pool>` | 根据地址获取 pool |

### Meteora DAMM V2 账户地址

```rust
pub const METEORA_DAMM_V2: Pubkey = pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");
pub const AUTHORITY: Pubkey = pubkey!("HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC");
```

---

## 通用查询参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `rpc` | `SolanaRpcClient` | RPC 客户端 |
| `mint` / `base_mint` / `quote_mint` | `Pubkey` | 代币 mint 地址 |
| `pool_address` / `pool_state` | `Pubkey` | 池子账户地址 |
| `amm_config` | `Pubkey` | AMM 配置地址（仅 CPMM） |
| `amount_in` | `u64` | 输入金额 |
| `is_base_in` / `is_token0_in` / `zero_for_one` | `bool` | 交换方向 |

---

## 缓存说明

目前 **没有专门的 Pool 查询缓存**。现有缓存：

- **MintInfo 缓存** (`src/utils/token.rs:32-33`): 缓存 `decimals`, `symbol`, `is_token2022`
- **ATA 缓存** (`src/utils/token.rs:44-45`): 缓存 ATA 地址
- **PDA 缓存** (`src/common/fast_fn.rs`): 缓存 PDA 计算结果（Bonk 使用）

如需添加 Pool 缓存，可参考上述结构实现。
