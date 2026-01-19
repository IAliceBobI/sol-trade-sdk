//! Raydium CPMM Buy & Sell 集成测试
//!
//! 本测试文件基于文档 `docs/raydium-cpmm-pool-lookup.md` 的设计，验证：
//! - 使用 `get_pool_by_mint` 基于 WSOL mint 查找 CPMM Pool
//! - 基于 PoolState 构建 `RaydiumCpmmParams`
//! - 通过 `SolanaTrade` 执行一条完整的 Raydium CPMM 买入 -> 卖出交易流程
//!
//! 测试假设：
//! - 本地 RPC `http://127.0.0.1:8899` 已接入主网数据（例如使用 surfpool）
//! - Raydium CPMM 协议已在该 RPC 上可用
//! - 存在至少一个包含 WSOL 的 Raydium CPMM 池
//!
//! 运行测试:
//!     cargo test --test raydium_cpmm_buy_sell_tests -- --nocapture

use serial_test::serial;
use sol_trade_sdk::{
    common::GasFeeStrategy,
    instruction::utils::raydium_cpmm::{
        clear_pool_cache, get_pool_by_address, get_pool_by_mint, get_pool_by_mint_force, list_pools_by_mint,
        get_token_price_in_usd_with_pool,
    },
    trading::core::params::{DexParamEnum, RaydiumCpmmParams},
    DexType, TradeBuyParams, TradeSellParams, TradeTokenType,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;

mod test_helpers;
use test_helpers::{create_test_client, print_balances, print_token_balance};

/// 已知的 WSOL mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// PIPE Token Mint
const PIPE_MINT: &str = "8ycz3kctoRb4LFrtoYG2r8tRyUYUeGf5Q16M2TEMp7A";

/// PIPE Token CPMM Pool
const PIPE_POOL: &str = "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp";

/// 测试：Raydium CPMM 完整买入-卖出流程
///
/// 流程：
/// 1. 通过 `get_pool_by_mint` 基于 WSOL mint 查找一个 CPMM 池
/// 2. 选择该池中非 WSOL 的另一侧 Token 作为目标代币
/// 3. 使用 SOL 买入目标代币
/// 4. 再将全部目标代币卖出换回 SOL
/// 5. 验证 Token 余额变化和 SOL 净变化
#[tokio::test]
async fn test_raydium_cpmm_buy_sell_complete() {
    println!("\n=== 测试：Raydium CPMM 完整买卖流程 ===");

    let client = create_test_client().await;
    let rpc_url = "http://127.0.0.1:8899";

    let payer_pubkey = client.payer.as_ref().pubkey();
    println!("测试钱包: {}", payer_pubkey);

    // 记录初始 SOL 余额
    let (initial_sol, _) =
        print_balances(rpc_url, &payer_pubkey).await.expect("Failed to fetch initial balances");

    // ===== 1. 使用指定的 CPMM Pool (PIPE-WSOL) =====
    let pool_address = Pubkey::from_str("BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp")
        .expect("Invalid pool address");
    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");

    println!("\n🔍 使用指定的 Raydium CPMM Pool: {}", pool_address);

    // 从 pool 地址获取 pool state
    let pool_state = get_pool_by_address(&client.rpc, &pool_address)
        .await
        .expect("Failed to get CPMM pool state");

    println!("Pool 信息:");
    println!("  token0_mint: {}", pool_state.token0_mint);
    println!("  token1_mint: {}", pool_state.token1_mint);

    // 验证池确实包含 WSOL
    assert!(
        pool_state.token0_mint == wsol_mint || pool_state.token1_mint == wsol_mint,
        "CPMM Pool 必须包含 WSOL",
    );

    // 选择目标代币：池中非 WSOL 的那一侧
    let target_mint = if pool_state.token0_mint == wsol_mint {
        pool_state.token1_mint
    } else {
        pool_state.token0_mint
    };

    println!("目标交易 Token Mint: {}", target_mint);

    // 记录初始目标代币余额
    let initial_token_balance = print_token_balance(rpc_url, &payer_pubkey, &target_mint, "Target")
        .await
        .expect("Failed to fetch initial token balance");

    // ===== 2. 从 Pool 地址构建 RaydiumCpmmParams =====
    println!("\n🧮 从 Pool 构建 RaydiumCpmmParams...");
    let cpmm_params = RaydiumCpmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
        .await
        .expect("Failed to build RaydiumCpmmParams from pool address");

    // ===== 3. 使用 SOL 买入目标代币 =====
    println!("\n💰 第一步：买入目标代币 (Raydium CPMM)");

    let input_amount = 20_000_000u64; // 0.02 SOL
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    let recent_blockhash =
        client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

    let buy_params = TradeBuyParams {
        dex_type: DexType::RaydiumCpmm,
        input_token_type: TradeTokenType::SOL,
        mint: target_mint,
        input_token_amount: input_amount,
        slippage_basis_points: Some(10000), // 10% 容忍度，避免因滑点导致测试偶发失败
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::RaydiumCpmm(cpmm_params.clone()),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: true,
        close_input_token_ata: false,
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy.clone(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let (success_buy, buy_sigs, _error_buy) =
        client.buy(buy_params).await.expect("Raydium CPMM 买入交易执行失败");
    assert!(success_buy, "买入交易应成功");
    println!("✅ 买入成功，签名: {:?}", buy_sigs.get(0));

    // 买入后的代币余额
    let token_after_buy = print_token_balance(rpc_url, &payer_pubkey, &target_mint, "Target")
        .await
        .expect("Failed to fetch token balance after buy");
    assert!(token_after_buy > initial_token_balance, "买入后目标代币余额应增加",);

    // ===== 4. 卖出全部目标代币换回 SOL =====
    println!("\n💸 第二步：卖出全部目标代币 (Raydium CPMM)");

    let cpmm_params_sell = RaydiumCpmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
        .await
        .expect("Failed to build RaydiumCpmmParams for sell");

    let recent_blockhash_sell =
        client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash for sell");

    let sell_params = TradeSellParams {
        dex_type: DexType::RaydiumCpmm,
        output_token_type: TradeTokenType::SOL,
        mint: target_mint,
        input_token_amount: token_after_buy,
        slippage_basis_points: Some(10000),
        recent_blockhash: Some(recent_blockhash_sell),
        with_tip: false,
        extension_params: DexParamEnum::RaydiumCpmm(cpmm_params_sell),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_output_token_ata: true,
        close_output_token_ata: false,
        close_mint_token_ata: false,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy,
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let (success_sell, sell_sigs, _error_sell) =
        client.sell(sell_params).await.expect("Raydium CPMM 卖出交易执行失败");
    assert!(success_sell, "卖出交易应成功");
    println!("✅ 卖出成功，签名: {:?}", sell_sigs.get(0));

    // 等待链上状态更新
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // ===== 5. 验证最终余额 =====
    let (final_sol, _) =
        print_balances(rpc_url, &payer_pubkey).await.expect("Failed to fetch final balances");
    let final_token_balance = print_token_balance(rpc_url, &payer_pubkey, &target_mint, "Target")
        .await
        .expect("Failed to fetch final token balance");

    println!("\n📊 完整流程结果:");
    let sol_diff = (final_sol as i128) - (initial_sol as i128);
    println!("  - SOL 净变化: {} lamports ({:.6} SOL)", sol_diff, sol_diff as f64 / 1e9);
    println!("  - 最终目标代币余额: {}", final_token_balance);

    // 目标代币应基本被卖出（可能存在极小 dust，但在典型场景下应为 0）
    assert_eq!(final_token_balance, 0, "卖出后目标代币余额应为 0");
    // 由于手续费和滑点，SOL 净变化应为负
    assert!(sol_diff < 0, "由于手续费和滑点，SOL 应该净减少");

    println!("\n=== Raydium CPMM 买入-卖出完整流程测试通过 ===");
}

/// 测试：基于 WSOL mint 查找 CPMM Pool，并验证缓存与强制刷新
///
/// 步骤：
/// 1. 清空 CPMM 缓存
/// 2. 使用 `get_pool_by_mint` 基于 WSOL mint 查找 Pool（应从链上扫描）
/// 3. 再次调用 `get_pool_by_mint`（应命中缓存，结果相同）
/// 4. 使用 `get_pool_by_mint_force` 强制刷新（结果通常相同）
#[tokio::test]
#[serial]
async fn test_raydium_cpmm_get_pool_by_mint_wsol_cache_and_force() {
    println!("=== 测试：Raydium CPMM get_pool_by_mint (WSOL, cache & force) ===");

    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 1. 清空缓存，确保从干净状态开始
    clear_pool_cache();

    // 2. 第一次查询：应从链上扫描并选择一个包含 WSOL 的池
    let (pool_addr_1, pool_state_1) =
        get_pool_by_mint(&rpc, &wsol_mint).await.expect("get_pool_by_mint failed");
    println!("第一次查询到的 Pool: {}", pool_addr_1);
    println!("  token0_mint: {}", pool_state_1.token0_mint);
    println!("  token1_mint: {}", pool_state_1.token1_mint);

    assert!(
        pool_state_1.token0_mint == wsol_mint || pool_state_1.token1_mint == wsol_mint,
        "返回的 CPMM Pool 不包含 WSOL",
    );

    // 3. 第二次查询：应命中缓存，返回相同的池地址
    let (pool_addr_2, pool_state_2) =
        get_pool_by_mint(&rpc, &wsol_mint).await.expect("get_pool_by_mint (cached) failed");
    assert_eq!(pool_addr_1, pool_addr_2, "缓存中的 pool_address 不一致");
    assert_eq!(pool_state_1.amm_config, pool_state_2.amm_config, "缓存中的 PoolState 不一致");

    // 4. 强制刷新：删除缓存后重新查询
    let (pool_addr_3, pool_state_3) =
        get_pool_by_mint_force(&rpc, &wsol_mint).await.expect("get_pool_by_mint_force failed");
    println!("强制刷新后的 Pool: {}", pool_addr_3);

    // 通常情况下，强制刷新前后返回的主池应相同（除非链上配置发生结构性变化）
    assert_eq!(pool_addr_2, pool_addr_3, "强制刷新后 pool_address 发生变化");
    assert_eq!(pool_state_2.token0_mint, pool_state_3.token0_mint, "强制刷新后 token0_mint 不一致");
    assert_eq!(pool_state_2.token1_mint, pool_state_3.token1_mint, "强制刷新后 token1_mint 不一致");
}

/// 测试：列出所有包含 WSOL 的 Raydium CPMM Pool
///
/// 使用 `list_pools_by_mint`，验证：
/// - 返回列表非空
/// - 所有池的 `token0_mint` 或 `token1_mint` 中至少一侧为 WSOL
#[tokio::test]
async fn test_raydium_cpmm_list_pools_by_mint_wsol() {
    println!("=== 测试：Raydium CPMM list_pools_by_mint (WSOL) ===");

    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = RpcClient::new(rpc_url.to_string());

    let pools = list_pools_by_mint(&rpc, &wsol_mint).await.expect("list_pools_by_mint failed");

    assert!(!pools.is_empty(), "WSOL 相关的 CPMM Pool 列表不应为空");

    for (addr, pool) in pools.iter() {
        println!(
            "WSOL CPMM Pool: {} (token0_mint={}, token1_mint={})",
            addr, pool.token0_mint, pool.token1_mint
        );
        assert!(
            pool.token0_mint == wsol_mint || pool.token1_mint == wsol_mint,
            "CPMM Pool {} 不包含 WSOL",
            addr,
        );
    }
}

/// 测试：获取 CPMM token 的 USD 价格
#[tokio::test]
async fn test_get_cpmm_token_price_in_usd() {
    println!("=== 测试：获取 CPMM token 的 USD 价格 ===");

    let token_mint = Pubkey::from_str(PIPE_MINT).unwrap();
    let pool_address = Pubkey::from_str(PIPE_POOL).unwrap();
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    println!("Token Mint: {}", token_mint);
    println!("Pool 地址: {}", pool_address);
    println!("WSOL-USDT 锚定池: 使用默认锚定池");

    // 调用价格计算函数
    let result = get_token_price_in_usd_with_pool(&rpc, &token_mint, &pool_address, None).await;

    // 验证结果
    assert!(result.is_ok(), "Failed to get token price in USD: {:?}", result.err());

    let price_usd = result.unwrap();
    println!("✅ Token USD 价格: ${:.8}", price_usd);

    // 验证价格合理性
    assert!(price_usd > 0.0, "Price should be positive");
    assert!(price_usd < 1000.0, "Price should be reasonable (< $1000)");
    println!("✅ 价格范围验证通过");
}
