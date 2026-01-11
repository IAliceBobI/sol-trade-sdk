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

use serial_test::serial;
use sol_trade_sdk::{
    common::GasFeeStrategy,
    instruction::utils::raydium_clmm::{
        clear_pool_cache, get_pool_by_mint, get_pool_by_mint_force,
    },
    trading::core::params::{DexParamEnum, RaydiumClmmParams},
    DexType, TradeBuyParams, TradeSellParams, TradeTokenType,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;

mod test_helpers;
use test_helpers::{create_test_client, print_balances, print_token_balance};

/// 已知的 WSOL mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// 测试：Raydium CLMM 完整买入-卖出流程
///
/// 流程：
/// 1. 基于 WSOL mint 选择一个 CLMM 池
/// 2. 选择该池中非 WSOL 的另一侧 Token 作为目标代币
/// 3. 使用 SOL 买入目标代币
/// 4. 再将全部目标代币卖出换回 SOL
/// 5. 验证 Token 余额变化和 SOL 净变化
#[tokio::test]
async fn test_raydium_clmm_buy_sell_complete() {
    println!("\n=== 测试：Raydium CLMM 完整买卖流程 ===");

    let client = create_test_client().await;
    let rpc_url = "http://127.0.0.1:8899";

    let payer_pubkey = client.payer.as_ref().pubkey();
    println!("测试钱包: {}", payer_pubkey);

    // 记录初始 SOL 余额
    let (initial_sol, _) =
        print_balances(rpc_url, &payer_pubkey).await.expect("Failed to fetch initial balances");

    // ===== 1. 基于 WSOL mint 查找一个 CLMM 池 =====
    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");

    println!("\n🔍 查找包含 WSOL 的 Raydium CLMM Pool...");
    clear_pool_cache();

    let (pool_address, pool_state) =
        get_pool_by_mint(&client.rpc, &wsol_mint).await.expect("get_pool_by_mint failed");

    println!("选中的 Pool: {}", pool_address);
    println!("  token_mint0: {}", pool_state.token_mint0);
    println!("  token_mint1: {}", pool_state.token_mint1);

    // 验证池确实包含 WSOL
    assert!(
        pool_state.token_mint0 == wsol_mint || pool_state.token_mint1 == wsol_mint,
        "CLMM Pool 必须包含 WSOL",
    );

    // 选择目标代币：池中非 WSOL 的那一侧
    let target_mint = if pool_state.token_mint0 == wsol_mint {
        pool_state.token_mint1
    } else {
        pool_state.token_mint0
    };

    println!("目标交易 Token Mint: {}", target_mint);

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

    // ===== 3. 使用 SOL 买入目标代币 =====
    println!("\n💰 第一步：买入目标代币 (Raydium CLMM)");

    let input_amount = 20_000_000u64; // 0.02 SOL
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    let recent_blockhash =
        client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash");

    let buy_params = TradeBuyParams {
        dex_type: DexType::RaydiumClmm,
        // 注意：当前 CLMM 实现要求池中必须包含 WSOL 或 USDC，这里以 SOL 作为输入，
        // 实际 SwapParams 中会通过 Trade 层映射为 SOL/WSOL 常量，若存在不一致将由测试暴露。
        input_token_type: TradeTokenType::SOL,
        mint: target_mint,
        input_token_amount: input_amount,
        slippage_basis_points: Some(10_000), // 10% 容忍度，避免因滑点导致测试偶发失败
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::RaydiumClmm(clmm_params.clone()),
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
        client.buy(buy_params).await.expect("Raydium CLMM 买入交易执行失败");
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
        slippage_basis_points: Some(10_000),
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

/// 测试：基于 WSOL mint 查找 CLMM Pool，并验证缓存与强制刷新
/// 这里复用池查找逻辑，确保与 `raydium_clmm_pool_tests` 行为一致
#[tokio::test]
#[serial]
async fn test_raydium_clmm_get_pool_by_mint_wsol_cache_and_force_for_trading() {
    println!("=== 测试：Raydium CLMM get_pool_by_mint (WSOL, cache & force, trading) ===");

    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 1. 清空缓存，确保从干净状态开始
    clear_pool_cache();

    // 2. 第一次查询：应从链上扫描并选择一个包含 WSOL 的池
    let (pool_addr_1, pool_state_1) =
        get_pool_by_mint(&rpc, &wsol_mint).await.expect("get_pool_by_mint failed");
    println!("第一次查询到的 Pool: {}", pool_addr_1);
    println!("  token_mint0: {}", pool_state_1.token_mint0);
    println!("  token_mint1: {}", pool_state_1.token_mint1);

    assert!(
        pool_state_1.token_mint0 == wsol_mint || pool_state_1.token_mint1 == wsol_mint,
        "返回的 CLMM Pool 不包含 WSOL",
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
    assert_eq!(
        pool_state_2.token_mint0, pool_state_3.token_mint0,
        "强制刷新后 token_mint0 不一致",
    );
    assert_eq!(
        pool_state_2.token_mint1, pool_state_3.token_mint1,
        "强制刷新后 token_mint1 不一致",
    );
}
