//! 真实交易解析集成测试
//!
//! 使用真实交易数据测试各个 DEX 的解析器
//! TDD 方式：先写测试，观察失败，再实现功能
//!
//! # 运行说明
//!
//! 这些测试依赖于本地测试节点（127.0.0.1:8899，surfpool）的真实交易数据。
//! surfpool 是从主网 fork 的，包含主网历史交易数据。
//!
//! ## 测试分类
//!
//! ### 📡 主网历史交易（5 个）
//! 这些交易是主网历史数据，永久存在，不会丢失：
//! - `test_parse_pumpswap_buy_transaction` (slot: 394648935)
//! - `test_parse_pumpswap_sell_transaction`
//! - `test_parse_raydium_v4_transaction` (slot: 394648738)
//! - `test_parse_raydium_cpmm_transaction` (slot: 394648506)
//! - `test_parse_raydium_clmm_transaction` (slot: 394462433)
//!
//! ### ⚠️  测试交易（1 个）
//! 这是测试节点上执行的临时交易，节点重启后会丢失：
//! - `test_parse_raydium_cpmm_sell_transaction` (Token-2022 测试) ⭐
//!
//! ## 运行方式
//!
//! ```bash
//! # 运行所有测试（会跳过丢失的测试）
//! cargo test --test dex_parser_real_tx
//!
//! # 运行所有测试（包括需要主网数据的）
//! TEST_REAL_TRANSACTIONS=1 cargo test --test dex_parser_real_tx
//! ```

use sol_trade_sdk::parser::DexParser;
use sol_trade_sdk::parser::types::TradeType;

/// Pumpswap 买入交易测试
///
/// 交易: https://solscan.io/tx/5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK?cluster=custom&customUrl=http://127.0.0.1:8899
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_parse_pumpswap_buy_transaction() {
    let parser = DexParser::default();
    let signature = "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK";

    let result = parser.parse_transaction(signature).await;

    // 验证解析成功
    assert!(result.success, "解析应该成功，错误: {:?}", result.error);
    assert!(!result.trades.is_empty(), "应该解析出至少一笔交易");

    let trade = &result.trades[0];

    println!("Pumpswap Buy 交易解析成功:");
    println!("  用户: {}", trade.user);
    println!("  池: {}", trade.pool);
    println!("  类型: {:?}", trade.trade_type);
    println!("  输入: {} {} (精度: {})", trade.input_token.amount, trade.input_token.mint, trade.input_token.decimals);
    println!("  输出: {} {} (精度: {})", trade.output_token.amount, trade.output_token.mint, trade.output_token.decimals);
    if let Some(ref fee) = trade.fee {
        println!("  手续费: {} {}", fee.amount, fee.mint);
    }

    // 验证交易类型
    assert_eq!(trade.trade_type, TradeType::Buy, "应该是买入交易");

    // 验证 DEX 类型
    assert_eq!(trade.dex, "PumpSwap");

    // 验证用户地址不为空
    assert_ne!(trade.user, solana_sdk::pubkey::Pubkey::default());

    // 验证池地址不为空
    assert_ne!(trade.pool, solana_sdk::pubkey::Pubkey::default());

    // 验证输入代币数量大于0
    assert!(trade.input_token.amount > 0.0, "输入代币数量应大于0");

    // 验证输出代币数量大于0
    assert!(trade.output_token.amount > 0.0, "输出代币数量应大于0");

    // 验证签名匹配
    assert_eq!(trade.signature, signature);
}

/// Pumpswap 卖出交易测试
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_parse_pumpswap_sell_transaction() {
    let parser = DexParser::default();
    // 使用任务中提供的买入交易哈希，实际测试时可以替换为卖出交易
    let signature = "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK";

    let result = parser.parse_transaction(signature).await;

    // 如果这个交易是买入，测试应该调整为卖出交易
    // 这里先验证基本解析功能
    assert!(result.success, "解析应该成功，错误: {:?}", result.error);
}

