//! Raydium AMM V4 Exact Out 链上模拟验证测试
//!
//! 通过 `simulateTransaction` 验证本地计算结果与实际链上执行结果的一致性
//!
//! # 测试目标
//!
//! 验证 `buy_exact_out_internal` 和 `sell_exact_out_internal` 的本地计算与链上模拟执行结果的一致性
//!
//! # 运行测试
//!
//! ```bash
//! # 运行所有 Raydium AMM V4 Exact Out 模拟测试
//! cargo nextest run raydium_amm_v4_exact_out_sim --nocapture 2>&1
//! ```

mod test_helpers;
use test_helpers::create_test_client;

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::raydium_amm_v4::RaydiumAmmV4InstructionBuilder,
    swqos::TradeType,
    trading::core::params::{DexParamEnum, RaydiumAmmV4Params, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::{
        calc::raydium_amm_v4::buy_exact_out_internal,
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

/// SOL-USDC Pool 地址（Raydium AMM V4 mainnet）
fn get_test_pool_address() -> Pubkey {
    Pubkey::from_str("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2").expect("Invalid pool address")
}

/// 完整的 Exact Out Buy 验证流程
///
/// # 流程
/// 1. 通过 `RaydiumAmmV4Params::from_amm_address_by_rpc` 获取所有参数
/// 2. 调用 `buy_exact_out_internal` 进行本地计算
/// 3. 构建 SwapParams（设置 `fixed_output_amount`）
/// 4. 调用 `RaydiumAmmV4InstructionBuilder.build_buy_instructions` 构建指令
/// 5. 调用 `simulate_swap_transaction` 进行链上模拟
/// 6. 对比本地计算与链上模拟结果
///
/// # 参数
/// * `rpc` - RPC 客户端
/// * `amm_address` - Raydium AMM V4 Pool 地址
/// * `payer` - 支付账户（用于签名，无需余额）
/// * `amount_out` - 期望获得的 USDC 数量
/// * `tolerance_percent` - 误差容忍度（百分比）
///
/// # 返回
/// * `Ok(VerificationResult)` - 验证结果
/// * `Err(anyhow::Error)` - 执行错误
pub async fn verify_exact_out_buy_full(
    rpc: &Arc<SolanaRpcClient>,
    amm_address: &Pubkey,
    payer: &Keypair,
    amount_out: u64,
    tolerance_percent: f64,
) -> Result<VerificationResult, anyhow::Error> {
    println!("\n========================================");
    println!("Raydium AMM V4 Exact Out Buy 验证");
    println!("========================================");
    println!("AMM 地址: {}", amm_address);
    println!("期望输出数量 (USDC): {}", amount_out);

    // 1. 使用 RaydiumAmmV4Params::from_amm_address_by_rpc 获取所有参数
    println!("\n[步骤 1] 获取 AMM 参数...");
    let protocol_params = RaydiumAmmV4Params::from_amm_address_by_rpc(rpc, *amm_address)
        .await
        .map_err(|e| anyhow::anyhow!("获取 AMM 参数失败: {}", e))?;

    println!("  Coin Mint: {}", protocol_params.coin_mint);
    println!("  PC Mint: {}", protocol_params.pc_mint);
    println!("  Coin Reserves: {}", protocol_params.coin_reserve);
    println!("  PC Reserves: {}", protocol_params.pc_reserve);

    // 2. 调用 buy_exact_out_internal 进行本地计算（用 SOL 买 USDC）
    println!("\n[步骤 2] 本地计算所需 SOL 数量...");
    let local_result = buy_exact_out_internal(
        protocol_params.coin_reserve,
        protocol_params.pc_reserve,
        amount_out,
    )
    .map_err(|e| anyhow::anyhow!("本地计算失败: {}", e))?;

    println!("  本地计算结果:");
    println!("    所需 SOL 数量: {}", local_result.amount_in);
    println!("    手续费: {}", local_result.fee_amount);
    println!("    价格影响: {:?} bps", local_result.price_impact_bps);

    // 3. 构建 SwapParams（设置 fixed_output_amount）
    println!("\n[步骤 3] 构建 SwapParams...");
    let gas_fee_strategy = sol_trade_test_utils::create_test_gas_fee_strategy();

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: Arc::new(payer.insecure_clone()),
        trade_type: TradeType::Buy,
        input_mint: protocol_params.coin_mint, // SOL
        input_token_program: Some(sol_trade_sdk::constants::TOKEN_PROGRAM),
        output_mint: protocol_params.pc_mint, // USDC
        output_token_program: Some(sol_trade_sdk::constants::TOKEN_PROGRAM),
        input_amount: Some(local_result.amount_in), // 使用本地计算的输入金额
        slippage_basis_points: Some(10),             // 0.1% 滑点（允许1单位的精度误差）
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: DexParamEnum::RaydiumAmmV4(protocol_params.clone()),
        open_seed_optimize: false,
        swqos_clients: vec![],
        middleware_manager: None,
        durable_nonce: None,
        with_tip: false,
        create_input_mint_ata: true,  // 创建 WSOL ATA
        close_input_mint_ata: true,   // 关闭 WSOL ATA
        create_output_mint_ata: true, // 创建 USDC ATA
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
    let builder = RaydiumAmmV4InstructionBuilder;
    let instructions: Vec<solana_sdk::instruction::Instruction> = builder
        .build_buy_instructions(&swap_params)
        .await
        .map_err(|e| anyhow::anyhow!("构建指令失败: {}", e))?;
    println!("  指令数量: {}", instructions.len());

    // 5. 获取用户 ATA 地址
    let user_base_token_account = get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &protocol_params.coin_mint,
        &sol_trade_sdk::constants::TOKEN_PROGRAM,
    );
    let user_quote_token_account = get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &protocol_params.pc_mint,
        &sol_trade_sdk::constants::TOKEN_PROGRAM,
    );

    // 6. 模拟执行
    println!("\n[步骤 5] 模拟执行...");
    let simulated_result: SimulatedSwapResult = simulate_swap_transaction(
        rpc,
        payer,
        instructions,
        user_base_token_account, // 输入账户 (WSOL)
        user_quote_token_account, // 输出账户 (USDC)
        protocol_params.coin_mint, // 输入 mint (SOL)
        protocol_params.pc_mint,   // 输出 mint (USDC)
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
    let expected_input = local_result.amount_in;
    let actual_input = simulated_result.actual_input_amount;
    let diff = expected_input.abs_diff(actual_input);
    let error_rate_percent =
        if actual_input > 0 { (diff as f64 / actual_input as f64) * 100.0 } else { 100.0 };

    println!("\n[步骤 6] 对比结果");
    println!("  本地计算的 SOL 金额: {}", expected_input);
    println!("  链上模拟的 SOL 金额: {}", actual_input);
    println!("  差值: {}", diff);
    println!("  误差率: {:.4}%", error_rate_percent);

    let passed = error_rate_percent <= tolerance_percent;
    println!("\n验证结果: {}", if passed { "✅ 通过" } else { "❌ 失败" });

    Ok(VerificationResult {
        expected_input,
        actual_input,
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
#[tokio::test]
#[serial_test::serial(raydium_amm_v4_exact_out_sim)]
async fn test_exact_out_buy_full_verification() {
    let client = create_test_client().await;
    let amm_address = get_test_pool_address();

    // 测试金额：1,000,000 USDC (0.001 USDC，考虑到 USDC 有 6 位小数)
    let amount_out = 1_000_000u64;
    let tolerance_percent = 0.5;

    let result = verify_exact_out_buy_full(
        &client.rpc,
        &amm_address,
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
        result.error_rate_percent,
        tolerance_percent
    );

    // 验证 error_rate_percent < 0.5%
    assert!(
        result.error_rate_percent < tolerance_percent,
        "误差率过高: {:.4}% >= {}%",
        result.error_rate_percent,
        tolerance_percent
    );

    println!("\n✅ Raydium AMM V4 Exact Out Buy 完整验证测试通过!");
}

/// Exact Out Buy 完整验证测试（小金额）
///
/// 测试目标：验证本地计算与链上模拟的一致性（小金额场景）
/// 误差容忍：0.5%
#[tokio::test]
#[serial_test::serial(raydium_amm_v4_exact_out_sim)]
async fn test_exact_out_buy_small_verification() {
    let client = create_test_client().await;
    let amm_address = get_test_pool_address();

    // 小金额测试：100,000 USDC
    let amount_out = 100_000u64;
    let tolerance_percent = 0.5;

    let result = verify_exact_out_buy_full(
        &client.rpc,
        &amm_address,
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

    println!("\n✅ Raydium AMM V4 Exact Out Buy 小金额验证测试通过!");
}

/// Exact Out Buy 完整验证测试（中等金额）
///
/// 测试目标：验证本地计算与链上模拟的一致性（中等金额场景）
/// 误差容忍：0.5%
#[tokio::test]
#[serial_test::serial(raydium_amm_v4_exact_out_sim)]
async fn test_exact_out_buy_medium_verification() {
    let client = create_test_client().await;
    let amm_address = get_test_pool_address();

    // 中等金额测试：5,000,000 USDC
    let amount_out = 5_000_000u64;
    let tolerance_percent = 0.5;

    let result = verify_exact_out_buy_full(
        &client.rpc,
        &amm_address,
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

    println!("\n✅ Raydium AMM V4 Exact Out Buy 中等金额验证测试通过!");
}

/// Exact Out Buy 完整验证测试（大金额）
///
/// 测试目标：验证本地计算与链上模拟的一致性（大金额场景）
/// 误差容忍：0.5%
#[tokio::test]
#[serial_test::serial(raydium_amm_v4_exact_out_sim)]
async fn test_exact_out_buy_large_verification() {
    let client = create_test_client().await;
    let amm_address = get_test_pool_address();

    // 大金额测试：50,000,000 USDC
    let amount_out = 50_000_000u64;
    let tolerance_percent = 0.5;

    let result = verify_exact_out_buy_full(
        &client.rpc,
        &amm_address,
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

    println!("\n✅ Raydium AMM V4 Exact Out Buy 大金额验证测试通过!");
}

/// 边界情况测试：极小金额
#[tokio::test]
#[serial_test::serial(raydium_amm_v4_exact_out_sim)]
async fn test_exact_out_buy_tiny_verification() {
    let client = create_test_client().await;
    let amm_address = get_test_pool_address();

    // 极小金额测试：10,000 USDC
    let amount_out = 10_000u64;
    let tolerance_percent = 1.0; // 小金额允许更大误差

    let result = verify_exact_out_buy_full(
        &client.rpc,
        &amm_address,
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

    println!("\n✅ Raydium AMM V4 Exact Out Buy 极小金额验证测试通过!");
}
