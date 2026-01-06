//! PumpSwap Pool 查找集成测试
//!
//! 测试所有 pool 查找方法：
//! - find_pool(rpc, mint) - 通过 mint 查找 pool 地址
//! - get_pool_by_address(rpc, pool_address) - 通过地址获取 pool 数据（带缓存）
//! - get_pool_by_mint(rpc, mint) - 通过 mint 获取 pool（带缓存，返回最优池）
//! - get_pool_by_address_force(rpc, pool_address) - 强制刷新缓存后获取
//! - get_pool_by_mint_force(rpc, mint) - 强制刷新缓存后通过 mint 获取
//!
//! 运行测试:
//!     cargo test --test pumpswap_pool_tests -- --nocapture
//!
//! 注意：使用公共 Solana RPC 端点

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use sol_trade_sdk::instruction::utils::pumpswap::{
    find_pool, get_pool_by_address, get_pool_by_address_force, get_pool_by_mint,
    get_pool_by_mint_force, get_token_balances,
};
use std::str::FromStr;

/// 已知的 Pump 代币 mint
const PUMP_MINT: &str = "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn";

/// 已知的 PumpSwap pool 地址
const PUMP_POOL_ADDRESS: &str = "539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR";

/// 测试：通过 mint 查找 pool 地址
#[tokio::test]
async fn test_find_pool_by_mint() {
    println!("=== 测试：通过 mint 查找 pool 地址 ===");

    let mint = Pubkey::from_str(PUMP_MINT).unwrap();
    let rpc_url = "http://127.0.0.1:8899";
    // let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 调用 find_pool
    let result = find_pool(&rpc, &mint).await;

    // 验证结果
    assert!(result.is_ok(), "Failed to find pool: {:?}", result.err());

    let pool_address = result.unwrap();
    println!("✅ 找到的 pool 地址: {}", pool_address);

    // 验证 pool 地址不是零地址
    assert!(!pool_address.eq(&Pubkey::default()), "Pool address should not be zero");
    println!("✅ Pool 地址验证通过（非零地址）");
}

/// 测试：通过地址获取 pool 数据（带缓存）
#[tokio::test]
async fn test_get_pool_by_address() {
    println!("=== 测试：通过地址获取 pool 数据（带缓存） ===");

    let pool_address = Pubkey::from_str(PUMP_POOL_ADDRESS).unwrap();
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 第一次调用（会写入缓存）
    println!("第一次调用（写入缓存）...");
    let result1 = get_pool_by_address(&rpc, &pool_address).await;
    assert!(result1.is_ok(), "Failed to get pool by address: {:?}", result1.err());

    let pool_state = result1.unwrap();
    println!("✅ Pool State 获取成功!");
    println!("  Pool Bump: {}", pool_state.pool_bump);
    println!("  Index: {}", pool_state.index);
    println!("  Creator: {}", pool_state.creator);
    println!("  Base Mint: {}", pool_state.base_mint);
    println!("  Quote Mint: {}", pool_state.quote_mint);
    println!("  LP Mint: {}", pool_state.lp_mint);
    println!("  Pool Base Token Account: {}", pool_state.pool_base_token_account);
    println!("  Pool Quote Token Account: {}", pool_state.pool_quote_token_account);
    println!("  LP Supply: {}", pool_state.lp_supply);
    println!("  Coin Creator: {}", pool_state.coin_creator);
    println!("  Is Mayhem Mode: {}", pool_state.is_mayhem_mode);

    // 获取 token 余额
    let (base_balance, quote_balance) = get_token_balances(&pool_state, &rpc).await.unwrap();
    println!("  Base Token Balance: {}", base_balance);
    println!("  Quote Token Balance: {}", quote_balance);

    // 验证基本字段约束
    assert!(!pool_state.base_mint.eq(&Pubkey::default()), "Base mint should not be zero");
    assert!(!pool_state.quote_mint.eq(&Pubkey::default()), "Quote mint should not be zero");
    assert!(pool_state.lp_supply > 0, "LP supply should be positive");
    assert!(base_balance > 0, "Base balance should be positive");
    assert!(quote_balance > 0, "Quote balance should be positive");
    println!("✅ 基本字段验证通过");

    // 第二次调用（应该从缓存读取）
    println!("\n第二次调用（从缓存读取）...");
    let result2 = get_pool_by_address(&rpc, &pool_address).await;
    assert!(result2.is_ok(), "Failed to get pool from cache: {:?}", result2.err());

    let pool_state2 = result2.unwrap();
    assert_eq!(pool_state.base_mint, pool_state2.base_mint, "Cached pool should match");
    assert_eq!(pool_state.quote_mint, pool_state2.quote_mint, "Cached pool should match");
    assert_eq!(pool_state.lp_supply, pool_state2.lp_supply, "Cached pool should match");
    println!("✅ 缓存验证通过（数据一致）");
}

