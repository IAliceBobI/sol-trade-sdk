//! CLMM Swap 链上模拟验证测试
//!
//! 通过构造真实的交易并模拟执行，验证本地计算的准确性
//!
//! 运行测试:
//!     cargo nextest run verify_clmm_with_simulation -- --nocapture
//!
//! 测试矩阵:
//! ┌─────────────┬──────────────┬─────────────────┐
//! │             │   Exact In   │    Exact Out   │
//! ├─────────────┼──────────────┼─────────────────┤
//! │ Buy         │ ✅ Test 1    │ ✅ Test 3      │
//! │ Sell        │ ✅ Test 2    │ ✅ Test 4      │
//! └─────────────┴──────────────┴─────────────────┘

use sol_trade_sdk::{
    TradingClient,
    common::{GasFeeStrategy, SolanaRpcClient, TradeConfig},
    instruction::utils::raydium_clmm::{get_pool_by_address, quote_exact_in, quote_exact_out},
    trading::core::params::{RaydiumClmmParams, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::simulation_based_calc::{simulate_swap_transaction, verify_calculation_accuracy},
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;
use std::sync::Arc;

// 导入公共模块
mod common;
use common::{ensure_ata_with_balance, get_simulation_test_keypair};

/// WSOL-JUP CLMM Pool
const WSOL_JUP_POOL: &str = "EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw";

/// WSOL Mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// JUP Mint
const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

// ========================================
// Test 1: Exact In Buy (WSOL -> JUP)
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_clmm_exact_in_buy_with_simulation() {
    println!("====================================================");
    println!("Test 1: CLMM Exact In Buy (WSOL -> JUP)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(WSOL_JUP_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let jup_mint = Pubkey::from_str(JUP_MINT).unwrap();

    // 测试金额：0.001 SOL
    let amount_in = 1_000_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("输入: {} lamports WSOL", amount_in);
    println!("期望输出: JUP tokens\n");

    // 初始化 ATA
    if let Err(e) = ensure_ata_with_balance(
        &rpc,
        &rpc_url,
        &payer,
        &[(wsol_mint, Some(amount_in)), (jup_mint, None)],
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

    // 判断方向：WSOL -> JUP (buy)
    let zero_for_one = wsol_mint.to_string() == pool_state.token_mint0.to_string();

    println!("交易方向: zero_for_one = {}", zero_for_one);
    println!(
        "含义: {} -> {}\n",
        if zero_for_one { "token0" } else { "token1" },
        if zero_for_one { "token1" } else { "token0" }
    );

    // 本地计算
    let local_output = match quote_exact_in(&rpc, &pool_address, amount_in, zero_for_one).await {
        Ok(quote) => quote.amount_out,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算: {} JUP\n", local_output);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (token0_mint, token1_mint) = (pool_state.token_mint0, pool_state.token_mint1);
    let token0_program =
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
    let token1_program =
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

    // 构造指令
    let clmm_params = RaydiumClmmParams {
        pool_state: pool_address,
        amm_config: pool_state.amm_config,
        token0_mint: pool_state.token_mint0,
        token1_mint: pool_state.token_mint1,
        token0_vault: pool_state.token_vault0,
        token1_vault: pool_state.token_vault1,
        observation_state: pool_state.observation_key,
        token0_decimals: pool_state.mint_decimals0,
        token1_decimals: pool_state.mint_decimals1,
        token0_program,
        token1_program,
    };

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: payer.clone(),
        trade_type: sol_trade_sdk::swqos::TradeType::Buy,
        input_mint: wsol_mint,
        input_token_program: Some(spl_token::id()),
        output_mint: jup_mint,
        output_token_program: Some(spl_token::id()),
        input_amount: Some(amount_in),
        slippage_basis_points: Some(1000),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumClmm(
            clmm_params,
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

    let instructions = match sol_trade_sdk::instruction::raydium_clmm::RaydiumClmmInstructionBuilder
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
            &jup_mint,
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
        jup_mint,
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
                // 只显示前 50 行
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
    let error_rate =
        if simulated_output > 0 { (diff as f64 / simulated_output as f64) * 100.0 } else { 0.0 };

    println!("│ 差值:         {:>15} │", diff);
    println!("│ 误差率:      {:>13.4}% │", error_rate);
    println!("└─────────────────────────────────────┘");

    match verify_calculation_accuracy(local_output, simulated_output, 1.0) {
        Ok(_) => println!("✅ 验证通过：误差 < 1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}

// ========================================
// Test 2: Exact In Sell (JUP -> WSOL)
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_clmm_exact_in_sell_with_simulation() {
    println!("====================================================");
    println!("Test 2: CLMM Exact In Sell (JUP -> WSOL)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(WSOL_JUP_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let jup_mint = Pubkey::from_str(JUP_MINT).unwrap();

    // 测试金额：0.001 SOL (卖出 JUP)
    let amount_in = 1_000_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("输入: {} lamports (期望等值的 JUP)", amount_in);
    println!("期望输出: WSOL tokens\n");

    // 初始化 ATA（只创建 WSOL ATA）
    if let Err(e) = ensure_ata_with_balance(&rpc, &rpc_url, &payer, &[(wsol_mint, None)], 1).await {
        println!("❌ 初始化失败: {}\n", e);
        return;
    }

    // 设置 JUP 余额（使用 surfnet_setTokenAccount）
    // 设置 1 JUP 用于测试
    if let Err(e) = common::set_token_balance(&rpc, &rpc_url, &payer, &jup_mint, "1").await {
        println!("❌ 设置 JUP 余额失败: {}\n", e);
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

    // 判断方向：JUP -> WSOL (sell)
    // zero_for_one = true 表示 token0 -> token1
    // 如果 JUP 是 token0，卖出 JUP 就是 token0 -> token1
    let zero_for_one = jup_mint.to_string() == pool_state.token_mint0.to_string();

    println!("交易方向: zero_for_one = {}", zero_for_one);
    println!(
        "含义: {} -> {}\n",
        if zero_for_one { "token0" } else { "token1" },
        if zero_for_one { "token1" } else { "token0" }
    );

    // 使用 TradingClient::sell_quote() 进行本地计算
    let trade_config = TradeConfig::new(rpc_url.clone(), vec![], CommitmentConfig::confirmed());
    let client = TradingClient::new(payer.clone(), trade_config).await;

    let sell_params = sol_trade_sdk::TradeSellParams {
        dex_type: sol_trade_sdk::DexType::RaydiumClmm,
        output_token_type: sol_trade_sdk::TradeTokenType::WSOL,
        mint: jup_mint,
        input_token_amount: amount_in,
        slippage_basis_points: Some(1000),
        recent_blockhash: None,
        with_tip: false,
        extension_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumClmm(
            RaydiumClmmParams {
                pool_state: pool_address,
                amm_config: pool_state.amm_config,
                token0_mint: pool_state.token_mint0,
                token1_mint: pool_state.token_mint1,
                token0_vault: pool_state.token_vault0,
                token1_vault: pool_state.token_vault1,
                observation_state: pool_state.observation_key,
                token0_decimals: pool_state.mint_decimals0,
                token1_decimals: pool_state.mint_decimals1,
                token0_program: spl_token::id(),
                token1_program: spl_token::id(),
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
    let (token0_mint, token1_mint) = (pool_state.token_mint0, pool_state.token_mint1);
    let token0_program =
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
    let token1_program =
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

    // 构造指令
    let clmm_params = RaydiumClmmParams {
        pool_state: pool_address,
        amm_config: pool_state.amm_config,
        token0_mint: pool_state.token_mint0,
        token1_mint: pool_state.token_mint1,
        token0_vault: pool_state.token_vault0,
        token1_vault: pool_state.token_vault1,
        observation_state: pool_state.observation_key,
        token0_decimals: pool_state.mint_decimals0,
        token1_decimals: pool_state.mint_decimals1,
        token0_program,
        token1_program,
    };

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: payer.clone(),
        trade_type: sol_trade_sdk::swqos::TradeType::Sell,
        input_mint: jup_mint, // 注意：卖出 JUP
        input_token_program: Some(token0_program),
        output_mint: wsol_mint,
        output_token_program: Some(token1_program),
        input_amount: Some(amount_in),
        slippage_basis_points: Some(1000),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumClmm(
            clmm_params,
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

    let instructions = match sol_trade_sdk::instruction::raydium_clmm::RaydiumClmmInstructionBuilder
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
        &jup_mint,
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
        jup_mint,
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
                // 只显示前 50 行
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
    let error_rate =
        if simulated_output > 0 { (diff as f64 / simulated_output as f64) * 100.0 } else { 0.0 };

    println!("│ 差值:         {:>15} │", diff);
    println!("│ 误差率:      {:>13.4}% │", error_rate);
    println!("└─────────────────────────────────────┘");

    match verify_calculation_accuracy(local_output, simulated_output, 1.0) {
        Ok(_) => println!("✅ 验证通过：误差 < 1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}

// ========================================
// Test 3: Exact Out Buy (指定 JUP 数量)
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_clmm_exact_out_buy_with_simulation() {
    println!("====================================================");
    println!("Test 3: CLMM Exact Out Buy (指定 JUP 数量)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(WSOL_JUP_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let jup_mint = Pubkey::from_str(JUP_MINT).unwrap();

    // 期望输出：500,000 JUP
    let amount_out = 500_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("期望输出: {} JUP", amount_out);
    println!("计算: 需要 WSOL 输入\n");

    // 初始化 ATA
    if let Err(e) = ensure_ata_with_balance(
        &rpc,
        &rpc_url,
        &payer,
        &[
            (wsol_mint, Some(1_000_000)), // 充值足够的 WSOL
            (jup_mint, None),
        ],
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

    // 判断方向：WSOL -> JUP (buy)
    let zero_for_one = wsol_mint.to_string() == pool_state.token_mint0.to_string();

    println!("交易方向: zero_for_one = {}", zero_for_one);
    println!(
        "含义: {} -> {}\n",
        if zero_for_one { "token0" } else { "token1" },
        if zero_for_one { "token1" } else { "token0" }
    );

    // 本地计算 (exact_out)
    let local_calc = match quote_exact_out(&rpc, &pool_address, amount_out, zero_for_one).await {
        Ok(result) => result,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算:");
    println!("  期望输出: {} JUP", amount_out);
    println!("  需要输入: {} WSOL (lamports)\n", local_calc.amount_in);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (token0_mint, token1_mint) = (pool_state.token_mint0, pool_state.token_mint1);
    let token0_program =
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
    let token1_program =
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

    // 构造指令 (使用 fixed_output_amount)
    let clmm_params = RaydiumClmmParams {
        pool_state: pool_address,
        amm_config: pool_state.amm_config,
        token0_mint: pool_state.token_mint0,
        token1_mint: pool_state.token_mint1,
        token0_vault: pool_state.token_vault0,
        token1_vault: pool_state.token_vault1,
        observation_state: pool_state.observation_key,
        token0_decimals: pool_state.mint_decimals0,
        token1_decimals: pool_state.mint_decimals1,
        token0_program,
        token1_program,
    };

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: payer.clone(),
        trade_type: sol_trade_sdk::swqos::TradeType::Buy,
        input_mint: wsol_mint,
        input_token_program: Some(token0_program),
        output_mint: jup_mint,
        output_token_program: Some(token1_program),
        input_amount: Some(local_calc.amount_in), // 使用计算出的输入
        slippage_basis_points: Some(1000),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumClmm(
            clmm_params,
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

    let instructions = match sol_trade_sdk::instruction::raydium_clmm::RaydiumClmmInstructionBuilder
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
            &jup_mint,
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
        jup_mint,
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
                // 只显示前 50 行
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
    let error_rate =
        if simulated_output > 0 { (diff as f64 / simulated_output as f64) * 100.0 } else { 0.0 };

    println!("│ 差值:         {:>15} │", diff);
    println!("│ 误差率:      {:>13.4}% │", error_rate);
    println!("└─────────────────────────────────────┘");

    match verify_calculation_accuracy(amount_out, simulated_output, 1.0) {
        Ok(_) => println!("✅ 验证通过：误差 < 1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}

// ========================================
// Test 4: Exact Out Sell (指定 WSOL 数量)
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_clmm_exact_out_sell_with_simulation() {
    println!("====================================================");
    println!("Test 4: CLMM Exact Out Sell (指定 WSOL 数量)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(WSOL_JUP_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let jup_mint = Pubkey::from_str(JUP_MINT).unwrap();

    // 期望输出：500,000 WSOL (lamports)
    let amount_out = 500_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("期望输出: {} WSOL (lamports)", amount_out);
    println!("计算: 需要 JUP 输入\n");

    // 初始化 ATA（只创建 WSOL ATA）
    if let Err(e) = ensure_ata_with_balance(&rpc, &rpc_url, &payer, &[(wsol_mint, None)], 1).await {
        println!("❌ 初始化失败: {}\n", e);
        return;
    }

    // 设置 JUP 余额（使用 surfnet_setTokenAccount）
    // 设置 1 JUP 用于测试
    if let Err(e) = common::set_token_balance(&rpc, &rpc_url, &payer, &jup_mint, "1").await {
        println!("❌ 设置 JUP 余额失败: {}\n", e);
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

    // 判断方向：JUP -> WSOL (sell)
    let zero_for_one = jup_mint.to_string() == pool_state.token_mint0.to_string();

    println!("交易方向: zero_for_one = {}", zero_for_one);
    println!(
        "含义: {} -> {}\n",
        if zero_for_one { "token0" } else { "token1" },
        if zero_for_one { "token1" } else { "token0" }
    );

    // 本地计算 (exact_out)
    let local_calc = match quote_exact_out(&rpc, &pool_address, amount_out, zero_for_one).await {
        Ok(result) => result,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算:");
    println!("  期望输出: {} WSOL (lamports)", amount_out);
    println!("  需要输入: {} JUP\n", local_calc.amount_in);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (token0_mint, token1_mint) = (pool_state.token_mint0, pool_state.token_mint1);
    let token0_program =
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
    let token1_program =
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

    // 构造指令 (使用 fixed_output_amount)
    let clmm_params = RaydiumClmmParams {
        pool_state: pool_address,
        amm_config: pool_state.amm_config,
        token0_mint: pool_state.token_mint0,
        token1_mint: pool_state.token_mint1,
        token0_vault: pool_state.token_vault0,
        token1_vault: pool_state.token_vault1,
        observation_state: pool_state.observation_key,
        token0_decimals: pool_state.mint_decimals0,
        token1_decimals: pool_state.mint_decimals1,
        token0_program,
        token1_program,
    };

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: payer.clone(),
        trade_type: sol_trade_sdk::swqos::TradeType::Sell,
        input_mint: jup_mint, // 注意：卖出 JUP
        input_token_program: Some(token0_program),
        output_mint: wsol_mint,
        output_token_program: Some(token1_program),
        input_amount: Some(local_calc.amount_in), // 使用计算出的输入
        slippage_basis_points: Some(1000),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumClmm(
            clmm_params,
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

    let instructions = match sol_trade_sdk::instruction::raydium_clmm::RaydiumClmmInstructionBuilder
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
        &jup_mint,
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
        jup_mint,
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
                // 只显示前 50 行
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
    let error_rate =
        if simulated_output > 0 { (diff as f64 / simulated_output as f64) * 100.0 } else { 0.0 };

    println!("│ 差值:         {:>15} │", diff);
    println!("│ 误差率:      {:>13.4}% │", error_rate);
    println!("└─────────────────────────────────────┘");

    match verify_calculation_accuracy(amount_out, simulated_output, 1.0) {
        Ok(_) => println!("✅ 验证通过：误差 < 1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}
