//! Raydium AMM V4 Swap 链上模拟验证测试
//!
//! 运行测试:
//!     cargo nextest run verify_raydium_amm_v4_exact_out_sell -- --nocapture

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::utils::raydium_amm_v4::{get_pool_by_address, quote_exact_out},
    trading::core::params::{RaydiumAmmV4Params, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::simulation_based_calc::{simulate_swap_transaction, verify_calculation_accuracy},
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;
use std::sync::Arc;

// 导入公共测试模块
mod common;
use common::{ensure_ata_with_balance, get_simulation_test_keypair};

/// WSOL-USDC Pool on Raydium AMM V4
const SOL_USDC_POOL: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";

/// WSOL Mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// USDC Mint
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

// ========================================
// Test 4: Exact Out Sell (指定 WSOL 数量)
// ========================================

#[tokio::test]
#[serial_test::serial(raydium_amm_v4_pool_tests)]
async fn test_raydium_amm_v4_exact_out_sell_with_simulation() {
    println!("====================================================");
    println!("Test 4: Raydium AMM V4 Exact Out Sell (指定 WSOL 数量)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(SOL_USDC_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let usdc_mint = Pubkey::from_str(USDC_MINT).unwrap();

    // 期望输出：100,000 WSOL (lamports)
    let amount_out = 100_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("期望输出: {} WSOL (lamports)", amount_out);
    println!("计算: 需要 USDC 输入\n");

    // 初始化 ATA
    if let Err(e) = ensure_ata_with_balance(&rpc, &rpc_url, &payer, &[(wsol_mint, None)], 1).await {
        println!("❌ 初始化失败: {}\n", e);
        return;
    }

    // 设置 USDC 余额
    if let Err(e) = common::set_token_balance(&rpc, &rpc_url, &payer, &usdc_mint, "10000").await {
        println!("❌ 设置 USDC 余额失败: {}\n", e);
        return;
    }

    // 获取 Pool 状态
    let pool_state = match get_pool_by_address(&rpc, &pool_address).await {
        Ok(state) => state,
        Err(e) => {
            println!("❌ 获取 Pool 失败: {}\n", e);
            return;
        },
    };

    // 获取 Token 信息和 decimals
    let (input_decimals, output_decimals) = match (
        sol_trade_sdk::utils::token::get_token_decimals(&rpc, &usdc_mint).await,
        sol_trade_sdk::utils::token::get_token_decimals(&rpc, &wsol_mint).await,
    ) {
        (Ok(d1), Ok(d2)) => (d1, d2),
        (e1, e2) => {
            println!("❌ 获取 decimals 失败: {:?}, {:?}\n", e1, e2);
            return;
        },
    };

    let amount_out_formatted = amount_out as f64 / 10_f64.powi(output_decimals as i32);
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                AMM V4 Swap 详细信息 - Exact Out Sell              ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();
    println!("📊 Pool 信息:");
    println!("  DEX: Raydium AMM V4 (经典 AMM)");
    println!("  Pool: {}", pool_address);
    println!();
    println!("💱 输入 Token:");
    println!("  Mint: {}", usdc_mint);
    println!("  Decimals: {}", input_decimals);
    println!();
    println!("💱 输出 Token:");
    println!("  Mint: {}", wsol_mint);
    println!("  Decimals: {}", output_decimals);
    println!("  期望数量: {} (最小单位)", amount_out);
    println!("  期望数量: {} (可读单位)", amount_out_formatted);
    println!();

    // 本地计算 (exact_out)
    let local_calc = match quote_exact_out(&rpc, &pool_address, amount_out, true).await {
        // true: pc -> coin (USDC 是 pc, WSOL 是 coin)
        Ok(result) => result,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (coin_mint, pc_mint) = (pool_state.coin_mint, pool_state.pc_mint);
    let coin_token_program =
        match sol_trade_sdk::utils::token::get_token_program_with_cache(&rpc, &coin_mint).await {
            Ok(program) => {
                println!("✅ 自动检测 coin_mint ({}) Token Program: {}", coin_mint, program);
                program
            },
            Err(e) => {
                println!("⚠️  无法获取 coin_mint Token Program，使用默认值: {}", e);
                spl_token::id()
            },
        };
    let pc_token_program =
        match sol_trade_sdk::utils::token::get_token_program_with_cache(&rpc, &pc_mint).await {
            Ok(program) => {
                println!("✅ 自动检测 pc_mint ({}) Token Program: {}", pc_mint, program);
                program
            },
            Err(e) => {
                println!("⚠️  无法获取 pc_mint Token Program，使用默认值: {}", e);
                spl_token::id()
            },
        };
    println!();

    // 获取储备余额
    let coin_balance = rpc.get_token_account_balance(&pool_state.token_coin).await;
    let pc_balance = rpc.get_token_account_balance(&pool_state.token_pc).await;

    let (coin_reserve, pc_reserve) = match (coin_balance, pc_balance) {
        (Ok(coin), Ok(pc)) => {
            let coin_amt = coin.amount.parse::<u64>().unwrap_or(0);
            let pc_amt = pc.amount.parse::<u64>().unwrap_or(0);
            (coin_amt, pc_amt)
        },
        _ => {
            println!("❌ 无法查询 Reserve\n");
            return;
        },
    };

    // 构造指令 (使用 fixed_output_amount)
    let amm_v4_params = RaydiumAmmV4Params {
        amm: pool_address,
        coin_mint: pool_state.coin_mint,
        pc_mint: pool_state.pc_mint,
        token_coin: pool_state.token_coin,
        token_pc: pool_state.token_pc,
        coin_reserve,
        pc_reserve,
    };

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: payer.clone(),
        trade_type: sol_trade_sdk::swqos::TradeType::Sell,
        input_mint: usdc_mint,
        input_token_program: Some(pc_token_program),
        output_mint: wsol_mint,
        output_token_program: Some(coin_token_program),
        input_amount: Some(local_calc.amount_in), // 使用计算出的输入
        slippage_basis_points: Some(1000),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumAmmV4(
            amm_v4_params,
        ),
        open_seed_optimize: false,
        swqos_clients: Vec::new(),
        middleware_manager: None,
        durable_nonce: None,
        with_tip: false,
        create_input_mint_ata: false,
        close_input_mint_ata: false,
        create_output_mint_ata: false,
        close_output_mint_ata: false,
        fixed_output_amount: Some(amount_out), // 关键：设置固定输出
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::default(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    let instructions =
        match sol_trade_sdk::instruction::raydium_amm_v4::RaydiumAmmV4InstructionBuilder
            .build_sell_instructions(&swap_params)
            .await
        {
            Ok(instrs) => instrs,
            Err(e) => {
                println!("❌ 构造指令失败: {}\n", e);
                return;
            },
        };

    let user_input_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &usdc_mint,
        &spl_token::id(),
    );
    let user_output_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &wsol_mint,
            &spl_token::id(),
        );

    // 链上模拟
    let simulation_result = match simulate_swap_transaction(
        &rpc,
        &payer,
        instructions,
        user_input_ata,
        user_output_ata,
        usdc_mint,
        wsol_mint,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            println!("❌ 模拟失败: {}\n", e);
            return;
        },
    };

    if !simulation_result.success {
        println!("❌ 模拟交易失败");
        if let Some(ref error) = simulation_result.error {
            println!("错误信息: {}\n", error);
        }
        if let Some(ref logs) = simulation_result.logs {
            println!("=== 交易日志 ===");
            for log in logs.iter().take(50) {
                println!("{}", log);
            }
            if logs.len() > 50 {
                println!("... (还有 {} 行)", logs.len() - 50);
            }
            println!("=================\n");
        }
        return;
    }

    let simulated_output = simulation_result.actual_output_amount;

    let simulated_output_formatted = simulated_output as f64 / 10_f64.powi(output_decimals as i32);

    // 结果对比
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│                           结果对比                                │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│                    │ 最小单位      │ 可读单位 (WSOL)             │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ 期望输出             │ {:>12} │ {:>20} │", amount_out, amount_out_formatted);
    println!(
        "│ 链上模拟             │ {:>12} │ {:>20} │",
        simulated_output, simulated_output_formatted
    );

    let diff = amount_out.abs_diff(simulated_output);
    let error_rate =
        if simulated_output > 0 { (diff as f64 / simulated_output as f64) * 100.0 } else { 0.0 };
    let diff_formatted = diff as f64 / 10_f64.powi(output_decimals as i32);

    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ 差值                 │ {:>12} │ {:>20} │", diff, diff_formatted);
    println!("│ 误差率               │ {:>12} │ {:>18.4}% │", "", error_rate);
    println!("└─────────────────────────────────────────────────────────────────┘");

    match verify_calculation_accuracy(amount_out, simulated_output, 1.0) {
        Ok(_) => println!("✅ 验证通过：误差 < 1%\n"),
        Err(e) => {
            println!("❌ 验证失败: {}\n", e);
            panic!("验证失败: {}", e);
        },
    }
}
