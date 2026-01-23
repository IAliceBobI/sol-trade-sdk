//! Seed 优化集成测试
//!
//! 使用 surfpool (localhost:8899) 进行测试
//!
//! 运行测试:
//!     cargo test --test seed_optimize_tests -- --nocapture
//!
//! 注意：需要确保 surfpool 正在运行

use sol_trade_sdk::{
    common::fast_fn::{
        get_associated_token_address_with_program_id_fast,
        get_associated_token_address_with_program_id_fast_use_seed,
    },
    constants::TOKEN_PROGRAM_2022,
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;

mod test_helpers;
use test_helpers::{create_test_client_with_seed_optimize, print_seed_optimize_balances};

/// Token 地址常量 (带 pump 前缀)
const PUMP_MINT: &str = "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn";

/// 测试：seed 优化生成的 ATA 地址特性 (Token-2022)
///
/// 发现：seed 优化生成的 ATA 地址与标准地址不同
/// 这是预期的行为，因为 seed 优化使用确定性 seed 生成地址
/// Pump 是 Token-2022，使用 TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb
#[tokio::test]
async fn test_seed_ata_address_characteristics() {
    let client = create_test_client_with_seed_optimize(true).await;
    let payer_pubkey = client.payer.try_pubkey().expect("Failed to get payer pubkey");
    let rpc_url = "http://127.0.0.1:8899".to_string();

    println!("=== 测试 seed 优化 ATA 地址特性 (Token-2022) ===");
    println!("钱包地址: {}", payer_pubkey);

    // 解析 token mint
    let mint = Pubkey::from_str(PUMP_MINT)
        .unwrap_or_else(|_| panic!("Invalid mint address: {}", PUMP_MINT));
    println!("Token Mint: {}", mint);
    println!("Token Program: Token-2022");

    // 使用标准方式计算 ATA 地址 (Token-2022)
    let standard_ata = get_associated_token_address_with_program_id_fast(
        &payer_pubkey,
        &mint,
        &TOKEN_PROGRAM_2022, // Pump 使用 Token-2022
    );
    println!("标准 ATA 地址: {}", standard_ata);

    // 使用 seed 优化计算 ATA 地址 (Token-2022)
    let seed_ata = get_associated_token_address_with_program_id_fast_use_seed(
        &payer_pubkey,
        &mint,
        &TOKEN_PROGRAM_2022, // Pump 使用 Token-2022
        true,                // 启用 seed 优化
    );
    println!("Seed 优化 ATA 地址: {}", seed_ata);

    // 重要发现：两个地址不同！
    // 这是预期的行为，seed 优化使用确定性 seed 生成不同的地址
    assert_ne!(standard_ata, seed_ata, "Seed 优化地址应该与标准地址不同（这是预期行为）");
    println!("✅ Seed 优化生成了不同的 ATA 地址（预期行为）");

    // 验证两个地址都是有效的 Pubkey (32 字节)
    assert_eq!(standard_ata.as_ref().len(), 32, "标准 ATA 应该是 32 字节");
    assert_eq!(seed_ata.as_ref().len(), 32, "Seed ATA 应该是 32 字节");
    println!("✅ 两个地址都是有效的 32 字节 Pubkey");

    // 验证都不是零地址
    assert!(!standard_ata.as_ref().iter().all(|&b| b == 0), "标准 ATA 不应为零地址");
    assert!(!seed_ata.as_ref().iter().all(|&b| b == 0), "Seed ATA 不应为零地址");
    println!("✅ 两个地址都不是零地址");

    // 打印 4 个地址的余额信息
    if let Err(e) = print_seed_optimize_balances(&rpc_url, &payer_pubkey, &mint).await {
        println!("⚠️  打印余额信息失败: {}", e);
    }

    println!("=== Seed ATA 地址特性测试通过 (Token-2022) ===");
    println!("注意：Seed 优化会生成确定性但不同的 ATA 地址");
    println!("     需要使用 create_associated_token_account_use_seed 创建账户");
}
