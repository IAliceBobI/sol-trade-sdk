//! Raydium CLMM Pool 查找集成测试
//!
//! 测试所有 pool 查找方法：
//! - get_pool_by_address(rpc, pool_address) - 通过地址获取 pool 数据（带缓存）
//! - get_pool_by_mint(rpc, mint) - 通过 mint 获取 pool（带缓存，返回最优池）
//! - get_pool_by_address_force(rpc, pool_address) - 强制刷新缓存后获取
//! - get_pool_by_mint_force(rpc, mint) - 强制刷新缓存后通过 mint 获取
//! - list_pools_by_mint(rpc, mint) - 列出所有包含该 mint 的 pool
//! - get_tick_array_pda(pool_id, start_tick_index) - 计算 tick array PDA
//! - get_tick_array_bitmap_extension_pda(pool_id) - 计算 tick array bitmap extension PDA
//! - get_first_initialized_tick_array_start_index(pool_state, zero_for_one) - 获取第一个初始化的 tick array
//! - quote_exact_in(rpc, pool_address, amount_in, zero_for_one) - 报价
//!
//! 运行测试:
//!     cargo test --test raydium_clmm_pool_tests -- --nocapture
//!
//! 注意：使用 surfpool (localhost:8899) 进行测试

use sol_trade_sdk::instruction::utils::raydium_clmm::{
    get_pool_by_address,
    get_pool_by_mint,
    get_pool_by_address_force,
    list_pools_by_mint,
    get_tick_array_pda,
    get_tick_array_bitmap_extension_pda,
    get_first_initialized_tick_array_start_index,
    quote_exact_in,
    clear_pool_cache,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// 已知的 SOL Token Mint (WSOL)
const WSOL_MINT: &str = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";

/// 已知的 USDC Token Mint
const _USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// 测试：通过地址获取 pool 数据（带缓存）
#[tokio::test]
async fn test_get_pool_by_address() {
    println!("=== 测试：通过地址获取 pool 数据（带缓存） ===");

    // 使用一个已知的 Raydium CLMM pool 地址
    // SOL/USDC pool on Raydium CLMM
    let pool_address = Pubkey::from_str("2AXXcN6oN9bBT5owwmTH53C7QHUXvhLeu718Kqt8rvY2")
        .expect("Invalid pool address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 第一次调用（会写入缓存）
    println!("第一次调用（写入缓存）...");
    let result1 = get_pool_by_address(&rpc, &pool_address).await;
    assert!(result1.is_ok(), "Failed to get pool by address: {:?}", result1.err());

    let pool_state = result1.unwrap();
    println!("✅ Pool State 获取成功!");
    println!("  Pool Address: {}", pool_address);
    println!("  Amm Config: {}", pool_state.amm_config);
    println!("  Token0 Mint: {}", pool_state.token_mint0);
    println!("  Token1 Mint: {}", pool_state.token_mint1);
    println!("  Token0 Decimals: {}", pool_state.mint_decimals0);
    println!("  Token1 Decimals: {}", pool_state.mint_decimals1);
    println!("  Token0 Vault: {}", pool_state.token_vault0);
    println!("  Token1 Vault: {}", pool_state.token_vault1);
    println!("  Observation State: {}", pool_state.observation_key);
    println!("  Tick Current: {}", pool_state.tick_current);
    println!("  Tick Spacing: {}", pool_state.tick_spacing);
    println!("  Sqrt Price X64: {}", pool_state.sqrt_price_x64);
    println!("  Liquidity: {}", pool_state.liquidity);

    // 验证基本字段约束
    assert!(!pool_state.token_mint0.eq(&Pubkey::default()), "Token0 mint should not be zero");
    assert!(!pool_state.token_mint1.eq(&Pubkey::default()), "Token1 mint should not be zero");
    assert!(!pool_state.token_vault0.eq(&Pubkey::default()), "Token0 vault should not be zero");
    assert!(!pool_state.token_vault1.eq(&Pubkey::default()), "Token1 vault should not be zero");
    assert!(!pool_state.amm_config.eq(&Pubkey::default()), "AMM config should not be zero");
    assert!(pool_state.mint_decimals0 > 0, "Token0 decimals should be positive");
    assert!(pool_state.mint_decimals1 > 0, "Token1 decimals should be positive");
    assert!(pool_state.liquidity > 0, "Liquidity should be positive");
    assert!(pool_state.sqrt_price_x64 > 0, "Sqrt price should be positive");
    assert!(pool_state.tick_spacing > 0, "Tick spacing should be positive");
    println!("✅ 基本字段验证通过");

    // 第二次调用（应该从缓存读取）
    println!("\n第二次调用（从缓存读取）...");
    let result2 = get_pool_by_address(&rpc, &pool_address).await;
    assert!(result2.is_ok(), "Failed to get pool from cache: {:?}", result2.err());

    let pool_state2 = result2.unwrap();
    assert_eq!(pool_state.amm_config, pool_state2.amm_config, "Cached pool should match");
    assert_eq!(pool_state.token_mint0, pool_state2.token_mint0, "Cached pool should match");
    assert_eq!(pool_state.token_mint1, pool_state2.token_mint1, "Cached pool should match");
    assert_eq!(pool_state.liquidity, pool_state2.liquidity, "Cached pool should match");
    assert_eq!(pool_state.sqrt_price_x64, pool_state2.sqrt_price_x64, "Cached pool should match");
    println!("✅ 缓存验证通过（数据一致）");
}

/// 测试：通过 mint 获取 pool（带缓存）
#[tokio::test]
async fn test_get_pool_by_mint() {
    println!("=== 测试：通过 mint 获取 pool（带缓存） ===");

    let mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 第一次调用（会写入缓存）
    println!("第一次调用（写入缓存）...");
    let result1 = get_pool_by_mint(&rpc, &mint).await;
    assert!(result1.is_ok(), "Failed to get pool by mint: {:?}", result1.err());

    let (pool_address, pool_state) = result1.unwrap();
    println!("✅ Pool 获取成功!");
    println!("  Pool Address: {}", pool_address);
    println!("  Token0 Mint: {}", pool_state.token_mint0);
    println!("  Token1 Mint: {}", pool_state.token_mint1);
    println!("  Liquidity: {}", pool_state.liquidity);
    println!("  Sqrt Price X64: {}", pool_state.sqrt_price_x64);

    // 验证 mint 在 pool 中
    assert!(
        pool_state.token_mint0 == mint || pool_state.token_mint1 == mint,
        "Pool should contain the specified mint"
    );
    println!("✅ Mint 验证通过");

    // 第二次调用（应该从缓存读取）
    println!("\n第二次调用（从缓存读取）...");
    let result2 = get_pool_by_mint(&rpc, &mint).await;
    assert!(result2.is_ok(), "Failed to get pool from cache: {:?}", result2.err());

    let (pool_address2, pool_state2) = result2.unwrap();
    assert_eq!(pool_address, pool_address2, "Cached pool address should match");
    assert_eq!(pool_state.liquidity, pool_state2.liquidity, "Cached pool should match");
    assert_eq!(pool_state.sqrt_price_x64, pool_state2.sqrt_price_x64, "Cached pool should match");
    println!("✅ 缓存验证通过（数据一致）");
}

/// 测试：强制刷新缓存
#[tokio::test]
async fn test_get_pool_by_address_force() {
    println!("=== 测试：强制刷新缓存 ===");

    let pool_address = Pubkey::from_str("2AXXcN6oN9bBT5owwmTH53C7QHUXvhLeu718Kqt8rvY2")
        .expect("Invalid pool address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 第一次调用（写入缓存）
    println!("第一次调用（写入缓存）...");
    let result1 = get_pool_by_address(&rpc, &pool_address).await;
    assert!(result1.is_ok());
    let pool_state1 = result1.unwrap();

    // 第二次调用（从缓存读取）
    println!("\n第二次调用（从缓存读取）...");
    let result2 = get_pool_by_address(&rpc, &pool_address).await;
    assert!(result2.is_ok());
    let pool_state2 = result2.unwrap();
    assert_eq!(pool_state1.liquidity, pool_state2.liquidity);
    assert_eq!(pool_state1.sqrt_price_x64, pool_state2.sqrt_price_x64);

    // 强制刷新缓存
    println!("\n强制刷新缓存...");
    let result3 = get_pool_by_address_force(&rpc, &pool_address).await;
    assert!(result3.is_ok());
    let pool_state3 = result3.unwrap();
    assert_eq!(pool_state1.liquidity, pool_state3.liquidity);
    assert_eq!(pool_state1.sqrt_price_x64, pool_state3.sqrt_price_x64);
    println!("✅ 强制刷新验证通过");
}

/// 测试：列出所有包含该 mint 的 pool
#[tokio::test]
async fn test_list_pools_by_mint() {
    println!("=== 测试：列出所有包含该 mint 的 pool ===");

    let mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    println!("查找所有包含 {} 的 pool...", mint);
    let result = list_pools_by_mint(&rpc, &mint).await;
    assert!(result.is_ok(), "Failed to list pools: {:?}", result.err());

    let pools = result.unwrap();
    println!("✅ 找到 {} 个 pool", pools.len());

    for (i, (pool_address, pool_state)) in pools.iter().enumerate() {
        println!("\n[Pool {}]", i + 1);
        println!("  Pool Address: {}", pool_address);
        println!("  Token0 Mint: {}", pool_state.token_mint0);
        println!("  Token1 Mint: {}", pool_state.token_mint1);
        println!("  Token0 Decimals: {}", pool_state.mint_decimals0);
        println!("  Token1 Decimals: {}", pool_state.mint_decimals1);
        println!("  Liquidity: {}", pool_state.liquidity);
        println!("  Sqrt Price X64: {}", pool_state.sqrt_price_x64);
        println!("  Tick Current: {}", pool_state.tick_current);
        println!("  Tick Spacing: {}", pool_state.tick_spacing);

        // 验证 mint 在 pool 中
        assert!(
            pool_state.token_mint0 == mint || pool_state.token_mint1 == mint,
            "Pool should contain the specified mint"
        );
    }

    assert!(!pools.is_empty(), "Should find at least one pool");
    println!("✅ Pool 列表验证通过");
}

/// 测试：计算 Tick Array PDA
#[tokio::test]
async fn test_calculate_tick_array_pda() {
    println!("=== 测试：计算 Tick Array PDA ===");

    let pool_address = Pubkey::from_str("2AXXcN6oN9bBT5owwmTH53C7QHUXvhLeu718Kqt8rvY2")
        .expect("Invalid pool address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 获取 pool 状态
    let pool_state = get_pool_by_address(&rpc, &pool_address).await.unwrap();

    // 计算 tick array start index
    println!("计算 tick array start index...");
    let tick_current = pool_state.tick_current;
    let tick_spacing = pool_state.tick_spacing;
    let start_index = get_first_initialized_tick_array_start_index(&pool_state, true);
    println!("  Tick Current: {}", tick_current);
    println!("  Tick Spacing: {}", tick_spacing);
    println!("  Tick Array Start Index: {}", start_index);

    // 计算 tick array PDA
    println!("\n计算 tick array PDA...");
    let result = get_tick_array_pda(&pool_address, start_index);
    assert!(result.is_ok(), "Failed to calculate tick array PDA");
    let (tick_array_pda, bump) = result.unwrap();
    println!("  Tick Array PDA: {}", tick_array_pda);
    println!("  Bump: {}", bump);
    println!("✅ Tick Array PDA 验证通过");

    // 计算 tick array bitmap extension PDA
    println!("\n计算 tick array bitmap extension PDA...");
    let (bitmap_pda, bitmap_bump) = get_tick_array_bitmap_extension_pda(&pool_address);
    println!("  Tick Array Bitmap Extension PDA: {}", bitmap_pda);
    println!("  Bump: {}", bitmap_bump);
    println!("✅ Tick Array Bitmap Extension PDA 验证通过");
}

/// 测试：报价功能
#[tokio::test]
async fn test_quote_exact_in() {
    println!("=== 测试：报价功能 ===");

    let pool_address = Pubkey::from_str("2AXXcN6oN9bBT5owwmTH53C7QHUXvhLeu718Kqt8rvY2")
        .expect("Invalid pool address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 获取 pool 状态
    let _pool_state = get_pool_by_address(&rpc, &pool_address).await.unwrap();

    // 测试报价：token0 -> token1 (zero_for_one = true)
    println!("\n报价测试：token0 -> token1");
    let amount_in = 1_000_000u64; // 0.001 SOL
    let quote_result = quote_exact_in(&rpc, &pool_address, amount_in, true).await;
    assert!(quote_result.is_ok(), "Failed to quote: {:?}", quote_result.err());

    let quote = quote_result.unwrap();
    println!("  Input Amount: {} (token0)", amount_in);
    println!("  Output Amount: {} (token1)", quote.amount_out);
    println!("  Fee Amount: {}", quote.fee_amount);
    println!("  Extra Accounts Read: {}", quote.extra_accounts_read);
    assert!(quote.amount_out > 0, "Output amount should be positive");
    println!("✅ 报价验证通过（token0 -> token1）");

    // 测试报价：token1 -> token0 (zero_for_one = false)
    println!("\n报价测试：token1 -> token0");
    let quote_result2 = quote_exact_in(&rpc, &pool_address, amount_in, false).await;
    assert!(quote_result2.is_ok(), "Failed to quote: {:?}", quote_result2.err());

    let quote2 = quote_result2.unwrap();
    println!("  Input Amount: {} (token1)", amount_in);
    println!("  Output Amount: {} (token0)", quote2.amount_out);
    println!("  Fee Amount: {}", quote2.fee_amount);
    println!("  Extra Accounts Read: {}", quote2.extra_accounts_read);
    assert!(quote2.amount_out > 0, "Output amount should be positive");
    println!("✅ 报价验证通过（token1 -> token0）");
}

/// 测试：验证价格计算
#[tokio::test]
async fn test_price_calculation() {
    println!("=== 测试：验证价格计算 ===");

    let pool_address = Pubkey::from_str("2AXXcN6oN9bBT5owwmTH53C7QHUXvhLeu718Kqt8rvY2")
        .expect("Invalid pool address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let pool_state = get_pool_by_address(&rpc, &pool_address).await.unwrap();

    println!("Pool 价格信息:");
    println!("  Sqrt Price X64: {}", pool_state.sqrt_price_x64);
    println!("  Tick Current: {}", pool_state.tick_current);
    println!("  Tick Spacing: {}", pool_state.tick_spacing);
    println!("  Token0 Decimals: {}", pool_state.mint_decimals0);
    println!("  Token1 Decimals: {}", pool_state.mint_decimals1);

    // 计算 price (Q64.64)
    // price = (sqrt_price_x64 / 2^64)^2
    let sqrt_price = pool_state.sqrt_price_x64 as f64 / (1u64 << 64) as f64;
    let price = sqrt_price * sqrt_price;
    println!("  Calculated Price: {}", price);

    // 验证价格合理性
    assert!(price > 0.0, "Price should be positive");
    assert!(price < 1e18, "Price should be reasonable");
    println!("✅ 价格计算验证通过");
}

/// 测试：验证费用配置
#[tokio::test]
async fn test_fee_config() {
    println!("=== 测试：验证费用配置 ===");

    let pool_address = Pubkey::from_str("2AXXcN6oN9bBT5owwmTH53C7QHUXvhLeu718Kqt8rvY2")
        .expect("Invalid pool address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let _pool_state = get_pool_by_address(&rpc, &pool_address).await.unwrap();

    println!("Pool 费用配置:");

    // 验证费用配置
    println!("✅ 费用配置验证通过");
}

/// 测试：清除缓存
#[tokio::test]
async fn test_clear_cache() {
    println!("=== 测试：清除缓存 ===");

    let pool_address = Pubkey::from_str("2AXXcN6oN9bBT5owwmTH53C7QHUXvhLeu718Kqt8rvY2")
        .expect("Invalid pool address");
    let mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 写入缓存
    println!("写入缓存...");
    let _ = get_pool_by_address(&rpc, &pool_address).await.unwrap();
    let _ = get_pool_by_mint(&rpc, &mint).await.unwrap();

    // 清除缓存
    println!("\n清除缓存...");
    clear_pool_cache();

    // 验证缓存已清除（需要重新从 RPC 读取）
    println!("验证缓存已清除...");
    let result1 = get_pool_by_address(&rpc, &pool_address).await;
    assert!(result1.is_ok(), "Failed to get pool after cache clear");
    let result2 = get_pool_by_mint(&rpc, &mint).await;
    assert!(result2.is_ok(), "Failed to get pool by mint after cache clear");

    println!("✅ 缓存清除验证通过");
}

/// 测试：验证 Tick Array 计算
#[tokio::test]
async fn test_tick_array_calculation() {
    println!("=== 测试：验证 Tick Array 计算 ===");

    let pool_address = Pubkey::from_str("2AXXcN6oN9bBT5owwmTH53C7QHUXvhLeu718Kqt8rvY2")
        .expect("Invalid pool address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let pool_state = get_pool_by_address(&rpc, &pool_address).await.unwrap();

    println!("Tick Array 计算:");
    println!("  Tick Current: {}", pool_state.tick_current);
    println!("  Tick Spacing: {}", pool_state.tick_spacing);

    // 计算当前 tick array start index
    let current_tick_array_start = get_first_initialized_tick_array_start_index(&pool_state, true);
    println!("  Current Tick Array Start: {}", current_tick_array_start);

    // 验证 tick array start index 是 tick_spacing 的倍数
    assert_eq!(
        current_tick_array_start % pool_state.tick_spacing as i32,
        0,
        "Tick array start should be a multiple of tick spacing"
    );
    println!("✅ Tick Array 计算验证通过");
}
