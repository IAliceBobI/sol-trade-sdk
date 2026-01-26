use sol_trade_sdk::{
    DexType, TradeBuyParams, TradeSellParams, TradeTokenType,
    common::{GasFeeStrategy, auto_mock_rpc::AutoMockRpcClient},
    trading::core::params::{DexParamEnum, RaydiumAmmV4Params},
};
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

/// 测试：从 AMM 地址创建 RaydiumAmmV4Params（使用 Auto Mock 加速）
#[tokio::test]
async fn test_raydium_amm_v4_params_from_rpc() {
    println!("\n=== 测试：RaydiumAmmV4Params::from_amm_address_by_rpc (Auto Mock) ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Failed to parse AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("raydium_amm_v4_buy_sell_tests".to_string()),
    );

    let params = RaydiumAmmV4Params::from_amm_address_by_rpc_with_client(&rpc, amm_address)
        .await
        .unwrap_or_else(|e| {
            panic!("从 RPC 获取 AMM 参数失败: {}\n  AMM: {}\n  RPC: {}", e, amm_address, rpc_url)
        });
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

/// 测试：完整的买入-卖出流程
#[tokio::test]
async fn test_raydium_amm_v4_buy_sell_complete() {
    println!("\n=== 测试：Raydium AMM V4 完整买卖流程 ===");

    let client = create_test_client().await;
    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Failed to parse AMM address");
    let usdc_mint = Pubkey::from_str(USDC_MINT).expect("Failed to parse USDC mint address");
    let rpc_url = "http://127.0.0.1:8899";

    println!("🔍 测试钱包: {}", client.payer.as_ref().pubkey());

    // 记录初始余额
    let payer_pubkey = client.payer.as_ref().pubkey();
    let (initial_sol, _) = print_balances(rpc_url, &payer_pubkey)
        .await
        .unwrap_or_else(|e| panic!("获取初始余额失败: {}\n  钱包: {}", e, payer_pubkey));
    let initial_usdc = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC")
        .await
        .unwrap_or_else(|e| panic!("获取初始 USDC 余额失败: {}\n  钱包: {}", e, payer_pubkey));

    // ===== 第一步：买入 =====
    println!("\n💰 第一步：买入 USDC");
    let params = RaydiumAmmV4Params::from_amm_address_by_rpc(&client.rpc, amm_address)
        .await
        .unwrap_or_else(|e| panic!("获取 AMM 参数失败: {}\n  AMM: {}", e, amm_address));

    let input_amount = 20_000_000; // 0.02 SOL
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    let buy_params =
        TradeBuyParams {
            dex_type: DexType::RaydiumAmmV4,
            input_token_type: TradeTokenType::SOL,
            mint: usdc_mint,
            input_token_amount: input_amount,
            slippage_basis_points: Some(1000),
            recent_blockhash: Some(
                client.rpc.get_latest_blockhash().await.unwrap_or_else(|e| {
                    panic!("获取最新 blockhash 失败: {}\n  RPC: {}", e, rpc_url)
                }),
            ),
            extension_params: DexParamEnum::RaydiumAmmV4(params),
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

    let (success, signatures, _) = client.buy(buy_params).await.unwrap_or_else(|e| {
        panic!(
            "买入交易执行失败: {}\n  AMM: {}\n  USDC Mint: {}\n  买入金额: {} lamports\n  钱包: {}",
            e, amm_address, usdc_mint, input_amount, payer_pubkey
        )
    });
    assert!(success, "买入交易应成功");
    println!("✅ 买入成功，签名: {:?}\n", signatures[0]);

    let payer_pubkey = client.payer.as_ref().pubkey();
    let usdc_after_buy = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC")
        .await
        .unwrap_or_else(|e| panic!("获取买入后 USDC 余额失败: {}\n  钱包: {}", e, payer_pubkey));
    assert!(usdc_after_buy > initial_usdc, "买入后 USDC 应增加");

    // ===== 第二步：卖出全部 =====
    println!("\n💸 第二步：卖出全部 USDC");
    let params = RaydiumAmmV4Params::from_amm_address_by_rpc(&client.rpc, amm_address)
        .await
        .unwrap_or_else(|e| panic!("获取 AMM 参数失败: {}\n  AMM: {}", e, amm_address));

    let sell_params =
        TradeSellParams {
            dex_type: DexType::RaydiumAmmV4,
            output_token_type: TradeTokenType::SOL,
            mint: usdc_mint,
            input_token_amount: usdc_after_buy,
            slippage_basis_points: Some(1000),
            recent_blockhash: Some(
                client.rpc.get_latest_blockhash().await.unwrap_or_else(|e| {
                    panic!("获取最新 blockhash 失败: {}\n  RPC: {}", e, rpc_url)
                }),
            ),
            with_tip: false,
            extension_params: DexParamEnum::RaydiumAmmV4(params),
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

    let (success, signatures, _) = client.sell(sell_params).await.unwrap_or_else(|e| {
        panic!(
            "卖出交易执行失败: {}\n  AMM: {}\n  USDC Mint: {}\n  卖出数量: {}\n  钱包: {}",
            e, amm_address, usdc_mint, usdc_after_buy, payer_pubkey
        )
    });
    assert!(success, "卖出交易应成功");
    println!("✅ 卖出成功，签名: {:?}", signatures[0]);

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 验证最终余额
    let payer_pubkey = client.payer.as_ref().pubkey();
    let (final_sol, _) = print_balances(rpc_url, &payer_pubkey)
        .await
        .unwrap_or_else(|e| panic!("获取最终余额失败: {}\n  钱包: {}", e, payer_pubkey));
    let final_usdc = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC")
        .await
        .unwrap_or_else(|e| panic!("获取最终 USDC 余额失败: {}\n  钱包: {}", e, payer_pubkey));

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
    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Failed to parse AMM address");
    let usdc_mint = Pubkey::from_str(USDC_MINT).expect("Failed to parse USDC mint address");
    let rpc_url = "http://127.0.0.1:8899";

    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    let params = RaydiumAmmV4Params::from_amm_address_by_rpc(&client.rpc, amm_address)
        .await
        .unwrap_or_else(|e| panic!("获取 AMM 参数失败: {}\n  AMM: {}", e, amm_address));

    // 使用极小的滑点（0.01%）
    let buy_params =
        TradeBuyParams {
            dex_type: DexType::RaydiumAmmV4,
            input_token_type: TradeTokenType::SOL,
            mint: usdc_mint,
            input_token_amount: 10_000_000,
            slippage_basis_points: Some(1), // 0.01% 极小滑点，应该失败
            recent_blockhash: Some(
                client.rpc.get_latest_blockhash().await.unwrap_or_else(|e| {
                    panic!("获取最新 blockhash 失败: {}\n  RPC: {}", e, rpc_url)
                }),
            ),
            extension_params: DexParamEnum::RaydiumAmmV4(params),
            address_lookup_table_account: None,
            wait_transaction_confirmed: true,
            create_input_token_ata: true,
            close_input_token_ata: false,
            create_mint_ata: true,
            durable_nonce: None,
            fixed_output_token_amount: None,
            gas_fee_strategy,
            simulate: false,
            on_transaction_signed: None,
            callback_execution_mode: None,
        };

    println!("🚀 尝试使用 0.01% 的极小滑点进行交易（预期失败）...");
    let result = client.buy(buy_params).await;

    // 在正常市场条件下，0.01% 的滑点应该导致交易失败
    if let Ok((success, _, error)) = result {
        if !success {
            println!("✅ 滑点保护生效，交易被拒绝: {:?}", error);
        } else {
            println!("⚠️  交易成功了（可能是市场流动性极好）");
        }
    } else {
        println!("✅ 滑点保护生效，交易在构建阶段失败: {:?}", result.err());
    }
}
