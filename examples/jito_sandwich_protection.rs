//! Jito 三明治防护示例
//!
//! 本示例展示如何使用 Jito 三明治攻击防护功能

use sol_trade_sdk::{
    common::TradeConfig, swqos::{SwqosConfig, SwqosRegion}, TradingClient,
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🛡️  Jito 三明治防护示例\n");

    // 1. 创建默认配置（三明治防护：禁用）
    println!("1️⃣  创建默认配置（三明治防护：禁用）");
    let config_default = TradeConfig::new(
        "http://127.0.0.1:8899".to_string(),
        vec![SwqosConfig::Jito("http://127.0.0.1:8899".to_string(), SwqosRegion::Default, None)],
        CommitmentConfig::confirmed(),
    );
    println!("   enable_jito_sandwich_protection = {}\n", config_default.enable_jito_sandwich_protection);

    // 2. 创建启用防护的配置（全局启用）
    println!("2️⃣  创建启用防护的配置（全局启用）");
    let config_with_protection = TradeConfig::new(
        "http://127.0.0.1:8899".to_string(),
        vec![SwqosConfig::Jito("http://127.0.0.1:8899".to_string(), SwqosRegion::Default, None)],
        CommitmentConfig::confirmed(),
    )
    .with_jito_sandwich_protection(true)
    .with_wsol_ata_config(false, false); // 禁用 WSOL ATA 自动创建（示例不需要实际交易）
    println!("   enable_jito_sandwich_protection = {}\n", config_with_protection.enable_jito_sandwich_protection);

    // 3. 创建客户端（使用全局配置）
    println!("3️⃣  创建客户端");
    let payer = Keypair::new();
    let client = TradingClient::new(
        std::sync::Arc::new(payer),
        config_with_protection.clone(),
    )
    .await;
    println!("   客户端全局配置: enable_jito_sandwich_protection = {}\n", client.enable_jito_sandwich_protection);

    // 4. 使用建议
    println!("4️⃣  使用建议\n");

    println!("   ✅ 推荐启用防护的场景：");
    println!("      - 套利交易（对价格敏感）");
    println!("      - 大额交易（容易被 MEV bot 盯上）");
    println!("      - MEV 策略（需要确保执行顺序）");
    println!();

    println!("   ❌ 不推荐启用防护的场景：");
    println!("      - 普通 Swap（原子性已足够）");
    println!("      - 小额交易（不值得 MEV bot 抢跑）");
    println!("      - 测试交易（简单快速即可）");
    println!();

    // 5. 交易级别覆盖
    println!("5️⃣  交易级别覆盖");
    println!("   即使全局禁用，也可以在单次交易中启用防护：\n");
    println!("   let mut buy_params = TradeBuyParams::new(...);");
    println!("   buy_params.enable_jito_sandwich_protection = Some(true); // 强制启用\n");

    println!("   即使全局启用，也可以在单次交易中禁用防护：\n");
    println!("   let mut buy_params = TradeBuyParams::new(...);");
    println!("   buy_params.enable_jito_sandwich_protection = Some(false); // 强制禁用\n");

    // 6. 技术细节
    println!("6️⃣  技术细节");
    println!("   当启用防护时，SDK 会自动：");
    println!("   - 在交易中添加 jitodontfront 账户（默认：jitodontfront111111111111111111111111111111）");
    println!("   - 标记为只读账户（不消耗额外的 Compute Unit）");
    println!("   - 确保 Jito Block Engine 将此交易放在 Bundle 第一位");
    println!();

    println!("   交易大小影响：+32 bytes（一个 Pubkey）");
    println!("   Compute Unit 影响：几乎无（只读账户）");
    println!("   执行速度影响：无\n");

    // 7. 防护效果对比
    println!("7️⃣  防护效果对比\n");

    println!("   ❌ 不启用防护：");
    println!("   Bundle: [Swap, tip]");
    println!("   ⚠️  风险：攻击者可以在前后插入交易");
    println!("   攻击者: [买入, 你的 Swap, 卖出, tip] ← 你被抢跑了！\n");

    println!("   ✅ 启用防护：");
    println!("   Bundle: [Swap + jitodontfront, tip]");
    println!("   ✅ 保护：Jito Block Engine 确保你的交易在第一位");
    println!("   ❌ 攻击者无法插入：[你的 Swap + jitodontfront, tip]\n");

    println!("🎉 示例完成！\n");

    println!("📚 相关文档：");
    println!("   - Jito 官方文档: https://docs.jito.wtf/lowlatencytxnsend/#sandwich-mitigation");
    println!("   - TradeConfig::enable_jito_sandwich_protection 字段文档");

    Ok(())
}
