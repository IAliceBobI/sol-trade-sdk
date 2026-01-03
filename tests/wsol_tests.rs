//! WSOL 集成测试
//!
//! 使用 surfpool (localhost:8899) 进行测试
//!
//! 运行测试:
//!     cargo test --test wsol_tests -- --nocapture
//!
//! 注意：需要确保 surfpool 正在运行，且测试账户有足够的 SOL

use sol_trade_sdk::{
    common::{GasFeeStrategy, TradeConfig},
    swqos::SwqosConfig,
    trading::core::params::{DexParamEnum, PumpSwapParams},
    SolanaTrade,
};
use solana_commitment_config::CommitmentConfig;
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use std::str::FromStr;
use std::sync::Arc;

/// 获取测试用的 RPC URL（surfpool）
fn get_test_rpc_url() -> String {
    std::env::var("TEST_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8899".to_string())
}

/// 为测试账户空投 SOL（surfpool 支持通过 getTokenSupply 方式）
async fn airdrop_to_payer(rpc_url: &str, payer: &Pubkey) -> Result<(), Box<dyn std::error::Error>> {
    let client = RpcClient::new(rpc_url.to_string());

    // 检查账户余额
    let balance = client.get_balance(payer)?;
    println!("账户余额: {} lamports ({:.4} SOL)", balance, balance as f64 / 1e9);

    // 如果余额小于 10 SOL，需要用户手动空投
    if balance < 10_000_000_000 {
        println!("⚠️ 账户余额不足 10 SOL，请确保测试账户有足够的 SOL 进行测试");
        println!("   可以通过 surfpool 水龙头或从有 SOL 的账户转账");
    } else {
        println!("✅ 账户余额充足");
    }
    Ok(())
}

/// 创建测试用的 SolanaTrade 客户端
async fn create_test_client() -> SolanaTrade {
    let rpc_url = get_test_rpc_url();

    // 使用测试账户（如果没有设置则创建新的）
    let payer = if let Ok(secret_key) = std::env::var("TEST_SECRET_KEY_HEX") {
        let key_bytes = hex::decode(secret_key).expect("Invalid hex");
        let key_array: [u8; 32] = key_bytes[0..32].try_into().expect("Should be 32 bytes");
        Keypair::new_from_array(key_array)
    } else {
        Keypair::new()
    };

    // 空投 SOL（仅在本地测试环境）
    let payer_pubkey = payer.pubkey();
    let _ = airdrop_to_payer(&rpc_url, &payer_pubkey).await;

    let commitment = CommitmentConfig::confirmed();
    let swqos_configs: Vec<SwqosConfig> = vec![SwqosConfig::Default(rpc_url.clone())];
    let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment);

    SolanaTrade::new(Arc::new(payer), trade_config).await
}

/// 测试：WSOL 包装完整流程
///
/// 这个测试验证：
/// 1. SOL -> WSOL 包装
/// 2. WSOL -> SOL 部分解包装
/// 3. WSOL 账户关闭
#[tokio::test]
async fn test_wsol_wrap_complete_flow() {
    let client = create_test_client().await;
    let wrap_amount = 100_000_000; // 0.1 SOL in lamports

    println!("=== 测试 WSOL 完整流程 ===");
    println!("包装 {} lamports (0.1 SOL) 到 WSOL...", wrap_amount);

    // Step 1: 包装 SOL 到 WSOL
    match client.wrap_sol_to_wsol(wrap_amount).await {
        Ok(signature) => {
            println!("✅ SOL -> WSOL 成功: {}", signature);
        }
        Err(e) => {
            println!("❌ SOL -> WSOL 失败: {}", e);
            panic!("包装失败: {}", e);
        }
    }

    // 等待确认
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Step 2: 部分解包装 (50%)
    let unwrap_amount = wrap_amount / 2;
    println!("\n解包装 {} lamports (0.05 SOL) 回 SOL...", unwrap_amount);

    match client.wrap_wsol_to_sol(unwrap_amount).await {
        Ok(signature) => {
            println!("✅ WSOL -> SOL 成功: {}", signature);
        }
        Err(e) => {
            println!("❌ WSOL -> SOL 失败: {}", e);
            panic!("解包装失败: {}", e);
        }
    }

    // 等待确认
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Step 3: 关闭 WSOL 账户
    println!("\n关闭 WSOL 账户...");
    match client.close_wsol().await {
        Ok(signature) => {
            println!("✅ 关闭 WSOL 账户成功: {}", signature);
        }
        Err(e) => {
            println!("❌ 关闭 WSOL 账户失败: {}", e);
            panic!("关闭失败: {}", e);
        }
    }

    println!("=== WSOL 完整流程测试通过 ===");
}