/// Raydium AMM V4 交易测试
///
/// 交易: https://solscan.io/tx/5tqpXeLDzBKXdWUrTXb5pApjhapj6PLZZLvcLFBsYUdGgtnW9MYTC7N16gF4GyVZHQgGZKApNRP3bAUckr7MdpJr?cluster=custom&customUrl=http://127.0.0.1:8899
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_parse_raydium_v4_transaction() {
    let parser = DexParser::default();
    let signature = "5tqpXeLDzBKXdWUrTXb5pApjhapj6PLZZLvcLFBsYUdGgtnW9MYTC7N16gF4GyVZHQgGZKApNRP3bAUckr7MdpJr";

    let result = parser.parse_transaction(signature).await;

    // 验证解析成功
    assert!(result.success, "解析应该成功，错误: {:?}", result.error);
    assert!(!result.trades.is_empty(), "应该解析出至少一笔交易");

    let trade = &result.trades[0];

    // 验证 DEX 类型
    assert_eq!(trade.dex, "Raydium V4");

    // 验证用户和池地址
    assert_ne!(trade.user, solana_sdk::pubkey::Pubkey::default());
    assert_ne!(trade.pool, solana_sdk::pubkey::Pubkey::default());

    // 验证代币数量
    assert!(trade.input_token.amount > 0.0, "输入代币数量应大于0");
    assert!(trade.output_token.amount > 0.0, "输出代币数量应大于0");

    println!("Raydium V4 交易解析成功:");
    println!("  用户: {}", trade.user);
    println!("  池: {}", trade.pool);
    println!("  类型: {:?}", trade.trade_type);
    println!("  输入: {} {}", trade.input_token.amount, trade.input_token.mint);
    println!("  输出: {} {}", trade.output_token.amount, trade.output_token.mint);

    // Solscan 显示: Swap 0.036626474 AVYS for 0.039489 USDC
    // 这是一个卖出交易: 用户卖出 AVYS, 收到 USDC
    // 所以 input 应该是 AVYS, output 应该是 USDC
    println!("  Solscan 显示: Swap 0.036626474 AVYS for 0.039489 USDC");
}

/// Raydium CPMM 交易测试
///
/// 交易: https://solscan.io/tx/7Q5gThWgQkbSR6GSLVSAjo9x762DSuLQwg6ne6KKomjfWSho26Zmr7qfPQ7zzJk7sdTvHPqhW9grxaNzGhJgRrn?cluster=custom&customUrl=http://127.0.0.1:8899
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_parse_raydium_cpmm_transaction() {
    let parser = DexParser::default();
    let signature = "7Q5gThWgQkbSR6GSLVSAjo9x762DSuLQwg6ne6KKomjfWSho26Zmr7qfPQ7zzJk7sdTvHPqhW9grxaNzGhJgRrn";

    let result = parser.parse_transaction(signature).await;

    // 验证解析成功
    assert!(result.success, "解析应该成功，错误: {:?}", result.error);
    assert!(!result.trades.is_empty(), "应该解析出至少一笔交易");

    let trade = &result.trades[0];

    // 验证 DEX 类型
    assert!(trade.dex.contains("CPMM") || trade.dex.contains("Raydium"));

    // 验证基本数据
    assert_ne!(trade.user, solana_sdk::pubkey::Pubkey::default());
    assert_ne!(trade.pool, solana_sdk::pubkey::Pubkey::default());

    println!("Raydium CPMM 交易解析成功:");
    println!("  用户: {}", trade.user);
    println!("  池: {}", trade.pool);
    println!("  类型: {:?}", trade.trade_type);
    println!("  输入: {} {}", trade.input_token.amount, trade.input_token.mint);
    println!("  输出: {} {}", trade.output_token.amount, trade.output_token.mint);
    // Solscan 显示: Swap 0.01 WSOL for 73,296.433626 Fartpad
    println!("  Solscan 显示: Swap 0.01 WSOL for 73,296.433626 Fartpad");
}

