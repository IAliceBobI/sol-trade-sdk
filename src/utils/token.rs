//! 统一的 Token 工具函数
//!
//! 提供跨项目使用的 Token 相关工具函数

use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use spl_token::solana_program::program_pack::Pack;
use spl_token::state::Mint;

/// 获取代币精度（统一实现，支持 Token 和 Token2022）
///
/// 这是项目中所有 get_token_decimals 的标准实现
/// 所有模块应引用此函数，避免重复实现
pub async fn get_token_decimals(
    rpc: &crate::common::SolanaRpcClient,
    mint: &Pubkey,
) -> Result<u8> {
    let account = rpc.get_account(mint).await?;

    // 尝试解析为传统 Token 程序的 Mint
    if let Ok(mint_account) = Mint::unpack(&account.data) {
        return Ok(mint_account.decimals);
    }

    // 尝试解析为 Token2022 的 Mint
    use spl_token_2022::extension::StateWithExtensions;
    use spl_token_2022::state::Mint as Mint2022;
    if let Ok(mint_account) = StateWithExtensions::<Mint2022>::unpack(&account.data) {
        return Ok(mint_account.base.decimals);
    }

    Err(anyhow::anyhow!(
        "无法解析 mint 账户数据: {} (数据长度: {}, owner: {})",
        mint,
        account.data.len(),
        account.owner
    ))
}

/// 获取代币 Symbol（支持 Token 和 Token2022 Metadata Extension）
///
/// 支持：
/// - 传统 spl-token：不支持链上 metadata，返回空字符串
/// - Token2022 metadata extension：直接从链上读取 symbol
///
/// 注意：Symbol 可能存储在链下（如 Jupiter Token List），此时返回空字符串
pub async fn get_token_symbol(
    rpc: &crate::common::SolanaRpcClient,
    mint: &Pubkey,
) -> Result<String> {
    let account = rpc.get_account(mint).await?;

    // 尝试解析为 Token2022 的 Mint（支持 Metadata Extension）
    if account.owner == spl_token_2022::ID {
        use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
        use spl_token_2022::state::Mint as Mint2022;
        if let Ok(mint_account) = StateWithExtensions::<Mint2022>::unpack(&account.data) {
            // get_variable_len_extension 已经返回解析好的类型
            if let Ok(metadata) = mint_account
                .get_variable_len_extension::<spl_token_metadata_interface::state::TokenMetadata>()
            {
                let symbol = metadata.symbol.to_string();
                if !symbol.is_empty() {
                    return Ok(symbol);
                }
            }
        }
    }

    // 传统 Token 程序不支持链上 metadata，或未找到 metadata 扩展
    Ok(String::new())
}

/// 计算 ATA 地址（自动识别 Token Program）
pub async fn calculate_ata(
    rpc: &crate::common::SolanaRpcClient,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Pubkey> {
    let account = rpc.get_account(mint).await?;
    let token_program = account.owner;

    Ok(spl_associated_token_account::get_associated_token_address_with_program_id(
        owner,
        mint,
        &token_program,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要 RPC 连接
    async fn test_get_token_decimals_wsol() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let wsol = Pubkey::from_str_const("So11111111111111111111111111111111111111112");
        
        let decimals = get_token_decimals(&rpc, &wsol).await.unwrap();
        assert_eq!(decimals, 9);
    }

    #[tokio::test]
    #[ignore] // 需要 RPC 连接
    async fn test_get_token_decimals_usdc() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let usdc = Pubkey::from_str_const("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        
        let decimals = get_token_decimals(&rpc, &usdc).await.unwrap();
        assert_eq!(decimals, 6);
    }

    #[tokio::test]
    #[ignore] // 需要 RPC 连接
    async fn test_get_token_symbol_pumpfun() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        // Pump.fun 测试代币
        let pump = Pubkey::from_str_const("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn");
        
        let symbol = get_token_symbol(&rpc, &pump).await.unwrap();
        // Pump.fun 代币的 symbol 应该在链上
        assert!(!symbol.is_empty(), "Pump.fun token should have symbol");
        println!("Pump.fun token symbol: {}", symbol);
    }
}

use std::str::FromStr;

#[allow(dead_code)]
trait PubkeyExt {
    fn from_str_const(s: &str) -> Self;
}

impl PubkeyExt for Pubkey {
    fn from_str_const(s: &str) -> Self {
        Pubkey::from_str(s).expect("Invalid pubkey")
    }
}
