//! Raydium CLMM Pool 查找集成测试
//!
//! 测试所有 pool 查找方法：
//! - get_pool_by_address(rpc, pool_address) - 通过地址获取 pool 数据（带缓存）
//! - get_pool_by_mint(rpc, mint) - 通过 mint 获取 pool（带缓存，返回最优池）
//! - get_pool_by_address_force(rpc, pool_address) - 强制刷新缓存后获取
//! - get_pool_by_mint_force(rpc, mint) - 强制刷新缓存后通过 mint 获取
//! - list_pools_by_mint(rpc, mint) - 列出所有包含该 mint 的 pool
//! - get_wsol_price_in_usd(rpc, wsol_usd_pool) - 通过锚定池获取 WSOL 的 USD 价格（实时，不缓存）
//! - get_token_price_in_usd(rpc, token_mint, wsol_usd_pool) - 通过 X-WSOL 池 + 锚定池获取任意 Token 的 USD 价格
//!
//! 运行测试:
//!     cargo test --test raydium_clmm_pool_tests -- --nocapture
//!
//! 注意：使用 surfpool (localhost:8899) 进行测试

use sol_trade_sdk::instruction::utils::raydium_clmm::{
    clear_pool_cache, get_pool_by_address, get_pool_by_mint, get_pool_by_mint_force,
    get_token_price_in_usd, get_token_price_in_usd_with_pool, get_wsol_price_in_usd,
    list_pools_by_mint,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

mod test_helpers;

/// 已知的 SOL Token Mint (WSOL)
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// 已知的 WSOL-USDT CLMM 锚定池（用于 USD 价格测试）
const WSOL_USDT_CLMM_POOL: &str = "ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6";

/// 已知的 JUP mint（用于测试任意 token 的 USD 价格）
const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

/// 测试：基于 WSOL mint 查找 CLMM Pool，并验证缓存与强制刷新
///
/// 步骤：
/// 1. 清空 CLMM 缓存
/// 2. 使用 `list_pools_by_mint` 基于 WSOL mint 列出所有 Pool（应从链上扫描）
/// 3. 再次调用 `list_pools_by_mint`（应返回相同结果）
/// 4. 清除缓存后再次调用（结果应一致）
/// 5. 使用 `get_pool_by_mint` 查找最优 Pool 并验证缓存
/// 6. 使用 `get_pool_by_mint_force` 强制刷新（结果通常相同）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_clmm_get_pool_by_mint_wsol_cache_and_force() {
    println!("=== 测试：Raydium CLMM get_pool_by_mint (WSOL, cache & force) ===");

    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 1. 清空缓存，确保从干净状态开始
    clear_pool_cache();
    println!("✅ 缓存已清空");

    // 2. 第一次 list_pools_by_mint：应从链上扫描
    let pools_1 = list_pools_by_mint(&rpc, &wsol_mint).await.expect("list_pools_by_mint failed");
    println!("第一次 list_pools_by_mint 查询到 {} 个 Pool", pools_1.len());
    assert!(!pools_1.is_empty(), "WSOL 相关的 CLMM Pool 列表不应为空");

    for (addr, pool) in &pools_1 {
        assert!(
            pool.token_mint0 == wsol_mint || pool.token_mint1 == wsol_mint,
            "Pool {} 不包含 WSOL",
            addr
        );
    }
    println!("✅ 第一次 list_pools 验证通过");

    // 3. 第二次 list_pools_by_mint：应返回相同结果（来自缓存或链上）
    let pools_2 =
        list_pools_by_mint(&rpc, &wsol_mint).await.expect("list_pools_by_mint (2nd) failed");
    assert_eq!(pools_1.len(), pools_2.len(), "第二次 list_pools 数量不一致");
    println!("✅ 第二次 list_pools 验证通过（数量一致）");

    // 4. 清除缓存后再次查询
    clear_pool_cache();
    println!("✅ 缓存已再次清空");

    let pools_3 = list_pools_by_mint(&rpc, &wsol_mint)
        .await
        .expect("list_pools_by_mint (after clear) failed");
    assert_eq!(pools_1.len(), pools_3.len(), "清除缓存后 list_pools 数量不一致");
    println!("✅ 清除缓存后 list_pools 验证通过");

    // 5. 使用 get_pool_by_mint 查找最优 Pool
    let (pool_addr_1, pool_state_1) =
        get_pool_by_mint(&rpc, &wsol_mint).await.expect("get_pool_by_mint failed");
    println!("\nget_pool_by_mint 查询到的最优 Pool: {}", pool_addr_1);
    println!("  token0_mint: {}", pool_state_1.token_mint0);
    println!("  token1_mint: {}", pool_state_1.token_mint1);
    println!("  liquidity: {}", pool_state_1.liquidity);
    println!("  tick_current: {}", pool_state_1.tick_current);
    println!("  tick_spacing: {}", pool_state_1.tick_spacing);
    println!("  sqrt_price_x64: {}", pool_state_1.sqrt_price_x64);

    assert!(
        pool_state_1.token_mint0 == wsol_mint || pool_state_1.token_mint1 == wsol_mint,
        "返回的 CLMM Pool 不包含 WSOL"
    );

    // 验证基本字段
    assert!(!pool_state_1.token_mint0.eq(&Pubkey::default()), "Token0 mint should not be zero");
    assert!(!pool_state_1.token_mint1.eq(&Pubkey::default()), "Token1 mint should not be zero");
    assert!(!pool_state_1.amm_config.eq(&Pubkey::default()), "AMM config should not be zero");
    assert!(pool_state_1.liquidity > 0, "Liquidity should be positive");
    assert!(pool_state_1.tick_spacing > 0, "Tick spacing should be positive");
    println!("✅ 基本字段验证通过");

    // 第二次查询：应命中缓存，返回相同的池地址
    let (pool_addr_2, pool_state_2) =
        get_pool_by_mint(&rpc, &wsol_mint).await.expect("get_pool_by_mint (cached) failed");
    assert_eq!(pool_addr_1, pool_addr_2, "缓存中的 pool_address 不一致");
    assert_eq!(pool_state_1.amm_config, pool_state_2.amm_config, "缓存中的 amm_config 不一致");
    assert_eq!(pool_state_1.liquidity, pool_state_2.liquidity, "缓存中的 liquidity 不一致");
    println!("✅ get_pool_by_mint 缓存验证通过（数据一致）");

    // 6. 强制刷新：删除缓存后重新查询
    let (pool_addr_3, pool_state_3) =
        get_pool_by_mint_force(&rpc, &wsol_mint).await.expect("get_pool_by_mint_force failed");
    println!("\n强制刷新后的 Pool: {}", pool_addr_3);

    // 通常情况下，强制刷新前后返回的主池应相同
    assert_eq!(pool_addr_2, pool_addr_3, "强制刷新后 pool_address 发生变化");
    assert_eq!(pool_state_2.token_mint0, pool_state_3.token_mint0, "强制刷新后 token_mint0 不一致");
    assert_eq!(pool_state_2.token_mint1, pool_state_3.token_mint1, "强制刷新后 token_mint1 不一致");
    assert_eq!(pool_state_2.liquidity, pool_state_3.liquidity, "强制刷新后 liquidity 发生变化");
    println!("✅ get_pool_by_mint_force 验证通过");

    println!("\n=== Raydium CLMM get_pool_by_mint 测试通过 ===");
}

