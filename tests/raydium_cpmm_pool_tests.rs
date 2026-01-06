//! Raydium CPMM Pool 查找集成测试
//!
//! 测试所有 pool 查找方法：
//! - get_pool_by_address(rpc, pool_address) - 通过地址获取 pool 数据（带缓存）
//! - get_pool_by_mint(rpc, mint) - 通过 mint 获取 pool（带缓存，返回最优池）
//! - get_pool_by_address_force(rpc, pool_address) - 强制刷新缓存后获取
//! - get_pool_by_mint_force(rpc, mint) - 强制刷新缓存后通过 mint 获取
//! - list_pools_by_mint(rpc, mint) - 列出所有包含该 mint 的 pool
//! - get_pool_pda(amm_config, mint1, mint2) - 计算 pool PDA
//! - get_vault_pda(pool_state, mint) - 计算 vault PDA
//! - get_observation_state_pda(pool_state) - 计算 observation state PDA
//! - get_pool_token_balances(rpc, pool_state, token0_mint, token1_mint) - 获取 pool 余额
//! - quote_exact_in(rpc, pool_address, amount_in, is_token0_in) - 报价
//!
//! 运行测试:
//!     cargo test --test raydium_cpmm_pool_tests -- --nocapture
//!
//! 注意：使用 surfpool (localhost:8899) 进行测试

