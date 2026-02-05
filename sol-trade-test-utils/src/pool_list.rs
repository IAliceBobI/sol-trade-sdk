//! Pool 列出和查询工具
//!
//! 提供便捷的工具函数用于列出和查询 DEX Pool

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use sol_trade_sdk::{
    common::auto_mock_rpc::AutoMockRpcClient,
    constants::TOKEN_2022_PROGRAM,
    instruction::utils::{
        pumpswap::{
            clear_pool_cache as pumpswap_clear_pool_cache,
            list_pools_by_mint as pumpswap_list_pools_by_mint,
        },
        raydium_amm_v4::{
            clear_pool_cache as amm_v4_clear_pool_cache,
            list_pools_by_mint as amm_v4_list_pools_by_mint,
        },
        raydium_clmm::{
            clear_pool_cache as clmm_clear_pool_cache,
            list_pools_by_mint as clmm_list_pools_by_mint,
        },
        raydium_cpmm::{
            clear_pool_cache as cpmm_clear_pool_cache,
            list_pools_by_mint as cpmm_list_pools_by_mint,
        },
    },
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

/// AMM V4 Pool 分类统计结果
#[derive(Debug)]
pub struct AmmV4PoolClassification {
    /// Token2022 配对的 Pool 列表
    pub token2022_pools: Vec<(Pubkey, AmmV4PoolInfo, Pubkey)>,
    /// Token 配对的 Pool 列表
    pub token_pools: Vec<(Pubkey, AmmV4PoolInfo, Pubkey)>,
}

/// AMM V4 Pool 简要信息
#[derive(Debug, Clone)]
pub struct AmmV4PoolInfo {
    pub pool_address: Pubkey,
    pub lp_amount: u64,
}

/// PumpSwap Pool 分类统计结果
#[derive(Debug)]
pub struct PumpSwapPoolClassification {
    /// Token2022 配对的 Pool 列表
    pub token2022_pools: Vec<(Pubkey, PumpSwapPoolInfo, Pubkey)>,
    /// Token 配对的 Pool 列表
    pub token_pools: Vec<(Pubkey, PumpSwapPoolInfo, Pubkey)>,
}

/// PumpSwap Pool 简要信息
#[derive(Debug, Clone)]
pub struct PumpSwapPoolInfo {
    pub pool_address: Pubkey,
    pub lp_supply: u64,
}

/// CLMM Pool 分类统计结果
#[derive(Debug)]
pub struct ClmmPoolClassification {
    /// Token2022 配对的 Pool 列表
    pub token2022_pools: Vec<(Pubkey, ClmmPoolInfo, Pubkey)>,
    /// Token 配对的 Pool 列表
    pub token_pools: Vec<(Pubkey, ClmmPoolInfo, Pubkey)>,
}

/// CLMM Pool 简要信息
#[derive(Debug, Clone)]
pub struct ClmmPoolInfo {
    pub pool_address: Pubkey,
    pub liquidity: u128,
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
    cpmm_clear_pool_cache();

    // 列出所有包含该 mint 的 Pool
    let pools = cpmm_list_pools_by_mint(rpc_client, mint)
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

        let pool_info = PoolInfo { pool_address: *addr, lp_supply: pool.lp_supply };

        if token_program == TOKEN_2022_PROGRAM {
            token2022_pools.push((*addr, pool_info, other_mint));
        } else if token_program == spl_token::ID {
            token_pools.push((*addr, pool_info, other_mint));
        } else {
            unknown_pools.push((*addr, pool_info, other_mint, token_program));
        }
    }

    Ok(PoolClassification { token2022_pools, token_pools, unknown_pools })
}

