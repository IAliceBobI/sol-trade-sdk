//! Raydium CLMM Exact Out 链上模拟验证测试
//!
//! 通过 `simulateTransaction` 验证本地计算结果与实际链上执行结果的一致性
//!
//! # 测试目标
//!
//! 验证 `buy_exact_out_internal` 的本地计算与链上模拟执行结果的一致性
//!
//! # 运行测试
//!
//! ```bash
//! # 运行所有 Raydium CLMM Exact Out 模拟测试
//! cargo nextest run raydium_clmm_exact_out_sim --nocapture 2>&1
//! ```

mod test_helpers;
use test_helpers::create_test_client;

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::raydium_clmm::RaydiumClmmInstructionBuilder,
    swqos::TradeType,
    trading::core::params::{DexParamEnum, RaydiumClmmParams, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::{
        calc::raydium_clmm::buy_exact_out_internal,
        simulation_based_calc::{SimulatedSwapResult, simulate_swap_transaction},
    },
};
use sol_trade_test_utils::{ensure_token_balance, wsol_mint};
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

/// JUP-WSOL Pool 地址（Raydium CLMM mainnet）
/// Pool: EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw
/// JUP Mint: JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN
fn get_test_pool_address() -> Pubkey {
    Pubkey::from_str("EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw").expect("Invalid pool address")
}

