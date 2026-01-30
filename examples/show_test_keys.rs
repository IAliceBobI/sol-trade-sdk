// 简单的测试密钥显示工具
// 使用方法: cargo run --example show_test_keys

use solana_sdk::signature::{Signer, read_keypair_file};
use std::env;

fn main() {
    // 获取环境变量
    let key_path1 = env::var("SOLANA_TEST_KEY_PATH1")
        .unwrap_or_else(|_| "/opt/projects/rstool/config/keys/id1.json".to_string());
    let key_path2 = env::var("SOLANA_TEST_KEY_PATH2")
        .unwrap_or_else(|_| "/opt/projects/rstool/config/keys/id2.json".to_string());

    println!("=== Solana 测试密钥信息 ===\n");

    // 读取第一个密钥
    println!("📁 SOLANA_TEST_KEY_PATH1: {}", key_path1);
    match read_and_display_key(&key_path1) {
        Ok(_) => println!(),
        Err(e) => println!("❌ 读取失败: {}\n", e),
    }

    // 读取第二个密钥
    println!("📁 SOLANA_TEST_KEY_PATH2: {}", key_path2);
    match read_and_display_key(&key_path2) {
        Ok(_) => println!(),
        Err(e) => println!("❌ 读取失败: {}\n", e),
    }
}

/// 读取并显示密钥文件信息
fn read_and_display_key(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 使用 solana_sdk 的 read_keypair_file 函数
    let keypair = read_keypair_file(path)?;

    // 获取地址
    let address = keypair.pubkey();

    // 获取 Base58 格式的私钥
    let secret_bytes = keypair.secret_bytes();
    let secret_key = bs58::encode(secret_bytes).into_string();

    println!("   地址: {}", address);
    println!("   私钥 (Base58): {}", secret_key);
    println!("   ✅ 读取成功");

    Ok(())
}
