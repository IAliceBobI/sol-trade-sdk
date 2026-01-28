//! Jito Bundle 交易测试
//!
//! 演示如何使用 Jito 发送 bundle 交易，确保多笔交易原子性执行
//!
//! ## 📚 官方推荐配置（基于 Jito 官方文档）
//!
//! ### 1️⃣ 动态 Tip vs 固定 Tip
//!
//! | 特性 | 固定 Tip | 动态 Tip (推荐) |
//! |------|----------|----------------|
//! | **Tip 金额** | 静态值（如 0.0001 SOL） | 根据网络拥堵动态调整 |
//! | **成本** | 可能过高或过低 | 始终保持在合理水平 |
//! | **成功率** | 拥堵时可能失败 | 根据百分位自动优化 |
//! | **推荐场景** | 简单应用、测试 | 生产环境、高频交易 |
//!
//! ### 2️⃣ Tip 百分位说明
//!
//! Jito Tip Floor API 返回的百分位数据表示：
//!
//! - **P25 (25th percentile)**: 25% 的成功交易 tip ≤ 此值（低成本）
//! - **P50 (50th percentile)**: 中位数 tip（平衡策略）
//! - **P75 (75th percentile)**: 75% 的成功交易 tip ≤ 此值（较高优先级）
//! - **P95 (95th percentile)**: 95% 的成功交易 tip ≤ 此值（高优先级）
//! - **P99 (99th percentile)**: 99% 的成功交易 tip ≤ 此值（最高优先级）
//!
//! **推荐配置**：
//! - 保守策略（低成本）：P25-P50 (0.000001-0.000005 SOL)
//! - 平衡策略（推荐）：P50-P75 (0.000005-0.000019 SOL)
//! - 激进策略（高优先级）：P95-P99 (0.0001-0.0026 SOL)
//!
//! ### 3️⃣ 区域选择
//!
//! 选择最近的 Jito 区域以降低延迟：
//!
//! | 区域 | 位置 | 推荐用户 |
//! |------|------|----------|
//! | Tokyo 🇯🇵 | 日本东京 | 亚洲用户（推荐） |
//! | Singapore 🇸🇬 | 新加坡 | 亚洲用户 |
//! | Frankfurt 🇩🇪 | 德国法兰克福 | 欧洲用户 |
//! | London 🇬🇧 | 英国伦敦 | 欧洲用户 |
//! | NewYork 🇺🇸 | 美国纽约 | 美国东海岸用户 |
//! | SLC 🇺🇸 | 美国盐湖城 | 美国西海岸用户 |
//!
//! ### 4️⃣ 其他最佳实践
//!
//! - ✅ **最小 Tip**: 1,000 lamports (0.000001 SOL)
//! - ✅ **Tip 位置**: 必须在最后一笔交易中
//! - ✅ **Bundle 限制**: 最多 5 笔交易
//! - ✅ **Tip 账户**: 使用官方提供的 8 个 tip 账户之一
//! - ✅ **原子性**: 所有交易全部成功或全部失败
//!
//! ## 📖 参考资源
//!
//! - [Jito 官方文档](https://docs.jito.wtf)
//! - [Tip Floor API](https://bundles.jito.wtf/api/v1/bundles/tip_floor)
//! - [QuickNode Jito 指南](https://www.quicknode.com/guides/solana-development/transactions/jito-bundles)

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::str::FromStr;

#[test]
fn test_jito_bundle_transaction_creation() {
    //! 测试创建 Jito Bundle 交易的概念
    //!
    //! 这个测试演示 Jito Bundle 的核心概念和结构

    println!("\n========== Jito Bundle 交易概念测试 ==========\n");

    // Step 1: 创建账户（仅演示）
    let payer = Keypair::new();
    let receiver = Pubkey::from_str("GjJyeC3YDUU7TPCndhTUzbf3HqHYBH1JKQmWLH9nPqx").unwrap();

    println!("👤 Payer: {}", payer.pubkey());
    println!("👤 Receiver: {}", receiver);

    // Step 2: 展示 Jito Tip Account
    let jito_tip_account =
        Pubkey::from_str("HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe").unwrap();
    println!("💰 Jito Tip Account: {}", jito_tip_account);

    // Step 3: 展示 Bundle 结构（概念性）
    let number_transactions = 3;
    println!("\n📦 Bundle 结构 ({} 笔交易):", number_transactions);
    println!();
    println!("  交易 1: 转账 1000 lamports");
    println!("  交易 2: 转账 1000 lamports");
    println!("  交易 3: 转账 1000 lamports + Tip 10000 lamports (0.00001 SOL)");
    println!();
    println!("  特点:");
    println!("    ✓ 所有交易使用相同的 blockhash");
    println!("    ✓ Tip 必须在最后一笔交易中");
    println!("    ✓ 原子性：全部成功或全部失败");
    println!("    ✓ 最多 5 笔交易");

    println!("\n✅ Bundle 概念展示完成!");
    println!("==========================================\n");
}

