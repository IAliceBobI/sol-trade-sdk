# PumpSwap WSOL Pool 发现结果

本文档记录了在测试节点上发现的 WSOL 相关的 PumpSwap Pool。

## ✅ 发现的 WSOL + Token Pool（纯 Token Program）

### 1. BONK-WSOL Pool
- **Pool 地址**: `Dwczp92NX3ngbE2HeTUH4p5dcQxrpDF2AJMbW581gq1E`
- **Base Mint**: `DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263` (BONK)
- **Quote Mint**: `So11111111111111111111111111111111111111112` (WSOL)
- **LP Supply**: 22,900,485
- **Token Program**: Token（传统 Token Program）
- **流动性**: 较好，适合测试

### 2. RAY-WSOL Pool
- **Pool 地址**: `FbRpnhQjeGpNW2e9JMQyJwxg6ei6hnUNAbrdzso1Gwhy`
- **Base Mint**: `4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R` (RAY)
- **Quote Mint**: `So11111111111111111111111111111111111111112` (WSOL)
- **LP Supply**: 10,000,000
- **Token Program**: Token（传统 Token Program）
- **流动性**: 适中

### 3. ORCA-WSOL Pool
- **Pool 地址**: `2p6MseFKDB9kkXcqEgdrspRcpwFEFPF2fg1fnroYtLPE`
- **Base Mint**: `orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE` (ORCA)
- **Quote Mint**: `So11111111111111111111111111111111111111111112` (WSOL)
- **LP Supply**: 100
- **Token Program**: Token（传统 Token Program）
- **流动性**: 很低，仅适合基本测试

## 🪙 发现的 WSOL + Token2022 Pool

### PUMP-WSOL Pool
- **Pool 地址**: `539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR`
- **Base Mint**: `pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn` (PUMP)
- **Quote Mint**: `So11111111111111111111111111111111111111112` (WSOL)
- **LP Supply**: 54,168,294,568
- **Token Program**: Token2022（PUMP） + Token（WSOL）
- **注意**: 这是一个混合 Pool（Token2022 + Token），需要特别处理

## 使用建议

### 推荐 Pool（按优先级）

1. **BONK-WSOL** ⭐⭐⭐
   - 流动性最好
   - 纯 Token Program，无兼容性问题
   - 适合大部分测试场景

2. **RAY-WSOL** ⭐⭐
   - 流动性适中
   - 纯 Token Program
   - 适合中等规模测试

3. **ORCA-WSOL** ⭐
   - 流动性很低
   - 仅适合基本功能测试

### 避免使用

- **PUMP-WSOL**: 混合 Pool（Token2022 + Token），可能需要特殊处理

## 测试代码示例

```rust
use sol_trade_sdk::instruction::utils::pumpswap::get_pool_by_mint;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::test]
async fn test_bonk_wsol_pool() {
    let rpc = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());
    let bonk_mint = Pubkey::from_str("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263").unwrap();

    let (pool_addr, pool) = get_pool_by_mint(&rpc, &bonk_mint).await.unwrap();

    assert_eq!(pool.quote_mint, wsol_mint());
    println!("Pool: {}", pool_addr);
    println!("LP Supply: {}", pool.lp_supply);
}
```

## 相关测试文件

- `sol-trade-test-utils/tests/pumpswap_wsol_token_pools.rs` - 完整的 Token Pool 发现测试
- `sol-trade-test-utils/tests/list_pumpswap_wsol_pools.rs` - 快速分类验证测试

## 发现时间

2025-02-05
