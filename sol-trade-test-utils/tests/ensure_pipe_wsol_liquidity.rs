//! 测试 ensure_pipe_pool_liquidity_via_swap 函数
//!
//! 这个测试会：
//! 1. 通过大额 Swap 确保 PIPE pool 有足够的流动性
//! 2. 使用更推荐的 Swap 方法而非直接添加流动性

use sol_trade_sdk::common::SolanaRpcClient;
use sol_trade_test_utils::{
    ensure_pipe_pool_liquidity_via_swap, ensure_sol_balance, get_simulation_test_keypair,
};
use solana_sdk::signer::Signer;
use std::sync::Arc;

#[tokio::test]
#[serial_test::serial]
async fn test_ensure_pipe_pool_wsol_liquidity() {
    println!("\n========================================");
    println!("测试: 确保 PIPE Pool 流动性（通过 Swap）");
    println!("========================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));
    let payer = get_simulation_test_keypair();

    // 确保账户有足够的 SOL 余额来支付交易费用
    println!("💰 确保账户 SOL 余额...");
    ensure_sol_balance(&rpc, &rpc_url, &payer.pubkey(), 10)
        .await
        .expect("SOL 余额不足");

    // 通过大额 Swap 确保 PIPE pool 流动性（Swap 10 SOL）
    match ensure_pipe_pool_liquidity_via_swap(&rpc, &rpc_url, &payer, 10).await {
        Ok(_) => {
            println!("\n✅ 流动性确保成功！\n");
        },
        Err(e) => {
            println!("\n❌ 流动性确保失败: {}\n", e);
            panic!("流动性确保失败");
        },
    }
}
