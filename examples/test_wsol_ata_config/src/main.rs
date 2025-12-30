use sol_trade_sdk::common::TradeConfig;
use sol_trade_sdk::TradeTokenType;
use sol_trade_sdk::{
    common::AnyResult,
    swqos::SwqosConfig,
    trading::{
        core::params::{DexParamEnum, PumpSwapParams},
        factory::DexType,
    },
    SolanaTrade,
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::sync::Arc;

/// 测试不同的 WSOL ATA 配置方案
///
/// 测试场景：
/// 1. 连续两次买入，测试不同的 close_input_token_ata 配置
/// 2. 验证哪种配置最可靠
///
/// 预期结果：
/// - 方案 A: create_input_token_ata=true, close_input_token_ata=true
///   - 第一次买入：成功
///   - 第二次买入：可能失败（WSOL ATA 状态问题）
///
/// - 方案 B: create_input_token_ata=true, close_input_token_ata=false
///   - 第一次买入：成功
///   - 第二次买入：成功（WSOL ATA 复用）
///
/// - 方案 C: create_input_token_ata=false, close_input_token_ata=false
///   - 第一次买入：失败（需要预先创建 WSOL ATA）
///   - 第二次买入：失败
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("🧪 WSOL ATA 配置测试程序");
    println!("================================\n");

    // 配置
    let payer = Keypair::new(); // 使用新钱包，避免安全问题
    let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
    let commitment = CommitmentConfig::confirmed();
    let swqos_configs: Vec<SwqosConfig> = vec![SwqosConfig::Default(rpc_url.clone())];
    let trade_config =
        TradeConfig::new(rpc_url, swqos_configs, commitment).with_wsol_ata_config(false, false); // 禁用启动时创建 WSOL ATA
    let client = SolanaTrade::new(Arc::new(payer), trade_config).await;

    // 使用一个真实的 PumpSwap 池进行测试
    // 注意：这是一个示例池地址，实际使用时需要替换为真实的池地址
    let pool = Pubkey::from_str("7qbRF6YsyGuLUVs6Y1q64bdVrfe4WcLzN1pVN3dRNwDq")?;

    // PumpSwap 池参数（示例值，实际使用时需要从 RPC 获取）
    let params = PumpSwapParams::new(
        pool,
        Pubkey::from_str("So11111111111111111111111111111111111111112")?, // WSOL
        Pubkey::from_str("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R")?, // USDC
        Pubkey::default(),
        Pubkey::default(),
        1_000_000_000,
        1_000_000_000,
        Pubkey::default(),
        Pubkey::default(),
        sol_trade_sdk::constants::TOKEN_PROGRAM,
        sol_trade_sdk::constants::TOKEN_PROGRAM,
        Pubkey::default(),
    );

    // 测试方案 A: create=true, close=true（当前配置）
    println!("📋 测试方案 A: create_input_token_ata=true, close_input_token_ata=true");
    println!("----------------------------------------------------------------");
    test_scenario_a(&client, &params).await?;

    // 测试方案 B: create=true, close=false（推荐方案）
    println!("\n📋 测试方案 B: create_input_token_ata=true, close_input_token_ata=false");
    println!("----------------------------------------------------------------");
    test_scenario_b(&client, &params).await?;

    // 测试方案 C: create=false, close=false
    println!("\n📋 测试方案 C: create_input_token_ata=false, close_input_token_ata=false");
    println!("----------------------------------------------------------------");
    test_scenario_c(&client, &params).await?;

    println!("\n✅ 所有测试完成！");
    Ok(())
}

/// 方案 A: create=true, close=true
///
/// 问题分析：
/// 1. 第一次买入：
///    - 创建 WSOL ATA
///    - 转账 SOL 到 WSOL ATA
///    - Sync Native
///    - 执行 Swap
///    - 关闭 WSOL ATA
///
/// 2. 第二次买入：
///    - 创建 WSOL ATA（幂等，如果已存在则跳过）
///    - 转账 SOL 到 WSOL ATA
///    - Sync Native
///    - 执行 Swap
///    - 关闭 WSOL ATA
///
/// 潜在问题：
/// - 如果第一次交易的 close_wsol 失败（账户有余额），WSOL ATA 仍然存在
/// - 第二次交易时，账户状态不一致
/// - 如果 Swap 失败，WSOL 被创建但未消耗，下次交易会失败
async fn test_scenario_a(client: &SolanaTrade, params: &PumpSwapParams) -> AnyResult<()> {
    println!("第一次买入...");
    let result = execute_buy(
        client, params, true, // create_input_token_ata
        true, // close_input_token_ata
        true, // simulate
    )
    .await;
    println!("结果: {:?}", result);

    println!("\n第二次买入...");
    let result = execute_buy(
        client, params, true, // create_input_token_ata
        true, // close_input_token_ata
        true, // simulate
    )
    .await;
    println!("结果: {:?}", result);

    println!("\n⚠️  方案 A 分析:");
    println!("  - 优点: 自动管理 WSOL ATA，释放租金");
    println!("  - 缺点: 连续交易时可能出现账户状态不一致问题");
    println!("  - 风险: 如果 Swap 失败，WSOL ATA 可能残留余额，导致下次交易失败");
    println!("  - 建议: 不推荐用于高频交易或连续交易场景");

    Ok(())
}

