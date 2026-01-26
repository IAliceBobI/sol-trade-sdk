use sol_trade_sdk::{
    common::{AnyResult, TradeConfig},
    swqos::{SwqosConfig, SwqosRegion},
    trading::{core::params::PumpFunParams, factory::DexType},
    SolanaTrade, TradeBuyParams, TradeTokenType,
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> AnyResult<()> {
    env_logger::init();

    println!("\n🚀 初始化 PumpFun 交易客户端...\n");

    // 生成临时测试密钥对
    let payer_key = env::var("PAYER_KEYPAIR").unwrap_or_else(|_| "GENERATE_NEW".to_string());
    let payer = if payer_key == "GENERATE_NEW" {
        println!("📝 生成临时测试密钥对...");
        Keypair::new()
    } else {
        Keypair::from_base58_string(&payer_key)
    };
    println!("📝 钱包地址: {}", payer.pubkey());

    let rpc_url = env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string());
    let commitment = CommitmentConfig::confirmed();

    // 配置4个SWQOS节点并发发送
    let swqos_configs: Vec<SwqosConfig> = vec![
        SwqosConfig::Jito(
            String::new(), // uuid
            SwqosRegion::Default,
            Some(env::var("SWQOS_JITO").unwrap_or_else(|_| "http://127.0.0.1:8899".to_string())),
        ),
        SwqosConfig::Bloxroute(
            String::new(), // api_token
            SwqosRegion::Default,
            Some(
                env::var("SWQOS_BLOXROUTE")
                    .unwrap_or_else(|_| "https://ny.solana.dex.blxrbdn.com".to_string()),
            ),
        ),
        SwqosConfig::NextBlock(
            String::new(), // api_token
            SwqosRegion::Default,
            Some(
                env::var("SWQOS_NEXTBLOCK")
                    .unwrap_or_else(|_| "https://api.nextblock.io/v1/solana".to_string()),
            ),
        ),
        SwqosConfig::FlashBlock(
            String::new(), // api_token
            SwqosRegion::Default,
            Some(
                env::var("SWQOS_FLASHBLOCK")
                    .unwrap_or_else(|_| "https://api.flashblock.io/v1/solana".to_string()),
            ),
        ),
    ];

    println!("🚀 SWQOS配置: {} 个并发节点", swqos_configs.len());

    let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment);
    let client = SolanaTrade::new(Arc::new(payer), trade_config).await;

    // 设置 PumpFun 的 gas 策略
    let gas_fee_strategy = sol_trade_sdk::common::GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(
        200000,  // buy_cu_limit
        1000000, // sell_cu_limit
        500000,  // buy_cu_price
        500000,  // sell_cu_price
        0.005,   // buy_tip
        0.01,    // sell_tip
    );

    println!("✅ 客户端初始化完成\n");

    let mint_str = env::var("TEST_MINT").expect("TEST_MINT not set");
    let mint = mint_str.parse().expect("Invalid mint address");
    let buy_amount = env::var("BUY_AMOUNT")
        .unwrap_or_else(|_| "1000000".to_string())
        .parse::<u64>()
        .expect("Invalid buy amount");
    let slippage = env::var("SLIPPAGE")
        .unwrap_or_else(|_| "1000".to_string())
        .parse::<u64>()
        .expect("Invalid slippage");

    println!("🔍 获取最新区块哈希...");
    let recent_blockhash = client.rpc.get_latest_blockhash().await?;
    println!("✅ 区块哈希: {}\n", recent_blockhash);

    println!("================================");
    println!("  PumpFun 买入延迟测试");
    println!("================================");
    println!("🪙 代币: {}", mint);
    println!("💰 金额: {} lamports", buy_amount);
    println!("📊 滑点: {} basis points", slippage);
    println!("================================\n");

    // PumpFun买入参数 (买入不需要特殊参数，使用零值)
    let params = PumpFunParams::from_trade(
        Pubkey::default(),                       // bonding_curve
        Pubkey::default(),                       // associated_bonding_curve
        mint,                                    // mint
        Pubkey::default(),                       // creator
        Pubkey::default(),                       // creator_vault
        0,                                       // virtual_token_reserves
        0,                                       // virtual_sol_reserves
        0,                                       // real_token_reserves
        0,                                       // real_sol_reserves
        None,                                    // close_token_account_when_sell
        Pubkey::default(),                       // fee_recipient
        sol_trade_sdk::constants::TOKEN_PROGRAM, // token_program
    )?;

    let buy_params = TradeBuyParams {
        dex_type: DexType::PumpFun,
        input_token_type: TradeTokenType::SOL,
        mint,
        input_token_amount: buy_amount,
        slippage_basis_points: Some(slippage),
        recent_blockhash: Some(recent_blockhash),
        extension_params: sol_trade_sdk::trading::core::params::DexParamEnum::PumpFun(params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: false, // 不等待确认，测试最快提交速度
        create_input_token_ata: true,
        close_input_token_ata: true,
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy.clone(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    println!("⏱️  开始执行买入流程...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    match client.buy(buy_params).await {
        Ok((success, signatures, _error)) => {
            println!("\n================================");
            println!("  ✅ 买入流程完成");
            println!("================================");
            println!("✅ 提交成功: {}", success);
            println!("📝 签名数量: {}", signatures.len());
            for (i, sig) in signatures.iter().enumerate() {
                println!("📝 签名 #{}: {}", i + 1, sig);
            }
            println!("================================\n");
        },
        Err(e) => {
            println!("\n================================");
            println!("  ⚠️  买入流程完成（交易失败）");
            println!("================================");
            println!("ℹ️  错误: {:?}", e);
            println!("\n💡 说明: 交易失败是预期的（测试账户无余额）");
            println!("   耗时统计见上方SDK日志输出");
            println!("================================\n");
        },
    }

    // 显示性能统计
    println!("================================");
    println!("  性能优化模块状态");
    println!("================================\n");

    use sol_trade_sdk::swqos::serialization::get_serializer_stats;
    let (available, capacity) = get_serializer_stats();
    println!("📦 序列化器缓冲池:");
    println!("   容量: {}", capacity);
    println!("   可用: {}", available);
    println!("   使用: {}", capacity - available);

    use sol_trade_sdk::trading::core::transaction_pool::get_pool_stats;
    let (pool_available, pool_capacity) = get_pool_stats();
    println!("\n🔧 交易构建器池:");
    println!("   容量: {}", pool_capacity);
    println!("   可用: {}", pool_available);
    println!("   使用: {}", pool_capacity - pool_available);

    println!("\n================================");
    println!("✅ 延迟测试完成！");
    println!("================================\n");

    println!("💡 提示: 查看上面的日志了解各环节详细耗时");
    println!("   日志中包含每个步骤的 step 和 total 时间\n");

    Ok(())
}
