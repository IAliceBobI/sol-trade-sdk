//! Raydium AMM V4 (Raydium Liquidity Pool V4) Pool 查找集成测试
//!
//! Raydium AMM V4 是 Raydium 的传统自动做市商（AMM）协议，使用恒定乘积公式（x * y = k）进行流动性提供和交易。
//!
//! ## 程序信息
//! - **程序名称**: Raydium Liquidity Pool V4
//! - **程序地址**: `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`
//! - **特性**: 集成 Serum 订单簿，支持限价单和市价单
//!
//! ## 费用结构
//! - **交易费**: 0.25% (25/10000)
//! - **Swap 费**: 0.25% (25/10000)
//! - **总费用**: 0.5%
//!
//! ## 已知 Pool
//! - **WSOL-USDC Pool**: `58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2`
//!   - Token0: WSOL (So11111111111111111111111111111111111111112)
//!   - Token1: USDC (EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
//!
//! ## 测试方法
//! - fetch_amm_info(rpc, amm) - 获取 AMM 信息
//!
//! 运行测试:
//!     cargo test --test raydium_amm_v4_pool_tests -- --nocapture
//!
//! 注意：使用 surfpool (localhost:8899) 进行测试

use sol_trade_sdk::instruction::utils::raydium_amm_v4::fetch_amm_info;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// 已知的 Raydium AMM V4 pool 地址
/// WSOL-USDC pool on Raydium AMM V4
/// - Pool Address: 58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2
/// - Token0: WSOL (So11111111111111111111111111111111111111112)
/// - Token1: USDC (EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
const SOL_USDC_AMM: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";

/// 测试：获取 AMM 信息
#[tokio::test]
async fn test_fetch_amm_info() {
    println!("=== 测试：获取 AMM 信息 ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    println!("获取 AMM 信息: {}", amm_address);
    let result = fetch_amm_info(&rpc, amm_address).await;

    assert!(result.is_ok(), "Failed to fetch AMM info: {:?}", result.err());
 
}
