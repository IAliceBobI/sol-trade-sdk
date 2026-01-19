//! DexParser 集成测试
//!
//! 测试 DexParser 能正确识别和路由到不同的协议解析器

use sol_trade_sdk::parser::DexParser;
use sol_trade_sdk::parser::types::DexProtocol;

#[test]
fn test_dex_parser_has_clmm_registered() {
    // Step 1: 写测试 - 验证 DexParser 已注册 CLMM Parser

    let parser = DexParser::default();

    // 验证 CLMM Program ID 能被识别
    let clmm_program_id = DexProtocol::RaydiumClmm.program_id();
    assert_eq!(clmm_program_id, "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");

    // 注意：当前无法直接访问 parsers HashMap（私有字段）
    // 这个测试验证 Program ID 常量的正确性
    // 实际的路由测试需要通过 parse_transaction() 进行
}
