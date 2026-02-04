//! Pool 列出和查询工具
//!
//! 提供便捷的工具函数用于列出和查询 DEX Pool

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use sol_trade_sdk::{
    common::auto_mock_rpc::AutoMockRpcClient,
    constants::TOKEN_2022_PROGRAM,
    instruction::utils::raydium_cpmm::{clear_pool_cache, list_pools_by_mint},
};

/// Pool 分类统计结果
#[derive(Debug)]
pub struct PoolClassification {
    /// Token2022 配对的 Pool 列表
    pub token2022_pools: Vec<(Pubkey, PoolInfo, Pubkey)>,
    /// Token 配对的 Pool 列表
    pub token_pools: Vec<(Pubkey, PoolInfo, Pubkey)>,
    /// 未知程序配对的 Pool 列表
    pub unknown_pools: Vec<(Pubkey, PoolInfo, Pubkey, Pubkey)>,
}

/// Pool 简要信息
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pool_address: Pubkey,
    pub lp_supply: u64,
}

/// 列出所有包含指定 mint 的 Raydium CPMM Pool，并按 Token Program 类型分类
///
/// # 参数
/// * `rpc_client` - AutoMock RPC 客户端
/// * `mint` - 要查询的 Token mint 地址
///
/// # 返回
/// 返回分类后的 Pool 列表
///
/// # 示例
/// ```ignore
/// use sol_trade_test_utils::pool_list::{list_and_classify_pools, print_pool_classification};
/// use sol_trade_sdk::common::auto_mock_rpc::AutoMockRpcClient;
/// use std::str::FromStr;
///
/// let rpc_client = AutoMockRpcClient::new_with_namespace(
///     "http://127.0.0.1:8899".to_string(),
///     Some("list_usdc_pools".to_string()),
/// );
///
/// let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
///
/// let classification = list_and_classify_pools(&rpc_client, &usdc_mint).await?;
/// print_pool_classification(&classification);
/// ```
pub async fn list_and_classify_pools(
    rpc_client: &AutoMockRpcClient,
    mint: &Pubkey,
) -> Result<PoolClassification, String> {
    clear_pool_cache();

    // 列出所有包含该 mint 的 Pool
    let pools = list_pools_by_mint(rpc_client, mint)
        .await
        .map_err(|e| format!("list_pools_by_mint 失败: {}", e))?;

    // 分类统计
    let mut token2022_pools = Vec::new();
    let mut token_pools = Vec::new();
    let mut unknown_pools = Vec::new();

    for (addr, pool) in pools.iter() {
        // 判断 mint 是 token0 还是 token1
        let (_is_token0, other_mint, token_program) = if pool.token0_mint == *mint {
            (true, pool.token1_mint, pool.token1_program)
        } else {
            (false, pool.token0_mint, pool.token0_program)
        };

        let pool_info = PoolInfo {
            pool_address: *addr,
            lp_supply: pool.lp_supply,
        };

        if token_program == TOKEN_2022_PROGRAM {
            token2022_pools.push((*addr, pool_info, other_mint));
        } else if token_program == spl_token::ID {
            token_pools.push((*addr, pool_info, other_mint));
        } else {
            unknown_pools.push((*addr, pool_info, other_mint, token_program));
        }
    }

    Ok(PoolClassification {
        token2022_pools,
        token_pools,
        unknown_pools,
    })
}

/// 打印 Pool 分类统计结果
///
/// # 参数
/// * `classification` - Pool 分类结果
/// * `show_limit` - 每类最多显示多少个 Pool（默认 10）
pub fn print_pool_classification(classification: &PoolClassification, show_limit: Option<usize>) {
    let limit = show_limit.unwrap_or(10);

    println!("📊 统计结果:");
    println!(
        "  • Token2022 配对: {} 个",
        classification.token2022_pools.len()
    );
    println!("  • Token 配对: {} 个", classification.token_pools.len());
    println!(
        "  • 未知程序配对: {} 个\n",
        classification.unknown_pools.len()
    );

    // 显示 Token2022 配对
    if !classification.token2022_pools.is_empty() {
        println!(
            "═══════════════════════════════════════════════════════════════"
        );
        println!("🪙 Token2022 配对 (显示前 {} 个)", limit);
        println!(
            "═══════════════════════════════════════════════════════════════\n"
        );

        for (i, (addr, pool, other_mint)) in
            classification.token2022_pools.iter().take(limit).enumerate()
        {
            println!("{}. Pool: {}", i + 1, addr);
            println!("   配对 Mint: {}", other_mint);
            println!("   LP Supply: {}", pool.lp_supply);
            println!();
        }
    } else {
        println!("🪙 Token2022 配对: (无)\n");
    }

    // 显示 Token 配对
    if !classification.token_pools.is_empty() {
        println!(
            "═══════════════════════════════════════════════════════════════"
        );
        println!("💰 Token 配对 (显示前 {} 个)", limit);
        println!(
            "═══════════════════════════════════════════════════════════════\n"
        );

        for (i, (addr, pool, other_mint)) in
            classification.token_pools.iter().take(limit).enumerate()
        {
            println!("{}. Pool: {}", i + 1, addr);
            println!("   配对 Mint: {}", other_mint);
            println!("   LP Supply: {}", pool.lp_supply);
            println!();
        }
    } else {
        println!("💰 Token 配对: (无)\n");
    }

    // 显示未知程序的配对
    if !classification.unknown_pools.is_empty() {
        println!(
            "═══════════════════════════════════════════════════════════════"
        );
        println!("❓ 未知程序配对 (显示前 {} 个)", limit.min(5));
        println!(
            "═══════════════════════════════════════════════════════════════\n"
        );

        for (i, (addr, pool, other_mint, token_program)) in
            classification.unknown_pools.iter().take(limit.min(5)).enumerate()
        {
            println!("{}. Pool: {}", i + 1, addr);
            println!("   配对 Mint: {}", other_mint);
            println!("   LP Supply: {}", pool.lp_supply);
            println!("   Token Program: {}", token_program);
            println!();
        }
    }
}

/// 便捷函数：列出并打印 USDC 相关的 Pool
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `show_limit` - 每类最多显示多少个 Pool（可选）
pub async fn list_usdc_pools(
    rpc_url: &str,
    show_limit: Option<usize>,
) -> Result<PoolClassification, String> {
    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

    println!("=== 查询所有 USDC 相关的 Raydium CPMM Pool ===\n");
    println!("USDC Mint: {}", usdc_mint);
    println!("正在查询...\n");

    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("list_usdc_pools".to_string()),
    );

    let classification = list_and_classify_pools(&auto_mock_client, &usdc_mint).await?;

    print_pool_classification(&classification, show_limit);

    println!("=== 查询完成 ===");

    Ok(classification)
}

