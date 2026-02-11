# Jito Tip 使用指南

本文档介绍 Jito Tip 的正确使用方式，包括 sendTransaction 和 sendBundle 两种场景的最佳实践。

## 核心概念

### 什么是 Jito Tip？

Jito Tip 是支付给 Jito 验证者的小费，用于激励验证者优先处理你的交易或 Bundle。Tip 是通过 System Program 的 transfer 指令将 SOL 转账到 Jito 指定的 tip 账户来实现的。

### Tip 账户

Jito 有 8 个官方 tip 账户（Mainnet）：

```rust
// src/constants/swqos.rs
pub const JITO_TIP_ACCOUNTS: &[Pubkey] = &[
    pubkey!("96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5"),
    pubkey!("HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe"),
    pubkey!("Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY"),
    pubkey!("ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49"),
    pubkey!("DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh"),
    pubkey!("ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt"),
    pubkey!("DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL"),
    pubkey!("3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT"),
];
```

**建议**：随机选择一个 tip 账户，以减少竞争。

### 最低 Tip 金额

- **最低 tip**：1000 lamports (0.000001 SOL)
- 在高竞争场景下，需要更高的 tip 才能成功

## sendTransaction vs sendBundle

### sendTransaction（单笔交易）

当使用 `sendTransaction` 时，Jito 建议使用 **70/30 分配**：

```
Total Fee = 1.0 SOL
├── Priority Fee (70%): 0.7 SOL  ← 支付给网络
└── Jito Tip (30%): 0.3 SOL      ← 支付给 Jito 验证者
```

**交易结构**：

```
Transaction:
├── Compute Budget Instructions (priority fee)
├── Business Instructions (swap, transfer, etc.)
└── Tip Transfer Instruction (SOL → Jito Tip Account)
```

### sendBundle（多笔交易打包）

当使用 `sendBundle` 时，**只有 Jito Tip 生效**，Priority Fee 可以忽略。

**重要**：一个 Bundle 只需要 **一个 tip 指令**，放在 **最后一个交易** 中。

## Bundle Tip 位置详解

### 正确的 Bundle 结构

```
Bundle = [tx1, tx2, tx3, tx4, tx5(tip)]
                                ↑
                         最后一个交易包含 tip
```

### Tip 指令的位置

有两种推荐方式：

#### 方式 1：最后一个交易的指令列表末尾（推荐 ✅）

```
tx1: [business_inst1, business_inst2, ...]
tx2: [business_inst1, business_inst2, ...]
...
txN: [business_inst1, business_inst2, ..., tip_transfer]  ← tip 作为最后一条指令
```

**优点**：
- ✅ 原子性保证：如果业务指令失败，tip 不会支付
- ✅ 节省 Bundle 空间
- ✅ Jito 官方推荐

#### 方式 2：单独的 tip 交易

```
tx1: [business_inst1, business_inst2, ...]
tx2: [business_inst1, business_inst2, ...]
...
txN: [tip_transfer]  ← 单独的 tip 交易
```

**优点**：
- ✅ 代码简单
- ⚠️ 如果业务失败，tip 仍可能支付（浪费）

### 错误的方式 ❌

```
❌ 每个交易都加 tip（浪费）：
tx1: [business_inst, tip]  ← 多余的 tip
tx2: [business_inst, tip]  ← 多余的 tip
tx3: [business_inst, tip]  ← 多余的 tip
```

## 代码示例

### 构建带 Tip 的 Bundle

```rust
use sol_trade_sdk::common::SolanaRpcClient;
use sol_trade_sdk::constants::swqos::JITO_TIP_ACCOUNTS;
use rand::seq::IndexedRandom;

// 1. 随机选择 tip 账户
let tip_account = JITO_TIP_ACCOUNTS.choose(&mut rand::rng()).unwrap();

// 2. 构建 Bundle 交易
let mut transactions = Vec::new();

for (i, params) in trade_params.iter().enumerate() {
    let is_last = i == trade_params.len() - 1;

    // 只有最后一个交易包含 tip
    let tx = build_transaction(
        payer.clone(),
        rpc.clone(),
        cu_limit,
        cu_price,
        business_instructions,
        alt_account,
        recent_blockhash,
        None,
        protocol_name,
        is_buy,
        is_last,        // ← 只有最后一个交易 with_tip = true
        tip_account,
        tip_amount,
        None,
        false,
    ).await?;

    transactions.push(tx);
}

// 3. 发送 Bundle
let jito_client = JitoClient::with_region(JitoRegion::Tokyo);
jito_client.send_transactions(TradeType::Buy, &transactions, true).await?;
```

