//! 测试工具函数
//!
//! 提供测试用的辅助函数，包括 SOL 空投和测试客户端创建

use sol_trade_sdk::{common::TradeConfig, swqos::SwqosConfig, SolanaTrade};
use solana_commitment_config::CommitmentConfig;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer,
};
use std::sync::Arc;

/// 为测试账户自动空投 SOL
pub async fn airdrop_to_payer(
    rpc_url: &str,
    payer: &Pubkey,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = RpcClient::new(rpc_url.to_string());

    // 检查账户余额
    let balance = client.get_balance(payer).await?;
    println!("账户余额: {} lamports ({:.4} SOL)", balance, balance as f64 / 1e9);

    // 如果余额小于 2 SOL，自动请求空投
    if balance < 2 * LAMPORTS_PER_SOL {
        println!("💧 请求 2 SOL 空投...");
        let airdrop_signature = client.request_airdrop(payer, 2 * LAMPORTS_PER_SOL).await?;
        println!("📤 空投交易签名: {}", airdrop_signature);

        // 等待空投确认
        loop {
            let confirmed = client.confirm_transaction(&airdrop_signature).await?;
            if confirmed {
                break;
            }
        }

        // 验证余额
        let new_balance = client.get_balance(payer).await?;
        println!(
            "✅ 空投成功！新余额: {} lamports ({:.4} SOL)",
            new_balance,
            new_balance as f64 / 1e9
        );
    } else {
        println!("✅ 账户余额充足");
    }
    Ok(())
}

/// 创建测试用的 SolanaTrade 客户端
pub async fn create_test_client() -> SolanaTrade {
    let rpc_url = "http://127.0.0.1:8899".to_string();

    // 使用 Keypair::new() 生成随机测试账户
    let payer = Keypair::new();

    // 空投 SOL
    let payer_pubkey = payer.pubkey();
    let _ = airdrop_to_payer(&rpc_url, &payer_pubkey).await;

    let commitment = CommitmentConfig::confirmed();
    let swqos_configs: Vec<SwqosConfig> = vec![SwqosConfig::Default(rpc_url.clone())];
    let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment);

    SolanaTrade::new(Arc::new(payer), trade_config).await
}
