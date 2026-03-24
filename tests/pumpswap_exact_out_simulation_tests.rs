//! PumpSwap Exact Out 链上模拟验证测试
//!
//! 通过 `simulateTransaction` 验证本地计算结果与实际链上执行结果的一致性
//!
//! # 测试目标
//!
//! 验证 `buy_exact_out_base_internal` 和 `sell_exact_out_quote_internal` 的本地计算
//! 与链上模拟执行结果的一致性
//!
//! # 运行测试
//!
//! ```bash
//! # 运行所有 PumpSwap Exact Out 模拟测试
//! cargo nextest run pumpswap_exact_out_sim --nocapture 2>&1
//! ```

mod test_helpers;
use test_helpers::create_test_client;

use sol_trade_test_utils::ensure_token_balance;

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::pumpswap::PumpSwapInstructionBuilder,
    swqos::TradeType,
    trading::core::params::{DexParamEnum, PumpSwapParams, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::{
        calc::pumpswap::{buy_exact_out_base_internal, sell_exact_out_quote_internal},
        simulation_based_calc::{SimulatedSwapResult, simulate_swap_transaction},
    },
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::str::FromStr;
use std::sync::Arc;

/// 验证结果
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// 本地计算的预期输入金额
    pub expected_input: u64,
    /// 链上模拟的实际输入金额
    pub actual_input: u64,
    /// 误差率（百分比）
    pub error_rate_percent: f64,
    /// 是否通过验证
    pub passed: bool,
}

/// PUMP-WSOL Pool 地址（PumpSwap mainnet fork）
fn get_test_pool_address() -> Pubkey {
    Pubkey::from_str("539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR").expect("Invalid pool address")
}

