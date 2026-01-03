//! WSOL 集成测试
//!
//! 使用 surfpool (localhost:8899) 进行测试
//!
//! 运行测试:
//!     cargo test --test wsol_tests -- --nocapture
//!
//! 注意：需要确保 surfpool 正在运行

use sol_trade_sdk::{
    common::GasFeeStrategy,
    trading::core::params::{DexParamEnum, PumpSwapParams},
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;

mod test_helpers;
use test_helpers::{create_test_client, print_balances};

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
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let payer_pubkey = client.payer.try_pubkey().expect("Failed to get payer pubkey");

    println!("=== 测试 WSOL 完整流程 ===");
    println!("包装 {} lamports (0.1 SOL) 到 WSOL...", wrap_amount);

    // 打印初始余额
    let (sol_before, wsol_before) = print_balances(&rpc_url, &payer_pubkey).await.unwrap();

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

    // 打印包装后余额
    let (sol_after_wrap, wsol_after_wrap) = print_balances(&rpc_url, &payer_pubkey).await.unwrap();
    assert!(wsol_after_wrap > wsol_before, "WSOL 余额应该增加");
    assert!(sol_after_wrap < sol_before, "SOL 余额应该减少");

    // Step 2: 解包装
    println!("\n解包装 {} lamports (0.1 SOL) 回 SOL...", wrap_amount);

    match client.wrap_wsol_to_sol(wrap_amount).await {
        Ok(signature) => {
            println!("✅ WSOL -> SOL 成功: {}", signature);
        }
        Err(e) => {
            println!("❌ WSOL -> SOL 失败: {}", e);
            panic!("解包装失败: {}", e);
        }
    }

    // 打印解包装后余额
    let (sol_after_unwrap, wsol_after_unwrap) =
        print_balances(&rpc_url, &payer_pubkey).await.unwrap();
    assert!(sol_after_unwrap > sol_after_wrap, "SOL 余额应该增加");
    assert!(wsol_after_unwrap < wsol_after_wrap, "WSOL 余额应该减少");

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

    // 打印关闭后余额
    let (_, wsol_final) = print_balances(&rpc_url, &payer_pubkey).await.unwrap();
    assert_eq!(wsol_final, 0, "WSOL 账户关闭后余额应该为 0");

    println!("=== WSOL 完整流程测试通过 ===");
}

/// 测试：连续多次包装 WSOL
///
/// 验证 ATA 复用机制是否正常工作
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
#[tokio::test]
async fn test_trade_with_wsol() {
    // cargo test --package sol-trade-sdk --test wsol_tests -- test_trade_with_wsol --exact --nocapture
    let client = create_test_client().await;

    println!("=== 测试交易中使用 WSOL ===");

    // 使用一个已知的 PumpSwap 池进行测试
    let pool = Pubkey::from_str("539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR")
        .expect("Invalid pool address");

    // 从 RPC 获取真实的池信息（参考 single_buy.rs）
    let pump_swap_params = PumpSwapParams::from_pool_address_by_rpc(&client.rpc, &pool)
        .await
        .expect("Failed to fetch pool info from RPC");

    // 从 RPC 获取最新的 blockhash
    let recent_blockhash =
        client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

    // 设置 Gas 策略
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    print!("{:?}\n", pump_swap_params);
    assert_eq!(
        pump_swap_params.base_mint,
        Pubkey::from_str_const("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn")
    );

    // 买入参数
    let buy_params = sol_trade_sdk::TradeBuyParams {
        dex_type: sol_trade_sdk::DexType::PumpSwap,
        input_token_type: sol_trade_sdk::TradeTokenType::SOL,
        mint: pump_swap_params.base_mint,
        input_token_amount: 10_000_000, // 0.01 SOL
        slippage_basis_points: Some(500),
        recent_blockhash: Some(recent_blockhash),
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
