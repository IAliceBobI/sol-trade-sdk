//! DEX 协议检测示例
//!
//! 展示如何使用 Pool 地址识别 DEX 协议

use sol_trade_sdk::common::SolanaRpcClient;
use sol_trade_sdk::common::dex_detector::{
    DexInfo, detect_dex_from_pool, detect_dex_from_pools_batch,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔍 Sol Trade SDK - DEX 协议检测示例\n");

    // 初始化 RPC 客户端
    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let rpc = SolanaRpcClient::new(rpc_url.clone());

    // 示例 1: 检测单个 Pool
    println!("📋 示例 1: 检测单个 Pool 的 DEX\n");

    let test_pools = vec![
        ("Raydium AMM V4", "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"),
        ("Raydium CPMM", "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp"),
        ("Raydium CLMM", "ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6"),
        ("PumpSwap", "539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR"),
    ];

    for (_name, pool_address) in test_pools {
        println!("   Pool: {}", pool_address);

        match detect_dex_from_pool(&rpc, pool_address).await {
            Ok(dex_info) => {
                println!("   ✅ 识别成功:");
                println!("      DEX 名称: {}", dex_info.display_name());
                println!("      代码名称: {}", dex_info.dex_name());
                println!("      Program ID: {}", dex_info.program_id);
            },
            Err(e) => {
                println!("   ❌ 识别失败: {}", e);
            },
        }
        println!();
    }

    // 示例 2: 批量检测多个 Pool
    println!("📊 示例 2: 批量检测多个 Pool\n");

    let pool_addresses = vec![
        "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2",
        "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp",
        "539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR",
    ];

    let results = detect_dex_from_pools_batch(&rpc, &pool_addresses).await;

    println!("   批量检测结果（共 {} 个）:\n", results.len());

    for dex_info in &results {
        println!("   {} - {}", dex_info.pool_address, dex_info.display_name());
    }

    // 示例 3: 使用 DexInfo 结构体
    println!("\n📦 示例 3: 手动创建 DexInfo\n");

    if let Some(dex_info) = DexInfo::new(
        "custom_pool_address".to_string(),
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(),
    ) {
        println!("   手动创建 DexInfo 成功:");
        println!("   DEX: {}", dex_info.display_name());
        println!("   代码: {}", dex_info.dex_name());
        println!("   Pool: {}", dex_info.pool_address);
        println!("   Program: {}", dex_info.program_id);
    }

    println!("\n✨ 所有示例执行完成！");

    Ok(())
}