/// 打印 Pool 分类统计结果
///
/// # 参数
/// * `classification` - Pool 分类结果
/// * `show_limit` - 每类最多显示多少个 Pool（默认 10）
pub fn print_pool_classification(classification: &PoolClassification, show_limit: Option<usize>) {
    let limit = show_limit.unwrap_or(10);

    println!("📊 统计结果:");
    println!("  • Token2022 配对: {} 个", classification.token2022_pools.len());
    println!("  • Token 配对: {} 个", classification.token_pools.len());
    println!("  • 未知程序配对: {} 个\n", classification.unknown_pools.len());

    // 显示 Token2022 配对
    if !classification.token2022_pools.is_empty() {
        println!("═══════════════════════════════════════════════════════════════");
        println!("🪙 Token2022 配对 (显示前 {} 个)", limit);
        println!("═══════════════════════════════════════════════════════════════\n");

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
        println!("═══════════════════════════════════════════════════════════════");
        println!("💰 Token 配对 (显示前 {} 个)", limit);
        println!("═══════════════════════════════════════════════════════════════\n");

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
        println!("═══════════════════════════════════════════════════════════════");
        println!("❓ 未知程序配对 (显示前 {} 个)", limit.min(5));
        println!("═══════════════════════════════════════════════════════════════\n");

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

/// 列出所有包含指定 mint 的 Raydium AMM V4 Pool，并按 Token Program 类型分类
///
/// # 参数
/// * `rpc_client` - AutoMock RPC 客户端
/// * `mint` - 要查询的 Token mint 地址
///
/// # 返回
/// 返回分类后的 Pool 列表
///
/// # 注意
/// AMM V4 Pool 不直接存储 Token Program 信息，需要查询 Token Mint 账户来判断
pub async fn list_and_classify_amm_v4_pools(
    rpc_client: &AutoMockRpcClient,
    mint: &Pubkey,
) -> Result<AmmV4PoolClassification, String> {
    amm_v4_clear_pool_cache();

    // 列出所有包含该 mint 的 Pool（包括不活跃的）
    let pools = amm_v4_list_pools_by_mint(rpc_client, mint, false)
        .await
        .map_err(|e| format!("amm_v4_list_pools_by_mint 失败: {}", e))?;

    // 分类统计
    let mut token2022_pools = Vec::new();
    let mut token_pools = Vec::new();

    for (addr, amm) in pools.iter() {
        // 判断 mint 是 coin_mint 还是 pc_mint
        let (_is_coin, other_mint) =
            if amm.coin_mint == *mint { (true, amm.pc_mint) } else { (false, amm.coin_mint) };

        // 使用 AutoMockRpcClient 的异步 get_account 方法查询配对 Token Mint 账户
        let token_program = match rpc_client.get_account(&other_mint).await {
            Ok(account) => {
                // 检查账户的 owner（Token Program 就是 Mint 账户的 owner）
                account.owner
            },
            Err(e) => {
                // 查询失败，跳过
                eprintln!("查询 Token Mint 账户失败 {}: {}", other_mint, e);
                continue;
            },
        };

        let pool_info = AmmV4PoolInfo { pool_address: *addr, lp_amount: amm.lp_amount };

        if token_program == TOKEN_2022_PROGRAM {
            token2022_pools.push((*addr, pool_info, other_mint));
        } else if token_program == spl_token::ID {
            token_pools.push((*addr, pool_info, other_mint));
        }
        // 未知程序的 Pool 不统计
    }

    Ok(AmmV4PoolClassification { token2022_pools, token_pools })
}

/// 打印 AMM V4 Pool 分类统计结果
///
/// # 参数
/// * `classification` - AMM V4 Pool 分类结果
/// * `show_limit` - 每类最多显示多少个 Pool（默认 10）
pub fn print_amm_v4_pool_classification(
    classification: &AmmV4PoolClassification,
    show_limit: Option<usize>,
) {
    let limit = show_limit.unwrap_or(10);

    println!("📊 AMM V4 Pool 统计结果:");
    println!("  • Token2022 配对: {} 个", classification.token2022_pools.len());
    println!("  • Token 配对: {} 个\n", classification.token_pools.len());

    // 显示 Token2022 配对
    if !classification.token2022_pools.is_empty() {
        println!("═══════════════════════════════════════════════════════════════");
        println!("🪙 Token2022 配对 (显示前 {} 个)", limit);
        println!("═══════════════════════════════════════════════════════════════\n");

        for (i, (addr, pool, other_mint)) in
            classification.token2022_pools.iter().take(limit).enumerate()
        {
            println!("{}. Pool: {}", i + 1, addr);
            println!("   配对 Mint: {}", other_mint);
            println!("   LP Amount: {}", pool.lp_amount);
            println!();
        }
    } else {
        println!("🪙 Token2022 配对: (无)\n");
    }

    // 显示 Token 配对
    if !classification.token_pools.is_empty() {
        println!("═══════════════════════════════════════════════════════════════");
        println!("💰 Token 配对 (显示前 {} 个)", limit);
        println!("═══════════════════════════════════════════════════════════════\n");

        for (i, (addr, pool, other_mint)) in
            classification.token_pools.iter().take(limit).enumerate()
        {
            println!("{}. Pool: {}", i + 1, addr);
            println!("   配对 Mint: {}", other_mint);
            println!("   LP Amount: {}", pool.lp_amount);
            println!();
        }
    } else {
        println!("💰 Token 配对: (无)\n");
    }
}

/// 便捷函数：列出并打印 USDC 相关的 AMM V4 Pool
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `show_limit` - 每类最多显示多少个 Pool（可选）
pub async fn list_usdc_amm_v4_pools(
    rpc_url: &str,
    show_limit: Option<usize>,
) -> Result<AmmV4PoolClassification, String> {
    let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

    println!("=== 查询所有 USDC 相关的 Raydium AMM V4 Pool ===\n");
    println!("USDC Mint: {}", usdc_mint);
    println!("正在查询...\n");

    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("list_usdc_amm_v4_pools".to_string()),
    );

    let classification = list_and_classify_amm_v4_pools(&auto_mock_client, &usdc_mint).await?;

    print_amm_v4_pool_classification(&classification, show_limit);

    println!("=== 查询完成 ===");

    Ok(classification)
}

/// 便捷函数：列出并打印 WSOL 相关的 AMM V4 Pool
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `show_limit` - 每类最多显示多少个 Pool（可选）
pub async fn list_wsol_amm_v4_pools(
    rpc_url: &str,
    show_limit: Option<usize>,
) -> Result<AmmV4PoolClassification, String> {
    let wsol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

    println!("=== 查询所有 WSOL 相关的 Raydium AMM V4 Pool ===\n");
    println!("WSOL Mint: {}", wsol_mint);
    println!("正在查询...\n");

    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("list_wsol_amm_v4_pools".to_string()),
    );

    let classification = list_and_classify_amm_v4_pools(&auto_mock_client, &wsol_mint).await?;

    print_amm_v4_pool_classification(&classification, show_limit);

    println!("=== 查询完成 ===");

    Ok(classification)
}

