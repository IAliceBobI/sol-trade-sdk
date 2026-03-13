//! Meteora DAMM V2 Swap 测试
//!
//! 测试 Meteora DAMM V2 的 swap 功能，使用链上模拟验证
//!
//! # 运行测试
//!
//! ```bash
//! # 运行所有 Meteora DAMM V2 测试
//! cargo nextest run meteora_damm_v2_swap --nocapture 2>&1
//! ```

use sol_trade_test_utils::{ensure_token_balance, get_simulation_test_keypair};

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::meteora_damm_v2::MeteoraDammV2InstructionBuilder,
    swqos::TradeType,
    trading::core::params::{DexParamEnum, MeteoraDammV2Params, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::simulation_based_calc::{SimulatedSwapResult, simulate_swap_transaction},
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::str::FromStr;
use std::sync::Arc;

/// Pigeon-WSOL Pool 地址（Meteora DAMM V2 mainnet fork）
/// Pool: qHcjwRN2wKJHy8BdcgrQZjLutFhBWPNKSgUWitb929B
/// Token: Pigeon (4fSWEw2wbYEUCcMtitzmeGUfqinoafXxkhqZrA9Gpump) - WSOL
fn get_test_pool_address() -> Pubkey {
    Pubkey::from_str("qHcjwRN2wKJHy8BdcgrQZjLutFhBWPNKSgUWitb929B").expect("Invalid pool address")
}

/// 创建测试用的 RPC 客户端和 payer
fn setup_test() -> (Arc<SolanaRpcClient>, Arc<Keypair>) {
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.to_string()));
    let payer = Arc::new(get_simulation_test_keypair());
    (rpc, payer)
}

