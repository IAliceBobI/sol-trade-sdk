//! 测试 sol-trade-test-utils crate
//!
//! 验证新的测试工具库是否正常工作

use sol_trade_sdk::common::SolanaRpcClient;
use solana_sdk::signer::Signer;
use std::sync::Arc;

// 使用新的测试工具 crate
use sol_trade_test_utils::{ensure_sol_balance, get_simulation_test_keypair};

#[tokio::test]
#[serial_test::serial]
async fn test_new_crate() {
    println!("\n========================================");
    println!("测试: sol-trade-test-utils crate");
    println!("========================================\n");

    let rpc_url = "http://127.0.0.1:8899";
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.to_string()));
    let payer = Arc::new(get_simulation_test_keypair());

    // 1. 测试 ensure_sol_balance
    println!("步骤 1: 测试 ensure_sol_balance");
    ensure_sol_balance(&rpc, rpc_url, &payer.pubkey(), 10)
        .await
        .expect("ensure_sol_balance 失败");

    println!("\n✅ 测试通过\n");
}