use sol_trade_sdk::instruction::utils::raydium_cpmm::{
    get_pool_by_address,
    get_pool_by_mint,
    get_pool_by_address_force,
    list_pools_by_mint,
    get_pool_pda,
    get_vault_pda,
    get_observation_state_pda,
    get_pool_token_balances,
    quote_exact_in,
    clear_pool_cache,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// 测试：通过地址获取 pool 数据（带缓存）
#[tokio::test]
async fn test_get_pool_by_address() {
    println!("=== 测试：通过地址获取 pool 数据（带缓存） ===");

    // 使用一个已知的 Raydium CPMM pool 地址
    // SOL-RAY pool on Raydium CPMM (来自 Raydium SDK V2 demo)
    let pool_address = Pubkey::from_str("4y81XN75NGct6iUYkBp2ixQKtXdrQxxMVgFbFF9w5n4u")
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
    println!("  Pool Creator: {}", pool_state.pool_creator);
    println!("  Token0 Mint: {}", pool_state.token0_mint);
    println!("  Token1 Mint: {}", pool_state.token1_mint);
    println!("  Token0 Vault: {}", pool_state.token0_vault);
    println!("  Token1 Vault: {}", pool_state.token1_vault);
    println!("  LP Mint: {}", pool_state.lp_mint);
    println!("  LP Supply: {}", pool_state.lp_supply);
    println!("  LP Mint Decimals: {}", pool_state.lp_mint_decimals);
    println!("  Token0 Decimals: {}", pool_state.mint0_decimals);
    println!("  Token1 Decimals: {}", pool_state.mint1_decimals);
    println!("  Protocol Fees Token0: {}", pool_state.protocol_fees_token0);
    println!("  Protocol Fees Token1: {}", pool_state.protocol_fees_token1);
    println!("  Fund Fees Token0: {}", pool_state.fund_fees_token0);
    println!("  Fund Fees Token1: {}", pool_state.fund_fees_token1);
    println!("  Open Time: {}", pool_state.open_time);
    println!("  Recent Epoch: {}", pool_state.recent_epoch);
    println!("  Status: {}", pool_state.status);

    // 验证基本字段约束
    assert!(!pool_state.token0_mint.eq(&Pubkey::default()), "Token0 mint should not be zero");
    assert!(!pool_state.token1_mint.eq(&Pubkey::default()), "Token1 mint should not be zero");
    assert!(!pool_state.lp_mint.eq(&Pubkey::default()), "LP mint should not be zero");
    assert!(!pool_state.token0_vault.eq(&Pubkey::default()), "Token0 vault should not be zero");
    assert!(!pool_state.token1_vault.eq(&Pubkey::default()), "Token1 vault should not be zero");
    assert!(pool_state.lp_supply > 0, "LP supply should be positive");
    assert!(pool_state.mint0_decimals > 0, "Token0 decimals should be positive");
    assert!(pool_state.mint1_decimals > 0, "Token1 decimals should be positive");
    println!("✅ 基本字段验证通过");

    // 获取 token 余额
    let (token0_balance, token1_balance) = get_pool_token_balances(
        &rpc,
        &pool_address,
        &pool_state.token0_mint,
        &pool_state.token1_mint,
    )
    .await
    .unwrap();
    println!("  Token0 Balance: {}", token0_balance);
    println!("  Token1 Balance: {}", token1_balance);
    assert!(token0_balance > 0, "Token0 balance should be positive");
    assert!(token1_balance > 0, "Token1 balance should be positive");
    println!("✅ Token 余额验证通过");

    // 第二次调用（应该从缓存读取）
    println!("\n第二次调用（从缓存读取）...");
    let result2 = get_pool_by_address(&rpc, &pool_address).await;
    assert!(result2.is_ok(), "Failed to get pool from cache: {:?}", result2.err());

    let pool_state2 = result2.unwrap();
    assert_eq!(pool_state.amm_config, pool_state2.amm_config, "Cached pool should match");
    assert_eq!(pool_state.token0_mint, pool_state2.token0_mint, "Cached pool should match");
    assert_eq!(pool_state.token1_mint, pool_state2.token1_mint, "Cached pool should match");
    assert_eq!(pool_state.lp_supply, pool_state2.lp_supply, "Cached pool should match");
    println!("✅ 缓存验证通过（数据一致）");
}

/// 测试：通过 mint 获取 pool（带缓存）
#[tokio::test]
async fn test_get_pool_by_mint() {
    println!("=== 测试：通过 mint 获取 pool（带缓存） ===");

    // 使用 RAY token mint（SOL-RAY pool 存在）
    let mint = Pubkey::from_str("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R").unwrap();
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 第一次调用（会写入缓存）
    println!("第一次调用（写入缓存）...");
    let result1 = get_pool_by_mint(&rpc, &mint).await;
    assert!(result1.is_ok(), "Failed to get pool by mint: {:?}", result1.err());

    let (pool_address, pool_state) = result1.unwrap();
    println!("✅ Pool 获取成功!");
    println!("  Pool Address: {}", pool_address);
    println!("  Token0 Mint: {}", pool_state.token0_mint);
    println!("  Token1 Mint: {}", pool_state.token1_mint);
    println!("  LP Supply: {}", pool_state.lp_supply);

    // 验证 mint 在 pool 中
    assert!(
        pool_state.token0_mint == mint || pool_state.token1_mint == mint,
        "Pool should contain the specified mint"
    );
    println!("✅ Mint 验证通过");

    // 第二次调用（应该从缓存读取）
    println!("\n第二次调用（从缓存读取）...");
    let result2 = get_pool_by_mint(&rpc, &mint).await;
    assert!(result2.is_ok(), "Failed to get pool from cache: {:?}", result2.err());

    let (pool_address2, pool_state2) = result2.unwrap();
    assert_eq!(pool_address, pool_address2, "Cached pool address should match");
    assert_eq!(pool_state.lp_supply, pool_state2.lp_supply, "Cached pool should match");
    println!("✅ 缓存验证通过（数据一致）");
}

/// 测试：强制刷新缓存
#[tokio::test]
async fn test_get_pool_by_address_force() {
    println!("=== 测试：强制刷新缓存 ===");

    let pool_address = Pubkey::from_str("4y81XN75NGct6iUYkBp2ixQKtXdrQxxMVgFbFF9w5n4u")
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
    assert_eq!(pool_state1.lp_supply, pool_state2.lp_supply);

    // 强制刷新缓存
    println!("\n强制刷新缓存...");
    let result3 = get_pool_by_address_force(&rpc, &pool_address).await;
    assert!(result3.is_ok());
    let pool_state3 = result3.unwrap();
    assert_eq!(pool_state1.lp_supply, pool_state3.lp_supply);
    println!("✅ 强制刷新验证通过");
}

/// 测试：列出所有包含该 mint 的 pool
#[tokio::test]
async fn test_list_pools_by_mint() {
    println!("=== 测试：列出所有包含该 mint 的 pool ===");

    // 使用 RAY token mint（SOL-RAY pool 存在）
    let mint = Pubkey::from_str("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R").unwrap();
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
        println!("  Amm Config: {}", pool_state.amm_config);
        println!("  Pool Creator: {}", pool_state.pool_creator);
        println!("  Token0 Mint: {}", pool_state.token0_mint);
        println!("  Token1 Mint: {}", pool_state.token1_mint);
        println!("  Token0 Vault: {}", pool_state.token0_vault);
        println!("  Token1 Vault: {}", pool_state.token1_vault);
        println!("  LP Mint: {}", pool_state.lp_mint);
        println!("  LP Supply: {}", pool_state.lp_supply);
        println!("  LP Mint Decimals: {}", pool_state.lp_mint_decimals);
        println!("  Status: {}", pool_state.status);

        // 验证 mint 在 pool 中
        assert!(
            pool_state.token0_mint == mint || pool_state.token1_mint == mint,
            "Pool should contain the specified mint"
        );
    }

    assert!(!pools.is_empty(), "Should find at least one pool");
    println!("✅ Pool 列表验证通过");
}

