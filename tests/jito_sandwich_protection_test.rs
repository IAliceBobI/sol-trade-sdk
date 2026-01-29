//! Jito 三明治防护功能测试
//!
//! 测试 jitodontfront 账户生成和配置功能

use sol_trade_sdk::swqos::jito::{generate_dont_front_account, types::JitoRegion};
use sol_trade_sdk::{
    common::TradeConfig, swqos::{SwqosConfig, SwqosRegion},
};
use solana_commitment_config::CommitmentConfig;

#[test]
fn test_generate_dont_front_account_default() {
    // 测试默认 jitodontfront 账户生成
    let account = generate_dont_front_account(None);
    assert_eq!(account, "jitodontfront111111111111111111111111111111");
    println!("✅ 默认 jitodontfront 账户: {}", account);
}

#[test]
fn test_generate_dont_front_account_custom() {
    // 测试自定义后缀的 jitodontfront 账户生成
    let account = generate_dont_front_account(Some("_myapp"));
    assert_eq!(account, "jitodontfront_myapp");
    println!("✅ 自定义 jitodontfront 账户: {}", account);
}

#[test]
fn test_trade_config_default_sandwich_protection() {
    // 测试默认配置下三明治防护为禁用状态
    let config = TradeConfig::new(
        "http://127.0.0.1:8899".to_string(),
        vec![SwqosConfig::Jito("http://127.0.0.1:8899".to_string(), SwqosRegion::Default, None)],
        CommitmentConfig::confirmed(),
    );

    assert_eq!(config.enable_jito_sandwich_protection, false);
    println!("✅ 默认配置下三明治防护应为禁用: {}", config.enable_jito_sandwich_protection);
}

#[test]
fn test_trade_config_enable_sandwich_protection() {
    // 测试启用三明治防护的配置
    let config = TradeConfig::new(
        "http://127.0.0.1:8899".to_string(),
        vec![SwqosConfig::Jito("http://127.0.0.1:8899".to_string(), SwqosRegion::Default, None)],
        CommitmentConfig::confirmed(),
    )
    .with_jito_sandwich_protection(true);

    assert_eq!(config.enable_jito_sandwich_protection, true);
    println!("✅ 启用三明治防护配置: {}", config.enable_jito_sandwich_protection);
}

#[test]
fn test_trade_config_sandwich_protection_chain() {
    // 测试链式配置
    let config = TradeConfig::new(
        "http://127.0.0.1:8899".to_string(),
        vec![SwqosConfig::Jito("http://127.0.0.1:8899".to_string(), SwqosRegion::Default, None)],
        CommitmentConfig::confirmed(),
    )
    .with_jito_sandwich_protection(true)
    .with_wsol_ata_config(false, false)
    .with_callback_execution_mode(sol_trade_sdk::common::CallbackExecutionMode::Sync);

    assert_eq!(config.enable_jito_sandwich_protection, true);
    assert_eq!(config.create_wsol_ata_on_startup, false);
    assert_eq!(config.use_seed_optimize, false);
    println!("✅ 链式配置成功");
    println!("   - 三明治防护: {}", config.enable_jito_sandwich_protection);
    println!("   - WSOL ATA 创建: {}", config.create_wsol_ata_on_startup);
    println!("   - 回调模式: {:?}", config.callback_execution_mode);
}

#[test]
fn test_jito_region_endpoints() {
    // 测试 Jito 区域端点
    let regions = vec![
        (JitoRegion::Default, "https://mainnet.block-engine.jito.wtf"),
        (JitoRegion::Tokyo, "https://tokyo.mainnet.block-engine.jito.wtf"),
        (JitoRegion::Singapore, "https://singapore.mainnet.block-engine.jito.wtf"),
        (JitoRegion::NewYork, "https://ny.mainnet.block-engine.jito.wtf"),
    ];

    for (region, expected_endpoint) in regions {
        let endpoint = region.endpoint();
        assert_eq!(endpoint, expected_endpoint);
        println!("✅ 区域 {:?} -> {}", region, endpoint);
    }
}

#[test]
fn test_jito_region_from_str() {
    // 测试从字符串解析 Jito 区域
    let test_cases = vec![
        ("tokyo", JitoRegion::Tokyo),
        ("TOKYO", JitoRegion::Tokyo),
        ("ny", JitoRegion::NewYork),
        ("newyork", JitoRegion::NewYork),
        ("singapore", JitoRegion::Singapore),
        ("sg", JitoRegion::Singapore),
    ];

    for (input, expected_region) in test_cases {
        let region = JitoRegion::from_str(input).unwrap();
        assert_eq!(region, expected_region);
        println!("✅ 解析 '{}' -> {:?}", input, region);
    }
}