### 使用 GasFeeStrategy 设置固定 Tip

```rust
use sol_trade_sdk::common::GasFeeStrategy;
use sol_trade_sdk::swqos::SwqosType;

let gas_fee_strategy = GasFeeStrategy::new();

// 设置全局固定 tip（适用于所有 SWQOS 服务）
gas_fee_strategy.set_global_fee_strategy(
    500000,   // buy_cu_limit
    500000,   // sell_cu_limit
    1000,     // buy_cu_price (micro-lamports)
    1000,     // sell_cu_price
    0.001,    // buy_tip (SOL) - 固定 0.001 SOL
    0.001,    // sell_tip (SOL) - 固定 0.001 SOL
);

// 或者为 Jito 单独设置
gas_fee_strategy.set(
    SwqosType::Jito,
    TradeType::Buy,
    GasFeeStrategyType::Normal,
    500000,   // cu_limit
    1000,     // cu_price
    0.001,    // tip (SOL)
);
```

### 动态获取 Tip 价格

```rust
use sol_trade_sdk::swqos::jito::{JitoTipFloorClient, DynamicTipConfig, TipPercentile};

let client = JitoTipFloorClient::new();
let config = DynamicTipConfig {
    enabled: true,
    percentile: TipPercentile::P50,
    multiplier: 1.0,
    min_tip: 0.00001,
    max_tip: 0.001,
};

let optimal_tip = client.get_optimal_tip(&config).await?;
println!("推荐 tip: {} SOL", optimal_tip);
```

## Tip 价格查询

### REST API

```bash
curl https://bundles.jito.wtf/api/v1/bundles/tip_floor
```

响应示例：

```json
[
  {
    "time": "2024-09-01T12:58:00Z",
    "landed_tips_25th_percentile": 0.000006,
    "landed_tips_50th_percentile": 0.00001,
    "landed_tips_75th_percentile": 0.000036,
    "landed_tips_95th_percentile": 0.001448,
    "landed_tips_99th_percentile": 0.010008
  }
]
```

### WebSocket

```bash
wscat -c wss://bundles.jito.wtf/api/v1/bundles/tip_stream
```

## 注意事项

### ⚠️ 不要使用 Address Lookup Tables 引用 Tip 账户

```rust
// ❌ 错误：不要将 tip 账户放入 ALT
let alt = AddressLookupTableAccount { ... };

// ✅ 正确：tip 账户直接在交易中
let tip_instruction = transfer(&payer, tip_account, tip_amount);
```

原因：非 Jito-Solana 验证者不会优先处理 tip，使用 ALT 引用 tip 账户会浪费资金。

### ⚠️ 原子性保护

当业务交易失败时：
- **方式 1（推荐）**：tip 作为最后一个交易的最后一条指令，业务失败则 tip 不支付
- **方式 2**：单独 tip 交易，业务失败可能仍支付 tip（浪费）

### ⚠️ Uncle Block 风险

在极少数情况下，Bundle 可能落在 uncle block 上，导致交易被重新广播。建议：
1. 在业务逻辑中添加 pre/post 检查
2. Tip 指令放在业务交易中而非单独交易

## 三明治攻击防护

Jito 提供了 `jitodontfront` 账户机制来防止三明治攻击：

```rust
use sol_trade_sdk::swqos::jito::generate_dont_front_account;

// 生成防护账户
let dont_front_account = generate_dont_front_account(None);
// 或自定义后缀
let dont_front_account = generate_dont_front_account(Some("myapp"));
```

将此账户添加到交易的只读账户列表中即可启用防护。

## 参考资料

- [Jito Labs Documentation](https://docs.jito.wtf/lowlatencytxnsend/)
- [QuickNode Jito Bundles Guide](https://www.quicknode.com/guides/solana-development/transactions/jito-bundles)
- [Jito Tip Payment Program](https://jito-foundation.gitbook.io/mev/mev-payment-and-distribution/tip-payment-program)

## 相关文档

- [Gas费策略.md](./Gas费策略.md) - GasFeeStrategy API 使用
- [Node1最小小费限制.md](./Node1最小小费限制.md) - Node1 MEV 服务限制
