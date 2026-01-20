//! Raydium CPMM 真实交易集成测试
//!
//! 使用真实交易数据验证 Raydium CPMM 交易解析功能
//!
//! 测试数据来源: docs/plans/task.md

use sol_trade_sdk::parser::DexParser;

/// Raydium CPMM 真实 Swap 交易测试
///
/// 交易签名: 7Q5gThWgQkbSR6GSLVSAjo9x762DSuLQwg6ne6KKomjfWSho26Zmr7qfPQ7zzJk7sdTvHPqhW9grxaNzGhJgRrn
/// Solscan: https://solscan.io/tx/7Q5gThWgQkbSR6GSLVSAjo9x762DSuLQwg6ne6KKomjfWSho26Zmr7qfPQ7zzJk7sdTvHPqhW9grxaNzGhJgRrn?cluster=custom&customUrl=http://127.0.0.1:8899
///
/// 交易详情:
/// - Swap 0.01 WSOL for 73,296.433626 Fartpad (67tUrY4td8W6zFb5uajXmtZsjRzWERuCvvjNhgdAbnVS)
/// - on Raydium CPMM
#[tokio::test]
async fn test_raydium_cpmm_swap_real_transaction() {
    let parser = DexParser::default();

    let signature = "7Q5gThWgQkbSR6GSLVSAjo9x762DSuLQwg6ne6KKomjfWSho26Zmr7qfPQ7zzJk7sdTvHPqhW9grxaNzGhJgRrn";

    let result = parser.parse_transaction(signature).await;

    // 验证解析成功
    assert!(result.success, "解析应该成功, 但是得到: {:?}", result.error);
    assert!(!result.trades.is_empty(), "应该至少解析出一笔交易");

    // 获取第一笔交易
    let trade = &result.trades[0];

    // 验证 DEX 类型
    assert_eq!(trade.dex, "Raydium CPMM");

    // 验证用户地址不为空
    assert!(!trade.user.to_string().is_empty());

    // 验证代币信息
    assert!(trade.input_token.amount > 0.0, "输入代币数量应该大于0");
    assert!(trade.output_token.amount > 0.0, "输出代币数量应该大于0");

    // 打印解析结果用于调试
    println!("Raydium CPMM Swap 交易解析结果:");
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
    let wsol_mint = "So11111111111111111111111111111111111111112";
    let fartpad_mint = "67tUrY4td8W6zFb5uajXmtZsjRzWERuCvvjNhgdAbnVS";

    let input_mint = trade.input_token.mint.to_string();
    let output_mint = trade.output_token.mint.to_string();

    assert!(
        (input_mint == wsol_mint && output_mint == fartpad_mint) ||
        (input_mint == fartpad_mint && output_mint == wsol_mint),
        "交易应该包含 WSOL 和 Fartpad"
    );

    // 验证数量在合理范围内
    if input_mint == wsol_mint {
        assert!(
            trade.input_token.amount > 0.009 && trade.input_token.amount < 0.011,
            "WSOL 输入数量应该在合理范围内"
        );
        assert!(
            trade.output_token.amount > 70000.0 && trade.output_token.amount < 80000.0,
            "Fartpad 输出数量应该在合理范围内"
        );
    } else {
        assert!(
            trade.output_token.amount > 0.009 && trade.output_token.amount < 0.011,
            "WSOL 输出数量应该在合理范围内"
        );
        assert!(
            trade.input_token.amount > 70000.0 && trade.input_token.amount < 80000.0,
            "Fartpad 输入数量应该在合理范围内"
        );
    }
}