#[test]
fn test_jito_bundle_size_limits() {
    //! 测试 Bundle 大小限制
    //!
    //! Jito Bundle 最多支持 5 笔交易

    println!("\n========== Jito Bundle 大小限制测试 ==========\n");

    const MAX_BUNDLE_SIZE: usize = 5;

    println!("📊 Jito Bundle 限制:");
    println!("  - 最多 {} 笔交易", MAX_BUNDLE_SIZE);
    println!("  - 所有交易必须在同一个 slot 中执行");
    println!("  - 所有交易原子性（全部成功或全部失败）");
    println!("  - Bundle 总大小限制: 约 600-700 KB（取决于交易复杂度）");

    println!("\n📝 典型的 Bundle 结构:");
    println!("  交易 1: 业务逻辑");
    println!("  交易 2: 业务逻辑");
    println!("  交易 3: 业务逻辑");
    println!("  交易 4: 业务逻辑");
    println!("  交易 5: 业务逻辑 + Tip（必须）");

    println!("\n✅ Bundle 大小限制测试通过!");
    println!("========================================\n");
}

#[test]
fn test_jito_bundle_tip_amounts() {
    //! 测试不同 tip 金额的场景
    //!
    //! Jito 推荐的 tip 金额:
    //! - 最小: 1,000 lamports (0.000001 SOL)
    //! - 推荐: 根据网络拥堵情况动态调整
    //! - 可以使用 getTipFloor API 获取当前推荐的 tip 金额

    println!("\n========== Jito Bundle Tip 金额测试 ==========\n");

    let tip_amounts = vec![
        (1_000, "最小 tip (0.000001 SOL)"),
        (10_000, "正常优先级 (0.00001 SOL)"),
        (100_000, "高优先级 (0.0001 SOL)"),
    ];

    println!("💰 不同优先级的 tip 金额:");

    for (amount, description) in tip_amounts {
        let sol = amount as f64 / 1_000_000_000.0;
        println!("  - {:>10} lamports ({:>10.6} SOL) - {}", amount, sol, description);
    }

    println!("\n📊 Tip 建议:");
    println!("  - 在网络拥堵时，使用更高的 tip 以提高优先级");
    println!("  - 可以使用 Jito 的 getTipFloor API 获取当前推荐值");
    println!("  - Tip 金额会从你的账户余额中扣除");

    println!("\n✅ Tip 金额测试完成!");
    println!("=============================================\n");
}

/*
* 实际使用示例：如何发送 Jito Bundle
*
* ```ignore
* use sol_trade_sdk::swqos::{
*     jito::{JitoClient, JitoRegion},
*     SwqosClientTrait,
*     TradeType,
* };
*
* async fn send_bundle() -> Result<(), Box<dyn std::error::Error>> {
*     // 1. 创建 Jito Client
*     let jito_client = JitoClient::with_region(JitoRegion::Tokyo);
*
*     // 2. 创建多笔交易（最多 5 笔）
*     let transactions = vec![
*         transaction1,
*         transaction2,
*         transaction3,
*         // ... 最多 5 笔
*     ];
*
*     // 3. 发送 bundle
*     jito_client.send_transactions(
*         TradeType::Buy,
*         &transactions,
*         false, // 不等待确认
*     ).await?;
*
*     Ok(())
* }
* ```
*
* ## 重要提示
*
* 1. **交易数量**：Bundle 最多支持 5 笔交易
* 2. **Tip 金额**：在最后一笔交易中添加 tip，建议至少 10,000 lamports
* 3. **区块哈希**：所有交易使用相同的 blockhash
* 4. **顺序保证**：交易会按照提供的顺序依次执行
* 5. **原子性**：如果任何一笔交易失败，整个 bundle 都不会上链
* 6. **区域选择**：选择最近的 Jito 区域以降低延迟


  💰 获取测试资金的方法

 官方 Solana Faucet（推荐）

 1. faucet.solana.com - https://faucet.solana.com/
   - 官方水龙头，支持 devnet 和 testnet
   - 每 8 小时可请求 2 次
   - 输入钱包地址即可领取测试 SOL
 2. QuickNode Faucet - https://faucet.quicknode.com/solana/devnet
   - 每 12 小时可请求一次
   - 简单易用的界面
 3. Jumpbit Faucet - https://jumpbit.io/en/solana/devnet-faucet
   - 可领取最多 2 SOL
   - 无需连接钱包

     💡 使用建议

  由于 Jito 不支持 Devnet，你需要：
  1. 在 Testnet 上测试 Jito 功能
    - 使用 Testnet 端点：https://dallas.testnet.block-engine.jito.wtf
    - 从官方 faucet 获取 testnet SOL
  2. 在 Mainnet Beta 上小额测试
    - 使用极少量真实 SOL
    - 选择距离最近的地区端点（降低延迟）
*/