/// 测试：通过 mint 获取 pool（带缓存，返回最优池）
#[tokio::test]
async fn test_get_pool_by_mint() {
    println!("=== 测试：通过 mint 获取 pool（带缓存，返回最优池） ===");

    let mint = Pubkey::from_str(PUMP_MINT).unwrap();
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 第一次调用（会写入缓存）
    println!("第一次调用（写入缓存）...");
    let result1 = get_pool_by_mint(&rpc, &mint).await;
    assert!(result1.is_ok(), "Failed to get pool by mint: {:?}", result1.err());

    let (pool_address, pool_state) = result1.unwrap();
    println!("✅ 通过 Mint 找到的 pool 地址: {}", pool_address);
    println!("  Base Mint: {}", pool_state.base_mint);
    println!("  Quote Mint: {}", pool_state.quote_mint);
    println!("  LP Supply: {}", pool_state.lp_supply);

    // 验证 mint 匹配
    assert!(
        pool_state.base_mint.eq(&mint) || pool_state.quote_mint.eq(&mint),
        "Pool should contain the requested mint"
    );
    println!("✅ Mint 匹配验证通过");

    // 第二次调用（应该从缓存读取）
    println!("\n第二次调用（从缓存读取）...");
    let result2 = get_pool_by_mint(&rpc, &mint).await;
    assert!(result2.is_ok(), "Failed to get pool from cache: {:?}", result2.err());

    let (pool_address2, pool_state2) = result2.unwrap();
    assert_eq!(pool_address, pool_address2, "Cached pool address should match");
    assert_eq!(pool_state.base_mint, pool_state2.base_mint, "Cached pool should match");
    assert_eq!(pool_state.lp_supply, pool_state2.lp_supply, "Cached pool should match");
    println!("✅ 缓存验证通过（数据一致）");
}

/// 测试：强制刷新缓存后通过地址获取 pool
#[tokio::test]
async fn test_get_pool_by_address_force() {
    println!("=== 测试：强制刷新缓存后通过地址获取 pool ===");

    let pool_address = Pubkey::from_str(PUMP_POOL_ADDRESS).unwrap();
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 第一次调用（写入缓存）
    println!("第一次调用（写入缓存）...");
    let _ = get_pool_by_address(&rpc, &pool_address).await.unwrap();
    println!("✅ 缓存已写入");

    // 强制刷新缓存
    println!("\n强制刷新缓存...");
    let result = get_pool_by_address_force(&rpc, &pool_address).await;
    assert!(result.is_ok(), "Failed to get pool with force refresh: {:?}", result.err());

    let pool_state = result.unwrap();
    println!("✅ 强制刷新后获取的 pool: {}", pool_address);
    println!("  Base Mint: {}", pool_state.base_mint);
    println!("  Quote Mint: {}", pool_state.quote_mint);
    println!("  LP Supply: {}", pool_state.lp_supply);
}