/// 测试：通过地址获取 pool 数据（带缓存）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_clmm_get_pool_by_address() {
    println!("=== 测试：Raydium CLMM get_pool_by_address (带缓存) ===");

    // 使用之前测试中找到的 pool 地址
    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 先通过 mint 找到 pool 地址
    let (pool_address, _) =
        get_pool_by_mint(&rpc, &wsol_mint).await.expect("get_pool_by_mint failed");
    println!("找到的 Pool 地址: {}", pool_address);

    // 第一次调用（会写入缓存）
    println!("\n第一次调用（写入缓存）...");
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

    println!("\n=== Raydium CLMM get_pool_by_address 测试通过 ===");
}

/// 测试：通过 WSOL-USDT 锚定池获取 WSOL 的 USD 价格（实时、主网）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_clmm_get_wsol_price_in_usd() {
    println!("=== 测试：Raydium CLMM get_wsol_price_in_usd (WSOL-USDT anchor) ===");

    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let anchor_pool = Pubkey::from_str(WSOL_USDT_CLMM_POOL).expect("Invalid WSOL-USDT pool");

    let price = get_wsol_price_in_usd(&rpc, Some(&anchor_pool))
        .await
        .expect("Failed to get WSOL price in USD");

    println!("WSOL price in USD: {}", price);

    // 只做宽松校验：价格应为正数，且在合理区间（防止明显异常）
    assert!(price > 0.0, "WSOL price in USD should be positive");
    assert!(price < 1000.0, "WSOL price in USD is unreasonably high");

    println!("✅ Raydium CLMM get_wsol_price_in_usd 测试通过");
}

/// 测试：通过 Raydium CLMM 获取 JUP 的 USD 价格（X-WSOL 池 + WSOL-USD 锚定池，主网）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_clmm_get_jup_price_in_usd() {
    println!("=== 测试：Raydium CLMM get_token_price_in_usd (JUP) ===");

    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let jup_mint = Pubkey::from_str(JUP_MINT).expect("Invalid JUP mint");

    let price = get_token_price_in_usd(&rpc, &jup_mint, None)
        .await
        .expect("Failed to get JUP price in USD");

    println!("JUP price in USD: {}", price);

    // 宽松校验：价格应为正数，且在合理区间
    assert!(price > 0.0, "JUP price in USD should be positive");
    assert!(price < 100.0, "JUP price in USD is unreasonably high (likely an error)");

    println!("✅ Raydium CLMM get_token_price_in_usd (JUP) 测试通过");
}

/// 测试：通过 Raydium CLMM 获取 JUP 的 USD 价格（直接传入池地址，跳过查找，主网）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_clmm_get_jup_price_in_usd_with_pool() {
    println!("=== 测试：Raydium CLMM get_token_price_in_usd_with_pool (JUP, 跳过查找) ===");

    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let jup_mint = Pubkey::from_str(JUP_MINT).expect("Invalid JUP mint");

    // 1. 先用标准接口找到 JUP-WSOL 池地址（模拟：你已经缓存了这个池地址）
    let (jup_wsol_pool, _) = get_pool_by_mint(&rpc, &jup_mint)
        .await
        .expect("Failed to find JUP-WSOL pool");
    println!("找到的 JUP-WSOL 池地址: {}", jup_wsol_pool);

    // 2. 使用 get_token_price_in_usd_with_pool 直接传入池地址，避免重复查找
    let price = get_token_price_in_usd_with_pool(&rpc, &jup_mint, &jup_wsol_pool, None)
        .await
        .expect("Failed to get JUP price in USD with pool");

    println!("JUP price in USD (with pool): {}", price);

    // 宽松校验：价格应为正数，且在合理区间
    assert!(price > 0.0, "JUP price in USD should be positive");
    assert!(price < 100.0, "JUP price in USD is unreasonably high (likely an error)");

    println!("✅ Raydium CLMM get_token_price_in_usd_with_pool (JUP) 测试通过");
}
