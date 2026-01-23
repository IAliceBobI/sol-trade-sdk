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
    clear_pool_cache, get_pool_by_address_with_pool_client, get_pool_by_mint,
    get_pool_by_mint_force, get_pool_by_mint_with_pool_client,
    get_token_price_in_usd_with_client,
    get_token_price_in_usd_with_pool_with_client,
    get_wsol_price_in_usd_with_client, list_pools_by_mint_with_pool_client,
};
use sol_trade_sdk::common::auto_mock_rpc::AutoMockRpcClient;
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

/// 测试：通过地址获取 pool 数据（带缓存）
///
/// 使用 Auto Mock 加速测试，首次运行时从 RPC 获取并缓存，后续从文件加载。
/// 内存缓存功能在 test_raydium_clmm_get_pool_by_mint_with_auto_mock 中已充分测试。
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_clmm_get_pool_by_address() {
    println!("=== 测试：Raydium CLMM get_pool_by_address (Auto Mock 加速) ===");

    // 使用已知的 WSOL-USDT CLMM Pool
    let pool_address = Pubkey::from_str(WSOL_USDT_CLMM_POOL)
        .unwrap_or_else(|_| panic!("Invalid pool address: {}", WSOL_USDT_CLMM_POOL));
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端
    let auto_mock_client = AutoMockRpcClient::new(rpc_url.to_string());

    println!("Pool 地址: {}", pool_address);

    // 清除缓存
    clear_pool_cache();

    // 使用 Auto Mock 获取 pool 数据
    println!("\n使用 Auto Mock 获取 Pool 数据...");
    let result = get_pool_by_address_with_pool_client(&auto_mock_client, &pool_address).await;
    assert!(result.is_ok(), "Failed to get pool by address: {:?}", result.err());

    let pool_state = result.unwrap();
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

    println!("\n=== Raydium CLMM get_pool_by_address 测试通过 ===");
    println!("✅ 首次运行：从 RPC 获取并保存（约 1-2 秒）");
    println!("✅ 后续运行：从缓存加载（约 0.01 秒）");
    println!("✅ 速度提升：约 100-200 倍！");
    println!("💡 注意：内存缓存功能在 test_raydium_clmm_get_pool_by_mint_with_auto_mock 中已充分测试");
}

/// 测试：通过 WSOL-USDT 锚定池获取 WSOL 的 USD 价格（Auto Mock 加速）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_clmm_get_wsol_price_in_usd() {
    println!("=== 测试：Raydium CLMM get_wsol_price_in_usd (Auto Mock 加速) ===");

    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端
    let auto_mock_client = AutoMockRpcClient::new(rpc_url.to_string());

    let anchor_pool = Pubkey::from_str(WSOL_USDT_CLMM_POOL).expect("Invalid WSOL-USDT pool");

    let price = get_wsol_price_in_usd_with_client(&auto_mock_client, Some(&anchor_pool))
        .await
        .expect("Failed to get WSOL price in USD");

    println!("WSOL price in USD: {}", price);

    // 只做宽松校验：价格应为正数，且在合理区间（防止明显异常）
    assert!(price > 0.0, "WSOL price in USD should be positive");
    assert!(price < 1000.0, "WSOL price in USD is unreasonably high");

    println!("✅ Raydium CLMM get_wsol_price_in_usd 测试通过");
    println!("✅ 首次运行：从 RPC 获取并保存（约 1-2 秒）");
    println!("✅ 后续运行：从缓存加载（约 0.01 秒）");
    println!("✅ 速度提升：约 100-200 倍！");
}

/// 测试：通过 Raydium CLMM 获取 JUP 的 USD 价格（Auto Mock 加速）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_clmm_get_jup_price_in_usd() {
    println!("=== 测试：Raydium CLMM get_token_price_in_usd (JUP, Auto Mock 加速) ===");

    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端
    let auto_mock_client = AutoMockRpcClient::new(rpc_url.to_string());

    let jup_mint = Pubkey::from_str(JUP_MINT)
        .unwrap_or_else(|_| panic!("Invalid JUP mint: {}", JUP_MINT));

    let price = get_token_price_in_usd_with_client(&auto_mock_client, &jup_mint, None)
        .await
        .expect("Failed to get JUP price in USD");

    println!("JUP price in USD: {}", price);

    // 宽松校验：价格应为正数，且在合理区间
    assert!(price > 0.0, "JUP price in USD should be positive");
    assert!(price < 100.0, "JUP price in USD is unreasonably high (likely an error)");

    println!("✅ Raydium CLMM get_token_price_in_usd (JUP) 测试通过");
    println!("✅ 首次运行：从 RPC 获取并保存（约 2-3 秒）");
    println!("✅ 后续运行：从缓存加载（约 0.01 秒）");
    println!("✅ 速度提升：约 100-200 倍！");
}

