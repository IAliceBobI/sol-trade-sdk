//! Raydium AMM V4 Swap 链上模拟验证测试
//!
//! 运行测试:
//!     cargo nextest run verify_raydium_amm_v4_exact_in_buy -- --nocapture

use base64::{Engine, engine::general_purpose::STANDARD};
use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::utils::raydium_amm_v4::{get_pool_by_address, quote_exact_in},
    trading::core::params::{RaydiumAmmV4Params, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::simulation_based_calc::{simulate_swap_transaction, verify_calculation_accuracy},
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;
use std::sync::Arc;

// 导入公共测试模块
use sol_trade_test_utils::{ensure_token_balance, get_simulation_test_keypair};

/// WSOL-USDC Pool on Raydium AMM V4
const SOL_USDC_POOL: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";

/// WSOL Mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// USDC Mint
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

// ========================================
// Test 1: Exact In Buy (WSOL -> USDC)
// ========================================

#[tokio::test]
#[serial_test::serial(raydium_amm_v4_pool_tests)]
async fn test_raydium_amm_v4_exact_in_buy_with_simulation() {
    println!("====================================================");
    println!("Test 1: Raydium AMM V4 Exact In Buy (WSOL -> USDC)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(SOL_USDC_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let usdc_mint = Pubkey::from_str(USDC_MINT).unwrap();

    // 测试金额：0.001 SOL
    let amount_in = 1_000_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("输入: {} lamports WSOL", amount_in);
    println!("期望输出: USDC\n");

    // 初始化 ATA
    if let Err(e) = ensure_token_balance(
        &rpc,
        &rpc_url,
        &payer,
        &[(wsol_mint, Some(amount_in)), (usdc_mint, None)],
        1,
    )
    .await
    {
        println!("❌ 初始化失败: {}\n", e);
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
        sol_trade_sdk::utils::token::get_token_decimals(&rpc, &wsol_mint).await,
        sol_trade_sdk::utils::token::get_token_decimals(&rpc, &usdc_mint).await,
    ) {
        (Ok(d1), Ok(d2)) => (d1, d2),
        (e1, e2) => {
            println!("❌ 获取 decimals 失败: {:?}, {:?}\n", e1, e2);
            return;
        },
    };

    let input_formatted = amount_in as f64 / 10_f64.powi(input_decimals as i32);
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                  AMM V4 Swap 详细信息 - Exact In Buy              ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();
    println!("📊 Pool 信息:");
    println!("  DEX: Raydium AMM V4 (经典 AMM)");
    println!("  Pool: {}", pool_address);
    println!();
    println!("💱 输入 Token:");
    println!("  Mint: {}", wsol_mint);
    println!("  Decimals: {}", input_decimals);
    println!("  数量: {} (最小单位)", amount_in);
    println!("  数量: {} (可读单位)", input_formatted);
    println!();
    println!("💱 输出 Token:");
    println!("  Mint: {}", usdc_mint);
    println!("  Decimals: {}", output_decimals);
    println!();
    println!("🔍 Pool Token 方向:");
    println!("  pool_state.coin_mint: {}", pool_state.coin_mint);
    println!("  pool_state.pc_mint: {}", pool_state.pc_mint);
    println!("  WSOL mint: {}", wsol_mint);
    println!("  USDC mint: {}", usdc_mint);
    println!("  WSOL 是 coin: {}", pool_state.coin_mint == wsol_mint);
    println!("  USDC 是 pc: {}", pool_state.pc_mint == usdc_mint);
    println!();

    // 本地计算
    let local_output = match quote_exact_in(&rpc, &pool_address, amount_in, true).await {
        // is_coin_in=true: Coin -> PC (WSOL 是 Coin, USDC 是 PC)
        // ray_log 验证: direction=2 (Coin2PC), 输入 Coin，输出 PC
        Ok(quote) => quote.amount_out,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算: {} USDC (smallest unit)\n", local_output);

    // 🔍 检查指令构建器的 is_base_in 计算
    let is_base_in_builder = pool_state.coin_mint == sol_trade_sdk::constants::WSOL_TOKEN_ACCOUNT
        || pool_state.coin_mint == sol_trade_sdk::constants::USDC_TOKEN_ACCOUNT;
    println!("🔍 指令构建器的 is_base_in: {}", is_base_in_builder);
    println!("  quote_exact_in 使用的 is_coin_in: true");
    println!("  两者一致: {}", is_base_in_builder == true);
    println!();

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (coin_mint, pc_mint) = (pool_state.coin_mint, pool_state.pc_mint);
    let coin_token_program =
        match sol_trade_sdk::utils::token::get_token_program_with_cache(&rpc, &coin_mint).await {
            Ok(program) => {
                println!("✅ 自动检测 coin_mint ({}) Token Program: {}", coin_mint, program);
                program
            },
            Err(e) => {
                eprintln!("❌ 无法获取 coin_mint Token Program: {}", e);
                eprintln!("   测试无法继续，因为无法构建正确的指令");
                panic!("测试失败: {}", e);
            },
        };
    let pc_token_program =
        match sol_trade_sdk::utils::token::get_token_program_with_cache(&rpc, &pc_mint).await {
            Ok(program) => {
                println!("✅ 自动检测 pc_mint ({}) Token Program: {}", pc_mint, program);
                program
            },
            Err(e) => {
                eprintln!("❌ 无法获取 pc_mint Token Program: {}", e);
                eprintln!("   测试无法继续，因为无法构建正确的指令");
                panic!("测试失败: {}", e);
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
            println!("📊 实际储备余额:");
            println!("  coin_reserve (WSOL): {} lamports", coin_amt);
            println!("  pc_reserve (USDC): {} smallest unit", pc_amt);
            (coin_amt, pc_amt)
        },
        _ => {
            println!("❌ 无法查询 Reserve\n");
            return;
        },
    };

    // 构造指令
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
        trade_type: sol_trade_sdk::swqos::TradeType::Buy,
        input_mint: wsol_mint,
        input_token_program: Some(coin_token_program),
        output_mint: usdc_mint,
        output_token_program: Some(pc_token_program),
        input_amount: Some(amount_in),
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
        fixed_output_amount: None,
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::default(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    let instructions =
        match sol_trade_sdk::instruction::raydium_amm_v4::RaydiumAmmV4InstructionBuilder
            .build_buy_instructions(&swap_params)
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
        &wsol_mint,
        &spl_token::id(),
    );
    let user_output_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &usdc_mint,
            &spl_token::id(),
        );

    // 链上模拟
    let simulation_result = match simulate_swap_transaction(
        &rpc,
        &payer,
        instructions,
        user_input_ata,
        user_output_ata,
        wsol_mint,
        usdc_mint,
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

    // 解析 ray_log 获取实际的储备金和输出
    if let Some(ref logs) = simulation_result.logs {
        for log in logs.iter() {
            if log.starts_with("Program log: ray_log:") {
                let encoded = log.split("Program log: ray_log: ").nth(1).unwrap_or("");
                if let Ok(decoded) = STANDARD.decode(encoded) {
                    if decoded.len() >= 57 {
                        // 按照 SwapBaseInLog 结构解析
                        let pool_coin_from_log =
                            u64::from_le_bytes(decoded[33..41].try_into().unwrap());
                        let pool_pc_from_log =
                            u64::from_le_bytes(decoded[41..49].try_into().unwrap());
                        let out_amount_from_log =
                            u64::from_le_bytes(decoded[49..57].try_into().unwrap());

                        println!("📊 Ray_log 中的实际数据:");
                        println!("  pool_coin (实际): {}", pool_coin_from_log);
                        println!("  pool_pc (实际): {}", pool_pc_from_log);
                        println!("  out_amount (链上计算): {}", out_amount_from_log);
                        println!();
                    }
                }
            }
        }
    }

    let simulated_output = simulation_result.actual_output_amount;

    // 从 ray_log 中获取实际的链上输出（更准确）
    let actual_chain_output = if let Some(ref logs) = simulation_result.logs {
        let mut found_output = None;
        for log in logs.iter() {
            if log.starts_with("Program log: ray_log:") {
                let encoded = log.split("Program log: ray_log: ").nth(1).unwrap_or("");
                if let Ok(decoded) = STANDARD.decode(encoded) {
                    if decoded.len() >= 57 {
                        let out_amount = u64::from_le_bytes(decoded[49..57].try_into().unwrap());
                        found_output = Some(out_amount);
                        break;
                    }
                }
            }
        }
        found_output
    } else {
        None
    };

    // 🔍 检查 PNL 值
    println!("🔍 PNL 分析:");
    println!("  need_take_pnl_coin: {}", pool_state.out_put.need_take_pnl_coin);
    println!("  need_take_pnl_pc: {}", pool_state.out_put.need_take_pnl_pc);

    let coin_total_without_pnl = coin_reserve
        .checked_sub(pool_state.out_put.need_take_pnl_coin)
        .unwrap_or(coin_reserve);
    let pc_total_without_pnl = pc_reserve
        .checked_sub(pool_state.out_put.need_take_pnl_pc)
        .unwrap_or(pc_reserve);

    println!("  计算的 total_without_take_pnl:");
    println!("    coin: {}", coin_total_without_pnl);
    println!("    pc: {}", pc_total_without_pnl);
    println!();

    // 使用 ray_log 中的实际输出作为基准
    let baseline_output = actual_chain_output.unwrap_or(simulated_output);

    let local_output_formatted = local_output as f64 / 10_f64.powi(output_decimals as i32);
    let baseline_output_formatted = baseline_output as f64 / 10_f64.powi(output_decimals as i32);

    // 结果对比
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│                           结果对比                                │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│                    │ 最小单位    │ 可读单位 (USDC)              │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ 本地计算             │ {:>12} │ {:>20} │", local_output, local_output_formatted);
    if actual_chain_output.is_some() {
        println!(
            "│ 链上实际 (ray_log)    │ {:>12} │ {:>20} │",
            baseline_output, baseline_output_formatted
        );
    } else {
        println!(
            "│ 链上模拟             │ {:>12} │ {:>20} │",
            baseline_output, baseline_output_formatted
        );
    }

    let diff = local_output.abs_diff(baseline_output);
    let error_rate =
        if baseline_output > 0 { (diff as f64 / baseline_output as f64) * 100.0 } else { 0.0 };
    let diff_formatted = diff as f64 / 10_f64.powi(output_decimals as i32);

    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│ 差值                 │ {:>12} │ {:>20} │", diff, diff_formatted);
    println!("│ 误差率               │ {:>12} │ {:>18.4}% │", "", error_rate);
    println!("└─────────────────────────────────────────────────────────────────┘");

    match verify_calculation_accuracy(local_output, baseline_output, 1.0) {
        Ok(_) => println!("✅ 验证通过：误差 < 1.0%\n"),
        Err(e) => {
            println!("❌ 验证失败: {}\n", e);
            // 调试：打印详细信息
            println!("📊 调试信息:");
            println!("  local_output: {}", local_output);
            println!("  baseline_output: {}", baseline_output);
            println!("  inner_instructions: {:?}", simulation_result.inner_instructions);
            println!("  logs (前30行):");
            if let Some(ref logs) = simulation_result.logs {
                for (i, log) in logs.iter().take(30).enumerate() {
                    println!("    [{:03}] {}", i, log);
                }
            }
            println!();
            panic!("验证失败: {}", e);
        },
    }
}