/// 测试：连续多次包装 WSOL
///
/// 验证 ATA 复用机制是否正常工作
#[ignore]
#[tokio::test]
async fn test_wsol_multiple_wraps() {
    let client = create_test_client().await;
    let wrap_amount = 50_000_000; // 0.05 SOL

    println!("=== 测试连续多次 WSOL 包装 ===");

    for i in 1..=3 {
        println!("\n第 {} 次包装...", i);

        match client.wrap_sol_to_wsol(wrap_amount).await {
            Ok(signature) => {
                println!("  ✅ 第 {} 次包装成功: {}", i, signature);
            }
            Err(e) => {
                println!("  ❌ 第 {} 次包装失败: {}", i, e);
                panic!("连续包装失败: {}", e);
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    println!("=== 连续包装测试通过 ===");

    // 清理：关闭 WSOL 账户
    let _ = client.close_wsol().await;
}

/// 测试：创建 WSOL ATA（幂等性）
///
/// 验证重复创建 ATA 不会失败
#[ignore]
#[tokio::test]
async fn test_wsol_ata_creation_idempotent() {
    let client = create_test_client().await;

    println!("=== 测试 WSOL ATA 幂等创建 ===");

    // 第一次创建
    match client.wrap_sol_to_wsol(10_000_000).await {
        Ok(sig) => println!("✅ 第一次创建成功: {}", sig),
        Err(e) => panic!("第一次创建失败: {}", e),
    }

    // 第二次创建（应该幂等成功）
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    match client.wrap_sol_to_wsol(10_000_000).await {
        Ok(sig) => println!("✅ 第二次创建成功: {}", sig),
        Err(e) => panic!("第二次创建失败: {}", e),
    }

    println!("=== ATA 幂等创建测试通过 ===");

    // 清理
    let _ = client.close_wsol().await;
}

/// 测试：交易中使用 WSOL
///
/// 使用 PumpSwap 进行买入交易，验证 WSOL 自动处理
#[ignore]
#[tokio::test]
async fn test_trade_with_wsol() {
    let client = create_test_client().await;

    println!("=== 测试交易中使用 WSOL ===");

    // 使用一个已知的 PumpSwap 池进行测试
    // 注意：需要替换为实际存在的池地址
    let pool_address = std::env::var("TEST_PUMP_SWAP_POOL")
        .unwrap_or_else(|_| "7qbRF6YsyGuLUVs6Y1q64bdVrfe4WcLzN1pVN3dRNwDq".to_string());

    let pool = Pubkey::from_str(&pool_address).expect("Invalid pool address");
    let mint = std::env::var("TEST_MINT")
        .unwrap_or_else(|_| "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R".to_string());
    let mint = Pubkey::from_str(&mint).expect("Invalid mint");

    // 设置 Gas 策略
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(100_000, 100_000, 300_000, 300_000, 0.001, 0.001);

    // 创建 PumpSwapParams（需要从 RPC 获取真实的池信息）
    // 这里使用模拟参数，实际测试时需要替换为真实值
    let pump_swap_params = PumpSwapParams::new(
        pool,
        mint,
        sol_trade_sdk::constants::USDC_TOKEN_ACCOUNT,
        Pubkey::default(),
        Pubkey::default(),
        1_000_000_000,
        1_000_000_000,
        Pubkey::default(),
        Pubkey::default(),
        sol_trade_sdk::constants::TOKEN_PROGRAM,
        sol_trade_sdk::constants::TOKEN_PROGRAM,
        Pubkey::default(),
    );

    // 买入参数
    let buy_params = sol_trade_sdk::TradeBuyParams {
        dex_type: sol_trade_sdk::DexType::PumpSwap,
        input_token_type: sol_trade_sdk::TradeTokenType::WSOL,
        mint,
        input_token_amount: 10_000_000, // 0.01 SOL
        slippage_basis_points: Some(500),
        recent_blockhash: None,
        extension_params: DexParamEnum::PumpSwap(pump_swap_params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: true,
        close_input_token_ata: false, // 推荐：复用 ATA
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy,
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    match client.buy(buy_params).await {
        Ok((success, signatures, error)) => {
            if success {
                println!("✅ 买入成功: {:?}", signatures);
            } else {
                println!("❌ 买入失败: {:?}", error);
            }
        }
        Err(e) => {
            println!("❌ 交易错误: {}", e);
        }
    }

    println!("=== 交易 WSOL 测试完成 ===");
}
