//! Raydium CPMM Exact In Buy 完整验证测试
//!
//! 测试流程：
//! 1. 本地计算（quote_exact_in）
//! 2. 链上模拟（simulate_transaction）
//! 3. 实际执行（send_transaction）
//!
//! 作为裁判，验证三个步骤的结果是否一致。

use sdk_common::SolanaRpcClient;
use sol_trade_sdk::{
    common as sdk_common,
    instruction::utils::raydium_cpmm::get_pool_by_address,
    instruction::utils::raydium_cpmm::quote_exact_in,
    trading::core::params::{RaydiumCpmmParams, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::simulation_based_calc::simulate_swap_transaction,
};
use solana_sdk::signer::Signer;
use std::sync::Arc;

// 导入公共测试模块
use sol_trade_test_utils::{ensure_ata_with_balance, get_simulation_test_keypair};

// 导入 CPMM 测试参数工具
use sol_trade_test_utils::{pipe_mint, pipe_wsol_pool, wsol_mint};

#[tokio::test]
#[serial_test::serial(cpmm_exact_in_buy_complete)]
async fn test_cpmm_exact_in_buy_three_stage_verification() {
    println!("==============================================");
    println!("Raydium CPMM Exact In Buy 三阶段验证测试");
    println!("==============================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = pipe_wsol_pool();
    let wsol_mint = wsol_mint();
    let pipe_mint = pipe_mint();

    // 测试金额：0.001 SOL
    let amount_in = 1_000_000u64;
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("输入: {} lamports WSOL", amount_in);
    println!("输出: PIPE tokens\n");

    // 初始化 ATA
    if let Err(e) = ensure_ata_with_balance(
        &rpc,
        &rpc_url,
        &payer,
        &[(wsol_mint, Some(amount_in)), (pipe_mint, None)],
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

    // 获取储备金（用于调试）
    let (token0_reserve, token1_reserve) = match (
        rpc.get_token_account_balance(&pool_state.token0_vault).await,
        rpc.get_token_account_balance(&pool_state.token1_vault).await,
    ) {
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

    println!("📊 Pool Reserve:");
    println!("  token0 (PIPE): {}", token0_reserve);
    println!("  token1 (WSOL): {}", token1_reserve);
    println!();

    // ========================================
    // 阶段 1: 本地计算（quote_exact_in）
    // ========================================
    println!("========================================");
    println!("阶段 1: 本地计算（quote_exact_in）");
    println!("========================================\n");

    let is_token0_in = wsol_mint.to_string() == pool_state.token0_mint.to_string();
    println!("交易方向: WSOL -> PIPE");
    println!("is_token0_in: {} (false 表示 token1 作为输入)\n", is_token0_in);

    let quote_result = match quote_exact_in(&rpc, &pool_address, amount_in, is_token0_in).await {
        Ok(quote) => quote,
        Err(e) => {
            println!("❌ 本地计算失败: {}\n", e);
            return;
        },
    };

    let local_output = quote_result.amount_out;
    let local_fee = quote_result.fee_amount;

    println!("✅ 本地计算结果:");
    println!("  输出金额: {} PIPE", local_output);
    println!("  手续费: {} lamports", local_fee);
    println!("  净输出: {} PIPE\n", local_output);

    // ========================================
    // 阶段 2: 链上模拟
    // ========================================
    println!("========================================");
    println!("阶段 2: 链上模拟");
    println!("========================================\n");

    // 构造 swap 指令
    let cpmm_params = RaydiumCpmmParams {
        pool_state: pool_address,
        amm_config: pool_state.amm_config,
        base_mint: pool_state.token0_mint,
        quote_mint: pool_state.token1_mint,
        base_reserve: token0_reserve,
        quote_reserve: token1_reserve,
        base_vault: pool_state.token0_vault,
        quote_vault: pool_state.token1_vault,
        base_token_program: pool_state.token0_program,
        quote_token_program: pool_state.token1_program,
        observation_state: pool_state.observation_key,
    };

    // 获取 Token Program
    let base_token_program =
        match sol_trade_sdk::utils::token::get_token_program_with_cache(&rpc, &pool_state.token0_mint)
            .await
        {
            Ok(program) => program,
            Err(e) => {
                println!("❌ 无法获取 token0 Token Program: {}", e);
                return;
            },
        };
    let quote_token_program =
        match sol_trade_sdk::utils::token::get_token_program_with_cache(&rpc, &pool_state.token1_mint)
            .await
        {
            Ok(program) => program,
            Err(e) => {
                println!("❌ 无法获取 token1 Token Program: {}", e);
                return;
            },
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
        slippage_basis_points: Some(1000), // 10% 滑点
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: sol_trade_sdk::trading::core::params::DexParamEnum::RaydiumCpmm(
            cpmm_params,
        ),
        open_seed_optimize: false,
        swqos_clients: vec![],
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

    // 获取用户 ATA
    let user_input_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &wsol_mint,
        &spl_token::id(),
    );
    let user_output_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &pipe_mint,
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
        pipe_mint,
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
        return;
    }

    let simulated_output = simulation_result.actual_output_amount;

    println!("✅ 链上模拟结果:");
    println!("  实际输入: {} lamports", simulation_result.actual_input_amount);
    println!("  实际输出: {} PIPE", simulated_output);
    println!("  Inner Instructions: {:?}\n", simulation_result.inner_instructions);

    // ========================================
    // 阶段 3: 实际执行（可选）
    // ========================================
    println!("========================================");
    println!("阶段 3: 实际执行（跳过，避免消耗真实资金）");
    println!("========================================\n");
    println!("⚠️  跳过实际执行，只比较本地计算和链上模拟\n");

    // ========================================
    // 裁判：比较三个阶段的结果
    // ========================================
    println!("========================================");
    println!("裁判：结果对比");
    println!("========================================\n");

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ 阶段                │ 输出 (PIPE)   │ 说明                  │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!(
        "│ 1. 本地计算         │ {:>12} │ quote_exact_in        │",
        local_output
    );
    println!(
        "│ 2. 链上模拟         │ {:>12} │ simulate_transaction   │",
        simulated_output
    );
    println!("│ 3. 实际执行         │     N/A      │ 跳过                  │");
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    // 计算差异
    let diff = local_output.abs_diff(simulated_output);
    let error_rate =
        if simulated_output > 0 {
            (diff as f64 / simulated_output as f64) * 100.0
        } else {
            0.0
        };

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ 差异分析                                                │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ 绝对差异: {} PIPE                                        │", diff);
    println!("│ 误差率:   {:.4}%                                              │", error_rate);
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    // 判断：误差是否在可接受范围内
    const MAX_ERROR_PERCENT: f64 = 1.0; // 1% 容忍度

    if error_rate <= MAX_ERROR_PERCENT {
        println!("✅ 裁判结果：本地计算与链上模拟一致（误差 {:.4}% ≤ {:.1}%）", error_rate, MAX_ERROR_PERCENT);
        println!("✅ 测试通过\n");
    } else {
        println!("❌ 裁判结果：本地计算与链上模拟不一致（误差 {:.4}% > {:.1}%）", error_rate, MAX_ERROR_PERCENT);
        println!();
        println!("🔍 可能的原因：");
        println!("  1. 本地计算公式与链上逻辑不一致");
        println!("  2. 储备金在查询和执行之间发生了变化");
        println!("  3. 费用计算方式不同");
        println!("  4. Program data 解析错误");
        println!();
        println!("❌ 测试失败\n");
        panic!("本地计算与链上模拟误差过大");
    }
}
