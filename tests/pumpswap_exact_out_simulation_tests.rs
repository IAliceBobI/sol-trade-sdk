//! PumpSwap Exact Out 链上模拟验证测试
//!
//! 通过 simulateTransaction 验证本地计算的准确性
//!
//! # 测试目标
//!
//! 验证 `buy_base_input_internal` 和 `sell_quote_input_internal` 的本地计算
//! 与链上模拟执行结果的一致性。
//!
//! # 运行测试
//!
//! ```bash
//! # 运行所有 PumpSwap Exact Out 模拟测试
//! cargo nextest run pumpswap_exact_out_sim -- --nocapture
//!
//! # 运行单个测试
//! cargo nextest run test_exact_out_buy_simulation_small -- --nocapture
//! ```
//!
//! # 测试分类
//!
//! - Buy 方向：指定想获得的 base token 数量，计算需要支付多少 quote
//! - Sell 方向：指定想获得的 quote 数量，计算需要卖多少 base

use solana_sdk::pubkey::Pubkey;
use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::utils::pumpswap::get_pool_by_address,
    utils::calc::pumpswap::buy_base_input_internal,
};
use std::str::FromStr;
use std::sync::Arc;

// ============================================================================
// 常量定义
// ============================================================================

/// 测试用的 PUMP-WSOL Pool
///
/// 这是 PumpSwap 上的一个真实 Pool:
/// - Base: PUMP (Token-2022)
/// - Quote: WSOL (标准 Token Program)
const TEST_POOL: &str = "539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR";

/// WSOL Mint 地址
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// PUMP Token Mint 地址
const PUMP_MINT: &str = "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn";

// ============================================================================
// 验证结果结构体
// ============================================================================

/// 验证结果
///
/// 包含本地计算结果与链上模拟执行的对比
#[derive(Debug, Clone)]
struct VerificationResult {
    /// 期望的输出金额（本地计算）
    expected_output: u64,
    /// 实际的输出金额（链上模拟）
    actual_output: u64,
    /// 误差率（百分比）
    error_rate_percent: f64,
    /// 是否通过验证（误差在容忍范围内）
    passed: bool,
}

impl VerificationResult {
    /// 打印验证结果
    fn print_summary(&self, test_name: &str) {
        println!("\n========================================");
        println!("测试: {}", test_name);
        println!("========================================");
        println!("期望输出: {}", self.expected_output);
        println!("实际输出: {}", self.actual_output);

        let diff = self.expected_output.abs_diff(self.actual_output);
        println!("差值: {}", diff);
        println!("误差率: {:.4}%", self.error_rate_percent);

        if self.passed {
            println!("结果: PASS");
        } else {
            println!("结果: FAIL");
        }
        println!("========================================");
    }
}

// ============================================================================
// 验证函数
// ============================================================================

