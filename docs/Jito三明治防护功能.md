# Jito 三明治防护功能实现总结

## 📋 实现内容

本次更新为 `sol-trade-sdk` 添加了 **Jito 三明治攻击防护**功能，用户可以通过简单的开关控制是否启用此功能。

## ✨ 新增功能

### 1. **TradeConfig 新增字段**

```rust
pub struct TradeConfig {
    // ... 其他字段

    /// 是否启用 Jito 三明治攻击防护（默认：false）
    pub enable_jito_sandwich_protection: bool,
}
```

**配置方法**：

```rust
// 方法 1：默认配置（防护禁用）
let config = TradeConfig::new(
    rpc_url,
    swqos_configs,
    commitment,
);

// 方法 2：全局启用防护
let config = TradeConfig::new(...)
    .with_jito_sandwich_protection(true);
```

### 2. **交易级别覆盖**

`TradeBuyParams` 和 `TradeSellParams` 新增字段：

```rust
pub struct TradeBuyParams {
    // ... 其他字段

    /// 是否启用 Jito 三明治攻击防护（可选，覆盖全局配置）
    ///
    /// - `Some(true)`：强制启用防护
    /// - `Some(false)`：强制禁用防护
    /// - `None`：使用全局配置
    pub enable_jito_sandwich_protection: Option<bool>,
}
```

**使用方法**：

```rust
// 全局禁用，但单次交易启用
let mut buy_params = TradeBuyParams::new(...);
buy_params.enable_jito_sandwich_protection = Some(true);
client.buy(buy_params).await?;

// 全局启用，但单次交易禁用
let mut buy_params = TradeBuyParams::new(...);
buy_params.enable_jito_sandwich_protection = Some(false);
client.buy(buy_params).await?;
```

### 3. **自动添加 jitodontfront 账户**

当启用防护时，SDK 会自动在交易中添加 `jitodontfront` 账户：

```rust
// 默认账户
jitodontfront111111111111111111111111111111

// 自定义账户（可选）
use sol_trade_sdk::swqos::jito::generate_dont_front_account;
let custom_account = generate_dont_front_account(Some("_myapp"));
// 结果: jitodontfront_myapp
```

## 🔧 技术实现

### 交易构建流程

```rust
// 启用防护后的交易结构
Transaction {
    instructions: [
        nonce_instruction,
        jitodontfront_marker,  // ← 新增：三明治防护标记
        tip_transfer,
        compute_budget,
        business_instructions,
        ...
    ]
}
```

### jitodontfront 标记指令

```rust
Instruction {
    program_id: System Program,
    accounts: [
        AccountMeta::new(payer, true),           // payer (签名者)
        AccountMeta::new_readonly(jitodontfront, false),  // jitodontfront (只读)
    ],
    data: [0, 0, 0, 0],  // transfer 金额为 0（无操作）
}
```

## 📊 性能影响

| 指标 | 影响 | 说明 |
|------|------|------|
| **交易大小** | +32 bytes | 添加一个 Pubkey |
| **Compute Unit** | 几乎无 | 只读账户，不消耗 CU |
| **执行速度** | 无影响 | 只读账户，无需额外计算 |
| **成功率** | 提高 | 防止三明治攻击导致的失败 |

## 🎯 使用建议

### ✅ 推荐启用防护的场景

- **套利交易**: 对价格敏感，抢跑会让策略无利可图
- **大额交易**: 容易被 MEV bot 盯上
- **MEV 策略**: 需要确保执行顺序的交易

### ❌ 不推荐启用防护的场景

- **普通 Swap**: 原子性已足够，滑点保护已够用
- **小额交易**: 不值得 MEV bot 抢跑
- **测试交易**: 简单快速即可

## 📚 完整示例

### 示例 1：全局禁用（默认）

```rust
use sol_trade_sdk::{TradingClient, TradeConfig};
use sol_trade_sdk::swqos::{SwqosConfig, SwqosRegion};
use solana_commitment_config::CommitmentConfig;

// 创建默认配置（三明治防护：禁用）
let config = TradeConfig::new(
    rpc_url,
    vec![SwqosConfig::Jito(rpc_url, SwqosRegion::Default, None)],
    CommitmentConfig::confirmed(),
);

let client = TradingClient::new(payer, config).await;
```

### 示例 2：全局启用

```rust
// 创建启用防护的配置
let config = TradeConfig::new(
    rpc_url,
    vec![SwqosConfig::Jito(rpc_url, SwqosRegion::Tokyo, None)],  // 亚洲用户使用东京
    CommitmentConfig::confirmed(),
)
.with_jito_sandwich_protection(true);  // ← 启用三明治防护

let client = TradingClient::new(payer, config).await;
```

### 示例 3：交易级别覆盖

```rust
// 全局禁用
let config = TradeConfig::new(...);  // enable_jito_sandwich_protection = false
let client = TradingClient::new(payer, config).await;

// 但这次交易启用防护
let mut buy_params = TradeBuyParams::new(...);
buy_params.enable_jito_sandwich_protection = Some(true);  // ← 单次启用
client.buy(buy_params).await?;
```

## 🧪 测试

运行测试验证功能：

```bash
# 单元测试
cargo test --test jito_sandwich_protection_test

# 示例程序
cargo run --example jito_sandwich_protection

# 完整测试
cargo test
```

## 📖 相关文档

- **Jito 官方文档**: https://docs.jito.wtf/lowlatencytxnsend/#sandwich-mitigation
- **配置示例**: `examples/jito_sandwich_protection.rs`
- **单元测试**: `tests/jito_sandwich_protection_test.rs`

## 🔍 工作原理

### 无防护时的风险

```
Bundle: [Swap, tip]
⚠️  攻击者可以在前后插入交易

攻击者操作:
[买入, 你的 Swap, 卖出, tip]
      ↑
  推高价格，你以更高价格买入
                      ↑
                  他们卖出获利
```

### 启用防护后的保护

```
Bundle: [Swap + jitodontfront, tip]
✅ Jito Block Engine 确保包含 jitodontfront 的交易必须在第一位

规则:
- 包含 jitodontfront 的交易必须在 index 0
- 无法在其前后插入其他交易
- Bundle 结构必须符合 Jito 规则
```

## ⚠️ 重要说明

1. **只对 Jito 有效**: 此功能只在 Jito Bundle 上生效，其他 SWQOS 不受影响
2. **不保证 100% 防护**: 官方文档说明此功能可能帮助减少但不能完全阻止三明治攻击
3. **账户不需要存在**: `jitodontfront` 账户只需是有效的 Pubkey，不需要在链上存在
4. **标记为只读**: 优化执行速度，不消耗额外的 Compute Unit

## 📝 修改的文件

1. **src/common/types.rs**: 添加 `enable_jito_sandwich_protection` 字段
2. **src/trading/core/params.rs**: 在 `SwapParams` 中添加可选字段
3. **src/trading/common/transaction_builder.rs**: 实现三明治防护逻辑
4. **src/trading/core/async_executor.rs**: 传递防护标志到交易构建
5. **src/trading/core/executor.rs**: 在 buy/sell 中传递防护标志
6. **src/lib.rs**: 在 `TradingClient`、`TradeBuyParams`、`TradeSellParams` 中添加支持
7. **examples/jito_sandwich_protection.rs**: 新增示例程序
8. **tests/jito_sandwich_protection_test.rs**: 新增单元测试

## 🎉 总结

此次更新为 `sol-trade-sdk` 添加了完整的 Jito 三明治防护功能，用户可以通过简单的开关控制，灵活应对不同的交易场景。功能实现遵循 Jito 官方文档，具有良好的性能和易用性。
