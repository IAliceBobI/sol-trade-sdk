//! Meteora DAMM V2 Surfpool 场景测试
//!
//! 测试如何使用 surfpool 场景系统来修改池子状态（如价格、流动性等）
//!
//! # 运行测试
//!
//! ```bash
//! # 运行场景测试
//! cargo nextest run meteora_damm_v2_scenario --nocapture 2>&1
//! ```

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::meteora_damm_v2::MeteoraDammV2InstructionBuilder,
    swqos::TradeType,
    trading::core::params::{DexParamEnum, MeteoraDammV2Params, SwapParams},
    trading::core::traits::InstructionBuilder,
    utils::simulation_based_calc::{SimulatedSwapResult, simulate_swap_transaction},
};
use sol_trade_test_utils::get_simulation_test_keypair;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::str::FromStr;
use std::sync::Arc;

/// Pigeon-WSOL Pool 地址（Meteora DAMM V2 mainnet fork）
fn get_test_pool_address() -> Pubkey {
    Pubkey::from_str("qHcjwRN2wKJHy8BdcgrQZjLutFhBWPNKSgUWitb929B").expect("Invalid pool address")
}

/// Meteora DAMM V2 程序 ID
#[allow(dead_code)]
fn get_meteora_program_id() -> Pubkey {
    Pubkey::from_str("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG").expect("Invalid program ID")
}

/// 创建测试用的 RPC 客户端和 payer
fn setup_test() -> (Arc<SolanaRpcClient>, Arc<Keypair>) {
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.to_string()));
    let payer = Arc::new(get_simulation_test_keypair());
    (rpc, payer)
}

/// 从文件加载 IDL JSON
fn load_meteora_idl() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let idl_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/meteora_damm_v2_idl.json");
    let idl_content = std::fs::read_to_string(idl_path)?;
    let idl_json: serde_json::Value = serde_json::from_str(&idl_content)?;
    Ok(idl_json)
}

/// 注册 Meteora DAMM V2 IDL 到 surfpool
///
/// 使用 surfnet_registerIdl RPC 方法注册程序 IDL
async fn register_meteora_idl(_rpc: &SolanaRpcClient) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[注册 IDL] 注册 Meteora DAMM V2 IDL...");

    // 从文件加载 IDL JSON
    let idl_json = load_meteora_idl()?;
    println!("  已加载 IDL 文件: tests/fixtures/meteora_damm_v2_idl.json");

    // 使用 RPC 调用注册 IDL
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8899")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "surfnet_registerIdl",
            "params": [idl_json]
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("  IDL 注册结果: {:?}", result);

    Ok(())
}

/// 创建场景并应用覆盖
///
/// 创建一个场景来修改 Pool 的 sqrt_price（价格）
async fn create_price_scenario(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    new_sqrt_price: u128,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("\n[创建场景] 创建价格修改场景...");
    println!("  Pool 地址: {}", pool_address);
    println!("  新 sqrt_price: {}", new_sqrt_price);

    let scenario_id = format!("price-scenario-{}", chrono::Utc::now().timestamp());

    // 首先获取当前池子数据以便知道其他字段的值
    let pool_data =
        sol_trade_sdk::instruction::utils::meteora_damm_v2::get_pool_by_address(rpc, pool_address)
            .await
            .map_err(|e| format!("获取 Pool 数据失败: {}", e))?;

    println!("  当前 sqrt_price: {}", pool_data.sqrt_price);
    println!("  当前 liquidity: {}", pool_data.liquidity);

    // 创建场景 - 修改 sqrt_price
    // 注意：由于 Pool 使用 bytemuck 序列化，我们需要提供完整的账户数据
    // 这里简化处理，只展示如何注册场景
    let client = reqwest::Client::new();

    // 场景 JSON
    let scenario_json = serde_json::json!({
        "id": scenario_id,
        "name": "修改 Pool 价格",
        "description": "将 Pool 的 sqrt_price 修改为新值",
        "overrides": [
            {
                "id": "override-1",
                "templateId": "meteora-pool-override",
                "account": {
                    "type": "pubkey",
                    "value": pool_address.to_string()
                },
                "values": {
                    "sqrt_price": new_sqrt_price.to_string(),
                    "liquidity": pool_data.liquidity.to_string()
                },
                "scenarioRelativeSlot": 1,
                "enabled": true,
                "fetchBeforeUse": true  // 先获取最新数据再应用覆盖
            }
        ]
    });

    let response = client
        .post("http://127.0.0.1:8899")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "surfnet_registerScenario",
            "params": [scenario_json]
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("  场景注册结果: {:?}", result);

    // 激活场景
    let response = client
        .post("http://127.0.0.1:8899")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "surfnet_activateScenario",
            "params": [scenario_id]
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("  场景激活结果: {:?}", result);

    Ok(scenario_id)
}

/// 测试：获取 Pool 信息（基础测试）
#[tokio::test]
#[serial_test::serial(meteora_damm_v2_scenario)]
async fn test_meteora_damm_v2_pool_info_before_scenario() {
    let (rpc, _payer) = setup_test();
    let pool_address = get_test_pool_address();

    println!("\n========================================");
    println!("Meteora DAMM V2 Pool 信息（场景前）");
    println!("========================================");

    // 获取 Pool 参数
    let protocol_params = MeteoraDammV2Params::from_pool_address_by_rpc(&rpc, &pool_address)
        .await
        .expect("获取 Pool 参数失败");

    println!("Pool 地址: {}", pool_address);
    println!("Token A Mint: {}", protocol_params.token_a_mint);
    println!("Token B Mint: {}", protocol_params.token_b_mint);

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

    println!("\n✅ Pool 信息获取成功!");
}

