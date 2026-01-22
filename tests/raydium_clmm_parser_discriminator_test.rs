use sol_trade_sdk::parser::DexParser;
use sol_trade_sdk::parser::discriminators::DiscriminatorRegistry;

/// 使用真实交易测试 CLMM Swap 解析
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_clmm_swap_with_discriminator() {
    // 这个交易应该包含 Swap 操作（非流动性操作）
    let signature = "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU";

    let parser = DexParser::default();
    let result = parser.parse_transaction(signature).await;

    assert!(result.success, "应该成功解析 CLMM Swap 交易");
    assert!(!result.trades.is_empty(), "应该解析出交易");

    let trade = &result.trades[0];
    assert_eq!(trade.dex, "Raydium CLMM");
}

/// 测试 discriminator 注册表
#[test]
fn test_discriminator_registry_clmm() {
    let registry = DiscriminatorRegistry::default();

    // openPosition - 应该被识别为流动性操作
    let open_position = [135, 128, 47, 77, 15, 152, 240, 49];
    assert!(registry.is_liquidity_discriminator(
        sol_trade_sdk::parser::discriminators::DexProtocol::RaydiumClmm,
        &open_position
    ));

    // 非 registered discriminator - 不应该是流动性操作
    let unknown = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    assert!(!registry.is_liquidity_discriminator(
        sol_trade_sdk::parser::discriminators::DexProtocol::RaydiumClmm,
        &unknown
    ));
}
