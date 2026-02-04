//! Pool 列表功能集成测试
//!
//! 测试 Raydium CPMM Pool 查询和列表功能：
//! - list_pools_by_mint - 列出所有包含指定 mint 的 Pool
//! - get_pool_by_mint - 获取最优 Pool
//! - Pool 字段验证
//!
//! 运行测试:
//!     cargo nextest run --package sol-trade-test-utils list_pools_tests -- --nocapture

use sol_trade_sdk::{
    common::auto_mock_rpc::AutoMockRpcClient,
    instruction::utils::raydium_cpmm::{clear_pool_cache, get_pool_by_mint, list_pools_by_mint},
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// 测试：列出所有包含 WSOL 的 Raydium CPMM Pool
///
/// 测试内容：
/// 1. 使用 list_pools_by_mint 查询所有 WSOL Pool
/// 2. 验证返回的 Pool 列表非空
/// 3. 打印前几个 Pool 的基本信息
#[tokio::test]
#[serial_test::serial]
#[ignore = "需要本地测试节点"]
async fn test_list_pools_by_mint_wsol() {
    println!("\n=== 测试：列出所有包含 WSOL 的 Raydium CPMM Pool ===");

    let wsol_mint = wsol_mint();
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端（使用独立命名空间）
    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("list_pools_wsol".to_string()),
    );

    println!("Token Mint: {}", wsol_mint);

    clear_pool_cache();

    // 使用 list_pools_by_mint 查询所有 WSOL Pool
    println!("\n📋 查询所有包含 WSOL 的 Pool...");
    let pools: Vec<(Pubkey, sol_trade_sdk::instruction::utils::raydium_cpmm_types::PoolState)> =
        list_pools_by_mint(&auto_mock_client, &wsol_mint)
            .await
            .expect("list_pools_by_mint failed");

    println!("✅ 查询到 {} 个 Pool", pools.len());
    assert!(!pools.is_empty(), "WSOL 相关的 CPMM Pool 列表不应为空");

    // 打印前 5 个 Pool 的详细信息
    for (i, (addr, pool)) in pools.iter().take(5).enumerate() {
        println!("\n{}. Pool: {}", i + 1, addr);
        println!("   Token0: {}", pool.token0_mint);
        println!("   Token1: {}", pool.token1_mint);
        println!("   LP Supply: {}", pool.lp_supply);
        println!("   Token0 Vault: {}", pool.token0_vault);
        println!("   Token1 Vault: {}", pool.token1_vault);
    }

    if pools.len() > 5 {
        println!("\n... 还有 {} 个 Pool", pools.len() - 5);
    }

    println!("\n✅ 测试通过");
}

/// 测试：列出所有包含 USDC 的 Raydium CPMM Pool
///
/// 测试内容：
/// 1. 使用 list_pools_by_mint 查询所有 USDC Pool
/// 2. 验证返回的 Pool 列表非空
#[tokio::test]
#[serial_test::serial]
#[ignore = "需要本地测试节点"]
async fn test_list_pools_by_mint_usdc() {
    println!("\n=== 测试：列出所有包含 USDC 的 Raydium CPMM Pool ===");

    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
    let rpc_url = "http://127.0.0.1:8899";

    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("list_pools_usdc".to_string()),
    );

    println!("Token Mint: {}", usdc_mint);

    clear_pool_cache();

    println!("\n📋 查询所有包含 USDC 的 Pool...");
    let pools = list_pools_by_mint(&auto_mock_client, &usdc_mint)
        .await
        .expect("list_pools_by_mint failed");

    println!("✅ 查询到 {} 个 Pool", pools.len());
    assert!(!pools.is_empty(), "USDC 相关的 CPMM Pool 列表不应为空");

    // 打印前 3 个 Pool
    for (i, (addr, pool)) in pools.iter().take(3).enumerate() {
        println!(
            "\n{}. Pool: {} | Token0: {} | Token1: {} | LP Supply: {}",
            i + 1,
            addr,
            pool.token0_mint,
            pool.token1_mint,
            pool.lp_supply
        );
    }

    if pools.len() > 3 {
        println!("\n... 还有 {} 个 Pool", pools.len() - 3);
    }

    println!("\n✅ 测试通过");
}