/// 完整的 Exact Out Buy 验证流程
///
/// # 流程
/// 1. 通过 `RaydiumClmmParams::from_pool_address_by_rpc` 获取所有参数
/// 2. 获取 Pool 状态和 Tick Arrays
/// 3. 调用 `quote_exact_out` 进行本地计算
/// 4. 构建 SwapParams（设置 `fixed_output_amount`）
/// 5. 调用 `RaydiumClmmInstructionBuilder.build_buy_instructions` 构建指令
/// 6. 调用 `simulate_swap_transaction` 进行链上模拟
/// 7. 对比本地计算与链上模拟结果
///
/// # 参数
/// * `rpc` - RPC 客户端
/// * `pool_address` - Raydium CLMM Pool 地址
/// * `payer` - 支付账户（用于签名，无需余额）
/// * `amount_out` - 期望获得的 token 数量
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
    println!("Raydium CLMM Exact Out Buy 验证");
    println!("========================================");
    println!("Pool 地址: {}", pool_address);
    println!("期望输出数量: {}", amount_out);

    // 0. 确保测试账户有足够的 WSOL 余额
    println!("\n[步骤 0] 确保 WSOL 余额...");
    let rpc_url = "http://127.0.0.1:8899";
    let wsol = wsol_mint();
    // 确保足够多的 WSOL（10 WSOL）用于测试
    if let Err(e) = ensure_token_balance(rpc, rpc_url, payer, &wsol, "10").await {
        println!("⚠️  确保 WSOL 余额警告: {}", e);
        println!("继续测试，但可能因余额不足而失败...");
    } else {
        println!("  ✅ WSOL 余额已确保");
    }

    // 1. 使用 RaydiumClmmParams::from_pool_address_by_rpc 获取所有参数
    println!("\n[步骤 1] 获取 Pool 参数...");
    let protocol_params = RaydiumClmmParams::from_pool_address_by_rpc(rpc, pool_address)
        .await
        .map_err(|e| anyhow::anyhow!("获取 Pool 参数失败: {}", e))?;

    println!("  Token0 Mint: {}", protocol_params.token0_mint);
    println!("  Token1 Mint: {}", protocol_params.token1_mint);
    println!("  Token0 Decimals: {}", protocol_params.token0_decimals);
    println!("  Token1 Decimals: {}", protocol_params.token1_decimals);

    // 2. 获取 Pool 状态
    println!("\n[步骤 2] 获取 Pool 状态...");
    let pool_state =
        sol_trade_sdk::instruction::utils::raydium_clmm::get_pool_by_address(rpc, pool_address)
            .await
            .map_err(|e| anyhow::anyhow!("获取 Pool 状态失败: {}", e))?;

    println!("  当前价格 (sqrt_price_x64): {}", pool_state.sqrt_price_x64);
    println!("  当前流动性: {}", pool_state.liquidity);
    println!("  当前 Tick: {}", pool_state.tick_current);
    println!("  Tick 间距: {}", pool_state.tick_spacing);

    // 3. 获取 AMM config 以获取费率
    let amm_config = sol_trade_sdk::instruction::utils::raydium_clmm::get_amm_config(
        rpc,
        &pool_state.amm_config,
    )
    .await
    .map_err(|e| anyhow::anyhow!("获取 AMM config 失败: {}", e))?;

    let fee_rate = amm_config.trade_fee_rate as u32;
    println!("  交易费率: {}", fee_rate);

    // 4. 获取 Tick Arrays
    println!("\n[步骤 3] 获取 Tick Arrays...");
    let current_tick_array_start =
        sol_trade_sdk::instruction::utils::raydium_clmm::get_tick_array_start_index(
            pool_state.tick_current,
            pool_state.tick_spacing,
        );

    let tick_spacing_i32 = pool_state.tick_spacing as i32;
    let ticks_per_array = 60 * tick_spacing_i32;

    let mut tick_array_indices = vec![current_tick_array_start];

    let prev_index = current_tick_array_start - ticks_per_array;
    let next_index = current_tick_array_start + ticks_per_array;

    const MIN_TICK: i32 = -443636;
    const MAX_TICK: i32 = 443636;

    if prev_index >= MIN_TICK {
        tick_array_indices.push(prev_index);
    }
    if next_index <= MAX_TICK {
        tick_array_indices.push(next_index);
    }

    let tick_arrays = sol_trade_sdk::instruction::utils::raydium_clmm::get_tick_arrays(
        rpc,
        pool_address,
        &tick_array_indices,
    )
    .await
    .map_err(|e| anyhow::anyhow!("获取 Tick Arrays 失败: {}", e))?;

    println!("  获取到 {} 个 Tick Arrays", tick_arrays.len());

    // 如果没有 tick arrays（fork 网络上可能不存在），跳过测试
    if tick_arrays.is_empty() {
        return Err(anyhow::anyhow!(
            "Tick arrays 不存在于 fork 网络，跳过测试。Pool: {}",
            pool_address
        ));
    }

    // 转换为计算所需的格式
    let tick_data: Vec<(i32, Vec<(i32, i128, u128)>)> = tick_arrays
        .iter()
        .map(|(start_index, tick_array)| {
            let ticks = tick_array
                .ticks
                .iter()
                .filter(|t| t.liquidity_gross > 0)
                .map(|t| (t.tick, t.liquidity_net, t.liquidity_gross))
                .collect();
            (*start_index, ticks)
        })
        .collect();

    // 5. 确定交易方向
    // Buy 操作：用 quote token（WSOL/USDC）买 base token
    // 对于 JUP-WSOL pool：token0=JUP(base), token1=WSOL(quote)
    // Buy = 用 WSOL 买 JUP = 输入 token1，输出 token0
    println!("  Buy: 输入 WSOL，输出 JUP");

    // 6. 调用 buy_exact_out_internal 进行本地计算
    println!("\n[步骤 4] 本地计算所需输入数量...");
    let local_result = buy_exact_out_internal(
        amount_out,
        pool_state.sqrt_price_x64,
        pool_state.liquidity,
        pool_state.tick_current,
        pool_state.tick_spacing,
        fee_rate,
        &tick_data,
    )
    .map_err(|e| anyhow::anyhow!("本地计算失败: {}", e))?;

    println!("  本地计算结果:");
    println!("    所需输入数量: {}", local_result.amount_in);
    println!("    手续费: {}", local_result.fee_amount);

    // 7. 构建 SwapParams（设置 fixed_output_amount）
    println!("\n[步骤 5] 构建 SwapParams...");
    let gas_fee_strategy = sol_trade_test_utils::create_test_gas_fee_strategy();

    // Buy 操作的输入输出 mint
    // 输入 = quote token (token1 = WSOL)
    // 输出 = base token (token0 = JUP)
    let input_mint = protocol_params.token1_mint;
    let output_mint = protocol_params.token0_mint;
    let input_token_program = protocol_params.token1_program;
    let output_token_program = protocol_params.token0_program;

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: Arc::new(payer.insecure_clone()),
        trade_type: TradeType::Buy,
        input_mint,
        input_token_program: Some(input_token_program),
        output_mint,
        output_token_program: Some(output_token_program),
        input_amount: Some(local_result.amount_in), // 使用本地计算的输入金额
        slippage_basis_points: Some(10),            // 0.1% 滑点
        address_lookup_table_account: None,
        recent_blockhash: None,
        wait_transaction_confirmed: false,
        protocol_params: DexParamEnum::RaydiumClmm(protocol_params.clone()),
        open_seed_optimize: false,
        swqos_clients: vec![],
        middleware_manager: None,
        durable_nonce: None,
        with_tip: false,
        create_input_mint_ata: true,  // 创建输入 token ATA
        close_input_mint_ata: true,   // 关闭输入 token ATA
        create_output_mint_ata: true, // 创建输出 token ATA
        close_output_mint_ata: false,
        fixed_output_amount: Some(amount_out), // Exact Out: 设置期望的输出金额
        gas_fee_strategy,
        simulate: true,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    // 8. 构建 buy 指令
    println!("\n[步骤 6] 构建买入指令...");
    let builder = RaydiumClmmInstructionBuilder;
    let instructions: Vec<solana_sdk::instruction::Instruction> = builder
        .build_buy_instructions(&swap_params)
        .await
        .map_err(|e| anyhow::anyhow!("构建指令失败: {}", e))?;
    println!("  指令数量: {}", instructions.len());

    // 9. 获取用户 ATA 地址
    let user_input_token_account = get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &input_mint,
        &input_token_program,
    );
    let user_output_token_account = get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &output_mint,
        &output_token_program,
    );

    // 10. 模拟执行
    println!("\n[步骤 7] 模拟执行...");
    let simulated_result: SimulatedSwapResult = simulate_swap_transaction(
        rpc,
        payer,
        instructions,
        user_input_token_account,  // 输入账户
        user_output_token_account, // 输出账户
        input_mint,                // 输入 mint
        output_mint,               // 输出 mint
    )
    .await
    .map_err(|e| anyhow::anyhow!("模拟执行失败: {}", e))?;

    println!("  模拟成功: {}", simulated_result.success);
    println!("  实际输入金额: {}", simulated_result.actual_input_amount);
    println!("  实际输出金额: {}", simulated_result.actual_output_amount);

    if let Some(error) = &simulated_result.error {
        return Err(anyhow::anyhow!("模拟执行失败: {}", error));
    }

    // 11. 计算误差率
    let expected_input = local_result.amount_in;
    let actual_input = simulated_result.actual_input_amount;
    let diff = expected_input.abs_diff(actual_input);
    let error_rate_percent =
        if actual_input > 0 { (diff as f64 / actual_input as f64) * 100.0 } else { 100.0 };

    println!("\n[步骤 8] 对比结果");
    println!("  本地计算的输入金额: {}", expected_input);
    println!("  链上模拟的输入金额: {}", actual_input);
    println!("  差值: {}", diff);
    println!("  误差率: {:.4}%", error_rate_percent);

    let passed = error_rate_percent <= tolerance_percent;
    println!("\n验证结果: {}", if passed { "✅ 通过" } else { "❌ 失败" });

    Ok(VerificationResult { expected_input, actual_input, error_rate_percent, passed })
}