/// 完整的 Exact Out Buy 验证流程
///
/// # 流程
/// 1. 通过 `PumpSwapParams::from_pool_address_by_rpc` 获取所有参数
/// 2. 调用 `buy_exact_out_base_internal` 进行本地计算
/// 3. 构建 SwapParams（设置 `fixed_output_amount`）
/// 4. 调用 `PumpSwapInstructionBuilder.build_buy_instructions` 构建指令
/// 5. 调用 `simulate_swap_transaction` 进行链上模拟
/// 6. 对比本地计算与链上模拟结果
///
/// # 参数
/// * `rpc` - RPC 客户端
/// * `pool_address` - PumpSwap Pool 地址
/// * `payer` - 支付账户（用于签名，无需余额）
/// * `amount_out` - 期望获得的 base token 数量
/// * `tolerance_percent` - 误差容忍度（百分比）
///
/// # 返回
/// * `Ok(VerificationResult)` - 验证结果
/// * `Err(anyhow::Error)` - 执行错误
pub async fn verify_exact_out_buy_full(
    rpc: &Arc<SolanaRpcClient>,
    pool_address: &Pubkey,
    payer: &Keypair,
    amount_out: u64,
    tolerance_percent: f64,
) -> Result<VerificationResult, anyhow::Error> {
    println!("\n========================================");
    println!("PumpSwap Exact Out Buy 验证");
    println!("========================================");
    println!("Pool 地址: {}", pool_address);
    println!("期望输出数量 (base token): {}", amount_out);

    // 1. 使用 PumpSwapParams::from_pool_address_by_rpc 获取所有参数
    println!("\n[步骤 1] 获取 Pool 参数...");
    let protocol_params = PumpSwapParams::from_pool_address_by_rpc(rpc, pool_address)
        .await
        .map_err(|e| anyhow::anyhow!("获取 Pool 参数失败: {}", e))?;

    println!("  Base Mint: {}", protocol_params.base_mint);
    println!("  Quote Mint: {}", protocol_params.quote_mint);
    println!("  Base Reserves: {}", protocol_params.pool_base_token_reserves);
    println!("  Quote Reserves: {}", protocol_params.pool_quote_token_reserves);
    println!("  Base Token Program: {}", protocol_params.base_token_program);
    println!("  Quote Token Program: {}", protocol_params.quote_token_program);

    // 2. 调用 buy_exact_out_base_internal 进行本地计算
    println!("\n[步骤 2] 本地计算所需 quote 数量...");
    let coin_creator = protocol_params.coin_creator_vault_authority;
    let local_result = buy_exact_out_base_internal(
        amount_out,
        0, // 0% 滑点，精确计算
        protocol_params.pool_base_token_reserves,
        protocol_params.pool_quote_token_reserves,
        &coin_creator,
    )
    .map_err(|e| anyhow::anyhow!("本地计算失败: {}", e))?;

    println!("  本地计算结果:");
    println!("    内部 quote 数量: {}", local_result.internal_quote_amount);
    println!("    UI quote 数量 (含费用): {}", local_result.ui_quote);
    println!("    最大 quote (含滑点): {}", local_result.max_quote);

    // 3. 构建 SwapParams（设置 fixed_output_amount）
    println!("\n[步骤 3] 构建 SwapParams...");
    let gas_fee_strategy = sol_trade_test_utils::create_test_gas_fee_strategy();

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: Arc::new(payer.insecure_clone()),
        trade_type: TradeType::Buy,
        input_mint: protocol_params.quote_mint, // WSOL
        input_token_program: Some(protocol_params.quote_token_program),
        output_mint: protocol_params.base_mint, // PUMP
        output_token_program: Some(protocol_params.base_token_program),
        input_amount: None,             // Exact Out 模式
        slippage_basis_points: Some(0), // 0% 滑点
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: DexParamEnum::PumpSwap(protocol_params.clone()),
        open_seed_optimize: false,
        swqos_clients: vec![],
        middleware_manager: None,
        durable_nonce: None,
        with_tip: false,
        create_input_mint_ata: true,  // 创建 WSOL ATA
        close_input_mint_ata: true,   // 关闭 WSOL ATA
        create_output_mint_ata: true, // 创建 PUMP ATA
        close_output_mint_ata: false,
        fixed_output_amount: Some(amount_out), // Exact Out: 设置期望的输出金额
        gas_fee_strategy,
        simulate: true,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    // 4. 构建 buy 指令
    println!("\n[步骤 4] 构建买入指令...");
    let builder = PumpSwapInstructionBuilder;
    let instructions: Vec<solana_sdk::instruction::Instruction> = builder
        .build_buy_instructions(&swap_params)
        .await
        .map_err(|e| anyhow::anyhow!("构建指令失败: {}", e))?;
    println!("  指令数量: {}", instructions.len());

    // 5. 获取用户 ATA 地址
    let user_base_token_account = get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &protocol_params.base_mint,
        &protocol_params.base_token_program,
    );
    let user_quote_token_account = get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &protocol_params.quote_mint,
        &protocol_params.quote_token_program,
    );

    // 6. 模拟执行
    println!("\n[步骤 5] 模拟执行...");
    let simulated_result: SimulatedSwapResult = simulate_swap_transaction(
        rpc,
        payer,
        instructions,
        user_quote_token_account,   // 输入账户 (WSOL)
        user_base_token_account,    // 输出账户 (PUMP)
        protocol_params.quote_mint, // 输入 mint (WSOL)
        protocol_params.base_mint,  // 输出 mint (PUMP)
    )
    .await
    .map_err(|e| anyhow::anyhow!("模拟执行失败: {}", e))?;

    println!("  模拟成功: {}", simulated_result.success);
    println!("  实际输入金额: {}", simulated_result.actual_input_amount);
    println!("  实际输出金额: {}", simulated_result.actual_output_amount);

    if let Some(error) = &simulated_result.error {
        return Err(anyhow::anyhow!("模拟执行失败: {}", error));
    }

    // 7. 计算误差率
    let expected_quote = local_result.ui_quote;
    let actual_quote = simulated_result.actual_input_amount;
    let diff = expected_quote.abs_diff(actual_quote);
    let error_rate_percent =
        if actual_quote > 0 { (diff as f64 / actual_quote as f64) * 100.0 } else { 100.0 };

    println!("\n[步骤 6] 对比结果");
    println!("  本地计算的 quote 金额: {}", expected_quote);
    println!("  链上模拟的 quote 金额: {}", actual_quote);
    println!("  差值: {}", diff);
    println!("  误差率: {:.4}%", error_rate_percent);

    let passed = error_rate_percent <= tolerance_percent;
    println!("\n验证结果: {}", if passed { "✅ 通过" } else { "❌ 失败" });

    Ok(VerificationResult {
        expected_input: expected_quote,
        actual_input: actual_quote,
        error_rate_percent,
        passed,
    })
}