/// 测试：获取最优 Pool（基于流动性排序）
///
/// 测试内容：
/// 1. 使用 get_pool_by_mint 获取最优 Pool
/// 2. 验证 Pool 字段的正确性
/// 3. 验证 Pool 包含指定的 Token
#[tokio::test]
#[serial_test::serial]
#[ignore = "需要本地测试节点"]
async fn test_get_pool_by_mint() {
    println!("\n=== 测试：获取最优 Pool（基于流动性排序） ===");

    let wsol_mint = wsol_mint();
    let rpc_url = "http://127.0.0.1:8899";

    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("get_pool_wsol".to_string()),
    );

    println!("Token Mint: {}", wsol_mint);

    clear_pool_cache();

    println!("\n🔍 查询最优 Pool...");
    let (pool_addr, pool_state): (
        Pubkey,
        sol_trade_sdk::instruction::utils::raydium_cpmm_types::PoolState,
    ) = get_pool_by_mint(&auto_mock_client, &wsol_mint)
        .await
        .expect("get_pool_by_mint failed");

    println!("✅ 找到最优 Pool: {}", pool_addr);
    println!("\n📊 Pool 详细信息:");
    println!("   Token0 Mint: {}", pool_state.token0_mint);
    println!("   Token1 Mint: {}", pool_state.token1_mint);
    println!("   LP Supply: {}", pool_state.lp_supply);
    println!("   Token0 Vault: {}", pool_state.token0_vault);
    println!("   Token1 Vault: {}", pool_state.token1_vault);
    println!("   LP Mint: {}", pool_state.lp_mint);
    println!("   AMM Config: {}", pool_state.amm_config);
    println!("   Observation Key: {}", pool_state.observation_key);

    // 验证基本字段
    assert!(
        pool_state.token0_mint == wsol_mint || pool_state.token1_mint == wsol_mint,
        "返回的 CPMM Pool 不包含 WSOL"
    );
    assert!(!pool_state.token0_mint.eq(&Pubkey::default()), "Token0 mint should not be zero");
    assert!(!pool_state.token1_mint.eq(&Pubkey::default()), "Token1 mint should not be zero");
    assert!(pool_state.lp_supply > 0, "LP supply should be positive");

    println!("\n✅ 基本字段验证通过");
    println!("\n✅ 测试通过");
}

/// 测试：Pool 列表和分类功能（使用 pool_list 模块）
///
/// 测试内容：
/// 1. 使用 list_and_classify_pools 列出并分类 Pool
/// 2. 验证 Token2022 和 Token Pool 的分类
#[tokio::test]
#[serial_test::serial]
#[ignore = "需要本地测试节点"]
async fn test_pool_classification() {
    println!("\n=== 测试：Pool 列表和分类功能 ===");

    let wsol_mint = wsol_mint();
    let rpc_url = "http://127.0.0.1:8899";

    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("pool_classification".to_string()),
    );

    println!("Token Mint: {}", wsol_mint);

    // 使用 pool_list 模块的分类功能
    use sol_trade_test_utils::pool_list::{list_and_classify_pools, print_pool_classification};

    let classification = list_and_classify_pools(&auto_mock_client, &wsol_mint)
        .await
        .expect("list_and_classify_pools failed");

    // 打印分类结果
    print_pool_classification(&classification, Some(5));

    // 验证分类结果
    println!("\n📊 分类验证:");
    println!(
        "  Token2022 配对: {} 个",
        classification.token2022_pools.len()
    );
    println!("  Token 配对: {} 个", classification.token_pools.len());
    println!(
        "  未知程序配对: {} 个",
        classification.unknown_pools.len()
    );

    let total_pools = classification.token2022_pools.len()
        + classification.token_pools.len()
        + classification.unknown_pools.len();

    println!("  总计: {} 个 Pool", total_pools);
    assert!(total_pools > 0, "应该找到至少一个 Pool");

    println!("\n✅ 测试通过");
}

// ============ Helper Functions ============

/// WSOL Mint 地址
fn wsol_mint() -> Pubkey {
    Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap()
}
