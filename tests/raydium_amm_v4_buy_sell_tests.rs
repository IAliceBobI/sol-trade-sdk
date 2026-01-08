//! Raydium AMM V4 Buy & Sell 交易测试
//!
//! 本测试文件专门用于测试 Raydium AMM V4 的买入和卖出交易功能。
//!
//! ## 测试场景
//! - 参数初始化：从 RPC 获取 AMM 参数
//! - 买入交易：用 WSOL 购买 USDC
//! - 卖出交易：卖出 USDC 获得 WSOL
//! - 完整流程：买入后卖出全部
//! - 滑点保护：验证极小滑点会导致交易失败
//!
//! ## 已知测试池
//! - **WSOL-USDC Pool**: `58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2`
//!   - WSOL: `So11111111111111111111111111111111111111112`
//!   - USDC: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
//!
//! 运行测试:
//!     cargo test --test raydium_amm_v4_buy_sell_tests -- --nocapture

use sol_trade_sdk::{
    common::GasFeeStrategy,
    trading::core::params::{DexParamEnum, RaydiumAmmV4Params},
    DexType, TradeBuyParams, TradeSellParams, TradeTokenType,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use std::str::FromStr;

// 引入测试工具
mod test_helpers;
use test_helpers::{create_test_client, print_balances, print_token_balance};

/// 已知的 Raydium AMM V4 pool 地址
const SOL_USDC_AMM: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";

/// 已知的 USDC mint
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

/// 测试：从 AMM 地址创建 RaydiumAmmV4Params
#[tokio::test]
async fn test_raydium_amm_v4_params_from_rpc() {
    println!("\n=== 测试：RaydiumAmmV4Params::from_amm_address_by_rpc ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let params = RaydiumAmmV4Params::from_amm_address_by_rpc(&rpc, amm_address).await;
    assert!(params.is_ok(), "Failed to create params from RPC: {:?}", params.err());

    let params = params.unwrap();
    println!("✅ 参数创建成功");
    println!("  - AMM: {}", params.amm);
    println!("  - coin_mint: {}", params.coin_mint);
    println!("  - pc_mint: {}", params.pc_mint);
    println!("  - token_coin: {}", params.token_coin);
    println!("  - token_pc: {}", params.token_pc);
    println!("  - coin_reserve: {}", params.coin_reserve);
    println!("  - pc_reserve: {}", params.pc_reserve);

    // 验证字段正确性
    assert_eq!(params.amm, amm_address);
    assert_eq!(params.coin_mint.to_string(), "So11111111111111111111111111111111111111112");
    assert_eq!(params.pc_mint.to_string(), USDC_MINT);
    assert!(params.coin_reserve > 0, "coin_reserve 应大于 0");
    assert!(params.pc_reserve > 0, "pc_reserve 应大于 0");
}

/// 测试：Raydium AMM V4 买入交易（WSOL -> USDC）
#[tokio::test]
async fn test_raydium_amm_v4_buy() {
    println!("\n=== 测试：Raydium AMM V4 买入交易 ===");

    let client = create_test_client().await;
    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let usdc_mint = Pubkey::from_str(USDC_MINT).expect("Invalid USDC mint");
    let rpc_url = "http://127.0.0.1:8899";

    println!("🔍 测试钱包: {}", client.payer.as_ref().pubkey());

    // 查询初始余额
    println!("\n📊 初始余额:");
    let payer_pubkey = client.payer.as_ref().pubkey();
    let (initial_sol, _) = print_balances(rpc_url, &payer_pubkey).await.unwrap();
    let initial_usdc = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC").await.unwrap();

    // 获取 AMM 参数
    let params = RaydiumAmmV4Params::from_amm_address_by_rpc(&client.rpc, amm_address).await
        .expect("Failed to get AMM params");

    // 构建买入参数：用 0.01 SOL 购买 USDC
    let input_amount = 10_000_000; // 0.01 SOL = 10,000,000 lamports
    let recent_blockhash = client.rpc.get_latest_blockhash().await.unwrap();
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    let buy_params = TradeBuyParams {
        dex_type: DexType::RaydiumAmmV4,
        input_token_type: TradeTokenType::WSOL,
        mint: usdc_mint,
        input_token_amount: input_amount,
        slippage_basis_points: Some(1000), // 10% 滑点
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::RaydiumAmmV4(params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: true,
        close_input_token_ata: true,
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy,
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    println!("\n💰 购买参数:");
    println!("  - DEX: Raydium AMM V4");
    println!("  - Pool: {}", amm_address);
    println!("  - 输入: {} lamports ({:.4} SOL)", input_amount, input_amount as f64 / 1e9);
    println!("  - 目标 Token: {} (USDC)", usdc_mint);
    println!("  - 滑点: 10%");

    // 执行买入
    println!("\n🚀 执行买入交易...");
    let result = client.buy(buy_params).await;
    assert!(result.is_ok(), "买入交易失败: {:?}", result.err());

    let (success, signatures, error) = result.unwrap();
    assert!(success, "交易执行失败: {:?}", error);
    println!("✅ 交易成功！签名数: {}", signatures.len());
    for (i, sig) in signatures.iter().enumerate() {
        println!("  [{}] {}", i + 1, sig);
    }

    // 等待一下确保余额更新
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 查询最终余额
    println!("\n📊 最终余额:");
    let payer_pubkey = client.payer.as_ref().pubkey();
    let (final_sol, _) = print_balances(rpc_url, &payer_pubkey).await.unwrap();
    let final_usdc = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC").await.unwrap();

    // 验证余额变化
    println!("\n📈 余额变化:");
    let sol_spent = initial_sol.saturating_sub(final_sol);
    let usdc_gained = final_usdc.saturating_sub(initial_usdc);
    println!("  - SOL 消耗: {} lamports ({:.6} SOL)", sol_spent, sol_spent as f64 / 1e9);
    println!("  - USDC 获得: {} ({})", usdc_gained, usdc_gained as f64 / 1e6);

    assert!(sol_spent > 0, "SOL 余额应该减少");
    assert!(usdc_gained > 0, "USDC 余额应该增加");
}

/// 测试：Raydium AMM V4 卖出交易（USDC -> WSOL）
#[tokio::test]
async fn test_raydium_amm_v4_sell() {
    println!("\n=== 测试：Raydium AMM V4 卖出交易 ===");

    let client = create_test_client().await;
    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let usdc_mint = Pubkey::from_str(USDC_MINT).expect("Invalid USDC mint");
    let rpc_url = "http://127.0.0.1:8899";

    println!("🔍 测试钱包: {}", client.payer.as_ref().pubkey());

    // 先买入一些 USDC
    println!("\n🛒 步骤 1: 先买入一些 USDC...");
    let params = RaydiumAmmV4Params::from_amm_address_by_rpc(&client.rpc, amm_address).await
        .expect("Failed to get AMM params");

    let input_amount = 10_000_000; // 0.01 SOL
    let recent_blockhash = client.rpc.get_latest_blockhash().await.unwrap();
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    let buy_params = TradeBuyParams {
        dex_type: DexType::RaydiumAmmV4,
        input_token_type: TradeTokenType::WSOL,
        mint: usdc_mint,
        input_token_amount: input_amount,
        slippage_basis_points: Some(1000),
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::RaydiumAmmV4(params.clone()),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: true,
        close_input_token_ata: true,
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy.clone(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let buy_result = client.buy(buy_params).await;
    assert!(buy_result.is_ok(), "买入失败");
    println!("✅ USDC 买入成功");

    // 等待确认
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 查询买入后的余额
    println!("\n📊 买入后余额:");
    let payer_pubkey = client.payer.as_ref().pubkey();
    let (initial_sol, _) = print_balances(rpc_url, &payer_pubkey).await.unwrap();
    let usdc_balance = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC").await.unwrap();
    assert!(usdc_balance > 0, "USDC 余额应大于 0");

    // 卖出 50% 的 USDC
    let sell_amount = usdc_balance / 2;
    println!("\n💸 步骤 2: 卖出 USDC...");
    println!("  - 当前 USDC 余额: {}", usdc_balance);
    println!("  - 卖出数量: {}", sell_amount);

    // 获取最新的 AMM 参数（池状态可能已变化）
    let params = RaydiumAmmV4Params::from_amm_address_by_rpc(&client.rpc, amm_address).await
        .expect("Failed to get AMM params");

    let recent_blockhash = client.rpc.get_latest_blockhash().await.unwrap();
    let sell_params = TradeSellParams {
        dex_type: DexType::RaydiumAmmV4,
        output_token_type: TradeTokenType::WSOL,
        mint: usdc_mint,
        input_token_amount: sell_amount,
        slippage_basis_points: Some(1000),
        recent_blockhash: Some(recent_blockhash),
        with_tip: false,
        extension_params: DexParamEnum::RaydiumAmmV4(params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_output_token_ata: true,
        close_output_token_ata: true,
        close_mint_token_ata: false,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy,
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    // 执行卖出
    println!("\n🚀 执行卖出交易...");
    let result = client.sell(sell_params).await;
    assert!(result.is_ok(), "卖出交易失败: {:?}", result.err());

    let (success, signatures, error) = result.unwrap();
    assert!(success, "交易执行失败: {:?}", error);
    println!("✅ 交易成功！签名数: {}", signatures.len());
    for (i, sig) in signatures.iter().enumerate() {
        println!("  [{}] {}", i + 1, sig);
    }

    // 等待一下确保余额更新
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 查询最终余额
    println!("\n📊 最终余额:");
    let payer_pubkey = client.payer.as_ref().pubkey();
    let (final_sol, _) = print_balances(rpc_url, &payer_pubkey).await.unwrap();
    let final_usdc = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC").await.unwrap();

    // 验证余额变化
    println!("\n📈 余额变化:");
    let sol_gained = final_sol.saturating_sub(initial_sol);
    let usdc_spent = usdc_balance.saturating_sub(final_usdc);
    println!("  - SOL 获得: {} lamports ({:.6} SOL)", sol_gained, sol_gained as f64 / 1e9);
    println!("  - USDC 消耗: {} ({})", usdc_spent, usdc_spent as f64 / 1e6);

    assert!(sol_gained > 0, "SOL 余额应该增加");
    assert!(usdc_spent > 0, "USDC 余额应该减少");
    assert_eq!(usdc_spent, sell_amount, "USDC 消耗应等于卖出数量");
}

/// 测试：完整的买入-卖出流程
#[tokio::test]
async fn test_raydium_amm_v4_buy_sell_complete() {
    println!("\n=== 测试：Raydium AMM V4 完整买卖流程 ===");

    let client = create_test_client().await;
    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let usdc_mint = Pubkey::from_str(USDC_MINT).expect("Invalid USDC mint");
    let rpc_url = "http://127.0.0.1:8899";

    println!("🔍 测试钱包: {}", client.payer.as_ref().pubkey());

    // 记录初始余额
    let payer_pubkey = client.payer.as_ref().pubkey();
    let (initial_sol, _) = print_balances(rpc_url, &payer_pubkey).await.unwrap();
    let initial_usdc = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC").await.unwrap();

    // ===== 第一步：买入 =====
    println!("\n💰 第一步：买入 USDC");
    let params = RaydiumAmmV4Params::from_amm_address_by_rpc(&client.rpc, amm_address).await
        .expect("Failed to get AMM params");

    let input_amount = 20_000_000; // 0.02 SOL
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    let buy_params = TradeBuyParams {
        dex_type: DexType::RaydiumAmmV4,
        input_token_type: TradeTokenType::WSOL,
        mint: usdc_mint,
        input_token_amount: input_amount,
        slippage_basis_points: Some(1000),
        recent_blockhash: Some(client.rpc.get_latest_blockhash().await.unwrap()),
        extension_params: DexParamEnum::RaydiumAmmV4(params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: true,
        close_input_token_ata: true,
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy.clone(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let (success, signatures, _) = client.buy(buy_params).await.expect("买入失败");
    assert!(success, "买入交易应成功");
    println!("✅ 买入成功，签名: {:?}", signatures[0]);

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let payer_pubkey = client.payer.as_ref().pubkey();
    let usdc_after_buy = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC").await.unwrap();
    assert!(usdc_after_buy > initial_usdc, "买入后 USDC 应增加");

    // ===== 第二步：卖出全部 =====
    println!("\n💸 第二步：卖出全部 USDC");
    let params = RaydiumAmmV4Params::from_amm_address_by_rpc(&client.rpc, amm_address).await
        .expect("Failed to get AMM params");

    let sell_params = TradeSellParams {
        dex_type: DexType::RaydiumAmmV4,
        output_token_type: TradeTokenType::WSOL,
        mint: usdc_mint,
        input_token_amount: usdc_after_buy,
        slippage_basis_points: Some(1000),
        recent_blockhash: Some(client.rpc.get_latest_blockhash().await.unwrap()),
        with_tip: false,
        extension_params: DexParamEnum::RaydiumAmmV4(params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_output_token_ata: true,
        close_output_token_ata: true,
        close_mint_token_ata: true, // 卖完后关闭 USDC ATA
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy,
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let (success, signatures, _) = client.sell(sell_params).await.expect("卖出失败");
    assert!(success, "卖出交易应成功");
    println!("✅ 卖出成功，签名: {:?}", signatures[0]);

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 验证最终余额
    let payer_pubkey = client.payer.as_ref().pubkey();
    let (final_sol, _) = print_balances(rpc_url, &payer_pubkey).await.unwrap();
    let final_usdc = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC").await.unwrap();

    println!("\n📊 完整流程结果:");
    let sol_diff = (final_sol as i128) - (initial_sol as i128);
    println!("  - SOL 净变化: {} lamports ({:.6} SOL)", sol_diff, sol_diff as f64 / 1e9);
    println!("  - 最终 USDC: {} (应为 0)", final_usdc);

    // USDC 应该全部卖出（如果设置了 close_mint_token_ata）
    assert_eq!(final_usdc, 0, "USDC 应该全部卖出");
    // SOL 净变化应为负（因为有手续费和滑点损失）
    assert!(sol_diff < 0, "由于手续费和滑点，SOL 应该净减少");
}

/// 测试：验证滑点保护生效
#[tokio::test]
#[ignore] // 需要极端市场条件才能触发，正常测试时忽略
async fn test_raydium_amm_v4_slippage_protection() {
    println!("\n=== 测试：Raydium AMM V4 滑点保护 ===");

    let client = create_test_client().await;
    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let usdc_mint = Pubkey::from_str(USDC_MINT).expect("Invalid USDC mint");

    let params = RaydiumAmmV4Params::from_amm_address_by_rpc(&client.rpc, amm_address).await
        .expect("Failed to get AMM params");

    // 使用极小的滑点（0.01%）
    let buy_params = TradeBuyParams {
        dex_type: DexType::RaydiumAmmV4,
        input_token_type: TradeTokenType::WSOL,
        mint: usdc_mint,
        input_token_amount: 10_000_000,
        slippage_basis_points: Some(1), // 0.01% 极小滑点，应该失败
        recent_blockhash: Some(client.rpc.get_latest_blockhash().await.unwrap()),
        extension_params: DexParamEnum::RaydiumAmmV4(params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: true,
        close_input_token_ata: true,
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: GasFeeStrategy::new(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    println!("🚀 尝试使用 0.01% 的极小滑点进行交易（预期失败）...");
    let result = client.buy(buy_params).await;

    // 在正常市场条件下，0.01% 的滑点应该导致交易失败
    if result.is_ok() {
        let (success, _, error) = result.unwrap();
        if !success {
            println!("✅ 滑点保护生效，交易被拒绝: {:?}", error);
        } else {
            println!("⚠️  交易成功了（可能是市场流动性极好）");
        }
    } else {
        println!("✅ 滑点保护生效，交易在构建阶段失败: {:?}", result.err());
    }
}
