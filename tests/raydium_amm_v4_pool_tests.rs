//! Raydium AMM V4 Pool 查找集成测试
//!
//! 测试 pool 查找方法：
//! - fetch_amm_info(rpc, amm) - 获取 AMM 信息
//!
//! 运行测试:
//!     cargo test --test raydium_amm_v4_pool_tests -- --nocapture
//!
//! 注意：使用 surfpool (localhost:8899) 进行测试

use sol_trade_sdk::instruction::utils::raydium_amm_v4::fetch_amm_info;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// 已知的 Raydium AMM V4 pool 地址
/// SOL/USDC pool on Raydium AMM V4
const SOL_USDC_AMM: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";

/// 测试：获取 AMM 信息
#[tokio::test]
async fn test_fetch_amm_info() {
    println!("=== 测试：获取 AMM 信息 ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    println!("获取 AMM 信息: {}", amm_address);
    let result = fetch_amm_info(&rpc, amm_address).await;

    assert!(result.is_ok(), "Failed to fetch AMM info: {:?}", result.err());

    let amm_info = result.unwrap();
    println!("✅ AMM Info 获取成功!");
    println!("  AMM Address: {}", amm_address);
    println!("  Status: {}", amm_info.status);
    println!("  Coin Mint: {}", amm_info.coin_mint);
    println!("  PC Mint: {}", amm_info.pc_mint);
    println!("  LP Mint: {}", amm_info.lp_mint);
    println!("  Open Orders: {}", amm_info.open_orders);
    println!("  Target Orders: {}", amm_info.target_orders);
    println!("  Coin Vault: {}", amm_info.coin_vault);
    println!("  PC Vault: {}", amm_info.pc_vault);
    println!("  Vault Signer: {}", amm_info.vault_signer);
    println!("  Coin Decimals: {}", amm_info.coin_decimals);
    println!("  PC Decimals: {}", amm_info.pc_decimals);
    println!("  LP Decimals: {}", amm_info.lp_decimals);
    println!("  Pool Token Mint: {}", amm_info.pool_token_mint);
    println!("  Pool Token Decimals: {}", amm_info.pool_token_decimals);
    println!("  Min Order Size: {}", amm_info.min_order_size);
    println!("  Min Tick Size: {}", amm_info.min_tick_size);
    println!("  Swap Fee Numerator: {}", amm_info.swap_fee_numerator);
    println!("  Swap Fee Denominator: {}", amm_info.swap_fee_denominator);
    println!("  Out Swap Fee Numerator: {}", amm_info.out_swap_fee_numerator);
    println!("  Out Swap Fee Denominator: {}", amm_info.out_swap_fee_denominator);
    println!("  Wallet Key: {}", amm_info.wallet_key);
    println!("  Authority: {}", amm_info.authority);
    println!("  Serum Program: {}", amm_info.serum_program);
    println!("  Serum Market: {}", amm_info.serum_market);
    println!("  Need Take Pnl: {}", amm_info.need_take_pnl);
    println!("  Total Pnl: {}", amm_info.total_pnl);
    println!("  Pool Open Time: {}", amm_info.pool_open_time);
    println!("  Recent Epoch: {}", amm_info.recent_epoch);
    println!("  Prix: {}", amm_info.prix);
    println!("  Prix Multiplier: {}", amm_info.prix_multiplier);

    // 验证基本字段约束
    assert!(!amm_info.coin_mint.eq(&Pubkey::default()), "Coin mint should not be zero");
    assert!(!amm_info.pc_mint.eq(&Pubkey::default()), "PC mint should not be zero");
    assert!(!amm_info.lp_mint.eq(&Pubkey::default()), "LP mint should not be zero");
    assert!(!amm_info.coin_vault.eq(&Pubkey::default()), "Coin vault should not be zero");
    assert!(!amm_info.pc_vault.eq(&Pubkey::default()), "PC vault should not be zero");
    assert!(!amm_info.vault_signer.eq(&Pubkey::default()), "Vault signer should not be zero");
    assert!(amm_info.coin_decimals > 0, "Coin decimals should be positive");
    assert!(amm_info.pc_decimals > 0, "PC decimals should be positive");
    assert!(amm_info.lp_decimals > 0, "LP decimals should be positive");
    assert!(amm_info.swap_fee_numerator > 0, "Swap fee numerator should be positive");
    assert!(amm_info.swap_fee_denominator > 0, "Swap fee denominator should be positive");
    println!("✅ 基本字段验证通过");
}

/// 测试：验证 AMM 状态
#[tokio::test]
async fn test_amm_status() {
    println!("=== 测试：验证 AMM 状态 ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let amm_info = fetch_amm_info(&rpc, amm_address).await.unwrap();

    println!("AMM 状态: {}", amm_info.status);
    println!("Pool Open Time: {}", amm_info.pool_open_time);
    println!("Recent Epoch: {}", amm_info.recent_epoch);

    // 验证状态
    assert!(amm_info.status >= 0, "Status should be valid");
    assert!(amm_info.pool_open_time > 0, "Pool open time should be positive");
    assert!(amm_info.recent_epoch >= 0, "Recent epoch should be valid");
    println!("✅ AMM 状态验证通过");
}

/// 测试：验证费用配置
#[tokio::test]
async fn test_amm_fee_config() {
    println!("=== 测试：验证费用配置 ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let amm_info = fetch_amm_info(&rpc, amm_address).await.unwrap();

    println!("Swap Fee: {}/{} ({:.4}%)",
        amm_info.swap_fee_numerator,
        amm_info.swap_fee_denominator,
        (amm_info.swap_fee_numerator as f64 / amm_info.swap_fee_denominator as f64) * 100.0
    );

    println!("Out Swap Fee: {}/{} ({:.4}%)",
        amm_info.out_swap_fee_numerator,
        amm_info.out_swap_fee_denominator,
        (amm_info.out_swap_fee_numerator as f64 / amm_info.out_swap_fee_denominator as f64) * 100.0
    );

    // 验证费用配置
    assert!(amm_info.swap_fee_numerator > 0, "Swap fee numerator should be positive");
    assert!(amm_info.swap_fee_denominator > 0, "Swap fee denominator should be positive");
    assert!(
        amm_info.swap_fee_numerator < amm_info.swap_fee_denominator,
        "Swap fee should be less than 100%"
    );

    assert!(amm_info.out_swap_fee_numerator > 0, "Out swap fee numerator should be positive");
    assert!(amm_info.out_swap_fee_denominator > 0, "Out swap fee denominator should be positive");
    assert!(
        amm_info.out_swap_fee_numerator < amm_info.out_swap_fee_denominator,
        "Out swap fee should be less than 100%"
    );

    println!("✅ 费用配置验证通过");
}

/// 测试：验证最小订单配置
#[tokio::test]
async fn test_amm_min_order_config() {
    println!("=== 测试：验证最小订单配置 ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let amm_info = fetch_amm_info(&rpc, amm_address).await.unwrap();

    println!("Min Order Size: {}", amm_info.min_order_size);
    println!("Min Tick Size: {}", amm_info.min_tick_size);

    // 验证最小订单配置
    assert!(amm_info.min_order_size > 0, "Min order size should be positive");
    assert!(amm_info.min_tick_size > 0, "Min tick size should be positive");
    println!("✅ 最小订单配置验证通过");
}

/// 测试：验证 Serum 市场配置
#[tokio::test]
async fn test_amm_serum_config() {
    println!("=== 测试：验证 Serum 市场配置 ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let amm_info = fetch_amm_info(&rpc, amm_address).await.unwrap();

    println!("Serum Program: {}", amm_info.serum_program);
    println!("Serum Market: {}", amm_info.serum_market);
    println!("Open Orders: {}", amm_info.open_orders);
    println!("Target Orders: {}", amm_info.target_orders);

    // 验证 Serum 配置
    assert!(!amm_info.serum_program.eq(&Pubkey::default()), "Serum program should not be zero");
    assert!(!amm_info.serum_market.eq(&Pubkey::default()), "Serum market should not be zero");
    assert!(!amm_info.open_orders.eq(&Pubkey::default()), "Open orders should not be zero");
    assert!(!amm_info.target_orders.eq(&Pubkey::default()), "Target orders should not be zero");
    println!("✅ Serum 市场配置验证通过");
}

/// 测试：验证 PNL 配置
#[tokio::test]
async fn test_amm_pnl_config() {
    println!("=== 测试：验证 PNL 配置 ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let amm_info = fetch_amm_info(&rpc, amm_address).await.unwrap();

    println!("Need Take Pnl: {}", amm_info.need_take_pnl);
    println!("Total Pnl: {}", amm_info.total_pnl);

    // 验证 PNL 配置
    assert!(amm_info.total_pnl >= 0, "Total PNL should be non-negative");
    println!("✅ PNL 配置验证通过");
}

/// 测试：验证价格相关配置
#[tokio::test]
async fn test_amm_price_config() {
    println!("=== 测试：验证价格相关配置 ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let amm_info = fetch_amm_info(&rpc, amm_address).await.unwrap();

    println!("Prix: {}", amm_info.prix);
    println!("Prix Multiplier: {}", amm_info.prix_multiplier);

    // 验证价格配置
    assert!(amm_info.prix > 0, "Prix should be positive");
    assert!(amm_info.prix_multiplier > 0, "Prix multiplier should be positive");
    println!("✅ 价格相关配置验证通过");
}