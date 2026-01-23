//! PumpSwap Discriminator 测试
//!
//! 验证 DiscriminatorRegistry 能正确识别 PumpSwap 的操作类型

use sol_trade_sdk::parser::DexParser;
use sol_trade_sdk::parser::discriminators::{DiscriminatorRegistry, DexProtocol as ParserDexProtocol};
use sol_trade_sdk::parser::types::TradeType;

#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_pumpswap_buy_with_discriminator() {
    // PumpSwap 买入交易签名
    let signature = "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK";

    // 使用 Auto Mock 模式,首次运行会录制 Mock 数据,后续运行使用缓存
    let config = sol_trade_sdk::parser::types::ParserConfig {
        rpc_url: "http://127.0.0.1:8899".to_string(),
        verbose: false,
    };
    let parser = DexParser::new_with_mock(config);
    let result = parser.parse_transaction(signature).await;

    assert!(result.success, "解析应该成功: {:?}", result.error);
    assert!(!result.trades.is_empty(), "应该至少有一个交易");

    let trade = &result.trades[0];
    assert_eq!(trade.dex, "PumpSwap");
    assert_eq!(trade.trade_type, TradeType::Buy);

    println!("✅ PumpSwap Buy 交易解析成功");
    println!("   用户: {}", trade.user);
    println!("   输入: {} {}", trade.input_token.amount, trade.input_token.mint);
    println!("   输出: {} {}", trade.output_token.amount, trade.output_token.mint);
}

#[test]
fn test_pumpswap_discriminators() {
    let registry = DiscriminatorRegistry::default();

    // BUY 操作的 discriminator (8 字节)
    let buy = [102, 6, 61, 18, 1, 218, 235, 234];
    let buy_type = registry.identify(
        ParserDexProtocol::PumpSwap,
        &buy
    );
    assert!(
        matches!(buy_type, sol_trade_sdk::parser::discriminators::InstructionType::Buy),
        "BUY discriminator 应该被识别为 Buy 操作，实际: {:?}",
        buy_type
    );

    // 验证 BUY 不是流动性操作
    assert!(!registry.is_liquidity_discriminator(
        ParserDexProtocol::PumpSwap,
        &buy
    ), "BUY 不应该被识别为流动性操作");

    // 验证 BUY 是 Swap 操作
    assert!(registry.is_swap_discriminator(
        ParserDexProtocol::PumpSwap,
        &buy
    ), "BUY 应该被识别为 Swap 操作");

    // SELL 操作的 discriminator (8 字节)
    let sell = [51, 230, 133, 164, 1, 127, 131, 173];
    let sell_type = registry.identify(
        ParserDexProtocol::PumpSwap,
        &sell
    );
    assert!(
        matches!(sell_type, sol_trade_sdk::parser::discriminators::InstructionType::Sell),
        "SELL discriminator 应该被识别为 Sell 操作，实际: {:?}",
        sell_type
    );

    // 验证 SELL 不是流动性操作
    assert!(!registry.is_liquidity_discriminator(
        ParserDexProtocol::PumpSwap,
        &sell
    ), "SELL 不应该被识别为流动性操作");

    // 验证 SELL 是 Swap 操作
    assert!(registry.is_swap_discriminator(
        ParserDexProtocol::PumpSwap,
        &sell
    ), "SELL 应该被识别为 Swap 操作");

    // ADD_LIQUIDITY 操作的 discriminator (8 字节)
    let add_liq = [242, 35, 198, 137, 82, 225, 242, 182];
    let add_liq_type = registry.identify(
        ParserDexProtocol::PumpSwap,
        &add_liq
    );
    assert!(
        matches!(add_liq_type, sol_trade_sdk::parser::discriminators::InstructionType::AddLiquidity),
        "ADD_LIQUIDITY discriminator 应该被识别为 AddLiquidity 操作，实际: {:?}",
        add_liq_type
    );

    // 验证 ADD_LIQUIDITY 是流动性操作
    assert!(registry.is_liquidity_discriminator(
        ParserDexProtocol::PumpSwap,
        &add_liq
    ), "ADD_LIQUIDITY 应该被识别为流动性操作");

    // 验证 ADD_LIQUIDITY 不是 Swap 操作
    assert!(!registry.is_swap_discriminator(
        ParserDexProtocol::PumpSwap,
        &add_liq
    ), "ADD_LIQUIDITY 不应该被识别为 Swap 操作");

    // REMOVE_LIQUIDITY 操作的 discriminator (8 字节)
    let remove_liq = [183, 18, 70, 156, 148, 109, 161, 34];
    let remove_liq_type = registry.identify(
        ParserDexProtocol::PumpSwap,
        &remove_liq
    );
    assert!(
        matches!(remove_liq_type, sol_trade_sdk::parser::discriminators::InstructionType::RemoveLiquidity),
        "REMOVE_LIQUIDITY discriminator 应该被识别为 RemoveLiquidity 操作，实际: {:?}",
        remove_liq_type
    );

    // 验证 REMOVE_LIQUIDITY 是流动性操作
    assert!(registry.is_liquidity_discriminator(
        ParserDexProtocol::PumpSwap,
        &remove_liq
    ), "REMOVE_LIQUIDITY 应该被识别为流动性操作");

    // 验证 REMOVE_LIQUIDITY 不是 Swap 操作
    assert!(!registry.is_swap_discriminator(
        ParserDexProtocol::PumpSwap,
        &remove_liq
    ), "REMOVE_LIQUIDITY 不应该被识别为 Swap 操作");

    // CREATE_POOL 操作的 discriminator (8 字节)
    let create_pool = [233, 146, 209, 142, 207, 104, 64, 188];
    let create_pool_type = registry.identify(
        ParserDexProtocol::PumpSwap,
        &create_pool
    );
    assert!(
        matches!(create_pool_type, sol_trade_sdk::parser::discriminators::InstructionType::CreatePool),
        "CREATE_POOL discriminator 应该被识别为 CreatePool 操作，实际: {:?}",
        create_pool_type
    );

    // 验证 CREATE_POOL 是流动性操作
    assert!(registry.is_liquidity_discriminator(
        ParserDexProtocol::PumpSwap,
        &create_pool
    ), "CREATE_POOL 应该被识别为流动性操作");

    // 验证 CREATE_POOL 不是 Swap 操作
    assert!(!registry.is_swap_discriminator(
        ParserDexProtocol::PumpSwap,
        &create_pool
    ), "CREATE_POOL 不应该被识别为 Swap 操作");

    println!("✅ 所有 PumpSwap discriminator 测试通过");
}