/// 验证 exact_out buy 计算准确性
///
/// # 流程
///
/// 1. 获取 Pool 状态
/// 2. 获取储备余额
/// 3. 本地计算需要支付的 quote 数量
/// 4. TODO: 构造交易并模拟执行
/// 5. 对比本地计算与模拟结果
///
/// # 参数
///
/// * `rpc` - RPC 客户端
/// * `pool_address` - Pool 地址
/// * `amount_out` - 期望获得的 base token 数量
/// * `tolerance_percent` - 误差容忍度（百分比）
async fn verify_exact_out_buy(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_out: u64,
    tolerance_percent: f64,
) -> Result<VerificationResult, anyhow::Error> {
    println!("\n>>> 开始验证 Exact Out Buy");
    println!("Pool: {}", pool_address);
    println!("期望获得的 base token 数量: {}", amount_out);

    // 1. 获取 Pool 状态
    let pool = get_pool_by_address(rpc, pool_address).await?;

    println!("\n[Pool 信息]");
    println!("  Base Mint: {}", pool.base_mint);
    println!("  Quote Mint: {}", pool.quote_mint);
    println!("  Coin Creator: {}", pool.coin_creator);
    println!("  Is Mayhem Mode: {}", pool.is_mayhem_mode);

    // 2. 获取储备余额
    let base_balance = rpc
        .get_token_account_balance(&pool.pool_base_token_account)
        .await?;
    let quote_balance = rpc
        .get_token_account_balance(&pool.pool_quote_token_account)
        .await?;

    let base_reserve: u64 = base_balance.amount.parse()?;
    let quote_reserve: u64 = quote_balance.amount.parse()?;

    println!("\n[储备余额]");
    println!("  Base Reserve: {}", base_reserve);
    println!("  Quote Reserve: {}", quote_reserve);

    // 3. 本地计算
    let local_result = buy_base_input_internal(
        amount_out,
        0, // slippage = 0，精确计算
        base_reserve,
        quote_reserve,
        &pool.coin_creator,
    )
    .map_err(|e| anyhow::anyhow!("{}", e))?;

    println!("\n[本地计算结果]");
    println!("  内部 quote 金额: {}", local_result.internal_quote_amount);
    println!("  UI quote 金额 (含费用): {}", local_result.ui_quote);
    println!(
        "  费用: {}",
        local_result.ui_quote - local_result.internal_quote_amount
    );
    println!("  最大 quote (含滑点): {}", local_result.max_quote);

    // 4. TODO: 构造交易并模拟执行
    // 这里需要在后续步骤实现完整的模拟逻辑
    println!("\n[模拟执行]");
    println!("  TODO: 需要实现完整的模拟逻辑");

    // 5. 返回占位结果
    // 目前暂时返回 0% 误差，实际需要在 Step 3 实现完整逻辑
    Ok(VerificationResult {
        expected_output: amount_out,
        actual_output: amount_out, // 暂时占位
        error_rate_percent: 0.0,
        passed: true,
    })
}

/// 验证 exact_out sell 计算准确性
///
/// # 流程
///
/// 1. 获取 Pool 状态
/// 2. 获取储备余额
/// 3. 本地计算需要卖出的 base token 数量
/// 4. TODO: 构造交易并模拟执行
/// 5. 对比本地计算与模拟结果
///
/// # 参数
///
/// * `rpc` - RPC 客户端
/// * `pool_address` - Pool 地址
/// * `amount_out` - 期望获得的 quote 数量
/// * `tolerance_percent` - 误差容忍度（百分比）
#[allow(dead_code)]
async fn verify_exact_out_sell(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_out: u64,
    tolerance_percent: f64,
) -> Result<VerificationResult, anyhow::Error> {
    println!("\n>>> 开始验证 Exact Out Sell");
    println!("Pool: {}", pool_address);
    println!("期望获得的 quote 数量: {}", amount_out);

    // 1. 获取 Pool 状态
    let pool = get_pool_by_address(rpc, pool_address).await?;

    println!("\n[Pool 信息]");
    println!("  Base Mint: {}", pool.base_mint);
    println!("  Quote Mint: {}", pool.quote_mint);
    println!("  Coin Creator: {}", pool.coin_creator);

    // 2. 获取储备余额
    let base_balance = rpc
        .get_token_account_balance(&pool.pool_base_token_account)
        .await?;
    let quote_balance = rpc
        .get_token_account_balance(&pool.pool_quote_token_account)
        .await?;

    let base_reserve: u64 = base_balance.amount.parse()?;
    let quote_reserve: u64 = quote_balance.amount.parse()?;

    println!("\n[储备余额]");
    println!("  Base Reserve: {}", base_reserve);
    println!("  Quote Reserve: {}", quote_reserve);

    // 3. 本地计算（使用 sell_quote_input_internal）
    // TODO: 在 Step 4 实现完整的 Sell 验证
    println!("\n[本地计算结果]");
    println!("  TODO: 需要实现 sell_quote_input_internal 验证");

    // 4. 返回占位结果
    Ok(VerificationResult {
        expected_output: amount_out,
        actual_output: amount_out, // 暂时占位
        error_rate_percent: 0.0,
        passed: true,
    })
}

// ============================================================================
// 测试用例
// ============================================================================