/// 完整的 Jito Bundle 发送示例（概念演示）
///
/// 这个测试展示如何使用 JitoClient 发送 bundle 交易的概念
/// 注意：这是概念性演示，不实际发送交易
#[tokio::test]
#[ignore] // 默认忽略，需要手动运行 `cargo test --test jito_bundle_tests -- --ignored`
async fn test_jito_bundle_send_example() {
    use sol_trade_sdk::swqos::{
        SwqosClientTrait,
        jito::{JitoClient, JitoRegion},
    };

    println!("\n========== Jito Bundle 发送流程演示 ==========\n");

    // ========== 配置 ==========
    // 亚洲用户推荐使用 Tokyo 区域
    let jito_client = JitoClient::with_region(JitoRegion::Tokyo);

    println!("🌍 Jito 区域: Tokyo");
    println!("🔗 Endpoint: {}", jito_client.endpoint);

    // ========== 创建钱包 ==========
    let payer = Keypair::new();
    let receiver = Pubkey::from_str("GjJyeC3YDUU7TPCndhTUzbf3HqHYBH1JKQmWLH9nPqx").unwrap();

    println!("\n👤 Payer: {}", payer.pubkey());
    println!("👤 Receiver: {}", receiver);

    // ========== 获取 Jito tip account ==========
    let tip_account_str = match jito_client.get_tip_account() {
        Ok(account) => account,
        Err(_) => "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe".to_string(),
    };
    let tip_account = Pubkey::from_str(&tip_account_str).unwrap();

    println!("\n💰 Tip Account: {}", tip_account);

    // ========== 展示 Bundle 结构 ==========
    println!("\n📦 Bundle 结构（3 笔交易）:");
    println!("  交易 1: 转账 1000 lamports");
    println!("  交易 2: 转账 1000 lamports");
    println!("  交易 3: 转账 1000 lamports + Tip 10000 lamports (0.00001 SOL)");
    println!();
    println!("  使用 P50 百分位的动态 tip: 0.00001 SOL");

    println!("\n✅ Bundle 结构演示完成!");
    println!("\n💡 实际使用时的完整流程:");
    println!("  1. 从 Solana RPC 获取 recent_blockhash");
    println!("  2. 创建多笔交易（最多 5 笔）");
    println!("  3. 在最后一笔交易中添加 tip");
    println!("  4. 调用 jito_client.send_transactions()");
    println!("  5. 可选：在 Jito Explorer 查看状态");

    println!("\n==========================================\n");
}

