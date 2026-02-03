//! PumpSwap Swap 链上模拟验证测试 - Exact In Sell
//!
//! 通过构造真实的交易并模拟执行，验证本地计算的准确性
//!
//! 运行测试:
//!     cargo nextest run verify_pumpswap_exact_in_sell -- --nocapture

use sol_trade_sdk::{
    common::SolanaRpcClient,
    constants::TOKEN_PROGRAM,
    instruction::utils::pumpswap::{get_pool_by_address, quote_exact_in},
    trading::core::params::{PumpSwapParams, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::simulation_based_calc::{simulate_swap_transaction, verify_calculation_accuracy},
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;
use std::sync::Arc;

// 导入公共模块
mod common;
use common::{ensure_ata_with_balance, get_simulation_test_keypair};

/// PUMP Token Pool
const PUMP_POOL: &str = "539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR";

/// PUMP Token Mint
const PUMP_MINT: &str = "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn";

/// WSOL Mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

// ========================================
// Test 2: Exact In Sell (PUMP -> WSOL)
// ========================================

#[tokio::test]
#[serial_test::serial(pumpswap_pool_tests)]
async fn test_pumpswap_exact_in_sell_with_simulation() {
    println!("====================================================");
    println!("Test 2: PumpSwap Exact In Sell (PUMP -> WSOL)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(PUMP_POOL).unwrap();
    let pump_mint = Pubkey::from_str(PUMP_MINT).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();

    // 测试金额：卖出 1000 PUMP
    let amount_in = 1_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("输入: {} PUMP", amount_in);
    println!("期望输出: WSOL (lamports)\n");

    // 初始化 ATA（只创建 WSOL ATA）
    if let Err(e) = ensure_ata_with_balance(&rpc, &rpc_url, &payer, &[(wsol_mint, None)], 1).await {
        println!("❌ 初始化失败: {}\n", e);
        return;
    }

    // 设置 PUMP 余额（使用 surfnet_setTokenAccount）
    // 设置 10000 PUMP 用于测试
    if let Err(e) = common::set_token_balance(&rpc, &rpc_url, &payer, &pump_mint, "10000").await {
        println!("❌ 设置 PUMP 余额失败: {}\n", e);
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
        sol_trade_sdk::utils::token::get_token_decimals(&rpc, &pump_mint).await,
        sol_trade_sdk::utils::token::get_token_decimals(&rpc, &wsol_mint).await,
    ) {
        (Ok(d1), Ok(d2)) => (d1, d2),
        (e1, e2) => {
            println!("❌ 获取 decimals 失败: {:?}, {:?}\n", e1, e2);
            return;
        },
    };

    let input_formatted = amount_in as f64 / 10_f64.powi(input_decimals as i32);
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                 PumpSwap Swap 详细信息 - Exact In Sell             ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();
    println!("📊 Pool 信息:");
    println!("  DEX: PumpSwap (Bonding Curve)");
    println!("  Pool: {}", pool_address);
    println!();
    println!("💱 输入 Token:");
    println!("  Mint: {}", pump_mint);
    println!("  Decimals: {}", input_decimals);
    println!("  数量: {} (最小单位)", amount_in);
    println!("  数量: {} (可读单位)", input_formatted);
    println!();
    println!("💱 输出 Token:");
    println!("  Mint: {}", wsol_mint);
    println!("  Decimals: {}", output_decimals);
    println!();

    // 使用 quote_exact_in 进行本地计算
    let local_output = match sol_trade_sdk::instruction::utils::pumpswap::quote_exact_in(
        &rpc,
        &pool_address,
        amount_in,
        true,
    )
    .await
    {
        // true: base -> quote (卖出 PUMP)
        Ok(quote) => quote.amount_out,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算 (使用 quote_exact_in): {} WSOL (lamports)\n", local_output);

    // 获取储备余额
    let base_balance = rpc.get_token_account_balance(&pool_state.pool_base_token_account).await;
    let quote_balance = rpc.get_token_account_balance(&pool_state.pool_quote_token_account).await;

    let (base_reserve, quote_reserve) = match (base_balance, quote_balance) {
        (Ok(base), Ok(quote)) => {
            let base_amt = base.amount.parse::<u64>().unwrap_or(0);
            let quote_amt = quote.amount.parse::<u64>().unwrap_or(0);
            (base_amt, quote_amt)
        },
        _ => {
            println!("❌ 无法查询 Reserve\n");
            return;
        },
    };

    // 确定 base 和 quote mint
    let (base_mint, quote_mint) = if pool_state.base_mint.to_string() == WSOL_MINT {
        (pool_state.base_mint, pool_state.quote_mint)
    } else {
        (pool_state.quote_mint, pool_state.base_mint)
    };

    // 🔧 自动从 mint 获取 Token Program（不需要手动记忆）
    let base_token_program =
        match sol_trade_sdk::utils::token::get_token_program_with_cache(&rpc, &base_mint).await {
            Ok(program) => {
                println!("✅ 自动检测 base_mint ({}) Token Program: {}", base_mint, program);
                program
            },
            Err(e) => {
                println!("⚠️  无法获取 base_mint Token Program，使用默认值: {}", e);
                TOKEN_PROGRAM
            },
        };

    let quote_token_program =
        match sol_trade_sdk::utils::token::get_token_program_with_cache(&rpc, &quote_mint).await {
            Ok(program) => {
                println!("✅ 自动检测 quote_mint ({}) Token Program: {}", quote_mint, program);
                program
            },
            Err(e) => {
                println!("⚠️  无法获取 quote_mint Token Program，使用默认值: {}", e);
                TOKEN_PROGRAM
            },
        };

    // 构造指令
    let pumpswap_params = PumpSwapParams {
        pool: pool_address,
        base_mint,
        quote_mint,
        pool_base_token_account: pool_state.pool_base_token_account,
        pool_quote_token_account: pool_state.pool_quote_token_account,
        pool_base_token_reserves: base_reserve,
        pool_quote_token_reserves: quote_reserve,
        coin_creator_vault_ata: Pubkey::default(),
        coin_creator_vault_authority: Pubkey::default(),
        base_token_program,  // 自动获取
        quote_token_program, // 自动获取
        is_mayhem_mode: pool_state.is_mayhem_mode,
    };

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: payer.clone(),
        trade_type: sol_trade_sdk::swqos::TradeType::Sell,
        input_mint: pump_mint,
        input_token_program: Some(spl_token::id()),
        output_mint: wsol_mint,
        output_token_program: Some(spl_token::id()),
        input_amount: Some(amount_in),
        slippage_basis_points: Some(1000),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::PumpSwap(
            pumpswap_params,
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
        fixed_output_amount: None,
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::default(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    let instructions = match sol_trade_sdk::instruction::pumpswap::PumpSwapInstructionBuilder
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
        &pump_mint,
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
        pump_mint,
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

    let local_output_formatted = local_output as f64 / 10_f64.powi(output_decimals as i32);
    let simulated_output_formatted = simulated_output as f64 / 10_f64.powi(output_decimals as i32);

    // 结果对比
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│                           结果对比                                │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│                    │ 最小单位      │ 可读单位 (WSOL)             │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ 本地计算             │ {:>12} │ {:>20} │", local_output, local_output_formatted);
    println!(
        "│ 链上模拟             │ {:>12} │ {:>20} │",
        simulated_output, simulated_output_formatted
    );

    let diff = local_output.abs_diff(simulated_output);
    let error_rate =
        if simulated_output > 0 { (diff as f64 / simulated_output as f64) * 100.0 } else { 0.0 };
    let diff_formatted = diff as f64 / 10_f64.powi(output_decimals as i32);

    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ 差值                 │ {:>12} │ {:>20} │", diff, diff_formatted);
    println!("│ 误差率               │ {:>12} │ {:>18.4}% │", "", error_rate);
    println!("└─────────────────────────────────────────────────────────────────┘");

    match verify_calculation_accuracy(local_output, simulated_output, 0.1) {
        Ok(_) => println!("✅ 验证通过：误差 < 0.1%\n"),
        Err(e) => {
            println!("❌ 验证失败: {}\n", e);
            panic!("验证失败: {}", e);
        },
    }
}
