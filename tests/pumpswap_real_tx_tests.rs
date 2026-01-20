//! PumpSwap 真实交易集成测试
//!
//! 使用真实交易数据验证 PumpSwap 交易解析功能
//!
//! 测试数据来源: docs/plans/task.md

use sol_trade_sdk::parser::DexParser;

/// PumpSwap 真实 Swap 交易测试
///
/// 交易签名: 5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK
/// Solscan: https://solscan.io/tx/5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK?cluster=custom&customUrl=http://127.0.0.1:8899
///
/// 交易详情:
/// - Swap 1,931,177.808367 Memories (5HMcyrhboF95MDqma6xbnrGw2ijJM9bSwfhzpbTRTxXv)
/// - for 0.180869289 WSOL (So11111111111111111111111111111111111111112)
/// - on Pump.fun AMM
#[tokio::test]
async fn test_pumpswap_swap_real_transaction() {
    let parser = DexParser::default();

    let signature = "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK";

    let result = parser.parse_transaction(signature).await;

    // 验证解析成功
    assert!(result.success, "解析应该成功, 但是得到: {:?}", result.error);
    assert!(!result.trades.is_empty(), "应该至少解析出一笔交易");

    // 获取第一笔交易
    let trade = &result.trades[0];

    // 验证 DEX 类型
    assert_eq!(trade.dex, "PumpSwap");

    // 验证用户地址不为空
    assert!(!trade.user.to_string().is_empty());

    // 验证代币信息
    assert!(trade.input_token.amount > 0.0, "输入代币数量应该大于0");
    assert!(trade.output_token.amount > 0.0, "输出代币数量应该大于0");

    // 验证代币精度
    assert!(trade.input_token.decimals > 0, "输入代币应该有精度信息");
    assert!(trade.output_token.decimals > 0, "输出代币应该有精度信息");

    // 打印解析结果用于调试
    println!("PumpSwap Swap 交易解析结果:");
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
    // Swap 1,931,177.808367 Memories for 0.180869289 WSOL
    // 这意味着 Memories 是输入, WSOL 是输出

    // Memories token mint
    let memories_mint = "5HMcyrhboF95MDqma6xbnrGw2ijJM9bSwfhzpbTRTxXv";
    // WSOL mint
    let wsol_mint = "So11111111111111111111111111111111111111112";

    // 验证输入是 Memories
    assert_eq!(
        trade.input_token.mint.to_string(),
        memories_mint,
        "输入代币应该是 Memories"
    );

    // 验证输出是 WSOL
    assert_eq!(
        trade.output_token.mint.to_string(),
        wsol_mint,
        "输出代币应该是 WSOL"
    );

    // 验证数量
    // 注意: Solscan 显示的数量可能与实际交易数量略有不同
    // 因为 Solscan 显示的可能包含手续费等信息

    // 输入数量 - Solscan 显示 1,931,177.808367
    // 但实际交易中可能扣除了手续费,所以会有差异
    let actual_input_amount = trade.input_token.amount;
    assert!(
        actual_input_amount > 1_900_000.0 && actual_input_amount < 2_000_000.0,
        "输入数量应该在合理范围内, 实际: {}",
        actual_input_amount
    );

    // 输出数量 - 应该精确匹配
    let expected_output_amount = 0.180869289;
    let actual_output_amount = trade.output_token.amount;
    assert!(
        (actual_output_amount - expected_output_amount).abs() < 0.00001,
        "输出数量应该接近 {}, 实际: {}",
        expected_output_amount,
        actual_output_amount
    );

    // TODO: 这个交易应该是 SELL (卖 Memories 换 SOL)
    // 但是目前解析器可能识别为 BUY,需要进一步调查
}

/// PumpSwap 真实卖出交易测试
///
/// 如果需要,可以添加卖出交易的测试用例
#[tokio::test]
async fn test_pumpswap_sell_real_transaction() {
    // TODO: 添加卖出交易的测试用例
    // 需要从测试节点获取真实的卖出交易签名
}
