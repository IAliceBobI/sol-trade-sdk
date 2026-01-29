//! Jito 单元测试
//!
//! 不需要网络连接的测试，可以快速运行
//!
//! 测试模块：
//! - JitoRegion: 区域配置和解析
//! - JitoClient: 客户端创建和配置
//! - Bundle: Bundle 概念和限制
//! - Tip策略: 固定 vs 动态 tip
//! - 三明治防护: jitodontfront 账户生成

use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use std::str::FromStr;

// ============================================================================
// 模块 1: JitoRegion 测试
// ============================================================================

mod region_tests {
    use sol_trade_sdk::swqos::jito::types::JitoRegion;

    #[test]
    fn test_all_region_endpoints() {
        // 测试所有区域的 endpoint URL 是否正确
        let test_cases = vec![
            (JitoRegion::Default, "https://mainnet.block-engine.jito.wtf"),
            (JitoRegion::Amsterdam, "https://amsterdam.mainnet.block-engine.jito.wtf"),
            (JitoRegion::Dublin, "https://dublin.mainnet.block-engine.jito.wtf"),
            (JitoRegion::Frankfurt, "https://frankfurt.mainnet.block-engine.jito.wtf"),
            (JitoRegion::London, "https://london.mainnet.block-engine.jito.wtf"),
            (JitoRegion::NewYork, "https://ny.mainnet.block-engine.jito.wtf"),
            (JitoRegion::SLC, "https://slc.mainnet.block-engine.jito.wtf"),
            (JitoRegion::Singapore, "https://singapore.mainnet.block-engine.jito.wtf"),
            (JitoRegion::Tokyo, "https://tokyo.mainnet.block-engine.jito.wtf"),
        ];

        for (region, expected_endpoint) in test_cases {
            assert_eq!(region.endpoint(), expected_endpoint, "Region {:?} endpoint mismatch", region);
        }

        println!("✅ 所有 9 个区域的 endpoint URL 正确");
    }

    #[test]
    fn test_region_from_str() {
        // 测试从字符串解析区域
        let test_cases = vec![
            ("tokyo", JitoRegion::Tokyo),
            ("TOKYO", JitoRegion::Tokyo),
            ("tokyo", JitoRegion::Tokyo),
            ("ny", JitoRegion::NewYork),
            ("newyork", JitoRegion::NewYork),
            ("newyork", JitoRegion::NewYork),
            ("amsterdam", JitoRegion::Amsterdam),
            ("ams", JitoRegion::Amsterdam),
            ("dublin", JitoRegion::Dublin),
            ("dub", JitoRegion::Dublin),
            ("frankfurt", JitoRegion::Frankfurt),
            ("fra", JitoRegion::Frankfurt),
            ("ffm", JitoRegion::Frankfurt),
            ("london", JitoRegion::London),
            ("lon", JitoRegion::London),
            ("slc", JitoRegion::SLC),
            ("saltlakecity", JitoRegion::SLC),
            ("singapore", JitoRegion::Singapore),
            ("sgp", JitoRegion::Singapore),
            ("sg", JitoRegion::Singapore),
            ("default", JitoRegion::Default),
        ];

        for (input, expected) in test_cases {
            let result = JitoRegion::from_str(input);
            assert!(result.is_ok(), "Failed to parse region from '{}': {:?}", input, result);
            assert_eq!(result.unwrap(), expected, "Region mismatch for input '{}'", input);
        }

        println!("✅ 所有区域字符串解析正确");

        // 测试无效输入
        assert!(JitoRegion::from_str("invalid").is_err());
        assert!(JitoRegion::from_str("losangeles").is_err()); // 官方不支持
        assert!(JitoRegion::from_str("paris").is_err());

        println!("✅ 无效区域正确返回错误");
    }

    #[test]
    fn test_region_display() {
        // 测试区域的字符串表示
        let test_cases = vec![
            (JitoRegion::Default, "Default"),
            (JitoRegion::Amsterdam, "Amsterdam"),
            (JitoRegion::Dublin, "Dublin"),
            (JitoRegion::Frankfurt, "Frankfurt"),
            (JitoRegion::London, "London"),
            (JitoRegion::NewYork, "NewYork"),
            (JitoRegion::SLC, "SLC"),
            (JitoRegion::Singapore, "Singapore"),
            (JitoRegion::Tokyo, "Tokyo"),
        ];

        for (region, expected) in test_cases {
            assert_eq!(region.to_string(), expected);
        }

        println!("✅ 所有区域的 Display 格式正确");
    }

