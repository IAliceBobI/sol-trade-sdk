//! 交易解析器集成测试

// TODO: 临时注释，等待基础框架完善后再启用
// use sol_trade_sdk::parser::DexParser;
// use sol_trade_sdk::parser::types::ParserConfig;
//
// #[tokio::test]
// async fn test_dex_parser_creation() {
//     let config = ParserConfig {
//         verbose: false,
//         rpc_url: "http://127.0.0.1:8899".to_string(),
//     };
//
//     let parser = DexParser::new(config);
//
//     // 验证解析器创建成功
//     assert!(!parser.config.rpc_url.is_empty());
// }
//
// #[tokio::test]
// async fn test_parse_invalid_transaction() {
//     let config = ParserConfig {
//         verbose: false,
//         rpc_url: "http://127.0.0.1:8899".to_string(),
//     };
//
//     let parser = DexParser::new(config);
//
//     // 使用无效的交易签名
//     let signature = "invalid_signature_base58";
//     let result = parser.parse_transaction(signature).await;
//
//     // 应该返回失败结果
//     assert!(!result.success);
//     assert!(result.error.is_some());
// }