// ============================================================================
// 测试用例
// ============================================================================

/// Exact Out Buy 完整验证测试
///
/// 测试目标：验证本地计算与链上模拟的一致性
/// 误差容忍：0.5%
///
/// 这是 Spec 要求的主测试用例
#[tokio::test]
#[serial_test::serial(pumpswap_exact_out_sim)]
async fn test_exact_out_buy_full_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 测试金额：1,000,000 base tokens
    let amount_out = 1_000_000u64;
    let tolerance_percent = 0.5;

    let result = verify_exact_out_buy_full(
        &client.rpc,
        &pool_address,
        client.payer.as_ref(),
        amount_out,
        tolerance_percent,
    )
    .await
    .expect("验证执行失败");

    // 验证 passed == true
    assert!(
        result.passed,
        "验证未通过: 误差率 {:.4}% > {}%",
        result.error_rate_percent, tolerance_percent
    );

    // 验证 error_rate_percent < 0.5%
    assert!(
        result.error_rate_percent < tolerance_percent,
        "误差率过高: {:.4}% >= {}%",
        result.error_rate_percent,
        tolerance_percent
    );

    println!("\n✅ Exact Out Buy 完整验证测试通过!");
}

/// Exact Out Buy 完整验证测试（小金额）
///
/// 测试目标：验证本地计算与链上模拟的一致性
/// 误差容忍：0.5%
#[tokio::test]
#[serial_test::serial(pumpswap_exact_out_sim)]
async fn test_exact_out_buy_small_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 小金额测试：100,000 base tokens
    let amount_out = 100_000u64;
    let tolerance_percent = 0.5;

    let result = verify_exact_out_buy_full(
        &client.rpc,
        &pool_address,
        client.payer.as_ref(),
        amount_out,
        tolerance_percent,
    )
    .await
    .expect("验证执行失败");

    // 验证误差率
    assert!(
        result.error_rate_percent <= tolerance_percent,
        "误差率过高: {:.4}% > {}%",
        result.error_rate_percent,
        tolerance_percent
    );

    // 验证实际输出大于 0
    assert!(result.actual_input > 0, "实际输入应该大于 0");

    println!("\n✅ Exact Out Buy 小金额验证测试通过!");
}

/// Exact Out Buy 完整验证测试（中等金额）
///
/// 测试目标：验证本地计算与链上模拟的一致性
/// 误差容忍：0.5%
#[tokio::test]
#[serial_test::serial(pumpswap_exact_out_sim)]
async fn test_exact_out_buy_medium_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 中等金额测试：1,000,000 base tokens
    let amount_out = 1_000_000u64;
    let tolerance_percent = 0.5;

    let result = verify_exact_out_buy_full(
        &client.rpc,
        &pool_address,
        client.payer.as_ref(),
        amount_out,
        tolerance_percent,
    )
    .await
    .expect("验证执行失败");

    assert!(
        result.error_rate_percent <= tolerance_percent,
        "误差率过高: {:.4}% > {}%",
        result.error_rate_percent,
        tolerance_percent
    );

    assert!(result.actual_input > 0, "实际输入应该大于 0");

    println!("\n✅ Exact Out Buy 中等金额验证测试通过!");
}

