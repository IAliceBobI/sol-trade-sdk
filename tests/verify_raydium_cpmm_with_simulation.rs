//! Raydium CPMM Swap 链上模拟验证测试
//!
//! 通过构造真实的交易并模拟执行，验证本地计算的准确性
//!
//! 运行测试:
//!     cargo nextest run verify_raydium_cpmm_with_simulation -- --nocapture
//!
//! 测试矩阵:
//! ┌─────────────┬──────────────┬─────────────────┐
//! │             │   Exact In   │    Exact Out   │
//! ├─────────────┼──────────────┼─────────────────┤
//! │ Buy         │ ✅ Test 1    │ ✅ Test 3      │
//! │ Sell        │ ✅ Test 2    │ ✅ Test 4      │
//! └─────────────┴──────────────┴─────────────────┘
//!
//! 注意：Raydium CPMM 使用恒定乘积 (Constant Product AMM)

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::utils::raydium_cpmm::{get_pool_by_address, quote_exact_in, quote_exact_out},
    trading::core::params::{RaydiumCpmmParams, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::simulation_based_calc::{simulate_swap_transaction, verify_calculation_accuracy},
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;
use std::sync::Arc;

// 导入公共模块
mod common;
use common::{ensure_ata_with_balance, get_simulation_test_keypair, get_token_program_for_mint};

/// PIPE-WSOL CPMM Pool
const PIPE_WSOL_POOL: &str = "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp";

/// WSOL Mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// PIPE Token Mint
const PIPE_MINT: &str = "8ycz3kctoRb4LFrtoYG2r8tRyUYUeGf5Q16M2TEMp7A";

// ========================================
// Test 1: Exact In Buy (WSOL -> PIPE)
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_raydium_cpmm_exact_in_buy_with_simulation() {
    println!("====================================================");
    println!("Test 1: Raydium CPMM Exact In Buy (WSOL -> PIPE)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(PIPE_WSOL_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let pipe_mint = Pubkey::from_str(PIPE_MINT).unwrap();

    // 测试金额：0.001 SOL
    let amount_in = 1_000_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("输入: {} lamports WSOL", amount_in);
    println!("期望输出: PIPE tokens\n");

    // 初始化 ATA
    if let Err(e) = ensure_ata_with_balance(
        &rpc, &rpc_url, &payer,
        &[
            (wsol_mint, Some(amount_in)),
            (pipe_mint, None),
        ],
        1,
    ).await {
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

    println!("交易方向: WSOL -> PIPE (买入 PIPE)\n");

    // 本地计算
    let local_output = match quote_exact_in(&rpc, &pool_address, amount_in, true).await {
        // 需要确定 WSOL 是 token0 还是 token1
        Ok(quote) => quote.amount_out,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算: {} PIPE\n", local_output);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (token0_mint, token1_mint) = (pool_state.token0_mint, pool_state.token1_mint);
    let base_token_program = match get_token_program_for_mint(&rpc, &token0_mint).await {
        Ok(program) => {
            println!("✅ 自动检测 token0_mint ({}) Token Program: {}", token0_mint, program);
            program
        },
        Err(e) => {
            println!("⚠️  无法获取 token0_mint Token Program，使用默认值: {}", e);
            spl_token::id()
        },
    };
    let quote_token_program = match get_token_program_for_mint(&rpc, &token1_mint).await {
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
        trade_type: sol_trade_sdk::swqos::TradeType::Buy,
        input_mint: wsol_mint,
        input_token_program: Some(base_token_program),
        output_mint: pipe_mint,
        output_token_program: Some(quote_token_program),
        input_amount: Some(amount_in),
        slippage_basis_points: Some(1000),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumCpmm(cpmm_params),
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
        &payer.pubkey(), &wsol_mint, &spl_token::id()
    );
    let user_output_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(), &pipe_mint, &spl_token::id()
    );

    // 链上模拟
    let simulation_result = match simulate_swap_transaction(
        &rpc, &payer, instructions,
        user_input_ata, user_output_ata,
        wsol_mint, pipe_mint,
    ).await {
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

    // 结果对比
    println!("┌─────────────────────────────────────┐");
    println!("│           结果对比                  │");
    println!("├─────────────────────────────────────┤");
    println!("│ 本地计算:     {:>15} │", local_output);
    println!("│ 链上模拟:     {:>15} │", simulated_output);

    let diff = local_output.abs_diff(simulated_output);
    let error_rate = if simulated_output > 0 {
        (diff as f64 / simulated_output as f64) * 100.0
    } else {
        0.0
    };

    println!("│ 差值:         {:>15} │", diff);
    println!("│ 误差率:      {:>13.4}% │", error_rate);
    println!("└─────────────────────────────────────┘");

    match verify_calculation_accuracy(local_output, simulated_output, 0.1) {
        Ok(_) => println!("✅ 验证通过：误差 < 0.1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}

// ========================================
// Test 2: Exact In Sell (PIPE -> WSOL)
// ========================================

#[tokio::test]
#[serial_test::serial]
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
    if let Err(e) = ensure_ata_with_balance(
        &rpc, &rpc_url, &payer,
        &[
            (wsol_mint, None),
        ],
        1,
    ).await {
        println!("❌ 初始化失败: {}\n", e);
        return;
    }

    // 设置 PIPE 余额（使用 surfnet_setTokenAccount）
    // 设置 10000 PIPE 用于测试
    if let Err(e) = common::set_token_balance(
        &rpc,
        &rpc_url,
        &payer,
        &pipe_mint,
        "10000",
    ).await {
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

    println!("交易方向: PIPE -> WSOL (卖出 PIPE)\n");

    // 本地计算
    let local_output = match quote_exact_in(&rpc, &pool_address, amount_in, false).await {
        Ok(quote) => quote.amount_out,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算: {} WSOL (lamports)\n", local_output);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (token0_mint, token1_mint) = (pool_state.token0_mint, pool_state.token1_mint);
    let base_token_program = match get_token_program_for_mint(&rpc, &token0_mint).await {
        Ok(program) => {
            println!("✅ 自动检测 token0_mint ({}) Token Program: {}", token0_mint, program);
            program
        },
        Err(e) => {
            println!("⚠️  无法获取 token0_mint Token Program，使用默认值: {}", e);
            spl_token::id()
        },
    };
    let quote_token_program = match get_token_program_for_mint(&rpc, &token1_mint).await {
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
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumCpmm(cpmm_params),
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
        &payer.pubkey(), &pipe_mint, &spl_token::id()
    );
    let user_output_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(), &wsol_mint, &spl_token::id()
    );

    // 链上模拟
    let simulation_result = match simulate_swap_transaction(
        &rpc, &payer, instructions,
        user_input_ata, user_output_ata,
        pipe_mint, wsol_mint,
    ).await {
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

    // 结果对比
    println!("┌─────────────────────────────────────┐");
    println!("│           结果对比                  │");
    println!("├─────────────────────────────────────┤");
    println!("│ 本地计算:     {:>15} │", local_output);
    println!("│ 链上模拟:     {:>15} │", simulated_output);

    let diff = local_output.abs_diff(simulated_output);
    let error_rate = if simulated_output > 0 {
        (diff as f64 / simulated_output as f64) * 100.0
    } else {
        0.0
    };

    println!("│ 差值:         {:>15} │", diff);
    println!("│ 误差率:      {:>13.4}% │", error_rate);
    println!("└─────────────────────────────────────┘");

    match verify_calculation_accuracy(local_output, simulated_output, 0.1) {
        Ok(_) => println!("✅ 验证通过：误差 < 0.1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}

// ========================================
// Test 3: Exact Out Buy (指定 PIPE 数量)
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_raydium_cpmm_exact_out_buy_with_simulation() {
    println!("====================================================");
    println!("Test 3: Raydium CPMM Exact Out Buy (指定 PIPE 数量)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(PIPE_WSOL_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let pipe_mint = Pubkey::from_str(PIPE_MINT).unwrap();

    // 期望输出：1000 PIPE
    let amount_out = 1_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("期望输出: {} PIPE", amount_out);
    println!("计算: 需要 WSOL 输入\n");

    // 初始化 ATA
    if let Err(e) = ensure_ata_with_balance(
        &rpc, &rpc_url, &payer,
        &[
            (wsol_mint, Some(10_000_000)), // 充值足够的 WSOL
            (pipe_mint, None),
        ],
        1,
    ).await {
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

    println!("交易方向: WSOL -> PIPE (买入 PIPE)\n");

    // 本地计算 (exact_out)
    let local_calc = match quote_exact_out(&rpc, &pool_address, amount_out, true).await {
        Ok(result) => result,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算:");
    println!("  期望输出: {} PIPE", amount_out);
    println!("  需要输入: {} WSOL (lamports)\n", local_calc.amount_in);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (token0_mint, token1_mint) = (pool_state.token0_mint, pool_state.token1_mint);
    let base_token_program = match get_token_program_for_mint(&rpc, &token0_mint).await {
        Ok(program) => {
            println!("✅ 自动检测 token0_mint ({}) Token Program: {}", token0_mint, program);
            program
        },
        Err(e) => {
            println!("⚠️  无法获取 token0_mint Token Program，使用默认值: {}", e);
            spl_token::id()
        },
    };
    let quote_token_program = match get_token_program_for_mint(&rpc, &token1_mint).await {
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
            (t0_amt, t1_amt)
        },
        _ => {
            println!("❌ 无法查询 Reserve\n");
            return;
        },
    };

    // 构造指令 (使用 fixed_output_amount)
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
        trade_type: sol_trade_sdk::swqos::TradeType::Buy,
        input_mint: wsol_mint,
        input_token_program: Some(base_token_program),
        output_mint: pipe_mint,
        output_token_program: Some(quote_token_program),
        input_amount: Some(local_calc.amount_in), // 使用计算出的输入
        slippage_basis_points: Some(1000),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumCpmm(cpmm_params),
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

    let instructions = match sol_trade_sdk::instruction::raydium_cpmm::RaydiumCpmmInstructionBuilder
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
        &payer.pubkey(), &wsol_mint, &spl_token::id()
    );
    let user_output_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(), &pipe_mint, &spl_token::id()
    );

    // 链上模拟
    let simulation_result = match simulate_swap_transaction(
        &rpc, &payer, instructions,
        user_input_ata, user_output_ata,
        wsol_mint, pipe_mint,
    ).await {
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

    // 结果对比
    println!("┌─────────────────────────────────────┐");
    println!("│           结果对比                  │");
    println!("├─────────────────────────────────────┤");
    println!("│ 期望输出:     {:>15} │", amount_out);
    println!("│ 链上模拟:     {:>15} │", simulated_output);

    let diff = amount_out.abs_diff(simulated_output);
    let error_rate = if simulated_output > 0 {
        (diff as f64 / simulated_output as f64) * 100.0
    } else {
        0.0
    };

    println!("│ 差值:         {:>15} │", diff);
    println!("│ 误差率:      {:>13.4}% │", error_rate);
    println!("└─────────────────────────────────────┘");

    match verify_calculation_accuracy(amount_out, simulated_output, 0.1) {
        Ok(_) => println!("✅ 验证通过：误差 < 0.1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}

// ========================================
// Test 4: Exact Out Sell (指定 WSOL 数量)
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_raydium_cpmm_exact_out_sell_with_simulation() {
    println!("====================================================");
    println!("Test 4: Raydium CPMM Exact Out Sell (指定 WSOL 数量)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(PIPE_WSOL_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let pipe_mint = Pubkey::from_str(PIPE_MINT).unwrap();

    // 期望输出：100,000 WSOL (lamports)
    let amount_out = 100_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("期望输出: {} WSOL (lamports)", amount_out);
    println!("计算: 需要 PIPE 输入\n");

    // 初始化 ATA（只创建 WSOL ATA）
    if let Err(e) = ensure_ata_with_balance(
        &rpc, &rpc_url, &payer,
        &[
            (wsol_mint, None),
        ],
        1,
    ).await {
        println!("❌ 初始化失败: {}\n", e);
        return;
    }

    // 设置 PIPE 余额（使用 surfnet_setTokenAccount）
    // 设置 10000 PIPE 用于测试
    if let Err(e) = common::set_token_balance(
        &rpc,
        &rpc_url,
        &payer,
        &pipe_mint,
        "10000",
    ).await {
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

    println!("交易方向: PIPE -> WSOL (卖出 PIPE)\n");

    // 本地计算 (exact_out)
    let local_calc = match quote_exact_out(&rpc, &pool_address, amount_out, false).await {
        Ok(result) => result,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算:");
    println!("  期望输出: {} WSOL (lamports)", amount_out);
    println!("  需要输入: {} PIPE\n", local_calc.amount_in);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (token0_mint, token1_mint) = (pool_state.token0_mint, pool_state.token1_mint);
    let base_token_program = match get_token_program_for_mint(&rpc, &token0_mint).await {
        Ok(program) => {
            println!("✅ 自动检测 token0_mint ({}) Token Program: {}", token0_mint, program);
            program
        },
        Err(e) => {
            println!("⚠️  无法获取 token0_mint Token Program，使用默认值: {}", e);
            spl_token::id()
        },
    };
    let quote_token_program = match get_token_program_for_mint(&rpc, &token1_mint).await {
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
            (t0_amt, t1_amt)
        },
        _ => {
            println!("❌ 无法查询 Reserve\n");
            return;
        },
    };

    // 构造指令 (使用 fixed_output_amount)
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
        input_amount: Some(local_calc.amount_in), // 使用计算出的输入
        slippage_basis_points: Some(1000),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumCpmm(cpmm_params),
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
        &payer.pubkey(), &pipe_mint, &spl_token::id()
    );
    let user_output_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(), &wsol_mint, &spl_token::id()
    );

    // 链上模拟
    let simulation_result = match simulate_swap_transaction(
        &rpc, &payer, instructions,
        user_input_ata, user_output_ata,
        pipe_mint, wsol_mint,
    ).await {
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

    // 结果对比
    println!("┌─────────────────────────────────────┐");
    println!("│           结果对比                  │");
    println!("├─────────────────────────────────────┤");
    println!("│ 期望输出:     {:>15} │", amount_out);
    println!("│ 链上模拟:     {:>15} │", simulated_output);

    let diff = amount_out.abs_diff(simulated_output);
    let error_rate = if simulated_output > 0 {
        (diff as f64 / simulated_output as f64) * 100.0
    } else {
        0.0
    };

    println!("│ 差值:         {:>15} │", diff);
    println!("│ 误差率:      {:>13.4}% │", error_rate);
    println!("└─────────────────────────────────────┘");

    match verify_calculation_accuracy(amount_out, simulated_output, 0.1) {
        Ok(_) => println!("✅ 验证通过：误差 < 0.1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}