/// 测试：强制刷新缓存后通过 mint 获取 pool
#[tokio::test]
async fn test_get_pool_by_mint_force() {
    println!("=== 测试：强制刷新缓存后通过 mint 获取 pool ===");

    let mint = Pubkey::from_str(PUMP_MINT).unwrap();
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 第一次调用（写入缓存）
    println!("第一次调用（写入缓存）...");
    let _ = get_pool_by_mint(&rpc, &mint).await.unwrap();
    println!("✅ 缓存已写入");

    // 强制刷新缓存
    println!("\n强制刷新缓存...");
    let result = get_pool_by_mint_force(&rpc, &mint).await;
    assert!(result.is_ok(), "Failed to get pool with force refresh: {:?}", result.err());

    let (pool_address, pool_state) = result.unwrap();
    println!("✅ 强制刷新后通过 Mint 找到的 pool: {}", pool_address);
    println!("  Base Mint: {}", pool_state.base_mint);
    println!("  Quote Mint: {}", pool_state.quote_mint);
    println!("  LP Supply: {}", pool_state.lp_supply);
}

/// 测试：验证 pool 的程序所有者
#[tokio::test]
#[ignore]
async fn test_pool_owner_validation() {
    println!("=== 测试：验证 pool 的程序所有者 ===");

    // 测试 get_pool_by_address 会验证程序所有者
    let invalid_address = Pubkey::from_str("11111111111111111111111111111111").unwrap(); // System program

    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    println!("尝试获取无效地址的 pool...");
    let result = get_pool_by_address(&rpc, &invalid_address).await;

    // 应该失败，因为 system program 不是 PumpSwap program
    assert!(result.is_err(), "Expected error for invalid program owner");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("PumpSwap") || err.to_string().contains("AMM"),
        "Error should mention PumpSwap program"
    );
    println!("✅ 程序所有者验证通过（正确拒绝无效地址）");
}

/// 测试：验证 find_pool 和 get_pool_by_mint 的一致性
#[tokio::test]
async fn test_find_pool_consistency() {
    println!("=== 测试：验证 find_pool 和 get_pool_by_mint 的一致性 ===");

    let mint = Pubkey::from_str(PUMP_MINT).unwrap();
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 使用 find_pool 获取地址
    println!("使用 find_pool 获取地址...");
    let pool_address_from_find = find_pool(&rpc, &mint).await.unwrap();
    println!("  find_pool 结果: {}", pool_address_from_find);

    // 使用 get_pool_by_mint 获取地址
    println!("使用 get_pool_by_mint 获取地址...");
    let (pool_address_from_get, _) = get_pool_by_mint(&rpc, &mint).await.unwrap();
    println!("  get_pool_by_mint 结果: {}", pool_address_from_get);

    // 两个方法应该返回相同的 pool 地址
    assert_eq!(
        pool_address_from_find,
        pool_address_from_get,
        "find_pool and get_pool_by_mint should return the same pool address"
    );
    println!("✅ 一致性验证通过（两个方法返回相同的 pool 地址）");
}

/// 测试：获取 token 余额
#[tokio::test]
async fn test_get_token_balances() {
    println!("=== 测试：获取 token 余额 ===");

    let pool_address = Pubkey::from_str(PUMP_POOL_ADDRESS).unwrap();
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 获取 pool 数据
    println!("获取 pool 数据...");
    let pool_state = get_pool_by_address(&rpc, &pool_address).await.unwrap();

    // 获取 token 余额
    println!("获取 token 余额...");
    let (base_balance, quote_balance) = get_token_balances(&pool_state, &rpc).await.unwrap();

    println!("✅ Token 余额获取成功:");
    println!("  Base Token Balance: {}", base_balance);
    println!("  Quote Token Balance: {}", quote_balance);

    // 验证余额大于 0
    assert!(base_balance > 0, "Base balance should be positive");
    assert!(quote_balance > 0, "Quote balance should be positive");
    println!("✅ 余额验证通过（大于 0）");
}
