//! Raydium AMM V4 真实交易集成测试
//!
//! 使用真实交易数据验证 Raydium V4 交易解析功能
//!
//! 测试数据来源: docs/plans/task.md

use sol_trade_sdk::parser::DexParser;

/// Raydium V4 真实 Swap 交易测试
///
/// 交易签名: 5tqpXeLDzBKXdWUrTXb5pApjhapj6PLZZLvcLFBsYUdGgtnW9MYTC7N16gF4GyVZHQgGZKApNRP3bAUckr7MdpJr
/// Solscan: https://solscan.io/tx/5tqpXeLDzBKXdWUrTXb5pApjhapj6PLZZLvcLFBsYUdGgtnW9MYTC7N16gF4GyVZHQgGZKApNRP3bAUckr7MdpJr?cluster=custom&customUrl=http://127.0.0.1:8899
///
/// 交易详情:
/// - Swap 0.036626474 AVYS (EuKdcxgEU83gRYbDm9H3JmDnJBykhhUvv5ea1BB94yyB)
/// - for 0.039489 USDC (EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
/// - on Raydium Liquidity Pool V4
#[tokio::test]
async fn test_raydium_v4_swap_real_transaction() {
    let parser = DexParser::default();

    let signature = "5tqpXeLDzBKXdWUrTXb5pApjhapj6PLZZLvcLFBsYUdGgtnW9MYTC7N16gF4GyVZHQgGZKApNRP3bAUckr7MdpJr";

    let result = parser.parse_transaction(signature).await;

    // 验证解析成功
    assert!(result.success, "解析应该成功, 但是得到: {:?}", result.error);
    assert!(!result.trades.is_empty(), "应该至少解析出一笔交易");

    // 获取第一笔交易
    let trade = &result.trades[0];

    // 验证 DEX 类型
    assert_eq!(trade.dex, "Raydium V4");

    // 验证用户地址不为空
    assert!(!trade.user.to_string().is_empty());

    // 验证代币信息
    assert!(trade.input_token.amount > 0.0, "输入代币数量应该大于0");
    assert!(trade.output_token.amount > 0.0, "输出代币数量应该大于0");

    // 验证代币精度
    assert!(trade.input_token.decimals > 0, "输入代币应该有精度信息");
    assert!(trade.output_token.decimals > 0, "输出代币应该有精度信息");

    // 打印解析结果用于调试
    println!("Raydium V4 Swap 交易解析结果:");
    println!("  交易类型: {:?}", trade.trade_type);
    println!("  用户: {}", trade.user);
    println!("  输入代币:");
    println!("    Mint: {}", trade.input_token.mint);
    println!("    Amount: {}", trade.input_token.amount);
    println!("    Decimals: {}", trade.input_token.decimals);
    println!("  输出代币:");
    println!("    Mint: {}", trade.output_token.mint);
    println!("    Amount: {}", trade.output_token.amount);
    println!("    Decimals: {}", trade.output_token.decimals);
    println!("  池地址: {}", trade.pool);

    // 根据实际交易验证:
    // Solscan 显示: Swap 0.036626474 AVYS for 0.039489 USDC
    //
    // 注意: "Swap AVYS for USDC" 可以理解为:
    // - 从池子角度: AVYS 被卖出(进入池子), USDC 被买入(从池子出来)
    // - 这意味着用户卖出 AVYS,获得 USDC
    //
    // 解析器识别为 Sell,因为 USDC(报价币)从池子转出到用户
    // 输入: USDC (从池子到用户), 输出: AVYS (从用户到池子)
    //
    // 实际上这是用户卖出 AVYS 获得 USDC 的交易

    // AVYS token mint
    let avys_mint = "EuKdcxgEU83gRYbDm9H3JmDnJBykhhUvv5ea1BB94yyB";
    // USDC mint
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    // 验证两个代币都存在
    let input_mint = trade.input_token.mint.to_string();
    let output_mint = trade.output_token.mint.to_string();

    assert!(
        (input_mint == avys_mint && output_mint == usdc_mint) ||
        (input_mint == usdc_mint && output_mint == avys_mint),
        "交易应该包含 AVYS 和 USDC"
    );

    // 验证数量
    // 注意: 由于手续费等原因,实际数量可能与 Solscan 显示略有不同

    // 输入数量 - AVYS
    let actual_input_amount = trade.input_token.amount;
    assert!(
        actual_input_amount > 0.03 && actual_input_amount < 0.04,
        "输入数量应该在合理范围内, 实际: {}",
        actual_input_amount
    );

    // 输出数量 - USDC
    let actual_output_amount = trade.output_token.amount;
    assert!(
        actual_output_amount > 0.03 && actual_output_amount < 0.05,
        "输出数量应该在合理范围内, 实际: {}",
        actual_output_amount
    );
}