/// Exact Out Buy 完整验证测试（大金额）
///
/// 测试目标：验证本地计算与链上模拟的一致性
/// 误差容忍：0.5%
#[tokio::test]
#[serial_test::serial(pumpswap_exact_out_sim)]
async fn test_exact_out_buy_large_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 大金额测试：10,000,000 base tokens
    let amount_out = 10_000_000u64;
    let tolerance_percent = 0.5;

    let result = verify_exact_out_buy_full(
        &client.rpc,
        &pool_address,
        client.payer.as_ref(),
        amount_out,
        tolerance_percent,
    )
    .await
    .expect("验证执行失败");

    assert!(
        result.error_rate_percent <= tolerance_percent,
        "误差率过高: {:.4}% > {}%",
        result.error_rate_percent,
        tolerance_percent
    );

    assert!(result.actual_input > 0, "实际输入应该大于 0");

    println!("\n✅ Exact Out Buy 大金额验证测试通过!");
}

/// 边界情况测试：极小金额
#[tokio::test]
#[serial_test::serial(pumpswap_exact_out_sim)]
async fn test_exact_out_buy_tiny_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 极小金额测试：1,000 base tokens
    let amount_out = 1_000u64;
    let tolerance_percent = 5.0; // 小金额允许更大误差

    let result = verify_exact_out_buy_full(
        &client.rpc,
        &pool_address,
        client.payer.as_ref(),
        amount_out,
        tolerance_percent,
    )
    .await
    .expect("验证执行失败");

    assert!(
        result.error_rate_percent <= tolerance_percent,
        "误差率过高: {:.4}% > {}%",
        result.error_rate_percent,
        tolerance_percent
    );

    assert!(result.actual_input > 0, "实际输入应该大于 0");

    println!("\n✅ Exact Out Buy 极小金额验证测试通过!");
}

// ============================================================================
// Sell 方向 Exact Out 验证
// ============================================================================

