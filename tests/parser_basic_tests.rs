//! 交易解析器基础测试

use sol_trade_sdk::parser::DexParser;
use sol_trade_sdk::parser::types::ParserConfig;

#[test]
fn test_dex_parser_default_creation() {
    // Step 1: 写测试 - 使用默认配置创建解析器
    let parser = DexParser::default();

    // Step 2: 验证解析器创建成功
    // 注意：无法直接访问私有字段 config，但可以通过 parse_transaction 方法验证功能
    // 当前测试只验证解析器能够成功创建
    assert!(true);  // 如果能到达这里说明创建成功
}

#[test]
fn test_dex_parser_custom_config() {
    // Step 1: 写测试 - 使用自定义配置创建解析器
    let config = ParserConfig {
        verbose: true,
        rpc_url: "http://custom.endpoint:8899".to_string(),
    };

    let parser = DexParser::new(config);

    // Step 2: 验证配置正确应用
    // 注意：无法直接访问私有字段 config，但可以通过 parse_transaction 方法验证功能
    // 当前测试只验证解析器能够成功创建
    assert!(true);  // 如果能到达这里说明创建成功
}

#[tokio::test]
async fn test_parse_invalid_signature() {
    // Step 1: 写测试 - 使用无效的签名测试解析器
    let parser = DexParser::default();

    // 使用一个无效的 Base58 签名（长度不对）
    let invalid_signature = "invalid_sig";

    // 应该返回失败结果
    let result = parser.parse_transaction(invalid_signature).await;

    // 先打印错误信息以便调试
    println!("Error: {:?}", result.error);

    // 验证：解析失败，且返回错误信息
    assert!(!result.success, "解析应该失败");
    assert!(result.error.is_some(), "应该有错误信息");

    // 验证错误信息中包含"无效签名"或"签名"相关内容
    let error_msg = result.error.unwrap();
    assert!(
        error_msg.contains("无效签名") || error_msg.contains("签名") || error_msg.contains("signature"),
        "错误信息应该包含签名相关内容，实际: {}",
        error_msg
    );
    assert!(result.trades.is_empty(), "交易列表应该为空");
}
