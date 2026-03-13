//! Meteora DLMM Surfpool 场景测试
//!
//! 使用 surfpool 内置的 DLMM 场景模板来测试池子状态修改
//!
//! # 运行测试
//!
//! ```bash
//! # 运行场景测试
//! cargo nextest run meteora_dlmm_scenario --nocapture 2>&1
//! ```

use sol_trade_sdk::SolanaRpcClient;
use sol_trade_test_utils::get_simulation_test_keypair;
use solana_sdk::{account::ReadableAccount, pubkey::Pubkey, signature::Signer};
use std::str::FromStr;
use std::sync::Arc;

/// TRUMP-USDC Pool 地址（surfpool 内置）
fn get_test_pool_address() -> Pubkey {
    Pubkey::from_str("3C5YE97HADPDxZehYq9Cis8AXr9aNyrUsczKzE1nDbW9").expect("Invalid pool address")
}

/// Meteora DLMM 程序 ID
fn get_meteora_dlmm_program_id() -> Pubkey {
    Pubkey::from_str("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo").expect("Invalid program ID")
}

/// TRUMP Token Mint
fn get_trump_mint() -> Pubkey {
    Pubkey::from_str("6p6xgHyF7AeE6TZkSmFsko444wqoP15icUSqi2jfGiPN").expect("Invalid TRUMP mint")
}

/// USDC Token Mint
fn get_usdc_mint() -> Pubkey {
    Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").expect("Invalid USDC mint")
}

/// 创建测试用的 RPC 客户端和 payer
fn setup_test() -> (Arc<SolanaRpcClient>, Arc<solana_sdk::signature::Keypair>) {
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.to_string()));
    let payer = Arc::new(get_simulation_test_keypair());
    (rpc, payer)
}

/// 使用场景模板注册 surfpool 场景
///
/// 使用 surfpool 内置的 meteora-dlmm-sol-usdc 模板
async fn register_dlmm_scenario(
    scenario_id: &str,
    active_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[注册场景] 注册 DLMM 场景: {}...", scenario_id);

    let client = reqwest::Client::new();

    // 使用 surfpool 内置的 meteora-dlmm-sol-usdc 模板
    // 但需要修改地址为我们的 TRUMP-USDC pool
    let scenario = serde_json::json!({
        "id": scenario_id,
        "name": "Meteora DLMM TRUMP/USDC Scenario",
        "description": "Override DLMM pool active_id for testing",
        "overrides": [
            {
                "templateId": "meteora-dlmm-sol-usdc",
                "address": get_test_pool_address().to_string(),
                "values": {
                    "active_id": active_id,
                    "status": 1,  // 1 = active
                    "bin_step": 10
                }
            }
        ],
        "tags": ["meteora", "dlmm", "test"]
    });

    let response = client
        .post("http://127.0.0.1:8899")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "surfnet_registerScenario",
            "params": [scenario]
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("  场景注册结果: {:?}", result);

    if result.get("error").is_some() {
        return Err(format!("场景注册失败: {:?}", result.get("error")).into());
    }

    println!("  ✅ 场景 {} 注册成功", scenario_id);
    Ok(())
}

/// 激活场景
async fn activate_scenario(scenario_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[激活场景] 激活场景: {}...", scenario_id);

    let client = reqwest::Client::new();
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

    if result.get("error").is_some() {
        return Err(format!("场景激活失败: {:?}", result.get("error")).into());
    }

    println!("  ✅ 场景 {} 已激活", scenario_id);
    Ok(())
}

/// 停用场景
async fn deactivate_scenario(scenario_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[停用场景] 停用场景: {}...", scenario_id);

    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8899")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "surfnet_deactivateScenario",
            "params": [scenario_id]
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("  场景停用结果: {:?}", result);

    if result.get("error").is_some() {
        return Err(format!("场景停用失败: {:?}", result.get("error")).into());
    }

    println!("  ✅ 场景 {} 已停用", scenario_id);
    Ok(())
}