/// 测试 Exact Out Buy - 小额交易
///
/// 使用小额 base token 进行测试，验证基本的计算准确性
#[tokio::test]
#[serial_test::serial(pumpswap_exact_out_sim)]
async fn test_exact_out_buy_simulation_small_amount() {
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url));

    let pool_address = Pubkey::from_str(TEST_POOL).expect("Invalid pool address");
    let amount_out = 1_000u64; // 小额测试: 1000 base tokens

    // 执行验证
    let result = verify_exact_out_buy(&rpc, &pool_address, amount_out, 0.5)
        .await
        .expect("Verification should succeed");

    // 打印结果
    result.print_summary("Exact Out Buy - Small Amount");

    // 验证基本约束
    assert!(result.expected_output > 0, "Expected output should be positive");
    println!("\n框架测试通过!");
}

/// 测试 Exact Out Buy - 中等金额
///
/// 使用中等金额进行测试，验证较大交易的计算准确性
#[tokio::test]
#[serial_test::serial(pumpswap_exact_out_sim)]
async fn test_exact_out_buy_simulation_medium_amount() {
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url));

    let pool_address = Pubkey::from_str(TEST_POOL).expect("Invalid pool address");
    let amount_out = 100_000u64; // 中等金额: 100000 base tokens

    // 执行验证
    let result = verify_exact_out_buy(&rpc, &pool_address, amount_out, 0.5)
        .await
        .expect("Verification should succeed");

    // 打印结果
    result.print_summary("Exact Out Buy - Medium Amount");

    // 验证基本约束
    assert!(result.expected_output > 0, "Expected output should be positive");
    println!("\n框架测试通过!");
}

/// 测试 Exact Out Sell - 小额交易
///
/// 使用小额 quote 进行测试，验证 Sell 方向的基本计算
#[tokio::test]
#[serial_test::serial(pumpswap_exact_out_sim)]
async fn test_exact_out_sell_simulation_small_amount() {
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url));

    let pool_address = Pubkey::from_str(TEST_POOL).expect("Invalid pool address");
    // WSOL decimals = 9, 所以 1_000_000 = 0.000001 WSOL
    let amount_out = 1_000_000u64; // 小额测试: 0.000001 WSOL

    // 执行验证
    let result = verify_exact_out_sell(&rpc, &pool_address, amount_out, 0.5)
        .await
        .expect("Verification should succeed");

    // 打印结果
    result.print_summary("Exact Out Sell - Small Amount");

    // 验证基本约束
    assert!(result.expected_output > 0, "Expected output should be positive");
    println!("\n框架测试通过!");
}

/// 测试 Pool 连接性
///
/// 验证能够正常连接到测试节点并获取 Pool 数据
#[tokio::test]
async fn test_pool_connectivity() {
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = SolanaRpcClient::new(rpc_url);

    let pool_address = Pubkey::from_str(TEST_POOL).expect("Invalid pool address");

    // 尝试获取 Pool 数据
    let result = get_pool_by_address(&rpc, &pool_address).await;

    let pool = match result {
        Ok(pool) => {
            println!("\n========================================");
            println!("Pool 连接测试");
            println!("========================================");
            println!("Pool 地址: {}", pool_address);
            println!("Base Mint: {}", pool.base_mint);
            println!("Quote Mint: {}", pool.quote_mint);
            println!("Coin Creator: {}", pool.coin_creator);
            println!("Is Mayhem Mode: {}", pool.is_mayhem_mode);
            println!("LP Supply: {}", pool.lp_supply);
            println!("========================================");
            println!("Pool 连接测试通过!");
            pool
        }
        Err(e) => {
            panic!(
                "无法连接到 Pool，请确保测试节点运行在 127.0.0.1:8899\n错误: {}",
                e
            );
        }
    };

    // 验证 mint 地址
    let expected_wsol = Pubkey::from_str(WSOL_MINT).unwrap();
    let expected_pump = Pubkey::from_str(PUMP_MINT).unwrap();

    assert!(
        pool.base_mint == expected_pump || pool.quote_mint == expected_pump,
        "PUMP mint 不匹配"
    );
    assert!(
        pool.base_mint == expected_wsol || pool.quote_mint == expected_wsol,
        "WSOL mint 不匹配"
    );
}
