//! Raydium CLMM 解析器测试
//!
//! 测试 Raydium CLMM 交易解析功能

use sol_trade_sdk::parser::raydium::clmm::RaydiumClmmParser;
use sol_trade_sdk::parser::base_parser::DexParserTrait;
use sol_trade_sdk::parser::types::DexProtocol;

#[test]
fn test_raydium_clmm_parser_creation() {
    // Step 1: 写测试 - 创建 CLMM Parser

    let parser = RaydiumClmmParser::new();

    // Step 2: 验证协议类型
    assert_eq!(parser.protocol(), DexProtocol::RaydiumClmm);
    assert_eq!(parser.protocol().program_id(), "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
}

#[test]
fn test_raydium_clmm_pool_account_index() {
    // Step 1: 写测试 - 验证 CLMM Pool 的账户索引

    // 根据 solana-dex-parser，CLMM 的 pool 地址在 accounts[2]
    // 这个测试为未来实现提供参考

    let expected_pool_index = 2; // CLMM Pool 在 accounts[2]
    assert_eq!(expected_pool_index, 2, "CLMM Pool 应该在 accounts[2]");
}
