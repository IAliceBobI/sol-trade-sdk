# 交易生命周期回调使用指南

## 概述

交易生命周期回调机制允许上游应用在交易签名后、发送前获取签名后的交易实体，用于入库、审计、日志记录等操作。

## 架构设计

### 回调执行模式

SDK 支持两种回调执行模式，满足不同业务场景的需求：

| 模式 | 交易延迟 | 失败影响 | 适用场景 |
|------|---------|---------|---------|
| **Async**（异步） | 0ms（不阻塞） | 不影响交易发送 | 监控、日志、非关键业务 |
| **Sync**（同步） | 取决于回调执行时间 | 阻止交易发送 | 入库、审计、关键业务 |

#### 异步模式（Async，默认）

```rust
// 默认行为：异步执行，不阻塞交易发送
let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment)
    .with_callback_execution_mode(CallbackExecutionMode::Async);

// 回调使用 tokio::spawn 异步执行
tokio::spawn(async move {
    if let Err(e) = callback.on_transaction_signed(context).await {
        eprintln!("[Callback Error] {:?}", e);
    }
});
```

**特性**：
- 回调失败不影响交易发送
- 使用 `tokio::spawn` 异步执行
- 交易延迟：0ms
- 适合：监控、日志、非关键业务

#### 同步模式（Sync）

```rust
// 同步模式：等待回调完成后再发送交易
let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment)
    .with_callback_execution_mode(CallbackExecutionMode::Sync);

// 回调使用 .await 同步等待
if let Err(e) = callback.on_transaction_signed(context).await {
    // 回调失败会阻止交易发送
    eprintln!("[Callback Error] {:?}", e);
    return Err(e);
}
```

**特性**：
- 回调失败会阻止交易发送
- 使用 `.await` 同步等待
- 交易延迟：取决于回调执行时间
- 适合：入库、审计、关键业务

### 配置层级

```
TradeConfig (全局默认)
    ↓
TradeBuyParams / TradeSellParams (单次交易覆盖)
    ↓
async_executor (执行)
```

#### 全局配置

```rust
let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment)
    .with_callback_execution_mode(CallbackExecutionMode::Sync); // 全局默认：同步模式
```

#### 单次交易覆盖

```rust
let buy_params = TradeBuyParams {
    // ... 其他参数
    on_transaction_signed: Some(callback),
    callback_execution_mode: Some(CallbackExecutionMode::Async), // 覆盖全局配置
};
```

#### 混合使用

```rust
// 全局默认异步
let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment)
    .with_callback_execution_mode(CallbackExecutionMode::Async);

// 关键交易使用同步模式（先入库再发送）
let critical_params = TradeBuyParams {
    // ... 其他参数
    on_transaction_signed: Some(database_callback),
    callback_execution_mode: Some(CallbackExecutionMode::Sync),
};

// 普通交易使用异步模式
let normal_params = TradeBuyParams {
    // ... 其他参数
    on_transaction_signed: Some(log_callback),
    callback_execution_mode: None, // 使用全局默认（异步）
};
```

### 回调时机

```
交易构建 → 签名 → 🎯 回调钩子 → 发送
```

回调在以下时机触发：
- **位置**：交易签名后、发送前
- **并发**：每个 SWQOS 服务都会触发一次回调
- **异步**：使用 `tokio::spawn` 异步执行，不阻塞交易发送

### 核心组件

#### 1. TransactionLifecycleCallback Trait

```rust
pub trait TransactionLifecycleCallback: Send + Sync {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>>;
}
```

#### 2. CallbackContext

包含签名后的交易和完整元数据：

```rust
pub struct CallbackContext {
    /// 签名后的交易
    pub transaction: VersionedTransaction,

    /// SWQOS 服务类型
    pub swqos_type: SwqosType,

    /// 交易类型（买入/卖出）
    pub trade_type: TradeType,

    /// 交易签名
    pub signature: String,

    /// 时间戳（纳秒）
    pub timestamp_ns: u64,

    /// 是否包含小费
    pub with_tip: bool,

    /// 小费金额（SOL）
    pub tip_amount: f64,
}
```

