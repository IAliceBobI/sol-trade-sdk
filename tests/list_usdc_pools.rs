//! 列出所有 USDC 相关的 Raydium CPMM Pool，区分 Token 和 Token2022

use sol_trade_sdk::{
    common::auto_mock_rpc::AutoMockRpcClient,
    constants::TOKEN_2022_PROGRAM,
    instruction::utils::raydium_cpmm::{clear_pool_cache, list_pools_by_mint},
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::test]
#[ignore = "探索用的"]
async fn list_usdc_cpmm_pools() {
    println!("\n=== 查询所有 USDC 相关的 Raydium CPMM Pool ===\n");

    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端
    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("list_usdc_pools".to_string()),
    );

    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

    println!("USDC Mint: {}", usdc_mint);
    println!("正在查询...\n");

    clear_pool_cache();

    // 列出所有 USDC Pool
    let pools = list_pools_by_mint(&auto_mock_client, &usdc_mint)
        .await
        .expect("list_pools_by_mint failed");

    println!("✅ 共找到 {} 个 USDC Pool\n", pools.len());

    // 分类统计（使用 PoolState 中已有的 token_program 字段）
    let mut token2022_pools = Vec::new();
    let mut token_pools = Vec::new();
    let mut unknown_pools = Vec::new();

    for (addr, pool) in pools.iter() {
        // 判断 USDC 是 token0 还是 token1
        let (_usdc_is_token0, other_mint, token_program) =
            if pool.token0_mint == usdc_mint {
                (true, pool.token1_mint, pool.token1_program)
            } else {
                (false, pool.token0_mint, pool.token0_program)
            };

        if token_program == TOKEN_2022_PROGRAM {
            token2022_pools.push((addr, pool, other_mint));
        } else if token_program == spl_token::ID {
            token_pools.push((addr, pool, other_mint));
        } else {
            unknown_pools.push((addr, pool, other_mint, token_program));
        }
    }

    println!("📊 统计结果:");
    println!("  • USDC 与 Token2022 配对: {} 个", token2022_pools.len());
    println!("  • USDC 与 Token 配对: {} 个", token_pools.len());
    println!("  • USDC 与未知程序配对: {} 个\n", unknown_pools.len());

    // 显示前 10 个 Token2022 配对
    println!("═══════════════════════════════════════════════════════════════");
    println!("🪙 USDC 与 Token2022 配对 (前 10 个)");
    println!("═══════════════════════════════════════════════════════════════\n");

    for (i, (addr, pool, other_mint)) in token2022_pools.iter().take(10).enumerate() {
        println!("{}. Pool: {}", i + 1, addr);
        println!("   交易对: USDC / {}", other_mint);
        println!("   LP Supply: {}", pool.lp_supply);
        println!();
    }

    if token2022_pools.is_empty() {
        println!("(无)\n");
    }

    // 显示前 10 个 Token 配对
    println!("═══════════════════════════════════════════════════════════════");
    println!("💰 USDC 与 Token 配对 (前 10 个)");
    println!("═══════════════════════════════════════════════════════════════\n");

    for (i, (addr, pool, other_mint)) in token_pools.iter().take(10).enumerate() {
        println!("{}. Pool: {}", i + 1, addr);
        println!("   交易对: USDC / {}", other_mint);
        println!("   LP Supply: {}", pool.lp_supply);
        println!();
    }

    if token_pools.is_empty() {
        println!("(无)\n");
    }

    // 显示未知程序的配对
    if !unknown_pools.is_empty() {
        println!("═══════════════════════════════════════════════════════════════");
        println!("❓ USDC 与未知程序配对 (前 5 个)");
        println!("═══════════════════════════════════════════════════════════════\n");

        for (i, (addr, pool, other_mint, token_program)) in unknown_pools.iter().take(5).enumerate() {
            println!("{}. Pool: {}", i + 1, addr);
            println!("   交易对: USDC / {}", other_mint);
            println!("   LP Supply: {}", pool.lp_supply);
            println!("   Token Program: {}", token_program);
            println!();
        }
    }

    println!("=== 查询完成 ===");
}
