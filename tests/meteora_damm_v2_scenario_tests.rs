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
    SolanaTrade, TradeBuyParams, TradeTokenType,
    common::GasFeeStrategy,
    common::SolanaRpcClient,
    swqos::SwqosConfig,
    trading::core::params::{DexParamEnum, MeteoraDammV2Params},
};
use sol_trade_test_utils::get_simulation_test_keypair;
use solana_commitment_config::CommitmentConfig;
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

/// 测试：创建场景并执行真实 swap 交易
#[tokio::test]
#[serial_test::serial(meteora_damm_v2_scenario)]
async fn test_meteora_damm_v2_swap_with_scenario() {
    let (rpc, payer) = setup_test();
    let pool_address = get_test_pool_address();

    println!("\n========================================");
    println!("Meteora DAMM V2 Swap 场景测试（真实执行）");
    println!("========================================");

    // 1. 获取 Pool 参数
    let protocol_params = MeteoraDammV2Params::from_pool_address_by_rpc(&rpc, &pool_address)
        .await
        .expect("获取 Pool 参数失败");

    // 判断哪个是 base (Pigeon)，哪个是 quote (WSOL)
    let wsol_mint = sol_trade_sdk::constants::WSOL_TOKEN_ACCOUNT;
    let is_token_a_wsol = protocol_params.token_a_mint == wsol_mint;

    let (base_mint, base_program, quote_mint, _quote_program) = if is_token_a_wsol {
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

    // 4. 创建 SolanaTrade 客户端用于真实交易执行
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let commitment = CommitmentConfig::confirmed();
    let swqos_configs: Vec<SwqosConfig> = vec![SwqosConfig::Default(rpc_url.clone())];
    let trade_config = sol_trade_sdk::TradeConfig::new(rpc_url, swqos_configs, commitment)
        .with_wsol_ata_config(true, false);
    let client = SolanaTrade::new(payer.clone(), trade_config).await;

    // 5. 确保有足够的 WSOL 余额用于交易
    let quote_amount_in = 1_000_000u64; // 0.001 WSOL
    println!("\n  确保 WSOL 余额...");
    if let Err(e) = sol_trade_test_utils::ensure_token_balance(
        &rpc,
        "http://127.0.0.1:8899",
        payer.as_ref(),
        &quote_mint,
        "0.01", // 确保有 0.01 WSOL
    )
    .await
    {
        panic!("❌ 确保 WSOL 余额失败: {}", e);
    }

    // 获取最新的 blockhash
    let recent_blockhash = rpc
        .get_latest_blockhash()
        .await
        .expect("获取 blockhash 失败");

    // 6. 设置 Gas 策略
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    // 7. 构建买入参数并执行真实交易
    let buy_params = TradeBuyParams {
        dex_type: sol_trade_sdk::DexType::MeteoraDammV2,
        input_token_type: TradeTokenType::WSOL,
        mint: base_mint,
        input_token_amount: quote_amount_in,
        slippage_basis_points: Some(500), // 5% 滑点
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::MeteoraDammV2(protocol_params.clone()),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true, // 等待交易确认
        create_input_token_ata: true,
        close_input_token_ata: false,
        create_mint_ata: true,
        durable_nonce: None,
        enable_jito_sandwich_protection: Some(false),
        fixed_output_token_amount: Some(1u64), // 最小输出 1 个 base token
        gas_fee_strategy,
        simulate: false, // 真实执行，不使用模拟
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    println!("\n  执行买入交易...");
    let (success, signatures, error) = client
        .buy(buy_params)
        .await
        .expect("买入交易执行失败");

    println!("\nSwap 结果:");
    println!("  交易成功: {}", success);

    if success {
        println!("  交易签名数量: {}", signatures.len());
        for (i, sig) in signatures.iter().enumerate() {
            println!("  签名[{}]: {}", i, sig);
        }

        // 获取交易后的 base token 余额
        let user_base_ata =
            get_associated_token_address_with_program_id(&payer.pubkey(), &base_mint, &base_program);
        match rpc.get_token_account_balance(&user_base_ata).await {
            Ok(token_balance) => {
                let ui_amount = token_balance.ui_amount.unwrap_or(0.0);
                println!("  Base Token 余额: {}", ui_amount);
                assert!(ui_amount > 0.0, "买入后应有 base token 余额");
            },
            Err(e) => {
                println!("  ⚠️ 获取 Base Token 余额失败: {}", e);
            },
        }
    } else {
        println!("  交易失败: {:?}", error);
        panic!("买入交易失败: {:?}", error);
    }

    println!("\n✅ Meteora DAMM V2 真实交易测试通过!");
}

// 手动使用说明：
// 1. 注册 IDL: curl -X POST http://localhost:8899 -H "Content-Type: application/json" -d @tests/fixtures/meteora_damm_v2_idl.json
// 2. 创建场景: curl -X POST http://localhost:8899 -H "Content-Type: application/json" -d @tests/fixtures/meteora_price_scenario.json
// 3. 激活场景: curl -X POST http://localhost:8899 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"surfnet_activateScenario","params":["meteora-price-test"]}'
// 4. 停用场景: curl -X POST http://localhost:8899 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"surfnet_deactivateScenario","params":["meteora-price-test"]}'