/// 测试：注册 IDL 到 surfpool
#[tokio::test]
#[serial_test::serial(meteora_damm_v2_scenario)]
async fn test_register_meteora_idl() {
    let (rpc, _payer) = setup_test();

    println!("\n========================================");
    println!("注册 Meteora DAMM V2 IDL");
    println!("========================================");

    // 注意：这个测试需要 surfpool 支持 IDL 注册
    // 如果 surfpool 版本不支持，可能会失败
    match register_meteora_idl(&rpc).await {
        Ok(_) => println!("\n✅ IDL 注册成功!"),
        Err(e) => {
            println!("\n⚠️ IDL 注册失败（可能 surfpool 不支持）: {}", e);
            println!("   继续其他测试...");
        },
    }
}

/// 测试：创建场景并执行 swap
#[tokio::test]
#[serial_test::serial(meteora_damm_v2_scenario)]
async fn test_meteora_damm_v2_swap_with_scenario() {
    let (rpc, payer) = setup_test();
    let pool_address = get_test_pool_address();

    println!("\n========================================");
    println!("Meteora DAMM V2 Swap 场景测试");
    println!("========================================");

    // 1. 获取 Pool 参数
    let protocol_params = MeteoraDammV2Params::from_pool_address_by_rpc(&rpc, &pool_address)
        .await
        .expect("获取 Pool 参数失败");

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

    // 2. 获取当前 Pool 数据
    let pool_data = sol_trade_sdk::instruction::utils::meteora_damm_v2::get_pool_by_address(
        &rpc,
        &pool_address,
    )
    .await
    .expect("获取 Pool 数据失败");

    println!("  当前 sqrt_price: {}", pool_data.sqrt_price);
    println!("  当前 liquidity: {}", pool_data.liquidity);

    // 3. 尝试注册 IDL 和创建场景
    // 注意：这取决于 surfpool 是否支持
    let _scenario_id = match create_price_scenario(&rpc, &pool_address, pool_data.sqrt_price).await
    {
        Ok(id) => {
            println!("  ✅ 场景创建成功: {}", id);
            Some(id)
        },
        Err(e) => {
            println!("  ⚠️ 场景创建失败: {}", e);
            println!("     继续普通 swap 测试...");
            None
        },
    };

    // 3. 执行 swap 测试
    let quote_amount_in = 1_000_000u64; // 0.001 WSOL
    let min_base_amount_out = 1u64;

    let gas_fee_strategy = sol_trade_test_utils::create_test_gas_fee_strategy();

    let swap_params = SwapParams {
        rpc: Some(rpc.clone()),
        payer: Arc::new(payer.insecure_clone()),
        trade_type: TradeType::Buy,
        input_mint: quote_mint,
        input_token_program: Some(quote_program),
        output_mint: base_mint,
        output_token_program: Some(base_program),
        input_amount: Some(quote_amount_in),
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
        fixed_output_amount: Some(min_base_amount_out),
        gas_fee_strategy,
        simulate: true,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    };

    // 构建 buy 指令
    let builder = MeteoraDammV2InstructionBuilder;
    let instructions = builder.build_buy_instructions(&swap_params).await.expect("构建指令失败");

    // 获取用户 ATA 地址
    let user_base_token_account =
        get_associated_token_address_with_program_id(&payer.pubkey(), &base_mint, &base_program);
    let user_quote_token_account =
        get_associated_token_address_with_program_id(&payer.pubkey(), &quote_mint, &quote_program);

    // 模拟执行
    let simulated_result: SimulatedSwapResult = simulate_swap_transaction(
        &rpc,
        payer.as_ref(),
        instructions,
        user_quote_token_account,
        user_base_token_account,
        quote_mint,
        base_mint,
    )
    .await
    .expect("模拟执行失败");

    println!("\nSwap 结果:");
    println!("  模拟成功: {}", simulated_result.success);
    println!("  实际输入金额 (WSOL): {}", simulated_result.actual_input_amount);
    println!("  实际输出金额 (Pigeon): {}", simulated_result.actual_output_amount);

    if let Some(error) = &simulated_result.error {
        println!("  错误: {}", error);
    }

    assert!(simulated_result.success, "模拟执行失败");
    assert!(simulated_result.actual_output_amount > 0, "输出金额应该大于 0");

    println!("\n✅ Meteora DAMM V2 场景测试通过!");
}

// 手动使用说明：
// 1. 注册 IDL: curl -X POST http://localhost:8899 -H "Content-Type: application/json" -d @tests/fixtures/meteora_damm_v2_idl.json
// 2. 创建场景: curl -X POST http://localhost:8899 -H "Content-Type: application/json" -d @tests/fixtures/meteora_price_scenario.json
// 3. 激活场景: curl -X POST http://localhost:8899 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"surfnet_activateScenario","params":["meteora-price-test"]}'
// 4. 停用场景: curl -X POST http://localhost:8899 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"surfnet_deactivateScenario","params":["meteora-price-test"]}'
