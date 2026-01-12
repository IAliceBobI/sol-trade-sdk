//! Raydium HTTP API Pool 查询集成测试
//!
//! 这些测试会直接访问 Raydium 公共 API，依赖：
//! - 正确的网络环境（如有需要，请在 .env 中配置 HTTP_PROXY/HTTPS_PROXY）
//! - Raydium 主网 API 可用
//!
//! 运行示例：
//!     cargo test --test raydium_api_pool_tests -- --nocapture

use sol_trade_sdk::common::raydium_api::{
    ClmmLiquidityPoint,
    FetchPoolsByMintsRequest,
    GetPoolListRequest,
    PoolFetchType,
    RaydiumApiClient,
    SortOrder,
};
use solana_sdk::pubkey::Pubkey;
use std::{env, str::FromStr};
use dotenvy::dotenv;

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

fn init_env_and_print_proxy() {
    // 加载 .env，如果存在的话
    let _ = dotenv();

    let https_proxy = env::var("HTTPS_PROXY").or_else(|_| env::var("https_proxy")).ok();
    let http_proxy = env::var("HTTP_PROXY").or_else(|_| env::var("http_proxy")).ok();

    match (https_proxy, http_proxy) {
        (Some(hs), _) => {
            println!("[raydium_api_pool_tests] 使用 HTTPS_PROXY = {}", hs);
        }
        (None, Some(hp)) => {
            println!("[raydium_api_pool_tests] 使用 HTTP_PROXY = {}", hp);
        }
        (None, None) => {
            println!("[raydium_api_pool_tests] 未检测到 HTTP_PROXY/HTTPS_PROXY，直接访问外网");
        }
    }
}

#[tokio::test]
async fn test_raydium_api_get_pool_list_standard() {
    init_env_and_print_proxy();
    println!("=== Raydium API：获取标准池列表（/pools/info/list） ===");

    let client = RaydiumApiClient::mainnet_default().expect("failed to create RaydiumApiClient");

    let req = GetPoolListRequest {
        r#type: Some(PoolFetchType::Standard),
        sort: Some("liquidity".to_string()),
        order: Some(SortOrder::Desc),
        page_size: Some(50),
        page: Some(0),
    };

    let page = client
        .get_pool_list(&req)
        .await
        .expect("get_pool_list request failed");

    println!("count = {}, has_next_page = {}", page.count, page.has_next_page);
    println!("first pool raw json = {:?}", page.data.get(0));

    assert!(page.count > 0, "pool count should be > 0");
    assert!(
        !page.data.is_empty(),
        "pool data should not be empty for standard pools",
    );
}

#[tokio::test]
async fn test_raydium_api_fetch_pools_by_mints_sol_usdc() {
    init_env_and_print_proxy();
    println!("=== Raydium API：按 SOL-USDC mint 查询池（/pools/info/mint） ===");

    let client = RaydiumApiClient::mainnet_default().expect("failed to create RaydiumApiClient");

    let sol = Pubkey::from_str(SOL_MINT).expect("invalid SOL mint");
    let usdc = Pubkey::from_str(USDC_MINT).expect("invalid USDC mint");

    let req = FetchPoolsByMintsRequest {
        mint1: sol.to_string(),
        mint2: Some(usdc.to_string()),
        r#type: Some(PoolFetchType::All),
        sort: Some("liquidity".to_string()),
        order: Some(SortOrder::Desc),
        page: Some(0),
    };

    let page = client
        .fetch_pools_by_mints(&req)
        .await
        .expect("fetch_pools_by_mints request failed");

    println!("SOL-USDC pools count = {}", page.count);
    if let Some(first) = page.data.get(0) {
        println!("first SOL-USDC pool = {:?}", first);
    }

    assert!(page.count > 0, "SOL-USDC pools count should be > 0");
    assert!(
        !page.data.is_empty(),
        "SOL-USDC pools data should not be empty",
    );
}

#[tokio::test]
async fn test_raydium_api_fetch_by_ids_and_clmm_liquidity_lines() {
    init_env_and_print_proxy();
    println!("=== Raydium API：按 ID 查询池 & CLMM 流动性曲线 ===");

    let client = RaydiumApiClient::mainnet_default().expect("failed to create RaydiumApiClient");

    // 1. 获取一个 Standard 池，用于 /pools/info/ids & /pools/key/ids 测试
    let list_req = GetPoolListRequest {
        r#type: Some(PoolFetchType::Standard),
        sort: Some("liquidity".to_string()),
        order: Some(SortOrder::Desc),
        page_size: Some(1),
        page: Some(0),
    };
    let list_page = client
        .get_pool_list(&list_req)
        .await
        .expect("get_pool_list (standard) failed");

    let first_pool = list_page
        .data
        .get(0)
        .expect("no pool returned for standard list");
    let first_id = first_pool
        .get("id")
        .and_then(|v| v.as_str())
        .expect("standard pool missing id field")
        .to_string();

    println!("standard pool id = {}", first_id);

    // 2. /pools/info/ids
    let pools_by_ids = client
        .fetch_pools_by_ids(&[first_id.clone()])
        .await
        .expect("fetch_pools_by_ids failed");
    assert_eq!(pools_by_ids.len(), 1, "should return exactly one pool");

    // 3. /pools/key/ids
    let pool_keys = client
        .fetch_pool_keys_by_ids(&[first_id.clone()])
        .await
        .expect("fetch_pool_keys_by_ids failed");
    assert_eq!(pool_keys.len(), 1, "should return exactly one pool keys object");

    // 4. 获取一个 CLMM 池并测试 /pools/line/liquidity
    let clmm_list_req = GetPoolListRequest {
        r#type: Some(PoolFetchType::Concentrated),
        sort: Some("liquidity".to_string()),
        order: Some(SortOrder::Desc),
        page_size: Some(1),
        page: Some(0),
    };
    let clmm_page = client
        .get_pool_list(&clmm_list_req)
        .await
        .expect("get_pool_list (concentrated) failed");

    let clmm_pool = clmm_page
        .data
        .get(0)
        .expect("no pool returned for concentrated list");
    let clmm_id = clmm_pool
        .get("id")
        .and_then(|v| v.as_str())
        .expect("concentrated pool missing id field");

    println!("concentrated pool id = {}", clmm_id);

    let lines: Vec<ClmmLiquidityPoint> = client
        .get_clmm_pool_liquidity_lines(clmm_id)
        .await
        .expect("get_clmm_pool_liquidity_lines failed");

    println!("CLMM liquidity points count = {}", lines.len());
    if let Some(first_line) = lines.get(0) {
        println!(
            "first liquidity point: price={}, liquidity={}",
            first_line.price, first_line.liquidity
        );
    }

    assert!(
        !lines.is_empty(),
        "CLMM liquidity lines should not be empty for a top concentrated pool",
    );
}