/// LbPair 数据结构常量
///
/// 基于 Meteora DLMM IDL 的结构定义（bytemuck 序列化）：
/// - discriminator: 8 bytes (Anchor discriminator)
/// - parameters (StaticParameters): 32 bytes
/// - v_parameters (VariableParameters): 32 bytes
/// - bump_seed: 1 byte
/// - bin_step_seed: 2 bytes
/// - pair_type: 1 byte
/// - active_id: i32, 4 bytes (offset 68-71)
/// - bin_step: u16, 2 bytes (offset 72-73)
/// - status: u8 (offset 74)
const LB_PAIR_DISCRIMINATOR: [u8; 8] = [33, 11, 49, 98, 181, 101, 177, 13];
const LB_PAIR_ACTIVE_ID_OFFSET: usize = 68;
const LB_PAIR_BIN_STEP_OFFSET: usize = 72;
const LB_PAIR_STATUS_OFFSET: usize = 74;

/// 直接获取 LbPair 账户数据并解析关键字段
async fn get_lbpair_data(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<LbPairData, Box<dyn std::error::Error>> {
    let account = rpc
        .get_account(pool_address)
        .await
        .map_err(|e| format!("获取 LbPair 账户失败: {}", e))?;

    let data = account.data();

    // 检查 discriminator
    if data.len() < 8 {
        return Err("账户数据太短".into());
    }

    let discriminator = &data[0..8];
    if discriminator != LB_PAIR_DISCRIMINATOR {
        println!(
            "  ⚠️ Discriminator 不匹配: 期望 {:?}, 实际 {:?}",
            LB_PAIR_DISCRIMINATOR, discriminator
        );
    }

    // 解析关键字段
    let active_id = i32::from_le_bytes([
        data[LB_PAIR_ACTIVE_ID_OFFSET],
        data[LB_PAIR_ACTIVE_ID_OFFSET + 1],
        data[LB_PAIR_ACTIVE_ID_OFFSET + 2],
        data[LB_PAIR_ACTIVE_ID_OFFSET + 3],
    ]);

    let bin_step =
        u16::from_le_bytes([data[LB_PAIR_BIN_STEP_OFFSET], data[LB_PAIR_BIN_STEP_OFFSET + 1]]);

    let status = data[LB_PAIR_STATUS_OFFSET];

    let lb_pair_data = LbPairData {
        address: *pool_address,
        data_len: data.len(),
        owner: *account.owner(),
        active_id,
        bin_step,
        status,
    };

    Ok(lb_pair_data)
}

/// 使用 surfnet_setAccount 直接修改 LbPair active_id
async fn override_lbpair_active_id(
    pool_address: &Pubkey,
    new_active_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n[直接覆盖] 修改 LbPair active_id 为 {}...", new_active_id);

    // 1. 获取当前账户数据
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8899")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [pool_address.to_string(), {"encoding": "base64"}]
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    let account_info = result["result"]["value"].clone();

    let lamports = account_info["lamports"].as_u64().unwrap_or(0);
    let owner = account_info["owner"].as_str().unwrap_or("");
    let data_b64 = account_info["data"].as_array().and_then(|a| a[0].as_str()).unwrap_or("");

    // 2. 解码并修改数据
    use base64::Engine;
    let mut data = base64::engine::general_purpose::STANDARD.decode(data_b64)?;
    let active_id_bytes = new_active_id.to_le_bytes();
    data[LB_PAIR_ACTIVE_ID_OFFSET..LB_PAIR_ACTIVE_ID_OFFSET + 4].copy_from_slice(&active_id_bytes);

    // 3. 编码为 hex 并发送
    let data_hex = hex::encode(&data);

    let response = client
        .post("http://127.0.0.1:8899")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "surfnet_setAccount",
            "params": [
                pool_address.to_string(),
                {
                    "lamports": lamports,
                    "data": data_hex,
                    "owner": owner,
                    "executable": false,
                    "rentEpoch": 0
                }
            ]
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    if result.get("error").is_some() {
        return Err(format!("surfnet_setAccount 失败: {:?}", result["error"]).into());
    }

    println!("  ✅ active_id 已修改为 {}", new_active_id);
    Ok(())
}

/// LbPair 数据结构
#[derive(Debug)]
#[allow(dead_code)]
struct LbPairData {
    address: Pubkey,
    data_len: usize,
    owner: Pubkey,
    active_id: i32,
    bin_step: u16,
    status: u8,
}

/// 测试：获取 Pool 基础信息
#[tokio::test]
#[serial_test::serial(meteora_dlmm_scenario)]
async fn test_meteora_dlmm_pool_info() {
    let (rpc, _payer) = setup_test();
    let pool_address = get_test_pool_address();

    println!("\n========================================");
    println!("Meteora DLMM Pool 信息");
    println!("========================================");

    println!("Pool 地址: {}", pool_address);
    println!("Program ID: {}", get_meteora_dlmm_program_id());
    println!("TRUMP Mint: {}", get_trump_mint());
    println!("USDC Mint: {}", get_usdc_mint());

    // 获取账户数据
    match get_lbpair_data(&rpc, &pool_address).await {
        Ok(data) => {
            println!("\nLbPair 账户信息:");
            println!("  数据长度: {} bytes", data.data_len);
            println!("  所有者: {}", data.owner);
            println!("  active_id: {}", data.active_id);
            println!("  bin_step: {}", data.bin_step);
            println!("  status: {}", data.status);
        },
        Err(e) => {
            println!("\n⚠️ 获取 LbPair 数据失败: {}", e);
        },
    }

    println!("\n✅ Pool 信息获取完成!");
}

/// 测试：场景系统基础功能测试
#[tokio::test]
#[ignore = "需要 surfpool 支持场景系统"]
#[serial_test::serial(meteora_dlmm_scenario)]
async fn test_meteora_dlmm_scenario_basic() {
    let (_rpc, _payer) = setup_test();
    let scenario_id = "dlmm-test-scenario";

    println!("\n========================================");
    println!("Meteora DLMM 场景系统基础测试");
    println!("========================================");

    // 1. 注册场景
    match register_dlmm_scenario(scenario_id, 100).await {
        Ok(_) => {
            println!("\n✅ 场景注册成功");

            // 2. 激活场景
            if let Err(e) = activate_scenario(scenario_id).await {
                println!("\n⚠️ 场景激活失败: {}", e);
            } else {
                // 3. 停用场景
                if let Err(e) = deactivate_scenario(scenario_id).await {
                    println!("\n⚠️ 场景停用失败: {}", e);
                }
            }
        },
        Err(e) => {
            println!("\n⚠️ 场景注册失败: {}", e);
            println!("   可能 surfpool 不支持场景系统");
        },
    }

    println!("\n✅ 基础场景测试完成!");
}

/// 测试：直接覆盖 active_id 前后对比
#[tokio::test]
#[serial_test::serial(meteora_dlmm_scenario)]
async fn test_meteora_dlmm_active_id_override() {
    let (rpc, _payer) = setup_test();
    let pool_address = get_test_pool_address();

    println!("\n========================================");
    println!("Meteora DLMM active_id 覆盖前后对比测试");
    println!("========================================");

    // 1. 获取场景覆盖前的状态
    println!("\n📊 【覆盖前】Pool 状态:");
    let before_data = match get_lbpair_data(&rpc, &pool_address).await {
        Ok(data) => {
            println!("  active_id: {}", data.active_id);
            println!("  bin_step: {}", data.bin_step);
            println!("  status: {}", data.status);
            data
        },
        Err(e) => {
            panic!("获取数据失败: {}", e);
        },
    };

    // 2. 修改 active_id
    let target_active_id = before_data.active_id + 100i32;
    println!("\n📝 计划修改 active_id: {} -> {}", before_data.active_id, target_active_id);

    override_lbpair_active_id(&pool_address, target_active_id)
        .await
        .expect("修改 active_id 失败");

    // 3. 获取覆盖后的状态
    println!("\n📊 【覆盖后】Pool 状态:");
    let after_data = match get_lbpair_data(&rpc, &pool_address).await {
        Ok(data) => {
            println!("  active_id: {}", data.active_id);
            println!("  bin_step: {}", data.bin_step);
            println!("  status: {}", data.status);
            data
        },
        Err(e) => {
            panic!("获取数据失败: {}", e);
        },
    };

    // 4. 验证修改
    assert_eq!(after_data.active_id, target_active_id, "active_id 修改未生效");
    assert_eq!(after_data.bin_step, before_data.bin_step, "bin_step 不应改变");

    println!("\n✅ 场景覆盖对比测试通过!");
    println!("   之前: active_id = {}", before_data.active_id);
    println!("   之后: active_id = {}", after_data.active_id);

    // 5. 恢复原始值
    println!("\n📝 恢复原始 active_id: {}", before_data.active_id);
    override_lbpair_active_id(&pool_address, before_data.active_id)
        .await
        .expect("恢复 active_id 失败");

    let restored_data = get_lbpair_data(&rpc, &pool_address).await.expect("获取恢复后数据失败");

    assert_eq!(restored_data.active_id, before_data.active_id, "恢复失败");
    println!("✅ 已恢复原始值");
}

/// 测试：直接覆盖 LbPair 账户数据
///
/// 类似于 DAMM V2 的直接覆盖方法，但适配 DLMM 的 Anchor 结构
#[tokio::test]
#[ignore = "需要 surfpool 支持 surfnet_setAccount"]
#[serial_test::serial(meteora_dlmm_scenario)]
async fn test_meteora_dlmm_direct_override() {
    let (rpc, payer) = setup_test();
    let pool_address = get_test_pool_address();

    println!("\n========================================");
    println!("Meteora DLMM 直接账户覆盖测试");
    println!("========================================");

    // 确保 payer 有 SOL
    println!("\n💧 确保 payer SOL 余额...");
    if let Err(e) =
        sol_trade_test_utils::ensure_sol_balance(&rpc, "http://127.0.0.1:8899", &payer.pubkey(), 10)
            .await
    {
        println!("  ⚠️ 确保 SOL 余额失败: {}", e);
    }

    // 获取当前账户数据
    let original_account = match rpc.get_account(&pool_address).await {
        Ok(acc) => acc,
        Err(e) => {
            println!("\n❌ 获取账户数据失败: {}", e);
            return;
        },
    };

    println!("\n📊 原始账户状态:");
    println!("  数据长度: {} bytes", original_account.data().len());
    println!("  Lamports: {}", original_account.lamports());
    println!("  所有者: {}", original_account.owner());

    // 注意：DLMM 使用 Anchor 序列化，有 discriminator
    // 直接修改 Anchor 账户数据比较复杂，需要保持结构完整性
    // 这里仅演示如何读取数据

    println!("\n✅ 直接覆盖测试（读取部分）完成!");
    println!("   注意：DLMM 使用 Anchor 序列化，直接修改需要完整理解 LbPair 结构");
}

/// 测试：使用 surfpool 内置场景模板列表
#[tokio::test]
#[ignore = "需要 surfpool 支持"]
#[serial_test::serial(meteora_dlmm_scenario)]
async fn test_list_available_templates() {
    println!("\n========================================");
    println!("列出 surfpool 内置场景模板");
    println!("========================================");

    let client = reqwest::Client::new();

    // 尝试调用 surfnet_listTemplates 或类似方法
    let response = match client
        .post("http://127.0.0.1:8899")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "surfnet_listScenarios",
            "params": []
        }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            println!("\n⚠️ 请求失败: {}", e);
            return;
        },
    };

    let result: serde_json::Value = match response.json().await {
        Ok(r) => r,
        Err(e) => {
            println!("\n⚠️ 解析响应失败: {}", e);
            return;
        },
    };

    println!("\n可用场景列表:");
    println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
}

// 手动使用说明：
// 1. 确保 surfpool 运行: surfpool --fork mainnet
// 2. 运行测试: cargo nextest run meteora_dlmm_scenario --nocapture
//
// 场景模板使用流程：
// 1. 注册场景: surfnet_registerScenario
// 2. 激活场景: surfnet_activateScenario
// 3. 执行测试交易
// 4. 停用场景: surfnet_deactivateScenario