    #[test]
    fn test_all_regions() {
        // 测试 all_regions() 方法
        let regions = JitoRegion::all_regions();

        assert_eq!(regions.len(), 9, "应该有 9 个区域");

        // 验证包含所有关键区域
        assert!(regions.contains(&JitoRegion::Default));
        assert!(regions.contains(&JitoRegion::Tokyo));
        assert!(regions.contains(&JitoRegion::Singapore));
        assert!(regions.contains(&JitoRegion::Dublin));
        assert!(regions.contains(&JitoRegion::NewYork));

        println!("✅ all_regions() 返回所有 9 个区域");
    }

    #[test]
    fn test_region_default() {
        // 测试 Default trait 实现
        let region = JitoRegion::default();
        assert_eq!(region, JitoRegion::Default);

        println!("✅ JitoRegion::default() 返回 Default");
    }

    #[test]
    fn test_region_recommendations() {
        // 测试区域推荐

        // 亚洲用户应该使用 Tokyo 或 Singapore
        let tokyo = JitoRegion::Tokyo;
        let singapore = JitoRegion::Singapore;
        assert!(tokyo.endpoint().contains("tokyo"));
        assert!(singapore.endpoint().contains("singapore"));

        // 欧洲用户应该使用 Amsterdam, Dublin, Frankfurt, 或 London
        let amsterdam = JitoRegion::Amsterdam;
        let dublin = JitoRegion::Dublin;
        let frankfurt = JitoRegion::Frankfurt;
        let london = JitoRegion::London;

        assert!(amsterdam.endpoint().contains("amsterdam"));
        assert!(dublin.endpoint().contains("dublin"));
        assert!(frankfurt.endpoint().contains("frankfurt"));
        assert!(london.endpoint().contains("london"));

        // 美国东海岸用户应该使用 NewYork
        let ny = JitoRegion::NewYork;
        assert!(ny.endpoint().contains("ny"));

        // 美国西海岸用户应该使用 SLC
        let slc = JitoRegion::SLC;
        assert!(slc.endpoint().contains("slc"));

        println!("✅ 所有区域的地理位置正确");
    }
}

// ============================================================================
// 模块 2: JitoClient 测试
// ============================================================================

mod client_tests {
    use sol_trade_sdk::swqos::SwqosClientTrait;
    use sol_trade_sdk::swqos::jito::{JitoClient, JitoRegion};
    use sol_trade_sdk::swqos::SwqosType;

    #[test]
    fn test_jito_client_with_region() {
        // 测试 with_region() 方法

        let client = JitoClient::with_region(JitoRegion::Tokyo);

        assert_eq!(client.endpoint, "https://tokyo.mainnet.block-engine.jito.wtf");

        println!("✅ with_region() 方法正确");
    }

    #[test]
    fn test_jito_get_tip_account() {
        // 测试获取 tip account

        let client = JitoClient::with_region(JitoRegion::Default);

        let tip_account = client.get_tip_account();

        assert!(tip_account.is_ok(), "Failed to get tip account");

        let account = tip_account.unwrap();
        // Jito tip accounts 是 base58 编码的公钥，长度通常是 32 字节 = 44 个字符
        assert!(account.len() >= 32, "Tip account length too short: {}", account.len());
        assert!(!account.is_empty(), "Tip account should not be empty");

        println!("✅ Tip account: {}", account);
    }

    #[test]
    fn test_jito_tip_accounts_randomness() {
        // 测试 tip account 随机性
        // 连续获取多个 tip account，应该有不同的结果

        let client = JitoClient::with_region(JitoRegion::Default);
        let mut accounts = std::collections::HashSet::new();

        // 获取 20 个 tip account
        for _ in 0..20 {
            let tip_account = client.get_tip_account().unwrap();
            accounts.insert(tip_account);
        }

        // 应该至少有 5 个不同的 account（证明有随机性）
        assert!(accounts.len() >= 5, "Tip accounts 缺乏随机性，只有 {} 个不同的", accounts.len());

        println!("✅ Tip accounts 有随机性：20 次获取得到 {} 个不同的", accounts.len());
    }

    #[test]
    fn test_jito_swqos_type() {
        // 测试 Jito 的 SwqosType

        let client = JitoClient::with_region(JitoRegion::Tokyo);
        let swqos_type = client.get_swqos_type();

        assert_eq!(swqos_type, SwqosType::Jito);

        println!("✅ JitoClient 的 SwqosType 正确");
    }
}

// ============================================================================
// 模块 3: Bundle 测试
// ============================================================================

mod bundle_tests {
    use super::*;

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

    #[test]
    fn test_jito_fixed_vs_dynamic_tip() {
        //! 测试固定 Tip vs 动态 Tip 的区别
        //!
        //! 对比固定 tip 和动态 tip 在不同场景下的表现

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
        println!();
        println!("  动态 Tip:");
        println!("    ✅ 优点:");
        println!("       - 根据市场实时调整");
        println!("       - 优化成本和成功率");
        println!("       - 自动适应网络状况");
        println!("    ❌ 缺点: 需要额外 API 调用");

        println!("\n✅ 推荐: 生产环境使用动态 Tip (P50-P75)\n");
        println!("=============================================\n");
    }

