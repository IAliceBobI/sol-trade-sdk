# 测试中的 DEX 和 Pool 汇总

本文档列出了 `tests/` 目录中使用的所有 DEX 协议和 Pool 地址，用于验证和参考。

## 📋 目录

- [PumpSwap](#1️⃣-pumpswap)
- [Raydium CLMM](#2️⃣-raydium-clmm)
- [Raydium AMM V4](#3️⃣-raydium-amm-v4)
- [Raydium CPMM](#4️⃣-raydium-cpmm)

---

## 1️⃣ **PumpSwap**

**测试文件**: `pumpswap_pool_tests.rs`

**Program ID**: `pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA`

### Pool 信息

| 项目 | 地址 |
|------|------|
| **Pool Address** | `539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR` |
| **Base Token Mint** | `pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn` (PUMP) |
| **Quote Token Mint** | `So11111111111111111111111111111111111111112` (WSOL) |

### 测试用例

- `test_find_pool_by_mint` - 通过 mint 查找 pool 地址
- `test_get_pool_by_address` - 通过地址获取 pool 数据
- `test_get_pumpswap_token_price_in_usd` - 获取 token 的 USD 价格
- `test_get_pool_by_mint_caching` - 验证缓存行为
- `test_get_pool_by_mint_force_refresh` - 强制刷新缓存

---

## 2️⃣ **Raydium CLMM**

**测试文件**: `raydium_clmm_pool_tests.rs`, `raydium_clmm_buy_sell_tests.rs`

**Program ID**: `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK`

### Pool 列表

| Pool 名称 | 交易对 | Pool 地址 |
|-----------|--------|----------|
| **WSOL-USDT** | WSOL/USDT | `ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6` |
| **WSOL-JUP** | WSOL/JUP | `EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw` |

### 常用 Token Mint

| Token | Mint 地址 |
|-------|----------|
| **WSOL** | `So11111111111111111111111111111111111111112` |
| **USDT** | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |
| **JUP** | `JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN` |

### 测试用例

#### raydium_clmm_pool_tests.rs
- `test_raydium_clmm_get_pool_by_address` - 通过地址获取 pool 数据
- `test_raydium_clmm_get_wsol_price_in_usd_with_client` - 获取 WSOL 的 USD 价格
- `test_raydium_clmm_get_jup_price_in_usd` - 获取 JUP 的 USD 价格
- `test_raydium_clmm_get_jup_price_in_usd_with_pool` - 直接传入池地址获取价格
- `test_raydium_clmm_get_pool_by_mint_with_auto_mock` - 使用 Auto Mock 加速查询

#### raydium_clmm_buy_sell_tests.rs
- `test_raydium_clmm_buy_and_sell_jup` - 完整的 JUP 买入卖出流程

---

## 3️⃣ **Raydium AMM V4**

**测试文件**: `raydium_amm_v4_pool_tests.rs`, `raydium_amm_v4_buy_sell_tests.rs`

**Program ID**: `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`

### Pool 列表

| Pool 名称 | 交易对 | Pool 地址 |
|-----------|--------|----------|
| **SOL-USDC** | SOL/USDC | `58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2` |
| **OIIAOIIA-SOL** | OIIAOIIA/WSOL | `HZ6rzhC96cTVx3HQiKoDbSdoRd3LH5nELYuYXGu4f3EE` |

### 常用 Token Mint

| Token | Mint 地址 |
|-------|----------|
| **WSOL** | `So11111111111111111111111111111111111111112` |
| **USDC** | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| **OIIAOIIA** | `VaxZxmFXV8tmsd72hUn22ex6GFzZ5uq9DVJ5wA5pump` |

### 测试用例

#### raydium_amm_v4_pool_tests.rs
- `test_fetch_amm_info` - 获取 AMM 信息并验证字段
- `test_get_pool_by_address_cache` - 验证缓存功能
- `test_public_rpc_limitations` - 验证公共 RPC 限制
- `test_get_amm_v4_token_price_in_usd` - 获取 token 的 USD 价格
- `test_raydium_amm_v4_get_pool_by_mint_with_auto_mock` - 使用 Auto Mock 加速查询

#### raydium_amm_v4_buy_sell_tests.rs
- `test_raydium_amm_v4_params_from_rpc` - 从 AMM 地址创建参数
- `test_raydium_amm_v4_buy_sell_complete` - 完整的买入-卖出流程
- `test_raydium_amm_v4_slippage_protection` - 验证滑点保护

---

## 4️⃣ **Raydium CPMM**

**测试文件**: `raydium_cpmm_buy_sell_tests.rs`

**Program ID**: `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C`

### Pool 列表

| Pool 名称 | 交易对 | Pool 地址 |
|-----------|--------|----------|
| **PIPE-WSOL** | PIPE/WSOL | `BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp` |

### 常用 Token Mint

| Token | Mint 地址 |
|-------|----------|
| **WSOL** | `So11111111111111111111111111111111111111112` |
| **PIPE** | `8ycz3kctoRb4LFrtoYG2r8tRyUYUeGf5Q16M2TEMp7A` |

### 测试用例

#### raydium_cpmm_buy_sell_tests.rs
- `test_raydium_cpmm_buy_sell_complete` - 完整的买入-卖出流程
- `test_get_cpmm_token_price_in_usd` - 获取 CPMM token 的 USD 价格
- `test_raydium_cpmm_get_pool_by_mint_with_auto_mock` - 使用 Auto Mock 加速查询

---

## 🔗 相关文档

- [Gas费策略](../docs/Gas费策略.md)
- [Nonce使用指南](../docs/Nonce使用指南.md)
- [地址查找表](../docs/地址查找表.md)
- [Pool查询方法](../docs/Pool查询方法.md)
- [交易参数参考](../docs/交易参数参考.md)

---

## 📝 说明

- 所有测试使用本地测试节点 `http://127.0.0.1:8899` (surfpool)
- WSOL Mint: `So11111111111111111111111111111111111111112`
- 测试时使用 `cargo nextest` 而不是 `cargo test` 以获得更快的执行速度
- 某些测试使用 `serial_test::serial` 标记，必须串行运行以避免冲突

---

最后更新: 2025-02-02
