// 交易生命周期回调示例
//
// 本示例演示如何使用交易生命周期回调在交易签名后、发送前获取签名后的交易实体，用于入库等操作

use anyhow::Result;
use sol_trade_sdk::{
    SolanaTrade, TradeBuyParams, TradeTokenType, DexType,
    CallbackContext, TransactionLifecycleCallback,
    trading::core::params::{PumpSwapParams, DexParamEnum},
    common::TradeConfig, swqos::SwqosConfig,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use solana_commitment_config::CommitmentConfig;
use std::{str::FromStr, sync::Arc};
use futures::future::BoxFuture;

/// 自定义数据库回调示例
///
/// 这是一个完整的实现示例，展示如何将交易保存到数据库
#[derive(Clone)]
struct MyDatabaseCallback;

impl TransactionLifecycleCallback for MyDatabaseCallback {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        let context_clone = context.clone();
        Box::pin(async move {
            println!(
                "[Database] Saving transaction: {}",
                context_clone.signature
            );
            println!("  - SWQOS Type: {:?}", context_clone.swqos_type);
            println!("  - Trade Type: {:?}", context_clone.trade_type);
            println!("  - Timestamp: {}", context_clone.timestamp_ns);
            println!("  - With Tip: {}", context_clone.with_tip);
            println!("  - Tip Amount: {} SOL", context_clone.tip_amount);

            // 示例：使用 SQLx 保存到 PostgreSQL
            // sqlx::query!(
            //     "INSERT INTO transactions (signature, swqos_type, trade_type, timestamp_ns, with_tip, tip_amount, transaction_base64) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            //     context_clone.signature,
            //     format!("{:?}", context_clone.swqos_type),
            //     format!("{:?}", context_clone.trade_type),
            //     context_clone.timestamp_ns as i64,
            //     context_clone.with_tip,
            //     context_clone.tip_amount,
            //     context_clone.to_base64(),
            // )
            // .execute(&pool)
            // .await?;

            Ok(())
        })
    }
}

/// 带数据库连接池的回调示例
///
/// 展示如何在回调中使用外部资源（如数据库连接池）
#[derive(Clone)]
struct DatabaseCallbackWithPool {
    // 在实际应用中，这里可以持有数据库连接池
    // pool: PgPool,
}

impl DatabaseCallbackWithPool {
    pub fn new() -> Self {
        Self {}
    }
}

impl TransactionLifecycleCallback for DatabaseCallbackWithPool {
    fn on_transaction_signed(&self, context: CallbackContext) -> BoxFuture<'static, Result<()>> {
        // pool = self.pool.clone();
        Box::pin(async move {
            println!(
                "[DatabaseWithPool] Saving transaction: {}",
                context.signature
            );

            // 示例：使用连接池保存到数据库
            // sqlx::query!(
            //     "INSERT INTO transactions (signature, swqos_type, trade_type, timestamp_ns) VALUES ($1, $2, $3, $4)",
            //     context.signature,
            //     format!("{:?}", context.swqos_type),
            //     format!("{:?}", context.trade_type),
            //     context.timestamp_ns as i64
            // )
            // .execute(&pool)
            // .await?;

            Ok(())
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== 交易生命周期回调示例 ===\n");

    // 配置钱包（请使用你自己的私钥）
    let payer = Arc::new(Keypair::new());

    // 配置 RPC
    let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    let commitment = CommitmentConfig::confirmed();

    // 配置 SWQOS 服务
    let swqos_configs = vec![SwqosConfig::Default(rpc_url.clone())];

    // 创建交易配置
    let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment);

    // 创建交易客户端
    let client = SolanaTrade::new(payer, trade_config).await;

    println!("--- 示例 1: 使用自定义数据库回调 ---");
    let custom_callback = Arc::new(MyDatabaseCallback);

    let buy_params = TradeBuyParams {
        dex_type: DexType::PumpSwap,
        input_token_type: TradeTokenType::SOL,
        mint: Pubkey::from_str("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn")?,
        input_token_amount: 100_000,
        slippage_basis_points: Some(100),
        recent_blockhash: Some(client.get_rpc().get_latest_blockhash().await?),
        extension_params: DexParamEnum::PumpSwap(
            PumpSwapParams::from_pool_address_by_rpc(
                client.get_rpc(),
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
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::new(),
        simulate: true, // 模拟模式，不实际发送交易
        on_transaction_signed: Some(custom_callback),
    };

    println!("执行买入交易（模拟模式）...");
    let (success, signatures, error) = client.buy(buy_params).await?;
    println!("结果: success={}, signatures={:?}", success, signatures);
    if let Some(err) = error {
        println!("错误: {:?}", err);
    }

    println!("\n--- 示例 2: 使用带连接池的数据库回调 ---");
    let callback_with_pool = Arc::new(DatabaseCallbackWithPool::new());

    let buy_params2 = TradeBuyParams {
        dex_type: DexType::PumpSwap,
        input_token_type: TradeTokenType::SOL,
        mint: Pubkey::from_str("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn")?,
        input_token_amount: 100_000,
        slippage_basis_points: Some(100),
        recent_blockhash: Some(client.get_rpc().get_latest_blockhash().await?),
        extension_params: DexParamEnum::PumpSwap(
            PumpSwapParams::from_pool_address_by_rpc(
                client.get_rpc(),
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
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::new(),
        simulate: true, // 模拟模式，不实际发送交易
        on_transaction_signed: Some(callback_with_pool),
    };

    println!("执行买入交易（模拟模式）...");
    let (success, signatures, error) = client.buy(buy_params2).await?;
    println!("结果: success={}, signatures={:?}", success, signatures);
    if let Some(err) = error {
        println!("错误: {:?}", err);
    }

    println!("\n--- 示例 3: 不使用回调（向后兼容）---");
    let buy_params3 = TradeBuyParams {
        dex_type: DexType::PumpSwap,
        input_token_type: TradeTokenType::SOL,
        mint: Pubkey::from_str("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn")?,
        input_token_amount: 100_000,
        slippage_basis_points: Some(100),
        recent_blockhash: Some(client.get_rpc().get_latest_blockhash().await?),
        extension_params: DexParamEnum::PumpSwap(
            PumpSwapParams::from_pool_address_by_rpc(
                client.get_rpc(),
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
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::new(),
        simulate: true, // 模拟模式，不实际发送交易
        on_transaction_signed: None, // 不使用回调
    };

    println!("执行买入交易（模拟模式，无回调）...");
    let (success, signatures, error) = client.buy(buy_params3).await?;
    println!("结果: success={}, signatures={:?}", success, signatures);
    if let Some(err) = error {
        println!("错误: {:?}", err);
    }

    println!("\n=== 示例完成 ===");
    Ok(())
}