#### 3. CallbackRef

Arc 包装的回调类型，便于共享：

```rust
pub type CallbackRef = Arc<dyn TransactionLifecycleCallback>;
```

## 自定义数据库回调

### 基础示例

```rust
use sol_trade_sdk::{CallbackContext, TransactionLifecycleCallback};
use futures::future::BoxFuture;

#[derive(Clone)]
struct MyDatabaseCallback;

impl TransactionLifecycleCallback for MyDatabaseCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        let context_clone = context.clone();
        Box::pin(async move {
            println!(
                "[Database] Saving transaction: {} (swqos: {:?})",
                context_clone.signature, context_clone.swqos_type
            );

            // 在这里添加你的数据库入库逻辑
            // 例如：使用 SQLx、SeaORM、Diesel 等

            Ok(())
        })
    }
}
```

### 使用 SQLx 保存到 PostgreSQL

```rust
use sqlx::PgPool;
use sol_trade_sdk::{CallbackContext, TransactionLifecycleCallback};
use futures::future::BoxFuture;

#[derive(Clone)]
struct PostgresCallback {
    pool: PgPool,
}

impl PostgresCallback {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        Ok(Self { pool })
    }
}

impl TransactionLifecycleCallback for PostgresCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query!(
                r#"
                INSERT INTO transactions
                (signature, swqos_type, trade_type, timestamp_ns, with_tip, tip_amount, transaction_base64)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (signature) DO NOTHING
                "#,
                context.signature,
                format!("{:?}", context.swqos_type),
                format!("{:?}", context.trade_type),
                context.timestamp_ns as i64,
                context.with_tip,
                context.tip_amount,
                context.to_base64(),
            )
            .execute(&pool)
            .await?;

            Ok(())
        })
    }
}

// 使用
let pool = PgPool::connect("postgres://user:pass@localhost/db").await?;
let callback = Arc::new(PostgresCallback { pool });
```

### 使用 SeaORM 保存到 MySQL

```rust
use sea_orm::{Database, EntityTrait, ActiveModelTrait, Set};
use sol_trade_sdk::{CallbackContext, TransactionLifecycleCallback};
use futures::future::BoxFuture;

#[derive(Clone)]
struct MysqlCallback {
    db: DatabaseConnection,
}

impl MysqlCallback {
    pub async fn new(database_url: &str) -> Result<Self> {
        let db = Database::connect(database_url).await?;
        Ok(Self { db })
    }
}

impl TransactionLifecycleCallback for MysqlCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        let db = self.db.clone();
        Box::pin(async move {
            let transaction = transaction::ActiveModel {
                signature: Set(context.signature.clone()),
                swqos_type: Set(format!("{:?}", context.swqos_type)),
                trade_type: Set(format!("{:?}", context.trade_type)),
                timestamp_ns: Set(context.timestamp_ns as i64),
                with_tip: Set(context.with_tip),
                tip_amount: Set(context.tip_amount),
                transaction_base64: Set(context.to_base64()),
                ..Default::default()
            };

            transaction.insert(&db).await?;

            Ok(())
        })
    }
}

// 使用
let db = Database::connect("mysql://user:pass@localhost/db").await?;
let callback = Arc::new(MysqlCallback { db });
```

### 使用 MongoDB 保存

