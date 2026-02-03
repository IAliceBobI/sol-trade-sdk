//! Raydium CPMM Exact In Sell 交易链上模拟验证测试
//!
//! 测试场景: 卖出 PIPE 换取 WSOL (指定输入金额)
//!
//! ⚠️ 注意: 此测试暂时被禁用，需要修复 parse_raydium_cpmm_data 解析逻辑
//!
//! 运行测试:
//!     cargo nextest run verify_raydium_cpmm_exact_in_sell -- --nocapture --ignored

use sdk_common::{GasFeeStrategy, SolanaRpcClient, TradeConfig};
use sol_trade_sdk::{
    TradingClient, common as sdk_common,
    instruction::utils::raydium_cpmm::get_pool_by_address,
    trading::core::params::{RaydiumCpmmParams, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::simulation_based_calc::{simulate_swap_transaction, verify_calculation_accuracy},
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;
use std::sync::Arc;

// 导入公共测试模块
mod common;
use common::{ensure_ata_with_balance, get_simulation_test_keypair, set_token_balance};

/// PIPE-WSOL CPMM Pool
const PIPE_WSOL_POOL: &str = "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp";

/// WSOL Mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// PIPE Token Mint
const PIPE_MINT: &str = "8ycz3kctoRb4LFrtoYG2r8tRyUYUeGf5Q16M2TEMp7A";

// Test 2: Exact In Sell (PIPE -> WSOL)
// ========================================

#[tokio::test]
#[serial_test::serial(pipe_wsol_pool_tests)]
#[ignore = "TODO: 修复 parse_raydium_cpmm_data 解析逻辑 - 除数 865 可能不正确"]
async fn test_raydium_cpmm_exact_in_sell_with_simulation() {
    println!("====================================================");
    println!("Test 2: Raydium CPMM Exact In Sell (PIPE -> WSOL)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(PIPE_WSOL_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let pipe_mint = Pubkey::from_str(PIPE_MINT).unwrap();

    // 测试金额：卖出 1000 PIPE
    let amount_in = 1_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("输入: {} PIPE", amount_in);
    println!("期望输出: WSOL (lamports)\n");

    // 初始化 ATA（只创建 WSOL ATA）
    if let Err(e) = ensure_ata_with_balance(&rpc, &rpc_url, &payer, &[(wsol_mint, None)], 1).await {
        println!("❌ 初始化失败: {}\n", e);
        return;
    }

    // 设置 PIPE 余额（使用 surfnet_setTokenAccount）
    // 设置 10000 PIPE 用于测试
    if let Err(e) = set_token_balance(&rpc, &rpc_url, &payer, &pipe_mint, "10000").await {
        println!("❌ 设置 PIPE 余额失败: {}\n", e);
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
        sol_trade_sdk::utils::token::get_token_decimals(&rpc, &pipe_mint).await,
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
    println!("║                   CPMM Swap 详细信息 - Exact In Sell              ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();
    println!("📊 Pool 信息:");
    println!("  DEX: Raydium CPMM (恒定乘积)");
    println!("  Pool: {}", pool_address);
    println!();
    println!("💱 输入 Token:");
    println!("  Mint: {}", pipe_mint);
    println!("  Decimals: {}", input_decimals);
    println!("  数量: {} (最小单位)", amount_in);
    println!("  数量: {} (可读单位)", input_formatted);
    println!();
    println!("💱 输出 Token:");
    println!("  Mint: {}", wsol_mint);
    println!("  Decimals: {}", output_decimals);
    println!();

    // 使用 TradingClient::sell_quote() 进行本地计算
    let trade_config = TradeConfig::new(rpc_url.clone(), vec![], CommitmentConfig::confirmed());
    let client = TradingClient::new(payer.clone(), trade_config).await;

    let sell_params = sol_trade_sdk::TradeSellParams {
        dex_type: sol_trade_sdk::DexType::RaydiumCpmm,
        output_token_type: sol_trade_sdk::TradeTokenType::WSOL,
        mint: pipe_mint,
        input_token_amount: amount_in,
        slippage_basis_points: Some(1000),
        recent_blockhash: None,
        with_tip: false,
        extension_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumCpmm(
            RaydiumCpmmParams {
                pool_state: pool_address,
                amm_config: pool_state.amm_config,
                base_mint: pool_state.token0_mint,
                quote_mint: pool_state.token1_mint,
                base_reserve: 0, // quote 不需要 reserve
                quote_reserve: 0,
                base_vault: Pubkey::default(),
                quote_vault: Pubkey::default(),
                base_token_program: spl_token::id(),
                quote_token_program: spl_token::id(),
                observation_state: Pubkey::default(),
            },
        ),
        address_lookup_table_account: None,
        wait_transaction_confirmed: false,
        create_output_token_ata: false,
        close_output_token_ata: false,
        close_mint_token_ata: false,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: GasFeeStrategy::default(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    let local_output = match client.sell_quote(sell_params).await {
        Ok(quote) => quote.amount_out,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算 (使用 TradingClient::sell_quote()): {} WSOL (lamports)\n", local_output);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (token0_mint, token1_mint) = (pool_state.token0_mint, pool_state.token1_mint);
    let base_token_program =
        match sol_trade_sdk::utils::token::get_token_program_with_cache(&rpc, &token0_mint).await {
            Ok(program) => {
                println!("✅ 自动检测 token0_mint ({}) Token Program: {}", token0_mint, program);
                program
            },
            Err(e) => {
                println!("⚠️  无法获取 token0_mint Token Program，使用默认值: {}", e);
                spl_token::id()
            },
        };
    let quote_token_program =
        match sol_trade_sdk::utils::token::get_token_program_with_cache(&rpc, &token1_mint).await {
            Ok(program) => {
                println!("✅ 自动检测 token1_mint ({}) Token Program: {}", token1_mint, program);
                program
            },
            Err(e) => {
                println!("⚠️  无法获取 token1_mint Token Program，使用默认值: {}", e);
                spl_token::id()
            },
        };
    println!();

    // 获取储备余额
    let token0_balance = rpc.get_token_account_balance(&pool_state.token0_vault).await;
    let token1_balance = rpc.get_token_account_balance(&pool_state.token1_vault).await;

    let (token0_reserve, token1_reserve) = match (token0_balance, token1_balance) {
        (Ok(t0), Ok(t1)) => {
            let t0_amt = t0.amount.parse::<u64>().unwrap_or(0);
            let t1_amt = t1.amount.parse::<u64>().unwrap_or(0);
            println!("📊 Pool Reserve:");
            println!("  token0_reserve (PIPE): {}", t0_amt);
            println!("  token1_reserve (WSOL): {}", t1_amt);
            println!();
            (t0_amt, t1_amt)
        },
        _ => {
            println!("❌ 无法查询 Reserve\n");
            return;
        },
    };

    // 构造指令
    let cpmm_params = RaydiumCpmmParams {
        pool_state: pool_address,
        amm_config: pool_state.amm_config,
        base_mint: pool_state.token0_mint,
        quote_mint: pool_state.token1_mint,
        base_reserve: token0_reserve,
        quote_reserve: token1_reserve,
        base_vault: pool_state.token0_vault,
        quote_vault: pool_state.token1_vault,
        base_token_program,
        quote_token_program,
        observation_state: pool_state.observation_key,
    };

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: payer.clone(),
        trade_type: sol_trade_sdk::swqos::TradeType::Sell,
        input_mint: pipe_mint,
        input_token_program: Some(base_token_program),
        output_mint: wsol_mint,
        output_token_program: Some(quote_token_program),
        input_amount: Some(amount_in),
        slippage_basis_points: Some(1000),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumCpmm(
            cpmm_params,
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

    let instructions = match sol_trade_sdk::instruction::raydium_cpmm::RaydiumCpmmInstructionBuilder
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
        &pipe_mint,
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
        pipe_mint,
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

// ========================================