// ============================================================================
// 测试用例
// ============================================================================

/// Exact Out Buy 完整验证测试
///
/// 测试目标：验证本地计算与链上模拟的一致性
/// 误差容忍：0.5%
#[tokio::test]
#[serial_test::serial(raydium_clmm_exact_out_sim)]
async fn test_exact_out_buy_full_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 测试金额：1,000,000 (根据 token decimals 调整)
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

    println!("\n✅ Raydium CLMM Exact Out Buy 完整验证测试通过!");
}

/// Exact Out Buy 完整验证测试（小金额）
///
/// 测试目标：验证本地计算与链上模拟的一致性（小金额场景）
/// 误差容忍：0.5%
#[tokio::test]
#[serial_test::serial(raydium_clmm_exact_out_sim)]
async fn test_exact_out_buy_small_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 小金额测试：100,000
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

    println!("\n✅ Raydium CLMM Exact Out Buy 小金额验证测试通过!");
}

/// Exact Out Buy 完整验证测试（中等金额）
///
/// 测试目标：验证本地计算与链上模拟的一致性（中等金额场景）
/// 误差容忍：0.5%
#[tokio::test]
#[serial_test::serial(raydium_clmm_exact_out_sim)]
async fn test_exact_out_buy_medium_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 中等金额测试：5,000,000
    let amount_out = 5_000_000u64;
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

    println!("\n✅ Raydium CLMM Exact Out Buy 中等金额验证测试通过!");
}

/// Exact Out Buy 完整验证测试（大金额）
///
/// 测试目标：验证本地计算与链上模拟的一致性（大金额场景）
/// 误差容忍：0.5%
#[tokio::test]
#[serial_test::serial(raydium_clmm_exact_out_sim)]
async fn test_exact_out_buy_large_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 大金额测试：50,000,000
    let amount_out = 50_000_000u64;
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

    println!("\n✅ Raydium CLMM Exact Out Buy 大金额验证测试通过!");
}

/// 边界情况测试：极小金额
#[tokio::test]
#[serial_test::serial(raydium_clmm_exact_out_sim)]
async fn test_exact_out_buy_tiny_verification() {
    let client = create_test_client().await;
    let pool_address = get_test_pool_address();

    // 极小金额测试：10,000
    let amount_out = 10_000u64;
    let tolerance_percent = 1.0; // 小金额允许更大误差

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

    println!("\n✅ Raydium CLMM Exact Out Buy 极小金额验证测试通过!");
}
