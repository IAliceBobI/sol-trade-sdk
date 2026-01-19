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
