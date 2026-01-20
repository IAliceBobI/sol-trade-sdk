//! 所有 DEX 的集成测试
//!
//! 综合测试 PumpSwap, Raydium V4, Raydium CPMM, Raydium CLMM 的交易解析功能
//!
//! 测试数据来源: docs/plans/task.md

use sol_trade_sdk::parser::DexParser;
use serial_test::serial;

/// 测试所有 4 个 DEX 的交易解析
///
/// 这个测试验证:
/// 1. PumpSwap 解析功能
/// 2. Raydium AMM V4 解析功能
/// 3. Raydium CPMM 解析功能
/// 4. Raydium CLMM 解析功能
#[tokio::test]
#[serial]
async fn test_all_dex_parsing() {
    let parser = DexParser::default();

    // 测试交易列表
    let test_cases = vec![
        // PumpSwap
        (
            "PumpSwap",
            "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK",
            "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
        ),
        // Raydium AMM V4
        (
            "Raydium V4",
            "5tqpXeLDzBKXdWUrTXb5pApjhapj6PLZZLvcLFBsYUdGgtnW9MYTC7N16gF4GyVZHQgGZKApNRP3bAUckr7MdpJr",
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
        ),
        // Raydium CPMM
        (
            "Raydium CPMM",
            "7Q5gThWgQkbSR6GSLVSAjo9x762DSuLQwg6ne6KKomjfWSho26Zmr7qfPQ7zzJk7sdTvHPqhW9grxaNzGhJgRrn",
            "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
        ),
        // Raydium CLMM
        (
            "Raydium CLMM",
            "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU",
            "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
        ),
    ];

    let mut total_trades = 0;
    let mut successful_dex = 0;

    for (dex_name, signature, expected_program_id) in test_cases {
        println!("\n=== 测试 {} ===", dex_name);
        println!("签名: {}", signature);
        println!("程序ID: {}", expected_program_id);

        let result = parser.parse_transaction(signature).await;

        // 验证解析成功
        assert!(result.success, "{} 解析应该成功, 但是得到: {:?}", dex_name, result.error);
        assert!(!result.trades.is_empty(), "{} 应该至少解析出一笔交易", dex_name);

        // 获取第一笔交易
        let trade = &result.trades[0];

        // 验证 DEX 类型
        assert_eq!(trade.dex, dex_name, "DEX 类型应该匹配");

        // 验证用户地址不为空
        assert!(!trade.user.to_string().is_empty(), "{} 用户地址不应为空", dex_name);

        // 验证代币信息
        assert!(trade.input_token.amount > 0.0, "{} 输入代币数量应该大于0", dex_name);
        assert!(trade.output_token.amount > 0.0, "{} 输出代币数量应该大于0", dex_name);

        // 验证代币精度
        assert!(trade.input_token.decimals > 0, "{} 输入代币应该有精度信息", dex_name);
        assert!(trade.output_token.decimals > 0, "{} 输出代币应该有精度信息", dex_name);

        // 验证池地址不为空
        assert!(!trade.pool.to_string().is_empty(), "{} 池地址不应为空", dex_name);

        // 打印解析结果
        println!("  ✓ 解析成功");
        println!("  交易类型: {:?}", trade.trade_type);
        println!("  用户: {}", trade.user);
        println!("  池地址: {}", trade.pool);
        println!("  输入代币: {} (数量: {})",
            trade.input_token.mint,
            trade.input_token.amount
        );
        println!("  输出代币: {} (数量: {})",
            trade.output_token.mint,
            trade.output_token.amount
        );

        total_trades += result.trades.len();
        successful_dex += 1;
    }

    println!("\n=== 测试总结 ===");
    println!("成功的 DEX 数量: {}", successful_dex);
    println!("解析出的交易总数: {}", total_trades);

    // 验证所有 4 个 DEX 都成功解析
    assert_eq!(successful_dex, 4, "所有 4 个 DEX 都应该成功解析");

    println!("✓ 所有 DEX 解析测试通过！");
}

/// 测试能正确识别不同的 DEX 协议
#[tokio::test]
async fn test_dex_protocol_detection() {
    use sol_trade_sdk::parser::types::DexProtocol;

    // 验证程序ID能正确识别
    let test_cases = vec![
        (DexProtocol::PumpSwap, "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"),
        (DexProtocol::RaydiumV4, "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"),
        (DexProtocol::RaydiumClmm, "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"),
        (DexProtocol::RaydiumCpmm, "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"),
    ];

    for (protocol, expected_program_id) in test_cases {
        let actual_program_id = protocol.program_id();
        assert_eq!(actual_program_id, expected_program_id,
            "程序ID应该匹配: {} != {}", actual_program_id, expected_program_id);

        // 验证反向解析
        let parsed_protocol = DexProtocol::from_program_id(expected_program_id);
        assert_eq!(parsed_protocol, Some(protocol),
            "反向解析应该成功: {:?}", protocol);

        println!("✓ {} 程序ID识别正确", protocol.name());
    }

    // 验证未知的程序ID返回 None
    let unknown_program_id = "UnknownProgram1111111111111111111111111111111";
    let parsed_protocol = DexProtocol::from_program_id(unknown_program_id);
    assert!(parsed_protocol.is_none(), "未知程序ID应该返回 None");

    println!("✓ 所有协议检测测试通过！");
}

