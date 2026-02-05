//! 列出前10个 WSOL CLMM Pool 的详细信息

use sol_trade_test_utils::list_wsol_clmm_pools as list_wsol_clmm_pools_fn;

#[tokio::test]
#[ignore = "探索用的，需要本地测试节点"]
async fn show_wsol_clmm_pools() {
    let rpc_url = "http://127.0.0.1:8899";

    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║   WSOL-Raydium CLMM Pool 查询 (前10个 Token2022 配对)          ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let result = list_wsol_clmm_pools_fn(rpc_url, Some(10)).await;

    assert!(result.is_ok(), "列出 WSOL CLMM Pool 失败: {:?}", result.err());

    let classification = result.unwrap();

    println!("═══════════════════════════════════════════════════════════════════");
    println!("📊 统计概览");
    println!("═══════════════════════════════════════════════════════════════════");
    println!("✅ Token2022 配对: {} 个", classification.token2022_pools.len());
    println!("✅ Token 配对: {} 个", classification.token_pools.len());
    println!();

    // 详细显示 Token2022 配对
    if !classification.token2022_pools.is_empty() {
        println!("═══════════════════════════════════════════════════════════════════");
        println!("🪙 Top 10 WSOL-Token2022 CLMM Pools");
        println!("═══════════════════════════════════════════════════════════════════");

        for (i, (pool_addr, pool_info, other_mint)) in classification.token2022_pools.iter().enumerate() {
            println!("\n【{}】", i + 1);
            println!("  🏊 Pool Address:        {}", pool_addr);
            println!("  🔀 Pair Token Mint:    {}", other_mint);
            println!("  💧 Liquidity:          {} ({:.2} M)",
                pool_info.liquidity,
                pool_info.liquidity as f64 / 1_000_000.0
            );
        }
    }

    // 详细显示 Token 配对
    if !classification.token_pools.is_empty() {
        println!("\n═══════════════════════════════════════════════════════════════════");
        println!("💰 Top 10 WSOL-Token CLMM Pools");
        println!("═══════════════════════════════════════════════════════════════════");

        for (i, (pool_addr, pool_info, other_mint)) in classification.token_pools.iter().take(10).enumerate() {
            println!("\n【{}】", i + 1);
            println!("  🏊 Pool Address:        {}", pool_addr);
            println!("  🔀 Pair Token Mint:    {}", other_mint);
            println!("  💧 Liquidity:          {} ({:.2} M)",
                pool_info.liquidity,
                pool_info.liquidity as f64 / 1_000_000.0
            );
        }
    }

    println!("\n═══════════════════════════════════════════════════════════════════");
    println!("✅ 查询完成");
    println!("═══════════════════════════════════════════════════════════════════");
}
