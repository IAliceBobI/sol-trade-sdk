//! 空投相关功能

use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey};

/// 为测试账户空投 SOL 并循环等待到账
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `payer` - 账户公钥
/// * `amount_sol` - 空投的 SOL 数量
///
/// # 返回
/// * `Ok(())` - 空投成功
/// * `Err(String)` - 空投失败
pub async fn airdrop_and_wait(
    rpc_url: &str,
    payer: &Pubkey,
    amount_sol: u64,
) -> Result<(), String> {
    let client = RpcClient::new(rpc_url.to_string());
    let amount_lamports = amount_sol * LAMPORTS_PER_SOL;

    // 尝试空投
    println!("💰 空投 {} SOL 到测试账户...", amount_sol);
    match client.request_airdrop(payer, amount_lamports).await {
        Ok(sig) => {
            println!("✅ 空投成功，签名: {}", sig);
            // 循环等待余额到账
            println!("⏳ 等待余额到账...");
            let mut retries = 0;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                match client.get_balance(payer).await {
                    Ok(balance) => {
                        if balance >= amount_lamports {
                            println!(
                                "✅ 余额已到账: {} lamports ({:.2} SOL)\n",
                                balance,
                                balance as f64 / 1_000_000_000.0
                            );
                            return Ok(());
                        }
                        retries += 1;
                        if retries > 20 {
                            return Err(format!("等待超时，当前余额: {} lamports", balance));
                        }
                    },
                    Err(e) => {
                        return Err(format!("查询余额失败: {}", e));
                    },
                }
            }
        },
        Err(e) => Err(format!("空投失败: {}", e)),
    }
}
