//! 列出所有 USDC 相关的 Raydium CPMM Pool，区分 Token 和 Token2022
//!
//! 这是一个探索性测试，用于查询和展示 USDC 相关的 Pool 信息

use sol_trade_test_utils::list_usdc_pools;

#[tokio::test]
#[ignore = "探索用的，需要本地测试节点"]
async fn list_usdc_cpmm_pools() {
    let rpc_url = "http://127.0.0.1:8899";

    // 使用便捷函数列出 USDC Pool
    let result = list_usdc_pools(rpc_url, Some(10)).await;

    assert!(result.is_ok(), "列出 USDC Pool 失败: {:?}", result.err());

    let classification = result.unwrap();

    // 验证结果
    println!("\n✅ 查询成功");
    println!("Token2022 配对: {} 个", classification.token2022_pools.len());
    println!("Token 配对: {} 个", classification.token_pools.len());
    println!("未知程序配对: {} 个", classification.unknown_pools.len());

    // 基本断言（确保至少有一些 Pool）
    let total_pools = classification.token2022_pools.len()
        + classification.token_pools.len()
        + classification.unknown_pools.len();

    assert!(total_pools > 0, "应该至少找到一个 USDC Pool");
}