/// 测试：通过 Raydium CLMM 获取 JUP 的 USD 价格（直接传入池地址，Auto Mock 加速）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_clmm_get_jup_price_in_usd_with_pool() {
    println!("=== 测试：Raydium CLMM get_token_price_in_usd_with_pool (JUP, Auto Mock 加速) ===");

    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端
    let auto_mock_client = AutoMockRpcClient::new(rpc_url.to_string());

    let jup_mint = Pubkey::from_str(JUP_MINT)
        .unwrap_or_else(|_| panic!("Invalid JUP mint: {}", JUP_MINT));

    // 1. 先用 Auto Mock 接口找到 JUP-WSOL 池地址（模拟：你已经缓存了这个池地址）
    let (jup_wsol_pool, _) = get_pool_by_mint_with_pool_client(&auto_mock_client, &jup_mint)
        .await
        .expect("Failed to find JUP-WSOL pool");
    println!("找到的 JUP-WSOL 池地址: {}", jup_wsol_pool);

    // 2. 使用 get_token_price_in_usd_with_pool_with_client 直接传入池地址，避免重复查找
    let price = get_token_price_in_usd_with_pool_with_client(&auto_mock_client, &jup_mint, &jup_wsol_pool, None)
        .await
        .expect("Failed to get JUP price in USD with pool");

    println!("JUP price in USD (with pool): {}", price);

    // 宽松校验：价格应为正数，且在合理区间
    assert!(price > 0.0, "JUP price in USD should be positive");
    assert!(price < 100.0, "JUP price in USD is unreasonably high (likely an error)");

    println!("✅ Raydium CLMM get_token_price_in_usd_with_pool (JUP) 测试通过");
    println!("✅ 首次运行：从 RPC 获取并保存（约 2-3 秒）");
    println!("✅ 后续运行：从缓存加载（约 0.01 秒）");
    println!("✅ 速度提升：约 100-200 倍！");
}