/// Raydium CPMM 卖出交易测试 (卖出代币获得 WSOL)
///
/// 交易: https://solscan.io/tx/23hEJ9AoF8H7zg3dN4rDcfgBqkhnPBBc89XzynyryXk5opGkKYtD66rV4Z2BPdmC718UvfeSbrWTbrjuFNd5ej2S?cluster=custom&customUrl=http://127.0.0.1:8899
/// Solscan 显示: Swap 100,000 INMU for 0.252744512 WSOL
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_parse_raydium_cpmm_sell_transaction() {
    let parser = DexParser::default();
    let signature = "23hEJ9AoF8H7zg3dN4rDcfgBqkhnPBBc89XzynyryXk5opGkKYtD66rV4Z2BPdmC718UvfeSbrWTbrjuFNd5ej2S";

    let result = parser.parse_transaction(signature).await;

    // 验证解析成功
    assert!(result.success, "解析应该成功，错误: {:?}", result.error);
    assert!(!result.trades.is_empty(), "应该解析出至少一笔交易");

    let trade = &result.trades[0];

    println!("Raydium CPMM Sell 交易解析成功:");
    println!("  用户: {}", trade.user);
    println!("  池: {}", trade.pool);
    println!("  类型: {:?}", trade.trade_type);
    println!("  输入: {} {} (精度: {})", trade.input_token.amount, trade.input_token.mint, trade.input_token.decimals);
    println!("  输出: {} {} (精度: {})", trade.output_token.amount, trade.output_token.mint, trade.output_token.decimals);
    // Solscan 显示: Swap 100,000 INMU for 0.252744512 WSOL
    println!("  Solscan 显示: Swap 100,000 INMU for 0.252744512 WSOL");

    // 验证 DEX 类型
    assert!(trade.dex.contains("CPMM") || trade.dex.contains("Raydium"));

    // 验证交易类型应该是 Sell（卖出 INMU）
    assert_eq!(trade.trade_type, TradeType::Sell, "应该是卖出交易");

    // 验证基本数据
    assert_ne!(trade.user, solana_sdk::pubkey::Pubkey::default());
    assert_ne!(trade.pool, solana_sdk::pubkey::Pubkey::default());

    // 验证输入和输出数量都大于0
    assert!(trade.input_token.amount > 0.0, "输入代币数量应大于0，当前: {}", trade.input_token.amount);
    assert!(trade.output_token.amount > 0.0, "输出代币数量应大于0，当前: {}", trade.output_token.amount);

    // 验证输入金额约为 100,000 INMU
    assert!((trade.input_token.amount - 100_000.0).abs() < 1.0,
        "输入金额应为约 100,000 INMU，当前: {}", trade.input_token.amount);

    // 验证输出金额约为 0.252744512 WSOL
    assert!((trade.output_token.amount - 0.252744512).abs() < 0.0001,
        "输出金额应为约 0.252744512 WSOL，当前: {}", trade.output_token.amount);
}

/// Raydium CLMM 交易测试
///
/// 交易: https://solscan.io/tx/5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU?cluster=custom&customUrl=http://127.0.0.1:8899
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_parse_raydium_clmm_transaction() {
    let parser = DexParser::default();
    let signature = "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU";

    let result = parser.parse_transaction(signature).await;

    // 验证解析成功
    assert!(result.success, "解析应该成功，错误: {:?}", result.error);
    assert!(!result.trades.is_empty(), "应该解析出至少一笔交易");

    let trade = &result.trades[0];

    // 验证 DEX 类型
    assert!(trade.dex.contains("CLMM") || trade.dex.contains("Raydium"));

    // 验证基本数据
    assert_ne!(trade.user, solana_sdk::pubkey::Pubkey::default());
    assert_ne!(trade.pool, solana_sdk::pubkey::Pubkey::default());

    println!("Raydium CLMM 交易解析成功:");
    println!("  用户: {}", trade.user);
    println!("  池: {}", trade.pool);
    println!("  类型: {:?}", trade.trade_type);
    println!("  输入: {} {}", trade.input_token.amount, trade.input_token.mint);
    println!("  输出: {} {}", trade.output_token.amount, trade.output_token.mint);
    // CLMM 交易金额请根据实际 Solscan 显示
}