```rust
use mongodb::{Client, Collection, bson::doc};
use sol_trade_sdk::{CallbackContext, TransactionLifecycleCallback};
use futures::future::BoxFuture;

#[derive(Clone)]
struct MongoCallback {
    collection: Collection<bson::Document>,
}

impl MongoCallback {
    pub async fn new(uri: &str, db_name: &str, collection_name: &str) -> Result<Self> {
        let client = Client::with_uri_str(uri).await?;
        let collection = client.database(db_name).collection(collection_name);
        Ok(Self { collection })
    }
}

impl TransactionLifecycleCallback for MongoCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        let collection = self.collection.clone();
        Box::pin(async move {
            let document = doc! {
                "signature": context.signature,
                "swqos_type": format!("{:?}", context.swqos_type),
                "trade_type": format!("{:?}", context.trade_type),
                "timestamp_ns": context.timestamp_ns as i64,
                "with_tip": context.with_tip,
                "tip_amount": context.tip_amount,
                "transaction_base64": context.to_base64(),
            };

            collection.insert_one(document, None).await?;

            Ok(())
        })
    }
}

// 使用
let collection = client.database("solana").collection("transactions");
let callback = Arc::new(MongoCallback { collection });
```

## 数据库表设计

### PostgreSQL 表结构

```sql
CREATE TABLE transactions (
    id SERIAL PRIMARY KEY,
    signature VARCHAR(255) UNIQUE NOT NULL,
    swqos_type VARCHAR(50),
    trade_type VARCHAR(50),
    timestamp_ns BIGINT,
    with_tip BOOLEAN,
    tip_amount DECIMAL(20, 9),
    transaction_base64 TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_signature ON transactions(signature);
CREATE INDEX idx_timestamp_ns ON transactions(timestamp_ns);
CREATE INDEX idx_swqos_type ON transactions(swqos_type);
```

### MySQL 表结构

```sql
CREATE TABLE transactions (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    signature VARCHAR(255) UNIQUE NOT NULL,
    swqos_type VARCHAR(50),
    trade_type VARCHAR(50),
    timestamp_ns BIGINT,
    with_tip BOOLEAN,
    tip_amount DECIMAL(20, 9),
    transaction_base64 TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_signature (signature),
    INDEX idx_timestamp_ns (timestamp_ns),
    INDEX idx_swqos_type (swqos_type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

### MongoDB 集合索引

```javascript
db.transactions.createIndex({ signature: 1 }, { unique: true });
db.transactions.createIndex({ timestamp_ns: -1 });
db.transactions.createIndex({ swqos_type: 1 });
```

## 完整使用示例

### 买入交易使用回调

```rust
use sol_trade_sdk::{
    SolanaTrade, TradeBuyParams, TradeTokenType, DexType,
    CallbackContext, TransactionLifecycleCallback,
    trading::core::params::{PumpSwapParams, DexParamEnum},
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::{str::FromStr, sync::Arc};
use futures::future::BoxFuture;

// 定义回调
#[derive(Clone)]
struct MyCallback;

impl TransactionLifecycleCallback for MyCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        Box::pin(async move {
            println!(
                "Transaction signed: {} (swqos: {:?}, trade: {:?})",
                context.signature, context.swqos_type, context.trade_type
            );

            // 入库操作
            save_to_database(&context).await?;

            Ok(())
        })
    }
}

// 创建客户端
let payer = Arc::new(Keypair::from_base58_string("your_keypair_here"));
let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
let commitment = CommitmentConfig::confirmed();
let swqos_configs = vec![SwqosConfig::Default(rpc_url.clone())];
let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment);
let client = SolanaTrade::new(payer, trade_config).await?;

// 创建回调实例
let callback = Arc::new(MyCallback {});

// 构建买入参数
let buy_params = TradeBuyParams {
    dex_type: DexType::PumpSwap,
    input_token_type: TradeTokenType::SOL,
    mint: Pubkey::from_str("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn")?,
    input_token_amount: 100_000,
    slippage_basis_points: Some(100),
    recent_blockhash: Some(client.rpc.get_latest_blockhash().await?),
    extension_params: DexParamEnum::PumpSwap(
        PumpSwapParams::from_pool_address_by_rpc(
            &client.rpc,
            &Pubkey::from_str("539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR")?,
        )
        .await?,
    ),
    address_lookup_table_account: None,
    wait_transaction_confirmed: true,
    create_input_token_ata: true,
    close_input_token_ata: true,
    create_mint_ata: true,
    durable_nonce: None,
    fixed_output_token_amount: None,
    gas_fee_strategy: GasFeeStrategy::new(),
    simulate: false,
    on_transaction_signed: Some(callback), // 设置回调
};

