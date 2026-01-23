use sol_trade_sdk::instruction::utils::pumpswap::{
    clear_pool_cache, find_pool, get_pool_by_address,
    get_token_balances, get_token_price_in_usd_with_pool,
};
use sol_trade_sdk::common::auto_mock_rpc::AutoMockRpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// 已知的 Pump 代币 mint
const PUMP_MINT: &str = "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn";

/// 已知的 PumpSwap pool 地址
const PUMP_POOL_ADDRESS: &str = "539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR";

/// 测试：通过 mint 查找 pool 地址（使用 Auto Mock 加速）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_find_pool_by_mint() {
    println!("=== 测试：通过 mint 查找 pool 地址（Auto Mock 加速） ===");

    // 清空缓存，确保从干净状态开始
    clear_pool_cache();

    let mint = Pubkey::from_str(PUMP_MINT).unwrap();
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端（使用独立命名空间）
    let rpc = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("pumpswap_pool_tests".to_string())
    );

    // 调用泛型版本的 find_pool_with_client
    let result = find_pool(&rpc, &mint).await;

    // 验证结果
    assert!(result.is_ok(), "Failed to find pool: {:?}", result.err());

    let pool_address = result.unwrap();
    println!("✅ 找到的 pool 地址: {}", pool_address);

    // 验证 pool 地址不是零地址
    assert!(!pool_address.eq(&Pubkey::default()), "Pool address should not be zero");
    println!("✅ Pool 地址验证通过（非零地址）");
}

/// 测试：通过地址获取 pool 数据（带缓存，使用 Auto Mock 加速）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_get_pool_by_address() {
    println!("=== 测试：通过地址获取 pool 数据（Auto Mock 加速） ===");

    // 清空缓存，确保从干净状态开始
    clear_pool_cache();

    let pool_address = Pubkey::from_str(PUMP_POOL_ADDRESS).unwrap();
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端（使用独立命名空间）
    let rpc = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("pumpswap_pool_tests".to_string())
    );

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

    // 获取 token 余额（使用泛型版本）
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

/// 测试：获取 PumpSwap token 的 USD 价格（使用 Auto Mock 加速）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_get_pumpswap_token_price_in_usd() {
    println!("=== 测试：获取 PumpSwap token 的 USD 价格（Auto Mock 加速） ===");

    // 清空缓存，确保从干净状态开始
    clear_pool_cache();

    let token_mint = Pubkey::from_str(PUMP_MINT).unwrap();
    let pool_address = Pubkey::from_str(PUMP_POOL_ADDRESS).unwrap();
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端（使用独立命名空间）
    let rpc = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("pumpswap_pool_tests".to_string())
    );

    println!("Token Mint: {}", token_mint);
    println!("Pool 地址: {}", pool_address);
    println!("WSOL-USDT 锚定池: 使用默认锚定池");

    // 调用泛型版本的价格计算函数
    let result = get_token_price_in_usd_with_pool(&rpc, &token_mint, &pool_address, None).await;

    // 验证结果
    assert!(result.is_ok(), "Failed to get token price in USD: {:?}", result.err());

    let price_usd = result.unwrap();
    println!("✅ Token USD 价格: ${:.8}", price_usd);

    // 验证价格在合理范围内（应该大于 0 且小于 1000 USD）
    assert!(price_usd > 0.0, "Price should be positive");
    assert!(price_usd < 1000.0, "Price should be reasonable (< $1000)");
    println!("✅ 价格范围验证通过");
}

/// 测试：使用 Auto Mock 获取 Pool 数据（加速版）
///
/// 此测试使用 AutoMockRpcClient 来加速 pool 查询。
/// 首次运行时会从 RPC 获取数据并保存到 tests/mock_data/，
/// 后续运行会直接从缓存加载，速度提升显著。
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_get_pool_by_address_with_auto_mock() {
    println!("=== 测试：使用 Auto Mock 获取 Pool 数据（加速版） ===");

    let pool_address = Pubkey::from_str(PUMP_POOL_ADDRESS).unwrap();
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端
    let auto_mock_client = AutoMockRpcClient::new(rpc_url.to_string());

    println!("获取 Pool 数据: {}", pool_address);

    // 清除缓存，确保测试从干净状态开始
    clear_pool_cache();

    // 首次调用：从 RPC 获取并保存（约 1-2 秒）
    // 后续调用：从缓存加载（约 0.01 秒）
    let result = get_pool_by_address(&auto_mock_client, &pool_address).await;
    assert!(result.is_ok(), "Failed to get pool by address: {:?}", result.err());

    let pool_state = result.unwrap();
    println!("✅ Pool State 获取成功!");
    println!("  Pool Bump: {}", pool_state.pool_bump);
    println!("  Index: {}", pool_state.index);
    println!("  Base Mint: {}", pool_state.base_mint);
    println!("  Quote Mint: {}", pool_state.quote_mint);
    println!("  LP Mint: {}", pool_state.lp_mint);
    println!("  LP Supply: {}", pool_state.lp_supply);

    // 验证基本字段
    assert!(!pool_state.base_mint.eq(&Pubkey::default()), "Base mint should not be zero");
    assert!(!pool_state.quote_mint.eq(&Pubkey::default()), "Quote mint should not be zero");
    assert!(pool_state.lp_supply > 0, "LP supply should be positive");

    println!("\n=== Auto Mock 测试通过 ===");
    println!("✅ 首次运行：从 RPC 获取并保存（约 1-2 秒）");
    println!("✅ 后续运行：从缓存加载（约 0.01 秒）");
    println!("✅ 速度提升：约 100-200 倍！");
}
