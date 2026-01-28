//! Jito 集成测试

use sol_trade_sdk::swqos::SwqosClientTrait;
use sol_trade_sdk::swqos::jito::{JitoClient, JitoRegion};

#[test]
fn test_jito_client_with_all_regions() {
    // 测试使用所有区域创建 JitoClient

    let regions = JitoRegion::all_regions();
    let rpc_url = "http://127.0.0.1:8899".to_string();

    for region in regions {
        let client = JitoClient::new(rpc_url.clone(), *region, String::new());

        // 验证 endpoint 正确
        assert_eq!(
            client.endpoint,
            region.endpoint(),
            "Client endpoint mismatch for region {:?}",
            region
        );

        println!("✅ Region {:?} client created: {}", region, client.endpoint);
    }

    println!("✅ 所有 9 个区域的 JitoClient 创建成功");
}

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
fn test_dont_front_account_generation() {
    // 测试 jitodontfront 账户生成

    use sol_trade_sdk::swqos::jito::generate_dont_front_account;

    // 默认账户
    let default = generate_dont_front_account(None);
    assert!(default.starts_with("jitodontfront"));

    println!("✅ 默认 dont_front account: {}", default);

    // 自定义后缀
    let custom = generate_dont_front_account(Some("myapp123"));
    assert_eq!(custom, "jitodontfrontmyapp123");
    assert!(custom.starts_with("jitodontfront"));

    println!("✅ 自定义 dont_front account: {}", custom);

    // 另一个自定义后缀
    let custom2 = generate_dont_front_account(Some("456"));
    assert_eq!(custom2, "jitodontfront456");

    println!("✅ dont_front account 生成正确");
}

#[test]
fn test_jito_swqos_type() {
    // 测试 Jito 的 SwqosType

    use sol_trade_sdk::swqos::SwqosType;

    let client = JitoClient::with_region(JitoRegion::Tokyo);
    let swqos_type = client.get_swqos_type();

    assert_eq!(swqos_type, SwqosType::Jito);

    println!("✅ JitoClient 的 SwqosType 正确");
}

#[test]
fn test_region_selection_guide() {
    // 测试区域选择指南

    println!("\n🌍 Jito 区域选择指南：\n");

    // 默认区域
    let default = JitoRegion::Default;
    println!("  默认区域: {} -> {}", default, default.endpoint());
    println!("    推荐用户: 大多数用户\n");

    // 亚洲区域
    let tokyo = JitoRegion::Tokyo;
    let singapore = JitoRegion::Singapore;
    println!("  亚洲区域:");
    println!("    Tokyo {} -> {}", tokyo, tokyo.endpoint());
    println!("    Singapore {} -> {}", singapore, singapore.endpoint());
    println!("    推荐用户: 亚洲用户（日本、新加坡、中国等）\n");

    // 欧洲区域
    let amsterdam = JitoRegion::Amsterdam;
    let dublin = JitoRegion::Dublin;
    let frankfurt = JitoRegion::Frankfurt;
    let london = JitoRegion::London;
    println!("  欧洲区域:");
    println!("    Amsterdam {} -> {}", amsterdam, amsterdam.endpoint());
    println!("    Dublin {} -> {}", dublin, dublin.endpoint());
    println!("    Frankfurt {} -> {}", frankfurt, frankfurt.endpoint());
    println!("    London {} -> {}", london, london.endpoint());
    println!("    推荐用户: 欧洲用户\n");

    // 美国区域
    let ny = JitoRegion::NewYork;
    let slc = JitoRegion::SLC;
    println!("  美国区域:");
    println!("    NewYork {} -> {}", ny, ny.endpoint());
    println!("    SLC {} -> {}", slc, slc.endpoint());
    println!("    推荐用户: NY-东海岸, SLC-西海岸\n");

    println!("✅ 区域选择指南显示完成");
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