/// 方案 B: create=true, close=false（推荐方案）
///
/// 优点：
/// 1. 第一次买入：
///    - 创建 WSOL ATA
///    - 转账 SOL 到 WSOL ATA
///    - Sync Native
///    - 执行 Swap
///    - 不关闭 WSOL ATA
///
/// 2. 第二次买入：
///    - 创建 WSOL ATA（幂等，已存在则跳过）
///    - 转账 SOL 到 WSOL ATA
///    - Sync Native
///    - 执行 Swap
///    - 不关闭 WSOL ATA
///
/// 优点：
/// - WSOL ATA 可以复用
/// - 避免账户状态问题
/// - 即使 Swap 失败，下次交易也不会受影响
/// - 性能更好（不需要重复创建/关闭账户）
async fn test_scenario_b(client: &SolanaTrade, params: &PumpSwapParams) -> AnyResult<()> {
    println!("第一次买入...");
    let result = execute_buy(
        client, params, true,  // create_input_token_ata
        false, // close_input_token_ata
        true,  // simulate
    )
    .await;
    println!("结果: {:?}", result);

    println!("\n第二次买入...");
    let result = execute_buy(
        client, params, true,  // create_input_token_ata
        false, // close_input_token_ata
        true,  // simulate
    )
    .await;
    println!("结果: {:?}", result);

    println!("\n✅ 方案 B 分析:");
    println!("  - 优点: WSOL ATA 可以复用，避免账户状态问题");
    println!("  - 优点: 即使 Swap 失败，下次交易也不会受影响");
    println!("  - 优点: 性能更好（不需要重复创建/关闭账户）");
    println!("  - 缺点: 需要支付 WSOL ATA 的租金（约 0.002 SOL）");
    println!("  - 建议: 推荐用于大多数场景，特别是高频交易");

    Ok(())
}

/// 方案 C: create=false, close=false
///
/// 问题：
/// - 第一次买入失败，因为 WSOL ATA 不存在
/// - 第二次买入也失败
///
/// 使用场景：
/// - 适用于预先创建 WSOL ATA 的情况
/// - 适用于使用 Seed 优化的情况
async fn test_scenario_c(client: &SolanaTrade, params: &PumpSwapParams) -> AnyResult<()> {
    println!("第一次买入...");
    let result = execute_buy(
        client, params, false, // create_input_token_ata
        false, // close_input_token_ata
        true,  // simulate
    )
    .await;
    println!("结果: {:?}", result);

    println!("\n第二次买入...");
    let result = execute_buy(
        client, params, false, // create_input_token_ata
        false, // close_input_token_ata
        true,  // simulate
    )
    .await;
    println!("结果: {:?}", result);

    println!("\n❌ 方案 C 分析:");
    println!("  - 优点: 无");
    println!("  - 缺点: 需要预先创建 WSOL ATA");
    println!("  - 缺点: 不适用于大多数场景");
    println!("  - 建议: 不推荐，除非有特殊需求");

    Ok(())
}

/// 执行买入交易
async fn execute_buy(
    client: &SolanaTrade,
    params: &PumpSwapParams,
    create_input_token_ata: bool,
    close_input_token_ata: bool,
    simulate: bool,
) -> AnyResult<()> {
    let gas_fee_strategy = sol_trade_sdk::common::GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150000, 150000, 500000, 500000, 0.001, 0.001);

    let recent_blockhash = client.rpc.get_latest_blockhash().await?;

    let buy_params = sol_trade_sdk::TradeBuyParams {
        dex_type: DexType::PumpSwap,
        input_token_type: TradeTokenType::WSOL,
        mint: Pubkey::from_str("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R")?, // 示例 mint
        input_token_amount: 100_000,                                             // 0.0001 SOL
        slippage_basis_points: Some(100),
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::PumpSwap(params.clone()),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata,
        close_input_token_ata,
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy.clone(),
        simulate,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let (success, signatures, error) = client.buy(buy_params).await?;

    if success {
        println!("  ✅ 交易成功: {:?}", signatures);
    } else {
        println!("  ❌ 交易失败: {:?}", error);
    }

    Ok(())
}

use std::str::FromStr;
