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

    // 验证 parsers HashMap 中已注册 CLMM Parser
    assert!(parser.parsers.contains_key(clmm_program_id));

    // 验证 parsers 中至少包含 PumpSwap 和 CLMM
    assert!(parser.parsers.len() >= 2);
    assert!(parser.parsers.contains_key(DexProtocol::PumpSwap.program_id()));
}
