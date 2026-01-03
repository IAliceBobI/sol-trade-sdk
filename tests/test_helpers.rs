//! 测试工具函数
//!
//! 提供测试用的辅助函数，包括 SOL 空投和测试客户端创建

use sol_trade_sdk::{
    common::fast_fn::get_associated_token_address_with_program_id_fast,
    common::TradeConfig,
    constants::{TOKEN_PROGRAM, WSOL_TOKEN_ACCOUNT},
    swqos::SwqosConfig,
    SolanaTrade,
};
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
    let trade_config =
        TradeConfig::new(rpc_url, swqos_configs, commitment).with_wsol_ata_config(true, false);
    SolanaTrade::new(Arc::new(payer), trade_config).await
}

/// 获取账户的 WSOL ATA 地址
#[inline]
pub fn get_wsol_ata_address(payer: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id_fast(payer, &WSOL_TOKEN_ACCOUNT, &TOKEN_PROGRAM)
}

/// 打印并返回账户的 SOL 和 WSOL 余额（同时使用 get_balance 和 get_token_account_balance）
/// 如果 WSOL 账户不存在（已关闭），返回 (sol_balance, 0)
pub async fn print_balances(
    rpc_url: &str,
    payer: &Pubkey,
) -> Result<(u64, u64), Box<dyn std::error::Error>> {
    let client = RpcClient::new(rpc_url.to_string());

    // 获取 SOL 余额
    let sol_balance = client.get_balance(payer).await?;

    // 获取 WSOL ATA 地址
    let wsol_ata = get_wsol_ata_address(payer);

    // 方式1: 使用 get_balance 获取 WSOL 余额（账户不存在时返回 0）
    let wsol_balance = match client.get_balance(&wsol_ata).await {
        Ok(balance) => balance,
        Err(e) => {
            println!("⚠️  get_balance 查询 WSOL 账户失败: {}，视为余额 0", e);
            0
        }
    };

    // 方式2: 使用 get_token_account_balance 获取 WSOL 余额（账户不存在时返回 0）
    let (wsol_amount, wsol_decimals, wsol_ui_amount_str) =
        match client.get_token_account_balance(&wsol_ata).await {
            Ok(token) => {
                let amount: u64 = token.amount.parse().unwrap_or(0);
                (amount, token.decimals, token.ui_amount_string)
            }
            Err(e) => {
                println!(
                    "⚠️  get_token_account_balance 查询 WSOL 账户失败: {}，视为余额 0",
                    e
                );
                (0, 9, "0".to_string())
            }
        };

    println!("\n========== 账户余额 ==========");
    println!("账户地址: {}", payer);
    println!("WSOL ATA: {}", wsol_ata);
    println!("--------------------------------");
    println!(
        "💰 SOL 余额: {} lamports ({:.4} SOL)",
        sol_balance,
        sol_balance as f64 / LAMPORTS_PER_SOL as f64
    );
    println!(
        "🪙 WSOL 余额 (get_balance): {} lamports ({:.4} SOL)",
        wsol_balance,
        wsol_balance as f64 / LAMPORTS_PER_SOL as f64
    );
    println!(
        "🪙 WSOL 余额 (get_token_account_balance): {} lamports",
        wsol_amount
    );
    println!(
        "🪙 WSOL uiAmountString: {} (decimals: {})",
        wsol_ui_amount_str, wsol_decimals
    );
    println!("================================\n");

    Ok((sol_balance, wsol_amount))
}
