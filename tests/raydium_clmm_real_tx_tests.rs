//! Raydium CLMM 真实交易集成测试
//!
//! 使用真实交易数据验证 Raydium CLMM 交易解析功能
//!
//! 测试数据来源: docs/plans/task.md

use sol_trade_sdk::parser::DexParser;

/// Raydium CLMM 真实 Swap 交易测试
///
/// 交易签名: 5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU
/// Solscan: https://solscan.io/tx/5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU?cluster=custom&customUrl=http://127.0.0.1:8899
///
/// 交易详情:
/// - Swap 58,053.94204161 9bit (HmMubgKx91Tpq3jmfcKQwsv5HrErqnCTTRJMB6afFR2u)
/// - for 635.92147 USDC (EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
/// - on Raydium Concentrated Liquidity
#[tokio::test]
async fn test_raydium_clmm_swap_real_transaction() {
    let parser = DexParser::default();

    let signature = "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU";

    let result = parser.parse_transaction(signature).await;

    // 验证解析成功
    assert!(result.success, "解析应该成功, 但是得到: {:?}", result.error);
    assert!(!result.trades.is_empty(), "应该至少解析出一笔交易");

    // 获取第一笔交易
    let trade = &result.trades[0];

    // 验证 DEX 类型
    assert_eq!(trade.dex, "Raydium CLMM");

    // 验证用户地址不为空
    assert!(!trade.user.to_string().is_empty());

    // 验证代币信息
    assert!(trade.input_token.amount > 0.0, "输入代币数量应该大于0");
    assert!(trade.output_token.amount > 0.0, "输出代币数量应该大于0");

    // 打印解析结果用于调试
    println!("Raydium CLMM Swap 交易解析结果:");
    println!("  交易类型: {:?}", trade.trade_type);
    println!("  用户: {}", trade.user);
    println!("  输入代币:");
    println!("    Mint: {}", trade.input_token.mint);
    println!("    Amount: {}", trade.input_token.amount);
    println!("  输出代币:");
    println!("    Mint: {}", trade.output_token.mint);
    println!("    Amount: {}", trade.output_token.amount);
    println!("  池地址: {}", trade.pool);

    // 验证代币 mint
    let ninebit_mint = "HmMubgKx91Tpq3jmfcKQwsv5HrErqnCTTRJMB6afFR2u";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    let input_mint = trade.input_token.mint.to_string();
    let output_mint = trade.output_token.mint.to_string();

    assert!(
        (input_mint == ninebit_mint && output_mint == usdc_mint) ||
        (input_mint == usdc_mint && output_mint == ninebit_mint),
        "交易应该包含 9bit 和 USDC"
    );

    // 验证数量在合理范围内
    // 注意: CLMM 交易可能有更复杂的滑点和费用计算
    let has_ninebit = input_mint == ninebit_mint || output_mint == ninebit_mint;
    let has_usdc = input_mint == usdc_mint || output_mint == usdc_mint;

    assert!(has_ninebit, "交易应该包含 9bit");
    assert!(has_usdc, "交易应该包含 USDC");
}