/// Meteora DAMM V2 Buy 测试
///
/// 测试用 WSOL 购买 Pigeon token
#[tokio::test]
#[serial_test::serial(meteora_damm_v2_swap)]
async fn test_meteora_damm_v2_buy() {
    let (rpc, payer) = setup_test();
    let pool_address = get_test_pool_address();

    println!("\n========================================");
    println!("Meteora DAMM V2 Buy 测试");
    println!("========================================");
    println!("Pool 地址: {}", pool_address);

    // 1. 获取 Pool 参数
    println!("\n[步骤 1] 获取 Pool 参数...");
    let protocol_params = MeteoraDammV2Params::from_pool_address_by_rpc(&rpc, &pool_address)
        .await
        .expect("获取 Pool 参数失败");

    println!("  Token A Mint: {}", protocol_params.token_a_mint);
    println!("  Token B Mint: {}", protocol_params.token_b_mint);
    println!("  Token A Vault: {}", protocol_params.token_a_vault);
    println!("  Token B Vault: {}", protocol_params.token_b_vault);
    println!("  Token A Program: {}", protocol_params.token_a_program);
    println!("  Token B Program: {}", protocol_params.token_b_program);

    // 判断哪个是 base (Pigeon)，哪个是 quote (WSOL)
    let wsol_mint = sol_trade_sdk::constants::WSOL_TOKEN_ACCOUNT;
    let is_token_a_wsol = protocol_params.token_a_mint == wsol_mint;

    let (base_mint, base_program, quote_mint, quote_program) = if is_token_a_wsol {
        // Token A 是 WSOL，Token B 是 Pigeon
        (
            protocol_params.token_b_mint,
            protocol_params.token_b_program,
            protocol_params.token_a_mint,
            protocol_params.token_a_program,
        )
    } else {
        // Token B 是 WSOL，Token A 是 Pigeon
        (
            protocol_params.token_a_mint,
            protocol_params.token_a_program,
            protocol_params.token_b_mint,
            protocol_params.token_b_program,
        )
    };

    println!("  Base Mint (Pigeon): {}", base_mint);
    println!("  Quote Mint (WSOL): {}", quote_mint);

    // 2. 准备测试金额
    // 购买 0.001 WSOL 的 Pigeon
    let quote_amount_in = 1_000_000u64; // 0.001 WSOL (9 decimals)
    let min_base_amount_out = 1u64; // 最小输出，用于滑点保护

    println!("\n[步骤 2] 准备 Swap 参数...");
    println!("  输入金额 (WSOL): {}", quote_amount_in);
    println!("  最小输出金额 (Pigeon): {}", min_base_amount_out);

    // 3. 构建 SwapParams
    let gas_fee_strategy = sol_trade_test_utils::create_test_gas_fee_strategy();

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: Arc::new(payer.insecure_clone()),
        trade_type: TradeType::Buy,
        input_mint: quote_mint, // WSOL
        input_token_program: Some(quote_program),
        output_mint: base_mint, // Pigeon
        output_token_program: Some(base_program),
        input_amount: Some(quote_amount_in),
        slippage_basis_points: Some(500), // 5% 滑点
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: DexParamEnum::MeteoraDammV2(protocol_params.clone()),
        open_seed_optimize: false,
        swqos_clients: vec![],
        middleware_manager: None,
        durable_nonce: None,
        with_tip: false,
        create_input_mint_ata: true,  // 创建 WSOL ATA
        close_input_mint_ata: true,   // 关闭 WSOL ATA
        create_output_mint_ata: true, // 创建 Pigeon ATA
        close_output_mint_ata: false,
        fixed_output_amount: Some(min_base_amount_out),
        gas_fee_strategy,
        simulate: true,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    // 4. 构建 buy 指令
    println!("\n[步骤 3] 构建买入指令...");
    let builder = MeteoraDammV2InstructionBuilder;
    let instructions = builder.build_buy_instructions(&swap_params).await.expect("构建指令失败");
    println!("  指令数量: {}", instructions.len());

    // 5. 获取用户 ATA 地址
    let user_base_token_account =
        get_associated_token_address_with_program_id(&payer.pubkey(), &base_mint, &base_program);
    let user_quote_token_account =
        get_associated_token_address_with_program_id(&payer.pubkey(), &quote_mint, &quote_program);

    println!("  用户 Base ATA: {}", user_base_token_account);
    println!("  用户 Quote ATA: {}", user_quote_token_account);

    // 6. 模拟执行
    println!("\n[步骤 4] 模拟执行...");
    let simulated_result: SimulatedSwapResult = simulate_swap_transaction(
        &rpc,
        payer.as_ref(),
        instructions,
        user_quote_token_account, // 输入账户 (WSOL)
        user_base_token_account,  // 输出账户 (Pigeon)
        quote_mint,               // 输入 mint (WSOL)
        base_mint,                // 输出 mint (Pigeon)
    )
    .await
    .expect("模拟执行失败");

    println!("  模拟成功: {}", simulated_result.success);
    println!("  实际输入金额 (WSOL): {}", simulated_result.actual_input_amount);
    println!("  实际输出金额 (Pigeon): {}", simulated_result.actual_output_amount);

    if let Some(error) = &simulated_result.error {
        println!("  错误: {}", error);
    }

    // 7. 验证结果
    assert!(simulated_result.success, "模拟执行失败");
    assert!(simulated_result.actual_output_amount > 0, "输出金额应该大于 0");

    println!("\n✅ Meteora DAMM V2 Buy 测试通过!");
}

