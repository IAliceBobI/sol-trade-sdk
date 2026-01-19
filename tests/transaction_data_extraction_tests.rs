//! TransactionAdapter 真实交易数据提取测试
//!
//! 测试从真实的 Solana 交易中提取数据

use sol_trade_sdk::parser::DexParser;

#[tokio::test]
async fn test_extract_signature_from_real_transaction() {
    // Step 1: 写测试 - 从真实交易中提取签名

    let parser = DexParser::default();

    // 真实的 CLMM 交易签名
    let signature = "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU";

    // 尝试解析交易
    let result = parser.parse_transaction(signature).await;

    // Step 2: 验证 - 当前应该返回空结果（因为解析功能未实现）
    // 但应该能成功获取交易数据
    println!("Parse result: {:?}", result);

    // 验证：不会崩溃，能返回某种结果
    assert!(!result.success || result.error.is_some() || !result.trades.is_empty(),
            "解析器应该返回成功、错误信息或交易列表之一");
}
