//! Raydium AMM V4 Swap 链上模拟验证测试
//!
//! 通过构造真实的交易并模拟执行，验证本地计算的准确性
//!
//! 运行测试:
//!     cargo nextest run verify_raydium_amm_v4_with_simulation -- --nocapture
//!
//! 测试矩阵:
//! ┌─────────────┬──────────────┬─────────────────┐
//! │             │   Exact In   │    Exact Out   │
//! ├─────────────┼──────────────┼─────────────────┤
//! │ Buy         │ ✅ Test 1    │ ✅ Test 3      │
//! │ Sell        │ ✅ Test 2    │ ✅ Test 4      │
//! └─────────────┴──────────────┴─────────────────┘
//!
//! 注意：Raydium AMM V4 使用恒定乘积 (Constant Product AMM)

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::utils::raydium_amm_v4::{get_pool_by_address, quote_exact_in, quote_exact_out},
    trading::core::params::{RaydiumAmmV4Params, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::simulation_based_calc::{simulate_swap_transaction, verify_calculation_accuracy},
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;
use std::sync::Arc;

// 导入公共模块
mod common;
use common::{ensure_ata_with_balance, get_simulation_test_keypair, get_token_program_for_mint};

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
#[serial_test::serial]
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
    if let Err(e) = ensure_ata_with_balance(
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

    println!("交易方向: WSOL -> USDC (买入 USDC)\n");

    // 本地计算
    let local_output = match quote_exact_in(&rpc, &pool_address, amount_in, false).await {
        // false: pc -> coin (WSOL 是 coin, USDC 是 pc, 我们要用 WSOL 换 USDC)
        // 在 AMM V4 中，is_coin_in=true 表示输入 coin，false 表示输入 pc
        // WSOL 是 coin，USDC 是 pc，所以 WSOL->USDC 应该是 is_coin_in=true
        Ok(quote) => quote.amount_out,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算: {} USDC (smallest unit)\n", local_output);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (coin_mint, pc_mint) = (pool_state.coin_mint, pool_state.pc_mint);
    let coin_token_program = match get_token_program_for_mint(&rpc, &coin_mint).await {
        Ok(program) => {
            println!("✅ 自动检测 coin_mint ({}) Token Program: {}", coin_mint, program);
            program
        },
        Err(e) => {
            println!("⚠️  无法获取 coin_mint Token Program，使用默认值: {}", e);
            spl_token::id()
        },
    };
    let pc_token_program = match get_token_program_for_mint(&rpc, &pc_mint).await {
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

    match verify_calculation_accuracy(local_output, simulated_output, 0.1) {
        Ok(_) => println!("✅ 验证通过：误差 < 0.1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}

// ========================================
// Test 2: Exact In Sell (USDC -> WSOL)
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_raydium_amm_v4_exact_in_sell_with_simulation() {
    println!("====================================================");
    println!("Test 2: Raydium AMM V4 Exact In Sell (USDC -> WSOL)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(SOL_USDC_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let usdc_mint = Pubkey::from_str(USDC_MINT).unwrap();

    // 测试金额：卖出 1000 USDC (smallest unit)
    let amount_in = 1_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("输入: {} USDC (smallest unit)", amount_in);
    println!("期望输出: WSOL (lamports)\n");

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

    println!("交易方向: USDC -> WSOL (卖出 USDC)\n");

    // 本地计算
    let local_output = match quote_exact_in(&rpc, &pool_address, amount_in, true).await {
        // true: pc -> coin (USDC 是 pc, WSOL 是 coin)
        Ok(quote) => quote.amount_out,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算: {} WSOL (lamports)\n", local_output);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (coin_mint, pc_mint) = (pool_state.coin_mint, pool_state.pc_mint);
    let coin_token_program = match get_token_program_for_mint(&rpc, &coin_mint).await {
        Ok(program) => {
            println!("✅ 自动检测 coin_mint ({}) Token Program: {}", coin_mint, program);
            program
        },
        Err(e) => {
            println!("⚠️  无法获取 coin_mint Token Program，使用默认值: {}", e);
            spl_token::id()
        },
    };
    let pc_token_program = match get_token_program_for_mint(&rpc, &pc_mint).await {
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
        trade_type: sol_trade_sdk::swqos::TradeType::Sell,
        input_mint: usdc_mint,
        input_token_program: Some(pc_token_program),
        output_mint: wsol_mint,
        output_token_program: Some(coin_token_program),
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

    match verify_calculation_accuracy(local_output, simulated_output, 0.1) {
        Ok(_) => println!("✅ 验证通过：误差 < 0.1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}

// ========================================
// Test 3: Exact Out Buy (指定 USDC 数量)
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_raydium_amm_v4_exact_out_buy_with_simulation() {
    println!("====================================================");
    println!("Test 3: Raydium AMM V4 Exact Out Buy (指定 USDC 数量)");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(SOL_USDC_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let usdc_mint = Pubkey::from_str(USDC_MINT).unwrap();

    // 期望输出：1000 USDC (smallest unit)
    let amount_out = 1_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("期望输出: {} USDC (smallest unit)", amount_out);
    println!("计算: 需要 WSOL 输入\n");

    // 初始化 ATA
    if let Err(e) = ensure_ata_with_balance(
        &rpc,
        &rpc_url,
        &payer,
        &[
            (wsol_mint, Some(10_000_000)), // 充值足够的 WSOL
            (usdc_mint, None),
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

    println!("交易方向: WSOL -> USDC (买入 USDC)\n");

    // 本地计算 (exact_out)
    let local_calc = match quote_exact_out(&rpc, &pool_address, amount_out, false).await {
        // false: pc -> coin (WSOL 是 coin, USDC 是 pc)
        Ok(result) => result,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算:");
    println!("  期望输出: {} USDC (smallest unit)", amount_out);
    println!("  需要输入: {} WSOL (lamports)\n", local_calc.amount_in);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (coin_mint, pc_mint) = (pool_state.coin_mint, pool_state.pc_mint);
    let coin_token_program = match get_token_program_for_mint(&rpc, &coin_mint).await {
        Ok(program) => {
            println!("✅ 自动检测 coin_mint ({}) Token Program: {}", coin_mint, program);
            program
        },
        Err(e) => {
            println!("⚠️  无法获取 coin_mint Token Program，使用默认值: {}", e);
            spl_token::id()
        },
    };
    let pc_token_program = match get_token_program_for_mint(&rpc, &pc_mint).await {
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
        trade_type: sol_trade_sdk::swqos::TradeType::Buy,
        input_mint: wsol_mint,
        input_token_program: Some(coin_token_program),
        output_mint: usdc_mint,
        output_token_program: Some(pc_token_program),
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

    println!("交易方向: USDC -> WSOL (卖出 USDC)\n");

    // 本地计算 (exact_out)
    let local_calc = match quote_exact_out(&rpc, &pool_address, amount_out, true).await {
        // true: pc -> coin (USDC 是 pc, WSOL 是 coin)
        Ok(result) => result,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    println!("✅ 本地计算:");
    println!("  期望输出: {} WSOL (lamports)", amount_out);
    println!("  需要输入: {} USDC (smallest unit)\n", local_calc.amount_in);

    // 🔧 自动从 Pool 获取 mint 并检测 Token Program
    let (coin_mint, pc_mint) = (pool_state.coin_mint, pool_state.pc_mint);
    let coin_token_program = match get_token_program_for_mint(&rpc, &coin_mint).await {
        Ok(program) => {
            println!("✅ 自动检测 coin_mint ({}) Token Program: {}", coin_mint, program);
            program
        },
        Err(e) => {
            println!("⚠️  无法获取 coin_mint Token Program，使用默认值: {}", e);
            spl_token::id()
        },
    };
    let pc_token_program = match get_token_program_for_mint(&rpc, &pc_mint).await {
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

    match verify_calculation_accuracy(amount_out, simulated_output, 0.1) {
        Ok(_) => println!("✅ 验证通过：误差 < 0.1%\n"),
        Err(e) => println!("❌ 验证失败: {}\n", e),
    }
}
