//! 列出所有 WSOL 相关的 Raydium AMM V4 Pool，区分 Token 和 Token2022
//!
//! 这是一个探索性测试，用于查询和展示 WSOL 相关的 AMM V4 Pool 信息
//!
//! **注意**: 此测试需要 RPC 节点支持 `getProgramAccounts` 查询 Raydium AMM V4
//! - 本地测试节点 (127.0.0.1:8899) 可能不支持此查询
//! - 建议使用付费 RPC 服务（Helius, QuickNode, Triton）或完整节点
//!
//! 运行测试:
//!     cargo test --package sol-trade-test-utils list_wsol_amm_v4_pools_test -- --ignored

use sol_trade_sdk::common::auto_mock_rpc::AutoMockRpcClient;
use sol_trade_test_utils::pool_list::{
    list_and_classify_amm_v4_pools, print_amm_v4_pool_classification,
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::test]
#[ignore = "需要支持 getProgramAccounts 的 RPC 节点"]
async fn list_wsol_amm_v4_pools_test() {
    // 注意：本地测试节点可能没有 AMM V4 Pool 数据
    // 如需测试，请使用支持完整 RPC 功能的节点
    let rpc_url = "http://127.0.0.1:8899";

    let wsol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

    println!("=== 查询所有 WSOL 相关的 Raydium AMM V4 Pool ===\n");
    println!("WSOL Mint: {}", wsol_mint);
    println!("正在查询...\n");

    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("list_wsol_amm_v4_pools".to_string()),
    );

    match list_and_classify_amm_v4_pools(&auto_mock_client, &wsol_mint).await {
        Ok(classification) => {
            print_amm_v4_pool_classification(&classification, Some(10));

            println!("\n✅ 查询成功");
            println!("Token2022 配对: {} 个", classification.token2022_pools.len());
            println!("Token 配对: {} 个", classification.token_pools.len());

            // 基本断言（确保至少有一些 Pool）
            let total_pools =
                classification.token2022_pools.len() + classification.token_pools.len();

            assert!(total_pools > 0, "应该至少找到一个 WSOL AMM V4 Pool");
        },
        Err(e) => {
            println!("\n⚠️  查询失败: {}", e);
            println!("提示：本地测试节点可能不支持 AMM V4 Pool 查询");
            println!("建议使用付费 RPC 服务（Helius, QuickNode, Triton）");
            panic!("RPC 节点不支持 AMM V4 Pool 查询");
        },
    }
}
