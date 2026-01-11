//! Raydium CLMM Buy & Sell 集成测试
//!
//! 本测试文件基于 `tests/raydium_cpmm_buy_sell_tests.rs` 的结构，验证：
//! - 基于 WSOL mint 查找 Raydium CLMM 池
//! - 基于 PoolState 构建 `RaydiumClmmParams`
//! - 通过 `SolanaTrade` 执行一条完整的 Raydium CLMM 买入 -> 卖出交易流程
//!
//! 测试假设：
//! - 本地 RPC `http://127.0.0.1:8899` 已接入主网数据（例如使用 surfpool）
//! - Raydium CLMM 协议已在该 RPC 上可用
//! - 存在至少一个包含 WSOL 的 Raydium CLMM 池
//!
//! 运行测试:
//!     cargo test --test raydium_clmm_buy_sell_tests -- --nocapture

use sol_trade_sdk::{
    common::GasFeeStrategy,
    trading::core::params::{DexParamEnum, RaydiumClmmParams},
    DexType, TradeBuyParams, TradeSellParams, TradeTokenType,
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;

mod test_helpers;
use test_helpers::{create_test_client, print_balances, print_token_balance};

/// JUP Token mint
const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

/// WSOL-JUP CLMM Pool
const WSOL_JUP_POOL: &str = "EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw";

/// 测试：Raydium CLMM 完整买入-卖出流程（使用 WSOL-JUP Pool）
///
/// 流程：
/// 1. 直接使用指定的 WSOL-JUP CLMM 池
/// 2. 使用 WSOL 买入 JUP token
/// 3. 再将全部 JUP token 卖出换回 SOL
/// 4. 验证 Token 余额变化和 SOL 净变化
#[tokio::test]
async fn test_raydium_clmm_buy_sell_complete() {
    println!("\n=== 测试：Raydium CLMM 完整买卖流程 (WSOL-JUP) ===");

    let client = create_test_client().await;
    let rpc_url = "http://127.0.0.1:8899";

    let payer_pubkey = client.payer.as_ref().pubkey();
    println!("测试钱包: {}", payer_pubkey);

    // 清理：关闭 WSOL ATA（如果存在），以确保测试环境干净
    println!("\n🧽 清理：尝试关闭已存在的 WSOL ATA...");
    let _ = client.close_wsol().await; // 忽略错误（如果不存在）

    // 记录初始 SOL 余额
    let (initial_sol, _) =
        print_balances(rpc_url, &payer_pubkey).await.expect("Failed to fetch initial balances");

    // ===== 1. 使用指定的 WSOL-JUP CLMM Pool =====
    let pool_address = Pubkey::from_str(WSOL_JUP_POOL).expect("Invalid pool address");
    let target_mint = Pubkey::from_str(JUP_MINT).expect("Invalid JUP mint");

    println!("\n🔍 使用 WSOL-JUP CLMM Pool: {}", pool_address);
    println!("目标交易 Token: JUP ({})", target_mint);

    // 记录初始目标代币余额
    let initial_token_balance =
        print_token_balance(rpc_url, &payer_pubkey, &target_mint, "Target")
            .await
            .expect("Failed to fetch initial token balance");

    // ===== 2. 从 Pool 地址构建 RaydiumClmmParams =====
    println!("\n🧮 从 Pool 构建 RaydiumClmmParams...");
    let clmm_params = RaydiumClmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
        .await
        .expect("Failed to build RaydiumClmmParams from pool address");

    println!("Pool 配置:");
    println!("  token0_mint: {}", clmm_params.token0_mint);
    println!("  token1_mint: {}", clmm_params.token1_mint);

    // ===== 3. 使用 SOL 买入目标代币 =====
    println!("\n💰 第一步：买入目标代币 (Raydium CLMM)");

    let input_amount = 10_000_000u64; // 0.01 SOL
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    // ===== 3.1. 预先创建并充值 WSOL ATA (分离的交易) =====
    println!("\n💧 预先创建并充值 WSOL ATA...");
    use sol_trade_sdk::trading::common::handle_wsol;
    let wsol_insts = handle_wsol(&payer_pubkey, input_amount);
    let recent_blockhash_wsol = client
        .rpc
        .get_latest_blockhash()
        .await
        .expect("Failed to get latest blockhash for WSOL");
    let wsol_tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
        &wsol_insts,
        Some(&payer_pubkey),
        &[client.payer.as_ref()],
        recent_blockhash_wsol,
    );
    let wsol_sig = client
        .rpc
        .send_and_confirm_transaction(&wsol_tx)
        .await
        .expect("Failed to create and fund WSOL ATA");
    println!("✅ WSOL ATA 创建并充值成功: {}", wsol_sig);
    
    // 等待确认
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let recent_blockhash =
        client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

    let buy_params = TradeBuyParams {
        dex_type: DexType::RaydiumClmm,
        // 使用 SOL 作为输入，在交易层会映射为 WSOL 进行池内兑换
        input_token_type: TradeTokenType::SOL,
        mint: target_mint,
        input_token_amount: input_amount,
        slippage_basis_points: Some(1000), // 10% slippage (1000 bp = 10%)
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::RaydiumClmm(clmm_params.clone()),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: false, // ❌ 不在 swap 交易中创建 WSOL ATA
        close_input_token_ata: false,
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy.clone(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let (success_buy, buy_sigs, error_buy) =
        client.buy(buy_params).await.expect("Raydium CLMM 买入交易执行失败");
    println!("\n[调试] success_buy: {}", success_buy);
    println!("[调试] buy_sigs: {:?}", buy_sigs);
    if let Some(err) = &error_buy {
        println!("[调试] error_buy: {:?}", err);
    }
    assert!(success_buy, "买入交易应成功");
    println!("✅ 买入成功，签名: {:?}", buy_sigs.get(0));

    // 买入后的代币余额
    let token_after_buy =
        print_token_balance(rpc_url, &payer_pubkey, &target_mint, "Target")
            .await
            .expect("Failed to fetch token balance after buy");
    assert!(
        token_after_buy > initial_token_balance,
        "买入后目标代币余额应增加",
    );

    // ===== 4. 卖出全部目标代币换回 SOL =====
    println!("\n💸 第二步：卖出全部目标代币 (Raydium CLMM)");

    let clmm_params_sell =
        RaydiumClmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
            .await
            .expect("Failed to build RaydiumClmmParams for sell");

    let recent_blockhash_sell = client
        .rpc
        .get_latest_blockhash()
        .await
        .expect("Failed to get latest blockhash for sell");

    let sell_params = TradeSellParams {
        dex_type: DexType::RaydiumClmm,
        output_token_type: TradeTokenType::SOL,
        mint: target_mint,
        input_token_amount: token_after_buy,
        slippage_basis_points: Some(1000), // 10% slippage
        recent_blockhash: Some(recent_blockhash_sell),
        with_tip: false,
        extension_params: DexParamEnum::RaydiumClmm(clmm_params_sell),
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
        client.sell(sell_params).await.expect("Raydium CLMM 卖出交易执行失败");
    assert!(success_sell, "卖出交易应成功");
    println!("✅ 卖出成功，签名: {:?}", sell_sigs.get(0));

    // 等待链上状态更新
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // ===== 5. 验证最终余额 =====
    let (final_sol, _) =
        print_balances(rpc_url, &payer_pubkey).await.expect("Failed to fetch final balances");
    let final_token_balance =
        print_token_balance(rpc_url, &payer_pubkey, &target_mint, "Target")
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

    println!("\n=== Raydium CLMM 买入-卖出完整流程测试通过 ===");
}