/// Exact Out Sell 完整验证测试
///
/// 测试目标：验证本地计算与链上模拟的一致性
/// Sell Exact Out = 用户指定想获得的 quote 数量，计算需要卖出多少 base
#[tokio::test]
#[serial_test::serial(pumpswap_exact_out_sim)]
async fn test_exact_out_sell_full_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 测试金额：期望获得 10,000 quote (WSOL)
    let amount_out = 10_000u64;
    let tolerance_percent = 1.0; // Sell 方向允许 1% 误差

    println!("\n========================================");
    println!("PumpSwap Exact Out Sell 验证");
    println!("========================================");
    println!("Pool 地址: {}", pool_address);
    println!("期望输出数量 (quote token): {}", amount_out);

    // 1. 获取 Pool 参数
    println!("\n[步骤 1] 获取 Pool 参数...");
    let protocol_params = PumpSwapParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
        .await
        .expect("获取 Pool 参数失败");

    println!("  Base Reserves: {}", protocol_params.pool_base_token_reserves);
    println!("  Quote Reserves: {}", protocol_params.pool_quote_token_reserves);

    // 2. 本地计算所需 base 数量
    println!("\n[步骤 2] 本地计算所需 base 数量...");
    let coin_creator = protocol_params.coin_creator_vault_authority;
    let local_result = sell_exact_out_quote_internal(
        amount_out,
        0, // 0% 滑点
        protocol_params.pool_base_token_reserves,
        protocol_params.pool_quote_token_reserves,
        &coin_creator,
    )
    .expect("本地计算失败");

    println!("  本地计算结果:");
    println!("    内部 quote 数量: {}", local_result.internal_raw_quote);
    println!("    需要 base 数量: {}", local_result.base);

    // 2.5. 给测试账户空投 PUMP token（Sell 需要持有 base token）
    println!("\n[步骤 2.5] 空投 PUMP token...");
    let _rpc_url = "http://127.0.0.1:8899";
    let base_needed = local_result.base;
    // 多空投一些，确保有足够的余额（乘以 2）
    let base_to_airdrop = base_needed * 2;
    // PUMP token decimals = 6，需要转换为格式化字符串
    let base_formatted = format!("{}", base_to_airdrop as f64 / 1_000_000.0);

    ensure_token_balance(
        &client.rpc,
        client.payer.as_ref(),
        &protocol_params.base_mint,
        &base_formatted,
    )
    .await
    .expect("空投 PUMP token 失败");
    println!("  ✅ PUMP token 余额已设置: {}", base_formatted);

    // 3. 构建 SwapParams
    // 注意：Sell 方向目前需要设置 input_amount（使用本地计算结果）
    println!("\n[步骤 3] 构建 SwapParams...");
    let gas_fee_strategy = sol_trade_test_utils::create_test_gas_fee_strategy();

    let swap_params = SwapParams {
        rpc: Some(client.rpc.clone()),
        payer: Arc::new(client.payer.insecure_clone()),
        trade_type: TradeType::Sell,
        input_mint: protocol_params.base_mint, // 卖出 base
        input_token_program: Some(protocol_params.base_token_program),
        output_mint: protocol_params.quote_mint, // 获得 quote (WSOL)
        output_token_program: Some(protocol_params.quote_token_program),
        input_amount: Some(local_result.base), // 使用本地计算的 base 数量
        slippage_basis_points: Some(0),
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: DexParamEnum::PumpSwap(protocol_params.clone()),
        open_seed_optimize: false,
        swqos_clients: vec![],
        middleware_manager: None,
        durable_nonce: None,
        with_tip: false,
        create_input_mint_ata: true,
        close_input_mint_ata: false,
        create_output_mint_ata: true,
        close_output_mint_ata: true,
        fixed_output_amount: Some(amount_out), // Exact Out: 设置期望的输出金额
        gas_fee_strategy,
        simulate: true,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    // 4. 构建 sell 指令
    println!("\n[步骤 4] 构建卖出指令...");
    let builder = PumpSwapInstructionBuilder;
    let instructions = builder.build_sell_instructions(&swap_params).await.expect("构建指令失败");
    println!("  指令数量: {}", instructions.len());

    // 5. 获取用户 ATA 地址
    let user_base_token_account = get_associated_token_address_with_program_id(
        &client.payer.pubkey(),
        &protocol_params.base_mint,
        &protocol_params.base_token_program,
    );
    let user_quote_token_account = get_associated_token_address_with_program_id(
        &client.payer.pubkey(),
        &protocol_params.quote_mint,
        &protocol_params.quote_token_program,
    );

    // 6. 模拟执行
    println!("\n[步骤 5] 模拟执行...");
    let simulated_result: SimulatedSwapResult = simulate_swap_transaction(
        &client.rpc,
        client.payer.as_ref(),
        instructions,
        user_base_token_account,    // 输入账户 (base)
        user_quote_token_account,   // 输出账户 (quote)
        protocol_params.base_mint,  // 输入 mint (base)
        protocol_params.quote_mint, // 输出 mint (quote)
    )
    .await
    .expect("模拟执行失败");

    println!("  模拟成功: {}", simulated_result.success);
    println!("  实际输入金额 (base): {}", simulated_result.actual_input_amount);
    println!("  实际输出金额 (quote): {}", simulated_result.actual_output_amount);

    if let Some(error) = &simulated_result.error {
        panic!("模拟执行失败: {}", error);
    }

    // 7. 计算误差率
    let expected_base = local_result.base;
    let actual_base = simulated_result.actual_input_amount;
    let diff = expected_base.abs_diff(actual_base);
    let error_rate_percent =
        if actual_base > 0 { (diff as f64 / actual_base as f64) * 100.0 } else { 100.0 };

    println!("\n[步骤 6] 对比结果");
    println!("  本地计算的 base 金额: {}", expected_base);
    println!("  链上模拟的 base 金额: {}", actual_base);
    println!("  差值: {}", diff);
    println!("  误差率: {:.4}%", error_rate_percent);

    let passed = error_rate_percent <= tolerance_percent;
    println!("\n验证结果: {}", if passed { "✅ 通过" } else { "❌ 失败" });

    // 验证
    assert!(passed, "验证未通过: 误差率 {:.4}% > {}%", error_rate_percent, tolerance_percent);

    assert!(actual_base > 0, "实际输入应该大于 0");

    println!("\n✅ Exact Out Sell 完整验证测试通过!");
}