/// 列出所有包含指定 mint 的 PumpSwap Pool，并按 Token Program 类型分类
///
/// # 参数
/// * `rpc_client` - AutoMock RPC 客户端
/// * `mint` - 要查询的 Token mint 地址
///
/// # 返回
/// 返回分类后的 Pool 列表
///
/// # 注意
/// PumpSwap Pool 不直接存储 Token Program 信息，需要查询 Token Mint 账户来判断
pub async fn list_and_classify_pumpswap_pools(
    rpc_client: &AutoMockRpcClient,
    mint: &Pubkey,
) -> Result<PumpSwapPoolClassification, String> {
    pumpswap_clear_pool_cache();

    // 列出所有包含该 mint 的 Pool
    let pools = pumpswap_list_pools_by_mint(rpc_client, mint)
        .await
        .map_err(|e| format!("pumpswap_list_pools_by_mint 失败: {}", e))?;

    // 分类统计
    let mut token2022_pools = Vec::new();
    let mut token_pools = Vec::new();

    for (addr, pool) in pools.iter() {
        // 判断 mint 是 base_mint 还是 quote_mint
        let (_is_base, other_mint) =
            if pool.base_mint == *mint { (true, pool.quote_mint) } else { (false, pool.base_mint) };

        // 使用 AutoMockRpcClient 的异步 get_account 方法查询配对 Token Mint 账户
        let token_program = match rpc_client.get_account(&other_mint).await {
            Ok(account) => {
                // 检查账户的 owner（Token Program 就是 Mint 账户的 owner）
                account.owner
            },
            Err(e) => {
                // 查询失败，跳过
                eprintln!("查询 Token Mint 账户失败 {}: {}", other_mint, e);
                continue;
            },
        };

        let pool_info = PumpSwapPoolInfo { pool_address: *addr, lp_supply: pool.lp_supply };

        if token_program == TOKEN_2022_PROGRAM {
            token2022_pools.push((*addr, pool_info, other_mint));
        } else if token_program == spl_token::ID {
            token_pools.push((*addr, pool_info, other_mint));
        }
        // 未知程序的 Pool 不统计
    }

    Ok(PumpSwapPoolClassification { token2022_pools, token_pools })
}