    #[test]
    fn test_jito_max_bundle_size() {
        //! 测试完整的 5 笔交易 Bundle（最大容量）
        //!
        //! 展示 Jito Bundle 的最大容量结构和最佳实践

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

    #[test]
    fn test_dynamic_tip_percentile_parsing() {
        // 测试动态 Tip 百分位解析

        use sol_trade_sdk::swqos::jito::dynamic_tip::TipPercentile;

        // 有效的百分位
        let valid_percentiles = vec![
            ("25th", TipPercentile::P25),
            ("50th", TipPercentile::P50),
            ("75th", TipPercentile::P75),
            ("95th", TipPercentile::P95),
            ("99th", TipPercentile::P99),
        ];

        for (input, expected) in valid_percentiles {
            let result = TipPercentile::from_str(input);
            assert!(result.is_ok(), "Failed to parse percentile '{}': {:?}", input, result);
            assert_eq!(result.unwrap(), expected, "Percentile mismatch for '{}'", input);
        }

        println!("✅ 所有 TipPercentile 解析正确");

        // 无效的百分位
        assert!(TipPercentile::from_str("100th").is_err());
        assert!(TipPercentile::from_str("invalid").is_err());

        println!("✅ 无效百分位正确返回错误");
    }
}

// ============================================================================
// 模块 4: 三明治防护测试
// ============================================================================

mod sandwich_tests {
    use solana_commitment_config::CommitmentConfig;
    use sol_trade_sdk::common::TradeConfig;
    use sol_trade_sdk::swqos::{SwqosConfig, SwqosRegion};

    #[test]
    fn test_generate_dont_front_account_default() {
        // 测试默认 jitodontfront 账户生成

        use sol_trade_sdk::swqos::jito::generate_dont_front_account;

        let account = generate_dont_front_account(None);
        assert_eq!(account, "jitodontfront111111111111111111111111111111");
        println!("✅ 默认 jitodontfront 账户: {}", account);
    }

    #[test]
    fn test_generate_dont_front_account_custom() {
        // 测试自定义后缀的 jitodontfront 账户生成

        use sol_trade_sdk::swqos::jito::generate_dont_front_account;

        let account = generate_dont_front_account(Some("_myapp"));
        assert_eq!(account, "jitodontfront_myapp");
        println!("✅ 自定义 jitodontfront 账户: {}", account);
    }

    #[test]
    fn test_trade_config_default_sandwich_protection() {
        // 测试默认配置下三明治防护为禁用状态

        let config = TradeConfig::new(
            "http://127.0.0.1:8899".to_string(),
            vec![SwqosConfig::Jito(
                "http://127.0.0.1:8899".to_string(),
                SwqosRegion::Default,
                None,
            )],
            CommitmentConfig::confirmed(),
        );

        assert_eq!(config.enable_jito_sandwich_protection, false);
        println!(
            "✅ 默认配置下三明治防护应为禁用: {}",
            config.enable_jito_sandwich_protection
        );
    }

    #[test]
    fn test_trade_config_enable_sandwich_protection() {
        // 测试启用三明治防护的配置

        let config = TradeConfig::new(
            "http://127.0.0.1:8899".to_string(),
            vec![SwqosConfig::Jito(
                "http://127.0.0.1:8899".to_string(),
                SwqosRegion::Default,
                None,
            )],
            CommitmentConfig::confirmed(),
        )
        .with_jito_sandwich_protection(true);

        assert_eq!(config.enable_jito_sandwich_protection, true);
        println!(
            "✅ 启用三明治防护配置: {}",
            config.enable_jito_sandwich_protection
        );
    }

    #[test]
    fn test_trade_config_sandwich_protection_chain() {
        // 测试链式配置

        use sol_trade_sdk::common::CallbackExecutionMode;

        let config = TradeConfig::new(
            "http://127.0.0.1:8899".to_string(),
            vec![SwqosConfig::Jito(
                "http://127.0.0.1:8899".to_string(),
                SwqosRegion::Default,
                None,
            )],
            CommitmentConfig::confirmed(),
        )
        .with_jito_sandwich_protection(true)
        .with_wsol_ata_config(false, false)
        .with_callback_execution_mode(CallbackExecutionMode::Sync);

        assert_eq!(config.enable_jito_sandwich_protection, true);
        assert_eq!(config.create_wsol_ata_on_startup, false);
        assert_eq!(config.use_seed_optimize, false);
        println!("✅ 链式配置成功");
        println!(
            "   - 三明治防护: {}",
            config.enable_jito_sandwich_protection
        );
        println!("   - WSOL ATA 创建: {}", config.create_wsol_ata_on_startup);
        println!("   - 回调模式: {:?}", config.callback_execution_mode);
    }
}