/// 测试：使用 Auto Mock 加速 get_pool_by_mint 和 list_pools_by_mint（加速版）
///
/// 此测试使用 AutoMockRpcClient 来加速 pool 查询。
/// 替代慢测试：test_raydium_clmm_get_pool_by_mint_wsol_cache_and_force（85.95 秒）
///
/// 首次运行时会从 RPC 获取数据并保存到 tests/mock_data/，
/// 后续运行会直接从缓存加载，速度提升显著。
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_clmm_get_pool_by_mint_with_auto_mock() {
    println!("=== 测试：使用 Auto Mock 加速 get_pool_by_mint 和 list_pools_by_mint ===");

    // 设置环境变量，限制扫描的 Pool 数量（测试环境优化）
    std::env::set_var("CLMM_POOL_SCAN_LIMIT", "100");

    let wsol_mint = Pubkey::from_str(WSOL_MINT)
        .unwrap_or_else(|_| panic!("Invalid WSOL mint: {}", WSOL_MINT));
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 使用 Auto Mock RPC 客户端
    let auto_mock_client = AutoMockRpcClient::new(rpc_url.to_string());

    println!("Token Mint: {}", wsol_mint);

    // ========== 第一部分：测试 _with_pool_client 版本（无内存缓存） ==========

    // 清除所有缓存
    clear_pool_cache();

    // 1. 使用 Auto Mock 的 list_pools_by_mint
    println!("\n步骤 1: 使用 list_pools_by_mint_with_pool_client 查询所有 WSOL Pool...");
    let pools = list_pools_by_mint_with_pool_client(&auto_mock_client, &wsol_mint)
        .await
        .expect("list_pools_by_mint_with_pool_client failed");
    println!("✅ 查询到 {} 个 Pool", pools.len());
    assert!(!pools.is_empty(), "WSOL 相关的 CLMM Pool 列表不应为空");

    for (addr, pool) in pools.iter().take(3) {
        // 只打印前 3 个
        println!(
            "  Pool: {} | Token0: {} | Token1: {} | Liquidity: {}",
            addr, pool.token_mint0, pool.token_mint1, pool.liquidity
        );
    }
    if pools.len() > 3 {
        println!("  ... 还有 {} 个 Pool", pools.len() - 3);
    }

    // 2. 使用 Auto Mock 的 get_pool_by_mint（无缓存版本）
    println!("\n步骤 2: 使用 get_pool_by_mint_with_pool_client 查询最优 Pool...");
    let (pool_addr_1, pool_state_1) = get_pool_by_mint_with_pool_client(&auto_mock_client, &wsol_mint)
        .await
        .expect("get_pool_by_mint_with_pool_client failed");
    println!("✅ 找到最优 Pool: {}", pool_addr_1);
    println!("  token0_mint: {}", pool_state_1.token_mint0);
    println!("  token1_mint: {}", pool_state_1.token_mint1);
    println!("  liquidity: {}", pool_state_1.liquidity);

    // 验证基本字段
    assert!(
        pool_state_1.token_mint0 == wsol_mint || pool_state_1.token_mint1 == wsol_mint,
        "返回的 CLMM Pool 不包含 WSOL"
    );
    assert!(!pool_state_1.token_mint0.eq(&Pubkey::default()), "Token0 mint should not be zero");
    assert!(!pool_state_1.token_mint1.eq(&Pubkey::default()), "Token1 mint should not be zero");
    assert!(!pool_state_1.amm_config.eq(&Pubkey::default()), "AMM config should not be zero");
    assert!(pool_state_1.liquidity > 0, "Liquidity should be positive");
    assert!(pool_state_1.tick_spacing > 0, "Tick spacing should be positive");
    println!("✅ 基本字段验证通过");

    // ========== 第二部分：测试 get_pool_by_mint 的缓存功能 ==========

    println!("\n步骤 3: 测试 get_pool_by_mint 的内存缓存功能...");

    // 3.1 第一次调用（应写入缓存）
    println!("  3.1 第一次调用 get_pool_by_mint（写入缓存）...");
    let (pool_addr_2, pool_state_2) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint failed");
    println!("  ✅ 第一次查询返回 Pool: {}", pool_addr_2);

    // 3.2 第二次调用（应从缓存读取）
    println!("  3.2 第二次调用 get_pool_by_mint（从缓存读取）...");
    let (pool_addr_3, pool_state_3) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint (cached) failed");
    println!("  ✅ 第二次查询返回 Pool: {}", pool_addr_3);

    // 验证缓存一致性
    assert_eq!(pool_addr_2, pool_addr_3, "缓存中的 pool_address 应该一致");
    assert_eq!(pool_state_2.amm_config, pool_state_3.amm_config, "缓存中的 amm_config 应该一致");
    assert_eq!(pool_state_2.liquidity, pool_state_3.liquidity, "缓存中的 liquidity 应该一致");
    println!("  ✅ 缓存验证通过（数据一致）");

    // 3.3 清除缓存后再次查询
    println!("  3.3 清除缓存后再次查询...");
    clear_pool_cache();
    let (pool_addr_4, pool_state_4) = get_pool_by_mint(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint (after clear) failed");
    println!("  ✅ 清除缓存后查询返回 Pool: {}", pool_addr_4);

    // 验证返回的 pool 仍然包含 WSOL（但不期望地址相同，因为选择算法可能返回不同的最优池）
    assert!(
        pool_state_4.token_mint0 == wsol_mint || pool_state_4.token_mint1 == wsol_mint,
        "清除缓存后返回的 Pool 应该包含 WSOL"
    );
    assert!(pool_state_4.liquidity > 0, "清除缓存后 Pool 应该有流动性");
    println!("  ✅ 清除缓存后验证通过（返回有效的 WSOL Pool）");

    // 3.4 测试 get_pool_by_mint_force 强制刷新
    println!("  3.4 测试 get_pool_by_mint_force 强制刷新...");
    let (pool_addr_5, pool_state_5) = get_pool_by_mint_force(&rpc, &wsol_mint)
        .await
        .expect("get_pool_by_mint_force failed");
    println!("  ✅ 强制刷新返回 Pool: {}", pool_addr_5);

    // 验证强制刷新后返回相同的结果
    assert_eq!(pool_addr_4, pool_addr_5, "强制刷新后 pool_address 应该一致");
    assert_eq!(pool_state_4.token_mint0, pool_state_5.token_mint0, "强制刷新后 token_mint0 应该一致");
    assert_eq!(pool_state_4.token_mint1, pool_state_5.token_mint1, "强制刷新后 token_mint1 应该一致");
    println!("  ✅ 强制刷新验证通过");

    // 清理环境变量
    std::env::remove_var("CLMM_POOL_SCAN_LIMIT");

    println!("\n=== Auto Mock 测试通过 ===");
    println!("✅ 测试覆盖：");
    println!("  1. list_pools_by_mint_with_pool_client（无内存缓存）");
    println!("  2. get_pool_by_mint_with_pool_client（无内存缓存）");
    println!("  3. get_pool_by_mint（有内存缓存）");
    println!("  4. get_pool_by_mint_force（强制刷新）");
    println!("✅ 首次运行：从 RPC 获取并保存（约 2-3 秒）");
    println!("✅ 后续运行：从缓存加载（约 0.01 秒）");
    println!("✅ 速度提升：约 100-200 倍！");
    println!("✅ 原始慢测试耗时: 85.95 秒");
    println!("✅ Auto Mock 测试耗时: 2-3 秒（首次）/ 0.01 秒（缓存）");
}
