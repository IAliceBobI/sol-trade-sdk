//! 测试便捷的 Ensure 函数
//!
//! 展示如何使用 ensure_sol_balance, ensure_token_balance, ensure_cpmm_liquidity

use sol_trade_sdk::common::SolanaRpcClient;
use solana_sdk::signer::Signer;
use std::sync::Arc;

// 导入公共测试模块
use sol_trade_test_utils::{get_simulation_test_keypair, ensure_sol_balance, ensure_token_balance};

// 导入 CPMM 测试参数工具
use sol_trade_test_utils::{pipe_mint, pipe_wsol_pool, wsol_mint};

#[tokio::test]
#[serial_test::serial]
async fn test_ensure_sol_balance() {
    println!("\n========================================");
    println!("测试: ensure_sol_balance");
    println!("========================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));
    let payer = Arc::new(get_simulation_test_keypair());

    // 确保至少有 10 SOL
    ensure_sol_balance(&rpc, &rpc_url, &payer.pubkey(), 10)
        .await
        .expect("ensure_sol_balance 失败");

    println!("\n✅ 测试通过\n");
}

#[tokio::test]
#[serial_test::serial]
async fn test_ensure_token_balance() {
    println!("\n========================================");
    println!("测试: ensure_token_balance");
    println!("========================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));
    let payer = Arc::new(get_simulation_test_keypair());

    // 确保至少有 1000 PIPE
    ensure_token_balance(&rpc, &rpc_url, &payer, &pipe_mint(), "1000")
        .await
        .expect("ensure_token_balance 失败");

    // 确保至少有 100 WSOL
    ensure_token_balance(&rpc, &rpc_url, &payer, &wsol_mint(), "100")
        .await
        .expect("ensure_token_balance 失败");

    println!("\n✅ 测试通过\n");
}

#[tokio::test]
#[serial_test::serial]
async fn test_ensure_cpmm_liquidity_full_flow() {
    println!("\n========================================");
    println!("测试: 完整流程 - 确保余额和流动性");
    println!("========================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));
    let payer = Arc::new(get_simulation_test_keypair());

    // 1. 确保 SOL 余额
    println!("步骤 1: 确保 SOL 余额");
    ensure_sol_balance(&rpc, &rpc_url, &payer.pubkey(), 10)
        .await
        .expect("ensure_sol_balance 失败");

    // 2. 确保 Token 余额
    println!("\n步骤 2: 确保 Token 余额");
    ensure_token_balance(&rpc, &rpc_url, &payer, &pipe_mint(), "10000")
        .await
        .expect("ensure_token_balance 失败");

    ensure_token_balance(&rpc, &rpc_url, &payer, &wsol_mint(), "100")
        .await
        .expect("ensure_token_balance 失败");

    // 3. 添加流动性（使用 cpmm_test_params 中的构建器）
    println!("\n步骤 3: 添加流动性");
    use sol_trade_sdk::instruction::utils::raydium_cpmm::get_pool_by_address;
    use sol_trade_sdk::liquidity::cpmm::{build_deposit_instruction, CpmmDepositParams};
    use solana_sdk::{signer::Signer, transaction::Transaction};

    let pool_address = pipe_wsol_pool();

    // 获取池子状态
    let pool_state = get_pool_by_address(&rpc, &pool_address)
        .await
        .expect("获取池子失败");

    // 使用构建器
    let (deposit_instruction, _calculated, _owner_lp_token) =
        cpmm_test_params::PipeWsolLiquidityBuilder::new(1_000_000_000) // 10 亿 LP
            .max_pipe(12_000_000_000_000_000)
            .max_wsol(100_000_000_000)
            .build_instruction(
                payer.pubkey(),
                &pool_state,
                pool_state.lp_supply,
                pool_state.lp_supply,
            );

    println!("📝 Deposit 指令已构建");

    // 检查 LP ATA
    let owner_lp_token = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &pool_state.lp_mint,
        &spl_token::id(),
    );

    let lp_ata_exists = rpc.get_account(&owner_lp_token).await.is_ok();

    let mut instructions = Vec::new();

    if !lp_ata_exists {
        println!("📝 创建 LP ATA");
        let create_lp_ata_instruction =
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer.pubkey(),
                &payer.pubkey(),
                &pool_state.lp_mint,
                &spl_token::id(),
            );
        instructions.push(create_lp_ata_instruction);
    }

    instructions.push(deposit_instruction);

    // 发送交易
    println!("🚀 发送交易...");

    let blockhash = rpc.get_latest_blockhash().await.expect("获取 blockhash 失败");
    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    match rpc.send_and_confirm_transaction(&transaction).await {
        Ok(signature) => {
            println!("✅ 交易成功: {}", signature);
        },
        Err(e) => {
            println!("❌ 交易失败: {}", e);
        },
    }

    println!("\n✅ 测试完成\n");
}