// 执行买入
let (success, signatures, error) = client.buy(buy_params).await?;
```

### 卖出交易使用回调

```rust
// 卖出参数同样支持回调
let sell_params = TradeSellParams {
    dex_type: DexType::PumpSwap,
    output_token_type: TradeTokenType::SOL,
    mint: Pubkey::from_str("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn")?,
    input_token_amount: amount_token,
    slippage_basis_points: Some(100),
    recent_blockhash: Some(client.rpc.get_latest_blockhash().await?),
    with_tip: false,
    extension_params: DexParamEnum::PumpSwap(params),
    address_lookup_table_account: None,
    wait_transaction_confirmed: true,
    create_output_token_ata: true,
    close_output_token_ata: true,
    close_mint_token_ata: false,
    durable_nonce: None,
    fixed_output_token_amount: None,
    gas_fee_strategy: GasFeeStrategy::new(),
    simulate: false,
    on_transaction_signed: Some(callback), // 设置回调
};

let (success, signatures, error) = client.sell(sell_params).await?;
```

## 高级用法

### 多 SWQOS 并发场景

当使用多个 SWQOS 服务时，每个服务都会触发一次回调：

```rust
let swqos_configs = vec![
    SwqosConfig::Jito("your_uuid".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::ZeroSlot("your_token".to_string(), SwqosRegion::Frankfurt, None),
    SwqosConfig::Default(rpc_url.clone()),
];

// 回调会被调用 3 次（每个 SWQOS 服务一次）
let callback = Arc::new(MyCallback {});
```

### 回调失败处理

回调失败不会阻止交易发送，仅记录错误：

```rust
impl TransactionLifecycleCallback for MyCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        Box::pin(async move {
            // 即使这里返回错误，交易仍然会发送
            if let Err(e) = save_to_database(&context).await {
                eprintln!("Failed to save transaction: {}", e);
                return Err(e);
            }
            Ok(())
        })
    }
}
```

### 使用连接池

在回调中使用数据库连接池以提高性能：

```rust
use sqlx::postgres::PgPoolOptions;

#[derive(Clone)]
struct DatabaseCallback {
    pool: PgPool,
}

impl DatabaseCallback {
    async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }
}
```

### 批量插入优化

对于高吞吐场景，考虑使用批量插入：

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::unbounded_channel::<CallbackContext>();

// 消费者：批量写入数据库
tokio::spawn(async move {
    let mut batch = Vec::with_capacity(100);
    let interval = tokio::time::interval(std::time::Duration::from_secs(1));

    loop {
        tokio::select! {
            result = rx.recv() => {
                if let Some(context) = result {
                    batch.push(context);
                    if batch.len() >= 100 {
                        save_batch(&batch).await;
                        batch.clear();
                    }
                }
            }
            _ = interval.tick() => {
                if !batch.is_empty() {
                    save_batch(&batch).await;
                    batch.clear();
                }
            }
        }
    }
});

// 回调：发送到通道
#[derive(Clone)]
struct ChannelCallback {
    sender: mpsc::UnboundedSender<CallbackContext>,
}

impl TransactionLifecycleCallback for ChannelCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        let sender = self.sender.clone();
        Box::pin(async move {
            sender.send(context)
                .map_err(|e| anyhow::anyhow!("Failed to send to channel: {}", e))?;
            Ok(())
        })
    }
}
```

## 性能考虑

### 1. 异步非阻塞

回调使用 `tokio::spawn` 异步执行，不会阻塞交易发送：

```rust
// SDK 内部实现
tokio::spawn(async move {
    if let Err(e) = callback.on_transaction_signed(context).await {
        eprintln!("[Callback Error] {:?}", e);
    }
});
```