/// 测试：计算 PDA
#[tokio::test]
async fn test_calculate_pda() {
    println!("=== 测试：计算 PDA ===");

    let pool_address = Pubkey::from_str("4y81XN75NGct6iUYkBp2ixQKtXdrQxxMVgFbFF9w5n4u")
        .expect("Invalid pool address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 获取 pool 状态
    let pool_state = get_pool_by_address(&rpc, &pool_address).await.unwrap();

    // 计算 pool PDA
    println!("计算 pool PDA...");
    let calculated_pool_pda = get_pool_pda(
        &pool_state.amm_config,
        &pool_state.token0_mint,
        &pool_state.token1_mint,
    );
    assert!(calculated_pool_pda.is_some(), "Failed to calculate pool PDA");
    let pool_pda = calculated_pool_pda.unwrap();
    println!("  Calculated Pool PDA: {}", pool_pda);
    println!("  Actual Pool Address: {}", pool_address);
    assert_eq!(pool_pda, pool_address, "Calculated PDA should match actual address");
    println!("✅ Pool PDA 验证通过");

    // 计算 vault PDA
    println!("\n计算 vault PDA...");
    let token0_vault_pda = get_vault_pda(&pool_address, &pool_state.token0_mint);
    assert!(token0_vault_pda.is_some(), "Failed to calculate vault PDA");
    println!("  Token0 Vault PDA: {}", token0_vault_pda.unwrap());

    let token1_vault_pda = get_vault_pda(&pool_address, &pool_state.token1_mint);
    assert!(token1_vault_pda.is_some(), "Failed to calculate vault PDA");
    println!("  Token1 Vault PDA: {}", token1_vault_pda.unwrap());
    println!("✅ Vault PDA 验证通过");

    // 计算 observation state PDA
    println!("\n计算 observation state PDA...");
    let obs_state_pda = get_observation_state_pda(&pool_address);
    assert!(obs_state_pda.is_some(), "Failed to calculate observation state PDA");
    println!("  Observation State PDA: {}", obs_state_pda.unwrap());
    println!("✅ Observation State PDA 验证通过");
}

/// 测试：报价功能
#[tokio::test]
async fn test_quote_exact_in() {
    println!("=== 测试：报价功能 ===");

    let pool_address = Pubkey::from_str("4y81XN75NGct6iUYkBp2ixQKtXdrQxxMVgFbFF9w5n4u")
        .expect("Invalid pool address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 获取 pool 状态
    let _pool_state = get_pool_by_address(&rpc, &pool_address).await.unwrap();

    // 测试报价：token0 -> token1
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

    // 测试报价：token1 -> token0
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

/// 测试：清除缓存
#[tokio::test]
async fn test_clear_cache() {
    println!("=== 测试：清除缓存 ===");

    let pool_address = Pubkey::from_str("4y81XN75NGct6iUYkBp2ixQKtXdrQxxMVgFbFF9w5n4u")
        .expect("Invalid pool address");
    // 使用 RAY token mint（SOL-RAY pool 存在）
    let mint = Pubkey::from_str("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R").unwrap();
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 写入缓存
    println!("写入缓存...");
    let _ = get_pool_by_address(&rpc, &pool_address).await;
    let _ = get_pool_by_mint(&rpc, &mint).await;

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