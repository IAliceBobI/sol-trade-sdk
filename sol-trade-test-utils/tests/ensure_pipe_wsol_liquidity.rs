//! 测试 ensure_pipe_pool_wsol_liquidity 函数
//!
//! 这个测试会：
//! 1. 确保 PIPE pool 至少有 1000 SOL 的流动性
//! 2. 如果不足，自动添加流动性

use sol_trade_sdk::common::SolanaRpcClient;
use sol_trade_test_utils::{
    ensure_pipe_pool_wsol_liquidity, get_simulation_test_keypair,
};
use std::sync::Arc;

#[tokio::test]
#[serial_test::serial]
async fn test_ensure_pipe_pool_wsol_liquidity() {
    println!("\n========================================");
    println!("测试: 确保 PIPE Pool 流动性");
    println!("========================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));
    let payer = get_simulation_test_keypair();

    // 确保 PIPE pool 至少有 1000 SOL 的流动性
    match ensure_pipe_pool_wsol_liquidity(&rpc, &rpc_url, &payer, 1000).await {
        Ok(_) => {
            println!("\n✅ 流动性确保成功！\n");
        },
        Err(e) => {
            println!("\n❌ 流动性确保失败: {}\n", e);
            panic!("流动性确保失败");
        },
    }
}