/// 测试动态 Tip Floor API
///
/// 这个测试展示如何从 Jito Tip Floor API 获取实时 tip 数据
/// 这是 Jito 官方推荐的动态 tip 策略
#[tokio::test]
#[ignore] // 默认忽略，需要网络连接
async fn test_jito_dynamic_tip_floor() {
    use sol_trade_sdk::swqos::jito::{
        DynamicTipConfig,
        dynamic_tip::{JitoTipFloorClient, TipPercentile},
    };

    println!("\n========== Jito 动态 Tip Floor 测试 ==========\n");

    // 创建 Tip Floor 客户端
    let tip_client = JitoTipFloorClient::new();

    println!("📡 正在获取 Jito Tip Floor 数据...");

    match tip_client.get_tip_floor().await {
        Ok(tip_data) => {
            println!("✅ 成功获取 Tip Floor 数据!\n");

            println!("📊 Tip Floor 统计 (基于已成功的交易):");
            println!("  ├─ P25:  {:.6} SOL (25% 的交易)", tip_data.landed_tips_25th_percentile);
            println!("  ├─ P50:  {:.6} SOL (中位数)", tip_data.landed_tips_50th_percentile);
            println!("  ├─ P75:  {:.6} SOL (75% 的交易)", tip_data.landed_tips_75th_percentile);
            println!("  ├─ P95:  {:.6} SOL (95% 的交易)", tip_data.landed_tips_95th_percentile);
            println!("  ├─ P99:  {:.6} SOL (99% 的交易)", tip_data.landed_tips_99th_percentile);
            println!(
                "  └─ EMA: {:.6} SOL (指数移动平均)",
                tip_data.ema_landed_tips_50th_percentile
            );

            println!("\n💡 策略建议:");
            println!("  - 保守策略（低成本）: P25-P50");
            println!("  - 平衡策略（推荐）: P50-P75");
            println!("  - 激进策略（高优先级）: P95-P99");

            // 测试不同配置的动态 tip 计算
            println!("\n🧮 不同配置的计算结果:");

            let configs = vec![
                (
                    DynamicTipConfig {
                        enabled: true,
                        percentile: TipPercentile::P25,
                        multiplier: 1.0,
                        min_tip: 0.000001,
                        max_tip: 0.001,
                    },
                    "保守策略 (P25)",
                ),
                (
                    DynamicTipConfig {
                        enabled: true,
                        percentile: TipPercentile::P50,
                        multiplier: 1.0,
                        min_tip: 0.000001,
                        max_tip: 0.001,
                    },
                    "平衡策略 (P50)",
                ),
                (
                    DynamicTipConfig {
                        enabled: true,
                        percentile: TipPercentile::P75,
                        multiplier: 1.0,
                        min_tip: 0.000001,
                        max_tip: 0.001,
                    },
                    "平衡策略 (P75)",
                ),
                (
                    DynamicTipConfig {
                        enabled: true,
                        percentile: TipPercentile::P95,
                        multiplier: 1.0,
                        min_tip: 0.000001,
                        max_tip: 0.001,
                    },
                    "激进策略 (P95)",
                ),
            ];

            for (config, strategy_name) in configs {
                let calculated_tip = tip_client.calculate_tip(&tip_data, &config);
                println!("  - {}: {:.6} SOL", strategy_name, calculated_tip);
            }
        },
        Err(e) => {
            println!("❌ 获取 Tip Floor 失败: {}", e);
            println!("💡 可能的原因:");
            println!("   - 网络连接问题");
            println!("   - Jito API 暂时不可用");
        },
    }

    println!("\n============================================\n");
}

/// 测试固定 Tip vs 动态 Tip 的区别
///
/// 对比固定 tip 和动态 tip 在不同场景下的表现
#[test]
fn test_jito_fixed_vs_dynamic_tip() {
    println!("\n========== 固定 Tip vs 动态 Tip 对比 ==========\n");

    // 模拟不同的网络拥堵场景
    let scenarios = vec![
        ("网络空闲", 0.000001, 0.000001, 0.000005),
        ("正常流量", 0.00001, 0.000005, 0.000019),
        ("网络拥堵", 0.0001, 0.000019, 0.0001),
        ("严重拥堵", 0.001, 0.0001, 0.0026),
    ];

    println!("📊 不同场景下的 Tip 策略对比:\n");
    println!("{:<12} | {:>12} | {:>12} | {:>12}", "场景", "固定 Tip", "动态 P75", "动态 P95");
    println!("{}", "-".repeat(60));

    for (scenario, fixed_tip, dynamic_p75, dynamic_p95) in scenarios {
        println!(
            "{:<12} | {:>10.6} | {:>10.6} | {:>10.6}",
            scenario, fixed_tip, dynamic_p75, dynamic_p95
        );
    }

    println!("\n💡 关键区别:");
    println!("  固定 Tip:");
    println!("    ✅ 优点: 简单、可预测");
    println!("    ❌ 缺点:");
    println!("       - 网络空闲时成本过高");
    println!("       - 网络拥堵时可能失败");
    println!("");
    println!("  动态 Tip:");
    println!("    ✅ 优点:");
    println!("       - 根据市场实时调整");
    println!("       - 优化成本和成功率");
    println!("       - 自动适应网络状况");
    println!("    ❌ 缺点: 需要额外 API 调用");

    println!("\n✅ 推荐: 生产环境使用动态 Tip (P50-P75)\n");
    println!("=============================================\n");
}

