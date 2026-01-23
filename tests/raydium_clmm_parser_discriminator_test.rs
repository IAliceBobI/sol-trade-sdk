//! Raydium CLMM 解析器 Discriminator 测试
//!
//! 测试 CLMM Swap 解析和 discriminator 注册表功能
//!
//! 运行测试:
//!     cargo test --test raydium_clmm_parser_discriminator_test -- --nocapture
//!
//! 注意：
//! - 使用 Auto Mock 模式，首次运行会从 RPC 获取并缓存数据
//! - 后续运行会从 tests/mock_data/ 目录加载缓存（速度提升 100 倍）
//! - 如需重新录制，删除 tests/mock_data/ 目录中的对应文件即可

use sol_trade_sdk::parser::DexParser;
use sol_trade_sdk::parser::discriminators::DiscriminatorRegistry;
use sol_trade_sdk::parser::types::ParserConfig;

/// 使用真实交易测试 CLMM Swap 解析
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_clmm_swap_with_discriminator() {
    // 这个交易应该包含 Swap 操作（非流动性操作）
    let signature = "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU";

    // 使用 Auto Mock 模式：首次运行从 RPC 获取并缓存，后续运行从缓存加载
    let config = ParserConfig::default();
    let parser = DexParser::new_with_mock(config);
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