/// 打印 PumpSwap Pool 分类统计结果
///
/// # 参数
/// * `classification` - PumpSwap Pool 分类结果
/// * `show_limit` - 每类最多显示多少个 Pool（默认 10）
pub fn print_pumpswap_pool_classification(
    classification: &PumpSwapPoolClassification,
    show_limit: Option<usize>,
) {
    let limit = show_limit.unwrap_or(10);

    println!("📊 PumpSwap Pool 统计结果:");
    println!("  • Token2022 配对: {} 个", classification.token2022_pools.len());
    println!("  • Token 配对: {} 个\n", classification.token_pools.len());

    // 显示 Token2022 配对
    if !classification.token2022_pools.is_empty() {
        println!("═══════════════════════════════════════════════════════════════");
        println!("🪙 Token2022 配对 (显示前 {} 个)", limit);
        println!("═══════════════════════════════════════════════════════════════\n");

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
        println!("═══════════════════════════════════════════════════════════════");
        println!("💰 Token 配对 (显示前 {} 个)", limit);
        println!("═══════════════════════════════════════════════════════════════\n");

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
}

/// 便捷函数：列出并打印 WSOL 相关的 PumpSwap Pool
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `show_limit` - 每类最多显示多少个 Pool（可选）
pub async fn list_wsol_pumpswap_pools(
    rpc_url: &str,
    show_limit: Option<usize>,
) -> Result<PumpSwapPoolClassification, String> {
    let wsol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

    println!("=== 查询所有 WSOL 相关的 PumpSwap Pool ===\n");
    println!("WSOL Mint: {}", wsol_mint);
    println!("正在查询...\n");

    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("list_wsol_pumpswap_pools".to_string()),
    );

    let classification = list_and_classify_pumpswap_pools(&auto_mock_client, &wsol_mint).await?;

    print_pumpswap_pool_classification(&classification, show_limit);

    println!("=== 查询完成 ===");

    Ok(classification)
}

