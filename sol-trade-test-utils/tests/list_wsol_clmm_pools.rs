//! 列出所有 WSOL 相关的 Raydium CLMM Pool，区分 Token 和 Token2022
//!
//! 这是一个探索性测试，用于查询和展示 WSOL 相关的 CLMM Pool 信息

use sol_trade_test_utils::list_wsol_clmm_pools as list_wsol_clmm_pools_fn;

#[tokio::test]
#[ignore = "探索用的，需要本地测试节点"]
async fn list_wsol_clmm_pools() {
    let rpc_url = "http://127.0.0.1:8899";

    // 使用便捷函数列出 WSOL CLMM Pool
    let result = list_wsol_clmm_pools_fn(rpc_url, Some(10)).await;

    assert!(result.is_ok(), "列出 WSOL CLMM Pool 失败: {:?}", result.err());

    let classification = result.unwrap();

    // 验证结果
    println!("\n✅ 查询成功");
    println!("Token2022 配对: {} 个", classification.token2022_pools.len());
    println!("Token 配对: {} 个", classification.token_pools.len());

    // 基本断言（确保至少有一些 Pool）
    let total_pools = classification.token2022_pools.len()
        + classification.token_pools.len();

    assert!(total_pools > 0, "应该至少找到一个 WSOL CLMM Pool");
}
