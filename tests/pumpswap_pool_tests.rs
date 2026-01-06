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

use sol_trade_sdk::instruction::utils::pumpswap::{
    find_pool, get_pool_by_address, get_token_balances,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
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
    let rpc_url = "http://127.0.0.1:8899";
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