### 2. 避免阻塞操作

在回调中避免使用阻塞操作：

```rust
// ❌ 错误：阻塞操作
impl TransactionLifecycleCallback for MyCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        Box::pin(async move {
            // 阻塞操作会降低性能
            std::thread::sleep(std::time::Duration::from_secs(1));
            Ok(())
        })
    }
}

// ✅ 正确：异步操作
impl TransactionLifecycleCallback for MyCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        Box::pin(async move {
            // 异步操作
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            Ok(())
        })
    }
}
```

### 3. 使用连接池

使用数据库连接池避免频繁创建连接：

```rust
// ✅ 正确：使用连接池
let pool = PgPoolOptions::new()
    .max_connections(10)
    .connect("postgres://user:pass@localhost/db")
    .await?;

let callback = Arc::new(DatabaseCallback { pool });
```

## 最佳实践

### 1. 使用 Arc 共享回调

```rust
let callback = Arc::new(MyCallback {});

// 可以在多个交易中复用
let mut params1 = buy_params.clone();
params1.on_transaction_signed = Some(callback.clone());

let mut params2 = sell_params.clone();
params2.on_transaction_signed = Some(callback.clone());
```

### 2. 错误处理

```rust
impl TransactionLifecycleCallback for MyCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        Box::pin(async move {
            match save_to_database(&context).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    // 记录错误但不影响交易
                    eprintln!("Failed to save transaction: {}", e);
                    Err(e)
                }
            }
        })
    }
}
```

### 3. 日志记录

```rust
impl TransactionLifecycleCallback for MyCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        Box::pin(async move {
            log::info!(
                "Transaction signed: signature={}, swqos={:?}, trade={:?}",
                context.signature, context.swqos_type, context.trade_type
            );

            // 业务逻辑
            save_to_database(&context).await?;

            Ok(())
        })
    }
}
```

### 4. 使用消息队列解耦

对于高吞吐场景，使用消息队列解耦交易发送和数据入库：

```rust
use tokio::sync::mpsc;

// 创建通道
let (tx, mut rx) = mpsc::unbounded_channel::<CallbackContext>();

// 后台消费者
tokio::spawn(async move {
    while let Some(context) = rx.recv().await {
        if let Err(e) = save_to_database(&context).await {
            eprintln!("Failed to save transaction: {}", e);
        }
    }
});

// 回调实现
#[derive(Clone)]
struct QueueCallback {
    sender: mpsc::UnboundedSender<CallbackContext>,
}

impl TransactionLifecycleCallback for QueueCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        let sender = self.sender.clone();
        Box::pin(async move {
            sender.send(context)
                .map_err(|e| anyhow::anyhow!("Failed to send to queue: {}", e))?;
            Ok(())
        })
    }
}
```

## 向后兼容

回调是可选的，不设置回调不会影响现有代码：

```rust
// 不使用回调（向后兼容）
let buy_params = TradeBuyParams {
    // ... 其他参数
    on_transaction_signed: None, // 不设置回调
};

let (success, signatures, error) = client.buy(buy_params).await?;
```

## 运行示例

```bash
cd examples/transaction_callback
cargo run
```

## 总结

交易生命周期回调机制提供了：

- ✅ **灵活性**：支持自定义数据库入库逻辑
- ✅ **高性能**：异步非阻塞，不影响交易发送
- ✅ **向后兼容**：可选参数，不破坏现有代码
- ✅ **完整元数据**：提供交易和完整的上下文信息
- ✅ **错误隔离**：回调失败不影响交易发送

适用于：
- 交易入库（PostgreSQL、MySQL、MongoDB 等）
- 审计日志
- 监控告警
- 数据分析
- 消息队列集成

## 相关资源

- **示例项目**：`examples/transaction_callback/`
- **核心模块**：`src/trading/lifecycle.rs`
- **API 文档**：`TransactionLifecycleCallback` trait