/// 列出所有包含指定 mint 的 Raydium CLMM Pool，并按 Token Program 类型分类
///
/// # 参数
/// * `rpc_client` - AutoMock RPC 客户端
/// * `mint` - 要查询的 Token mint 地址
///
/// # 返回
/// 返回分类后的 Pool 列表
///
/// # 注意
/// CLMM Pool 需要查询配对 Token Mint 账户来判断 Token Program 类型
pub async fn list_and_classify_clmm_pools(
    rpc_client: &AutoMockRpcClient,
    mint: &Pubkey,
) -> Result<ClmmPoolClassification, String> {
    use futures::stream::{self, StreamExt};

    clmm_clear_pool_cache();

    // 列出所有包含该 mint 的 Pool
    let pools = clmm_list_pools_by_mint(rpc_client, mint)
        .await
        .map_err(|e| format!("clmm_list_pools_by_mint 失败: {}", e))?;

    // 准备查询数据: (pool_address, pool, other_mint)
    let queries: Vec<_> = pools
        .iter()
        .map(|(addr, pool)| {
            let other_mint =
                if pool.token_mint0 == *mint { pool.token_mint1 } else { pool.token_mint0 };
            (*addr, pool.clone(), other_mint)
        })
        .collect();

    // 并发查询所有配对 Token Mint 账户（控制并发数为 100）
    let results: Vec<_> = stream::iter(queries)
        .map(|(addr, pool, other_mint)| {
            async move {
                let token_program = match rpc_client.get_account(&other_mint).await {
                    Ok(account) => account.owner,
                    Err(_) => {
                        // 查询失败，返回 None
                        return None;
                    },
                };

                let pool_info = ClmmPoolInfo { pool_address: addr, liquidity: pool.liquidity };

                Some((addr, pool_info, other_mint, token_program))
            }
        })
        .buffer_unordered(100) // 控制并发数为 100
        .collect()
        .await;

    // 分类统计
    let mut token2022_pools = Vec::new();
    let mut token_pools = Vec::new();

    for (addr, pool_info, other_mint, token_program) in results.into_iter().flatten() {
        if token_program == TOKEN_2022_PROGRAM {
            token2022_pools.push((addr, pool_info, other_mint));
        } else if token_program == spl_token::ID {
            token_pools.push((addr, pool_info, other_mint));
        }
    }

    Ok(ClmmPoolClassification { token2022_pools, token_pools })
}

/// 打印 CLMM Pool 分类统计结果
///
/// # 参数
/// * `classification` - CLMM Pool 分类结果
/// * `show_limit` - 每类最多显示多少个 Pool（默认 10）
pub fn print_clmm_pool_classification(
    classification: &ClmmPoolClassification,
    show_limit: Option<usize>,
) {
    let limit = show_limit.unwrap_or(10);

    println!("📊 CLMM Pool 统计结果:");
    println!("  • Token2022 配对: {} 个", classification.token2022_pools.len());
    println!("  • Token 配对: {} 个\n", classification.token_pools.len());

    // 显示 Token2022 配对
    if !classification.token2022_pools.is_empty() {
        println!("═══════════════════════════════════════════════════════════════");
        println!("🪙 Token2022 配对 (显示前 {} 个)", limit);
        println!("═══════════════════════════════════════════════════════════════\n");

        for (i, (addr, pool, other_mint)) in
            classification.token2022_pools.iter().take(limit).enumerate()
        {
            println!("{}. Pool: {}", i + 1, addr);
            println!("   配对 Mint: {}", other_mint);
            println!("   Liquidity: {}", pool.liquidity);
            println!();
        }
    } else {
        println!("🪙 Token2022 配对: (无)\n");
    }

    // 显示 Token 配对
    if !classification.token_pools.is_empty() {
        println!("═══════════════════════════════════════════════════════════════");
        println!("💰 Token 配对 (显示前 {} 个)", limit);
        println!("═══════════════════════════════════════════════════════════════\n");

        for (i, (addr, pool, other_mint)) in
            classification.token_pools.iter().take(limit).enumerate()
        {
            println!("{}. Pool: {}", i + 1, addr);
            println!("   配对 Mint: {}", other_mint);
            println!("   Liquidity: {}", pool.liquidity);
            println!();
        }
    } else {
        println!("💰 Token 配对: (无)\n");
    }
}

/// 便捷函数：列出并打印 WSOL 相关的 CLMM Pool
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `show_limit` - 每类最多显示多少个 Pool（可选）
pub async fn list_wsol_clmm_pools(
    rpc_url: &str,
    show_limit: Option<usize>,
) -> Result<ClmmPoolClassification, String> {
    let wsol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

    println!("=== 查询所有 WSOL 相关的 Raydium CLMM Pool ===\n");
    println!("WSOL Mint: {}", wsol_mint);
    println!("正在查询...\n");

    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("list_wsol_clmm_pools".to_string()),
    );

    let classification = list_and_classify_clmm_pools(&auto_mock_client, &wsol_mint).await?;

    print_clmm_pool_classification(&classification, show_limit);

    println!("=== 查询完成 ===");

    Ok(classification)
}
