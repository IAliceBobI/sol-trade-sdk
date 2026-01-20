//! 探索 Raydium V4 交易结构
//!
//! 使用真实交易数据来理解如何解析

use sol_trade_sdk::parser::DexParser;
use sol_trade_sdk::parser::types::DexProtocol;

#[test]
fn test_explore_raydium_v4_transaction() {
    let parser = DexParser::default();
    let signature = "5tqpXeLDzBKXdWUrTXb5pApjhapj6PLZZLvcLFBsYUdGgtnW9MYTC7N16gF4GyVZHQgGZKApNRP3bAUckr7MdpJr";

    // 使用 tokio 运行时
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(parser.parse_transaction(signature));

    println!("解析结果:");
    println!("  成功: {}", result.success);
    println!("  交易数量: {}", result.trades.len());
    if let Some(ref error) = result.error {
        println!("  错误: {}", error);
    }

    // 打印程序 ID
    let raydium_v4_program_id = DexProtocol::RaydiumV4.program_id();
    println!("\nRaydium V4 Program ID: {}", raydium_v4_program_id);

    // 检查是否已注册
    assert!(parser.parsers.contains_key(raydium_v4_program_id));
    println!("Raydium V4 Parser 已注册");
}