/// 测试解析器配置
#[tokio::test]
async fn test_parser_config() {
    use sol_trade_sdk::parser::types::ParserConfig;

    // 测试默认配置
    let default_config = ParserConfig::default();
    assert!(!default_config.rpc_url.is_empty(), "默认 RPC URL 不应为空");
    assert_eq!(default_config.rpc_url, "http://127.0.0.1:8899", "默认 RPC URL 应该是本地测试节点");
    assert!(!default_config.verbose, "默认不应该开启详细日志");

    // 测试自定义配置
    let custom_config = ParserConfig {
        verbose: true,
        rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
    };
    assert!(custom_config.verbose, "自定义配置应该启用详细日志");
    assert_eq!(custom_config.rpc_url, "https://api.mainnet-beta.solana.com", "自定义 RPC URL 应该匹配");

    println!("✓ 解析器配置测试通过！");
}

/// 测试单个 DEX 的详细解析（PumpSwap）
#[tokio::test]
#[serial]
async fn test_pumpswap_detailed_parsing() {
    let parser = DexParser::default();
    let signature = "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK";

    let result = parser.parse_transaction(signature).await;

    assert!(result.success, "PumpSwap 解析应该成功");
    assert_eq!(result.trades.len(), 1, "应该解析出 1 笔交易");

    let trade = &result.trades[0];

    // 详细验证
    assert_eq!(trade.dex, "PumpSwap");
    assert!(!trade.user.to_string().is_empty());
    assert!(!trade.pool.to_string().is_empty());
    assert!(trade.input_token.amount > 0.0);
    assert!(trade.output_token.amount > 0.0);
    assert!(trade.input_token.decimals > 0);
    assert!(trade.output_token.decimals > 0);

    // 验证手续费存在（PumpSwap 有手续费）
    assert!(trade.fee.is_some(), "PumpSwap 交易应该有手续费");

    println!("✓ PumpSwap 详细解析测试通过");
}

/// 测试单个 DEX 的详细解析（Raydium V4）
#[tokio::test]
#[serial]
async fn test_raydium_v4_detailed_parsing() {
    let parser = DexParser::default();
    let signature = "5tqpXeLDzBKXdWUrTXb5pApjhapj6PLZZLvcLFBsYUdGgtnW9MYTC7N16gF4GyVZHQgGZKApNRP3bAUckr7MdpJr";

    let result = parser.parse_transaction(signature).await;

    assert!(result.success, "Raydium V4 解析应该成功");
    assert_eq!(result.trades.len(), 1, "应该解析出 1 笔交易");

    let trade = &result.trades[0];

    // 详细验证
    assert_eq!(trade.dex, "Raydium V4");
    assert!(!trade.user.to_string().is_empty());
    assert!(!trade.pool.to_string().is_empty());
    assert!(trade.input_token.amount > 0.0);
    assert!(trade.output_token.amount > 0.0);
    assert!(trade.input_token.decimals > 0);
    assert!(trade.output_token.decimals > 0);

    println!("✓ Raydium V4 详细解析测试通过");
}

/// 测试单个 DEX 的详细解析（Raydium CPMM）
#[tokio::test]
#[serial]
async fn test_raydium_cpmm_detailed_parsing() {
    let parser = DexParser::default();
    let signature = "7Q5gThWgQkbSR6GSLVSAjo9x762DSuLQwg6ne6KKomjfWSho26Zmr7qfPQ7zzJk7sdTvHPqhW9grxaNzGhJgRrn";

    let result = parser.parse_transaction(signature).await;

    assert!(result.success, "Raydium CPMM 解析应该成功");
    assert_eq!(result.trades.len(), 1, "应该解析出 1 笔交易");

    let trade = &result.trades[0];

    // 详细验证
    assert_eq!(trade.dex, "Raydium CPMM");
    assert!(!trade.user.to_string().is_empty());
    assert!(!trade.pool.to_string().is_empty());
    assert!(trade.input_token.amount > 0.0);
    assert!(trade.output_token.amount > 0.0);
    assert!(trade.input_token.decimals > 0);
    assert!(trade.output_token.decimals > 0);

    println!("✓ Raydium CPMM 详细解析测试通过");
}

/// 测试单个 DEX 的详细解析（Raydium CLMM）
#[tokio::test]
#[serial]
async fn test_raydium_clmm_detailed_parsing() {
    let parser = DexParser::default();
    let signature = "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU";

    let result = parser.parse_transaction(signature).await;

    assert!(result.success, "Raydium CLMM 解析应该成功");
    assert_eq!(result.trades.len(), 1, "应该解析出 1 笔交易");

    let trade = &result.trades[0];

    // 详细验证
    assert_eq!(trade.dex, "Raydium CLMM");
    assert!(!trade.user.to_string().is_empty());
    assert!(!trade.pool.to_string().is_empty());
    assert!(trade.input_token.amount > 0.0);
    assert!(trade.output_token.amount > 0.0);
    assert!(trade.input_token.decimals > 0);
    assert!(trade.output_token.decimals > 0);

    println!("✓ Raydium CLMM 详细解析测试通过");
}