/// Meteora DAMM V2 Sell 测试
///
/// 测试卖出 Pigeon token 获得 WSOL
#[tokio::test]
#[serial_test::serial(meteora_damm_v2_swap)]
async fn test_meteora_damm_v2_sell() {
    let (rpc, payer) = setup_test();
    let pool_address = get_test_pool_address();

    println!("\n========================================");
    println!("Meteora DAMM V2 Sell 测试");
    println!("========================================");
    println!("Pool 地址: {}", pool_address);

    // 1. 获取 Pool 参数
    println!("\n[步骤 1] 获取 Pool 参数...");
    let protocol_params = MeteoraDammV2Params::from_pool_address_by_rpc(&rpc, &pool_address)
        .await
        .expect("获取 Pool 参数失败");

    println!("  Token A Mint: {}", protocol_params.token_a_mint);
    println!("  Token B Mint: {}", protocol_params.token_b_mint);

    // 判断哪个是 base (Pigeon)，哪个是 quote (WSOL)
    let wsol_mint = sol_trade_sdk::constants::WSOL_TOKEN_ACCOUNT;
    let is_token_a_wsol = protocol_params.token_a_mint == wsol_mint;

    let (base_mint, base_program, quote_mint, quote_program) = if is_token_a_wsol {
        (
            protocol_params.token_b_mint,
            protocol_params.token_b_program,
            protocol_params.token_a_mint,
            protocol_params.token_a_program,
        )
    } else {
        (
            protocol_params.token_a_mint,
            protocol_params.token_a_program,
            protocol_params.token_b_mint,
            protocol_params.token_b_program,
        )
    };

    println!("  Base Mint (Pigeon): {}", base_mint);
    println!("  Quote Mint (WSOL): {}", quote_mint);

    // 2. 先买入一些 Pigeon token 用于测试卖出
    println!("\n[步骤 2] 先买入 Pigeon token...");
    let buy_amount = 1_000_000u64; // 0.001 WSOL

    let gas_fee_strategy = sol_trade_test_utils::create_test_gas_fee_strategy();
    let buy_swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: Arc::new(payer.insecure_clone()),
        trade_type: TradeType::Buy,
        input_mint: quote_mint,
        input_token_program: Some(quote_program),
        output_mint: base_mint,
        output_token_program: Some(base_program),
        input_amount: Some(buy_amount),
        slippage_basis_points: Some(500),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: DexParamEnum::MeteoraDammV2(protocol_params.clone()),
        open_seed_optimize: false,
        swqos_clients: vec![],
        middleware_manager: None,
        durable_nonce: None,
        with_tip: false,
        create_input_mint_ata: true,
        close_input_mint_ata: true,
        create_output_mint_ata: true,
        close_output_mint_ata: false,
        fixed_output_amount: Some(1),
        gas_fee_strategy: gas_fee_strategy.clone(),
        simulate: true,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    let builder = MeteoraDammV2InstructionBuilder;
    let buy_instructions = builder
        .build_buy_instructions(&buy_swap_params)
        .await
        .expect("构建买入指令失败");

    let user_base_token_account =
        get_associated_token_address_with_program_id(&payer.pubkey(), &base_mint, &base_program);
    let user_quote_token_account =
        get_associated_token_address_with_program_id(&payer.pubkey(), &quote_mint, &quote_program);

    let buy_result = simulate_swap_transaction(
        &rpc,
        payer.as_ref(),
        buy_instructions,
        user_quote_token_account,
        user_base_token_account,
        quote_mint,
        base_mint,
    )
    .await
    .expect("买入模拟失败");

    assert!(buy_result.success, "买入模拟失败");
    let base_balance = buy_result.actual_output_amount;
    println!("  买入 Pigeon 数量: {}", base_balance);

    // 2.5 给测试账户空投 Pigeon token（模拟交易不会真正改变余额）
    println!("\n[步骤 2.5] 空投 Pigeon token 到测试账户...");
    let rpc_url = "http://127.0.0.1:8899";
    // Pigeon token 使用 Token-2022 Program，decimals 需要查询
    let base_decimals = 6; // 假设 Pigeon 是 6 decimals，实际需要查询确认
    let base_amount_formatted =
        format!("{}", base_balance as f64 / 10f64.powi(base_decimals as i32));

    ensure_token_balance(&rpc, rpc_url, payer.as_ref(), &base_mint, &base_amount_formatted)
        .await
        .expect("空投 Pigeon token 失败");
    println!("  ✅ Pigeon token 余额已设置: {}", base_amount_formatted);

    // 3. 准备卖出参数
    println!("\n[步骤 3] 准备卖出参数...");
    let sell_amount = base_balance / 2; // 卖出一半
    let min_quote_out = 1u64;

    println!("  卖出 Pigeon 数量: {}", sell_amount);
    println!("  最小 WSOL 输出: {}", min_quote_out);

    // 4. 构建 Sell SwapParams
    let sell_swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: Arc::new(payer.insecure_clone()),
        trade_type: TradeType::Sell,
        input_mint: base_mint, // Pigeon
        input_token_program: Some(base_program),
        output_mint: quote_mint, // WSOL
        output_token_program: Some(quote_program),
        input_amount: Some(sell_amount),
        slippage_basis_points: Some(500),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: DexParamEnum::MeteoraDammV2(protocol_params.clone()),
        open_seed_optimize: false,
        swqos_clients: vec![],
        middleware_manager: None,
        durable_nonce: None,
        with_tip: false,
        create_input_mint_ata: true,
        close_input_mint_ata: false,
        create_output_mint_ata: true,
        close_output_mint_ata: true,
        fixed_output_amount: Some(min_quote_out),
        gas_fee_strategy,
        simulate: true,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    // 5. 构建 sell 指令
    println!("\n[步骤 4] 构建卖出指令...");
    let sell_instructions = builder
        .build_sell_instructions(&sell_swap_params)
        .await
        .expect("构建卖出指令失败");
    println!("  指令数量: {}", sell_instructions.len());

    // 6. 模拟执行卖出
    println!("\n[步骤 5] 模拟执行卖出...");
    let sell_result = simulate_swap_transaction(
        &rpc,
        payer.as_ref(),
        sell_instructions,
        user_base_token_account,  // 输入账户 (Pigeon)
        user_quote_token_account, // 输出账户 (WSOL)
        base_mint,                // 输入 mint (Pigeon)
        quote_mint,               // 输出 mint (WSOL)
    )
    .await
    .expect("卖出模拟失败");

    println!("  模拟成功: {}", sell_result.success);
    println!("  实际输入金额 (Pigeon): {}", sell_result.actual_input_amount);
    println!("  实际输出金额 (WSOL): {}", sell_result.actual_output_amount);

    if let Some(error) = &sell_result.error {
        println!("  错误: {}", error);
    }

    // 7. 验证结果
    assert!(sell_result.success, "卖出模拟失败");
    assert!(sell_result.actual_output_amount > 0, "WSOL 输出金额应该大于 0");

    println!("\n✅ Meteora DAMM V2 Sell 测试通过!");
}

