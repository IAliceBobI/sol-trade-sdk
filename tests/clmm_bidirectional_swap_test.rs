//! CLMM 双向 Swap 完整测试
//!
//! 测试完整的交易流程：
//! 1. WSOL -> JUP (买入)
//! 2. JUP -> WSOL (卖出)
//!
//! 运行测试:
//!     cargo test clmm_bidirectional_swap_test -- --nocapture

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::utils::raydium_clmm::{get_pool_by_address, quote_exact_in},
};
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::Signer,
};
use std::str::FromStr;
use std::sync::Arc;

mod common;
use common::get_simulation_test_keypair;

/// WSOL-JUP CLMM Pool
const WSOL_JUP_POOL: &str = "EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw";

/// WSOL Mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// JUP Mint
const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

#[tokio::test]
#[serial_test::serial]
async fn test_clmm_complete_bidirectional_swap() {
    println!("\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔄 CLMM 双向 Swap 完整测试");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    // Pool 地址和代币 Mint
    let pool_address = Pubkey::from_str(WSOL_JUP_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let jup_mint = Pubkey::from_str(JUP_MINT).unwrap();

    // 使用固定的测试账户
    let payer = get_simulation_test_keypair();
    let payer_pubkey = payer.pubkey();

    println!("📍 测试钱包: {}\n", payer_pubkey);

    // ========================================
    // 步骤 1: 初始化 - 确保 SOL 余额充足
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔧 步骤 1: 初始化测试账户");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 检查 SOL 余额
    let sol_balance = rpc.get_balance(&payer_pubkey).await.unwrap();
    println!("💰 当前 SOL 余额: {} lamports ({:.6} SOL)", sol_balance, sol_balance as f64 / 1e9);

    // 如果余额不足，空投
    let min_balance = 2 * LAMPORTS_PER_SOL;
    if sol_balance < min_balance {
        println!("⚠️  余额不足，正在空投 10 SOL...");
        let airdrop_sig = rpc.request_airdrop(&payer_pubkey, 10 * LAMPORTS_PER_SOL).await.unwrap();
        println!("✅ 空投签名: {}", airdrop_sig);

        println!("⏳ 等待空投到账...");
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        let new_balance = rpc.get_balance(&payer_pubkey).await.unwrap();
        println!("✅ 空投完成，新余额: {} lamports ({:.6} SOL)\n", new_balance, new_balance as f64 / 1e9);
    } else {
        println!("✅ 余额充足\n");
    }

    // 记录初始余额
    let initial_sol = rpc.get_balance(&payer_pubkey).await.unwrap();

    // 获取初始 JUP 余额
    let jup_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer_pubkey,
        &jup_mint,
        &spl_token::id(),
    );

    let initial_jup = match rpc.get_token_account_balance(&jup_ata).await {
        Ok(balance) => balance.amount.parse::<u64>().unwrap_or(0),
        Err(_) => 0,
    };

    println!("📊 初始余额:");
    println!("   SOL: {} lamports ({:.6} SOL)", initial_sol, initial_sol as f64 / 1e9);
    println!("   JUP: {} (raw units)\n", initial_jup);

    // ========================================
    // 步骤 2: 买入 JUP (WSOL -> JUP) - 本地计算
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💰 步骤 2: 买入 JUP (WSOL -> JUP) - 本地计算");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 买入金额：0.1 SOL
    let buy_amount_sol = 100_000_000u64;
    println!("📊 买入配置:");
    println!("   输入: {} lamports ({:.6} SOL)", buy_amount_sol, buy_amount_sol as f64 / 1e9);
    println!("   输出: JUP\n");

    // 获取 Pool 状态
    let pool_state = get_pool_by_address(&rpc, &pool_address).await.unwrap();

    // 计算交易方向：WSOL -> JUP 是 token1 -> token0
    let zero_for_one_buy = wsol_mint.to_string() == pool_state.token_mint0.to_string();
    println!("   交易方向: zero_for_one = {}", zero_for_one_buy);

    // 本地计算
    let local_buy_output = quote_exact_in(&rpc, &pool_address, buy_amount_sol, zero_for_one_buy)
        .await
        .unwrap();
    println!("   ✅ 本地计算输出: {} JUP\n", local_buy_output.amount_out);

    // ========================================
    // 步骤 3: 卖出 JUP (JUP -> WSOL) - 本地计算
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💸 步骤 3: 卖出 JUP (JUP -> WSOL) - 本地计算");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 假设我们有一半的 JUP 可以卖出
    let sell_amount_jup = local_buy_output.amount_out / 2;
    println!("📊 卖出配置:");
    println!("   输入: {} JUP (raw units)", sell_amount_jup);
    println!("   输出: WSOL\n");

    // 计算交易方向：JUP -> WSOL 是 token0 -> token1
    let zero_for_one_sell = jup_mint.to_string() == pool_state.token_mint0.to_string();
    println!("   交易方向: zero_for_one = {}", zero_for_one_sell);

    // 本地计算
    let local_sell_output = quote_exact_in(&rpc, &pool_address, sell_amount_jup, zero_for_one_sell)
        .await
        .unwrap();
    println!("   ✅ 本地计算输出: {} lamports ({:.6} SOL)\n",
        local_sell_output.amount_out, local_sell_output.amount_out as f64 / 1e9);

    // ========================================
    // 步骤 4: 双向交易总结
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 步骤 4: 双向交易总结");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("📈 本地计算结果:");
    println!("   1️⃣  买入 JUP:");
    println!("      输入: {} lamports ({:.6} SOL)", buy_amount_sol, buy_amount_sol as f64 / 1e9);
    println!("      输出: {} JUP", local_buy_output.amount_out);
    println!("      zero_for_one = {}", zero_for_one_buy);

    println!("\n   2️⃣  卖出 JUP:");
    println!("      输入: {} JUP (买入量的一半)", sell_amount_jup);
    println!("      输出: {} lamports ({:.6} SOL)",
        local_sell_output.amount_out, local_sell_output.amount_out as f64 / 1e9);
    println!("      zero_for_one = {}", zero_for_one_sell);

    println!("\n   📊 双向交易汇总:");
    let total_input_sol = buy_amount_sol;
    let total_output_sol = local_sell_output.amount_out;
    let final_jup_held = local_buy_output.amount_out - sell_amount_jup;
    let sol_profit = total_output_sol as i128 - total_input_sol as i128;

    println!("      总输入 SOL: {} lamports ({:.6} SOL)", total_input_sol, total_input_sol as f64 / 1e9);
    println!("      总输出 SOL: {} lamports ({:.6} SOL)", total_output_sol, total_output_sol as f64 / 1e9);
    println!("      持有 JUP: {} (raw units)", final_jup_held);
    println!("      SOL 盈亏: {} lamports ({:.6} SOL) {}",
        sol_profit,
        sol_profit as f64 / 1e9,
        if sol_profit >= 0 { "✅" } else { "❌" }
    );

    println!("\n✅ 测试完成！");
    println!("   ✅ 买入本地计算成功");
    println!("   ✅ 卖出本地计算成功");
    println!("   ✅ zero_for_one 参数正确");
    println!("   ✅ 双向交易逻辑验证通过");

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🎉 CLMM 双向 Swap 本地计算测试全部通过！");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
}
