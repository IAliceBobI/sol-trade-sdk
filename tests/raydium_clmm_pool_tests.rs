//! Raydium CLMM Pool 查找集成测试
//!
//! 测试所有 pool 查找方法：
//! - get_pool_by_address(rpc, pool_address) - 通过地址获取 pool 数据（带缓存）
//! - get_pool_by_mint(rpc, mint) - 通过 mint 获取 pool（带缓存，返回最优池）
//! - get_pool_by_address_force(rpc, pool_address) - 强制刷新缓存后获取
//! - get_pool_by_mint_force(rpc, mint) - 强制刷新缓存后通过 mint 获取
//! - list_pools_by_mint(rpc, mint) - 列出所有包含该 mint 的 pool
//!
//! 运行测试:
//!     cargo test --test raydium_clmm_pool_tests -- --nocapture
//!
//! 注意：使用 surfpool (localhost:8899) 进行测试

use serial_test::serial;
use sol_trade_sdk::instruction::utils::raydium_clmm::{
    get_pool_by_address,
    get_pool_by_mint,
    get_pool_by_address_force,
    get_pool_by_mint_force,
    list_pools_by_mint,
    clear_pool_cache,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

mod test_helpers;

/// 已知的 SOL Token Mint (WSOL)
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

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
#[serial]
async fn test_raydium_clmm_get_pool_by_mint_wsol_cache_and_force() {
    println!("=== 测试：Raydium CLMM get_pool_by_mint (WSOL, cache & force) ===");

    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 1. 清空缓存，确保从干净状态开始
    clear_pool_cache();
    println!("✅ 缓存已清空");

    // 2. 第一次 list_pools_by_mint：应从链上扫描
    let pools_1 = list_pools_by_mint(&rpc, &wsol_mint)
        .await
        .expect("list_pools_by_mint failed");
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
    let pools_2 = list_pools_by_mint(&rpc, &wsol_mint)
        .await
        .expect("list_pools_by_mint (2nd) failed");
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
    let (pool_addr_1, pool_state_1) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint failed");
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
    let (pool_addr_2, pool_state_2) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint (cached) failed");
    assert_eq!(pool_addr_1, pool_addr_2, "缓存中的 pool_address 不一致");
    assert_eq!(pool_state_1.amm_config, pool_state_2.amm_config, "缓存中的 amm_config 不一致");
    assert_eq!(pool_state_1.liquidity, pool_state_2.liquidity, "缓存中的 liquidity 不一致");
    println!("✅ get_pool_by_mint 缓存验证通过（数据一致）");

    // 6. 强制刷新：删除缓存后重新查询
    let (pool_addr_3, pool_state_3) = get_pool_by_mint_force(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint_force failed");
    println!("\n强制刷新后的 Pool: {}", pool_addr_3);

    // 通常情况下，强制刷新前后返回的主池应相同
    assert_eq!(pool_addr_2, pool_addr_3, "强制刷新后 pool_address 发生变化");
    assert_eq!(pool_state_2.token_mint0, pool_state_3.token_mint0, "强制刷新后 token_mint0 不一致");
    assert_eq!(pool_state_2.token_mint1, pool_state_3.token_mint1, "强制刷新后 token_mint1 不一致");
    assert_eq!(pool_state_2.liquidity, pool_state_3.liquidity, "强制刷新后 liquidity 发生变化");
    println!("✅ get_pool_by_mint_force 验证通过");

    println!("\n=== Raydium CLMM get_pool_by_mint 测试通过 ===");
}

/// 测试：列出所有包含 WSOL 的 Raydium CLMM Pool
///
/// 使用 `list_pools_by_mint`，验证：
/// - 返回列表非空
/// - 所有池的 `token_mint0` 或 `token_mint1` 中至少一侧为 WSOL
#[tokio::test]
async fn test_raydium_clmm_list_pools_by_mint_wsol() {
    println!("=== 测试：Raydium CLMM list_pools_by_mint (WSOL) ===");

    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    // 使用主网 RPC，因为本地 RPC 可能没有 CLMM 池数据
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    let pools = list_pools_by_mint(&rpc, &wsol_mint)
        .await
        .expect("list_pools_by_mint failed");

    assert!(!pools.is_empty(), "WSOL 相关的 CLMM Pool 列表不应为空");
    println!("✅ 找到 {} 个包含 WSOL 的 CLMM Pool", pools.len());

    for (i, (addr, pool)) in pools.iter().enumerate() {
        println!("\n[Pool {}]", i + 1);
        println!("  Pool Address: {}", addr);
        println!("  Token0 Mint: {}", pool.token_mint0);
        println!("  Token1 Mint: {}", pool.token_mint1);
        println!("  Liquidity: {}", pool.liquidity);
        println!("  Sqrt Price X64: {}", pool.sqrt_price_x64);
        println!("  Tick Current: {}", pool.tick_current);
        println!("  Tick Spacing: {}", pool.tick_spacing);

        assert!(
            pool.token_mint0 == wsol_mint || pool.token_mint1 == wsol_mint,
            "CLMM Pool {} 不包含 WSOL",
            addr,
        );

        // 验证基本约束（注意：有些池可能 liquidity 为 0，已被关闭或清空）
        if pool.liquidity == 0 {
            println!("  ⚠️  池子 {} liquidity 为 0（可能已关闭）", addr);
        }
        assert!(pool.tick_spacing > 0, "Pool {} tick_spacing should be positive", addr);
    }

    println!("\n=== Raydium CLMM list_pools_by_mint 测试通过 ===");
}

/// 测试：通过地址获取 pool 数据（带缓存）
#[tokio::test]
async fn test_raydium_clmm_get_pool_by_address() {
    println!("=== 测试：Raydium CLMM get_pool_by_address (带缓存) ===");

    // 使用之前测试中找到的 pool 地址
    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 先通过 mint 找到 pool 地址
    let (pool_address, _) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint failed");
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

/// 测试：强制刷新缓存
#[tokio::test]
#[serial]
async fn test_raydium_clmm_get_pool_by_address_force() {
    println!("=== 测试：Raydium CLMM get_pool_by_address_force ===");

    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 清空缓存
    clear_pool_cache();

    // 先获取 pool 地址
    let (pool_address, _) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint failed");
    println!("Pool 地址: {}", pool_address);

    // 第一次调用（写入缓存）
    println!("\n第一次调用（写入缓存）...");
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

    println!("\n=== Raydium CLMM get_pool_by_address_force 测试通过 ===");
}

/// 测试：清除缓存
#[tokio::test]
#[serial]
async fn test_raydium_clmm_clear_cache() {
    println!("=== 测试：Raydium CLMM clear_pool_cache ===");

    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 清空缓存
    clear_pool_cache();
    println!("缓存已清空");

    // 写入缓存
    println!("\n写入缓存...");
    let (pool_address, pool_state) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint failed");
    println!("  Pool Address: {}", pool_address);
    println!("  Liquidity: {}", pool_state.liquidity);

    // 再次查询确认缓存已写入
    let (_, pool_state_cached) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint (cached) failed");
    assert_eq!(pool_state.liquidity, pool_state_cached.liquidity);
    println!("  缓存验证成功");

    // 清除缓存
    println!("\n清除缓存...");
    clear_pool_cache();
    println!("缓存已清除");

    // 验证缓存已清除（需要重新从 RPC 读取）
    println!("\n验证缓存已清除...");
    let (pool_address2, pool_state2) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint after cache clear failed");
    assert_eq!(pool_address, pool_address2, "Pool address should be the same");
    assert_eq!(pool_state.liquidity, pool_state2.liquidity, "Pool state should be the same");
    println!("✅ 缓存清除验证通过");

    println!("\n=== Raydium CLMM clear_pool_cache 测试通过 ===");
}

/// 测试：通过非 WSOL mint 查找 CLMM Pool
///
/// 验证 get_pool_by_mint 对于非基础代币也能正常工作
#[tokio::test]
async fn test_raydium_clmm_get_pool_by_non_wsol_mint() {
    println!("=== 测试：Raydium CLMM get_pool_by_mint (非 WSOL mint) ===");

    // 先找到包含 WSOL 的池，然后获取其另一个 token 的 mint
    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 获取一个包含 WSOL 的池
    let (_, pool_state) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint failed");

    // 确定另一个 token 的 mint
    let target_mint = if pool_state.token_mint0 == wsol_mint {
        pool_state.token_mint1
    } else {
        pool_state.token_mint0
    };
    println!("目标 Token Mint: {}", target_mint);

    // 使用非 WSOL mint 查找池
    println!("\n使用非 WSOL mint 查找池...");
    let (pool_address2, pool_state2) = get_pool_by_mint(&rpc, &target_mint)
        .await
        .expect("get_pool_by_mint with target mint failed");

    println!("  Pool Address: {}", pool_address2);
    println!("  Token0 Mint: {}", pool_state2.token_mint0);
    println!("  Token1 Mint: {}", pool_state2.token_mint1);

    // 验证 mint 在池中
    assert!(
        pool_state2.token_mint0 == target_mint || pool_state2.token_mint1 == target_mint,
        "Pool should contain the target mint"
    );

    // 验证找到的是同一个池
    // 注意：由于可能返回不同的池（如果有多个池包含该 token），这里只验证 token 匹配
    println!("✅ 非 WSOL mint 查找验证通过");

    println!("\n=== Raydium CLMM get_pool_by_mint (非 WSOL) 测试通过 ===");
}