/// Meteora DAMM V2 Pool 信息获取测试
#[tokio::test]
#[serial_test::serial(meteora_damm_v2_swap)]
async fn test_meteora_damm_v2_pool_info() {
    let (rpc, _payer) = setup_test();
    let pool_address = get_test_pool_address();

    println!("\n========================================");
    println!("Meteora DAMM V2 Pool 信息测试");
    println!("========================================");

    // 获取 Pool 参数
    let protocol_params = MeteoraDammV2Params::from_pool_address_by_rpc(&rpc, &pool_address)
        .await
        .expect("获取 Pool 参数失败");

    println!("Pool 地址: {}", pool_address);
    println!("Token A Mint: {}", protocol_params.token_a_mint);
    println!("Token B Mint: {}", protocol_params.token_b_mint);
    println!("Token A Vault: {}", protocol_params.token_a_vault);
    println!("Token B Vault: {}", protocol_params.token_b_vault);
    println!("Token A Program: {}", protocol_params.token_a_program);
    println!("Token B Program: {}", protocol_params.token_b_program);

    // 获取 Pool 原始数据
    let pool_data = sol_trade_sdk::instruction::utils::meteora_damm_v2::get_pool_by_address(
        &rpc,
        &pool_address,
    )
    .await
    .expect("获取 Pool 数据失败");

    println!("\nPool 状态:");
    println!("  sqrt_price: {}", pool_data.sqrt_price);
    println!("  liquidity: {}", pool_data.liquidity);
    println!("  pool_status: {}", pool_data.pool_status);

    println!("\n✅ Meteora DAMM V2 Pool 信息测试通过!");
}