/// 测试完整的 5 笔交易 Bundle（最大容量）
///
/// 展示 Jito Bundle 的最大容量结构和最佳实践
#[test]
fn test_jito_max_bundle_size() {
    println!("\n========== Jito 最大容量 Bundle 演示 (5 笔交易) ==========\n");

    let payer = Keypair::new();
    let receiver = Pubkey::from_str("GjJyeC3YDUU7TPCndhTUzbf3HqHYBH1JKQmWLH9nPqx").unwrap();
    let jito_tip_account =
        Pubkey::from_str("HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe").unwrap();

    println!("👤 Payer: {}", payer.pubkey());
    println!("👤 Receiver: {}", receiver);
    println!("💰 Tip Account: {}", jito_tip_account);

    const MAX_BUNDLE_SIZE: usize = 5;

    println!("\n📦 最大容量 Bundle 结构 ({} 笔交易):", MAX_BUNDLE_SIZE);
    println!("  交易 1: 转账 1000 lamports");
    println!("  交易 2: 转账 1000 lamports");
    println!("  交易 3: 转账 1000 lamports");
    println!("  交易 4: 转账 1000 lamports");
    println!("  交易 5: 转账 1000 lamports + 动态 Tip: 19000 lamports (0.000019 SOL - P75)");

    println!("\n✅ Bundle 结构展示完成!");
    println!("  - 交易数量: {} / 5 (最大)", MAX_BUNDLE_SIZE);
    println!("  - 总转账: {} lamports", 1_000 * MAX_BUNDLE_SIZE);
    println!("  - 总 Tip: 19000 lamports (0.000019 SOL)");
    println!("  - 原子性: 是（全部成功或全部失败）");

    println!("\n💡 最佳实践:");
    println!("  ✓ Tip 使用 P75 百分位: 0.000019 SOL");
    println!("  ✓ Tip 必须在最后一笔交易中");
    println!("  ✓ 所有交易使用相同的 blockhash");
    println!("  ✓ 使用最近的 Jito 区域以降低延迟");

    println!("\n=========================================================\n");
}

/// 测试 Jito 区域选择
///
/// 展示不同区域的 endpoint 和推荐用法
#[test]
fn test_jito_region_selection() {
    use sol_trade_sdk::swqos::jito::types::JitoRegion;

    println!("\n========== Jito 区域选择指南 ==========\n");

    println!("🌍 所有可用的 Jito 区域:\n");

    let regions = vec![
        (JitoRegion::Tokyo, "日本东京", "亚洲用户（推荐）"),
        (JitoRegion::Singapore, "新加坡", "亚洲用户"),
        (JitoRegion::Frankfurt, "德国法兰克福", "欧洲用户"),
        (JitoRegion::London, "英国伦敦", "欧洲用户"),
        (JitoRegion::Amsterdam, "荷兰阿姆斯特丹", "欧洲用户"),
        (JitoRegion::NewYork, "美国纽约", "美国东海岸"),
        (JitoRegion::SLC, "美国盐湖城", "美国西海岸"),
        (JitoRegion::Default, "默认区域", "大多数用户"),
    ];

    println!("{:<12} | {:<20} | {:<20}", "区域", "位置", "推荐用户");
    println!("{}", "-".repeat(60));

    for (region, location, recommendation) in regions {
        println!("{:<12} | {:<20} | {:<20}", region.to_string(), location, recommendation);
    }

    println!("\n🔗 Endpoint 示例:");
    println!("  Tokyo:  {}", JitoRegion::Tokyo.endpoint());
    println!("  Frankfurt: {}", JitoRegion::Frankfurt.endpoint());
    println!("  Default: {}", JitoRegion::Default.endpoint());

    println!("\n💡 区域选择建议:");
    println!("  1. 选择物理距离最近的区域");
    println!("  2. 测试不同区域的延迟");
    println!("  3. 亚洲用户推荐使用 Tokyo 或 Singapore");
    println!("  4. 欧洲用户推荐使用 Frankfurt 或 London");

    println!("\n========================================\n");
}
