//! 统一的 Token 工具函数
//!
//! 提供跨项目使用的 Token 相关工具函数

use crate::constants::{SOL_MINT, USDC_MINT, USDT_MINT, RAY_MINT};
use anyhow::Result;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;
use spl_token::solana_program::program_pack::Pack;
use spl_token::state::Mint;

// Increased cache sizes for better performance
const MAX_TOKEN_METADATA_CACHE_SIZE: usize = 100_000;

/// 全局 Token Decimal 缓存
static DECIMALS_CACHE: Lazy<DashMap<Pubkey, u8>> =
    Lazy::new(|| DashMap::with_capacity(MAX_TOKEN_METADATA_CACHE_SIZE));

/// 全局 Token Symbol 缓存
static SYMBOL_CACHE: Lazy<DashMap<Pubkey, String>> =
    Lazy::new(|| DashMap::with_capacity(MAX_TOKEN_METADATA_CACHE_SIZE));

/// 获取代币精度（统一实现，支持 Token 和 Token2022）
///
/// 使用全局缓存减少 RPC 调用
pub async fn get_token_decimals(
    rpc: &crate::common::SolanaRpcClient,
    mint: &Pubkey,
) -> Result<u8> {
    // Fast path: 检查缓存
    if let Some(cached) = DECIMALS_CACHE.get(mint) {
        return Ok(*cached);
    }

    let account = rpc.get_account(mint).await?;

    // 尝试解析为传统 Token 程序的 Mint
    if let Ok(mint_account) = Mint::unpack(&account.data) {
        let decimals = mint_account.decimals;
        DECIMALS_CACHE.insert(*mint, decimals);
        return Ok(decimals);
    }

    // 尝试解析为 Token2022 的 Mint
    use spl_token_2022::extension::StateWithExtensions;
    use spl_token_2022::state::Mint as Mint2022;
    if let Ok(mint_account) = StateWithExtensions::<Mint2022>::unpack(&account.data) {
        let decimals = mint_account.base.decimals;
        DECIMALS_CACHE.insert(*mint, decimals);
        return Ok(decimals);
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
/// 使用全局缓存减少 RPC 调用
pub async fn get_token_symbol(
    rpc: &crate::common::SolanaRpcClient,
    mint: &Pubkey,
) -> Result<String> {
    // Fast path: 检查缓存
    if let Some(cached) = SYMBOL_CACHE.get(mint) {
        return Ok(cached.clone());
    }

    let account = rpc.get_account(mint).await?;

    // 尝试解析为 Token2022 的 Mint（支持 Metadata Extension）
    if account.owner == spl_token_2022::ID {
        use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
        use spl_token_2022::state::Mint as Mint2022;
        if let Ok(mint_account) = StateWithExtensions::<Mint2022>::unpack(&account.data) {
            if let Ok(metadata) = mint_account
                .get_variable_len_extension::<spl_token_metadata_interface::state::TokenMetadata>()
            {
                let symbol = metadata.symbol.to_string();
                if !symbol.is_empty() {
                    SYMBOL_CACHE.insert(*mint, symbol.clone());
                    return Ok(symbol);
                }
            }
        }
    }

    // 传统 Token 程序不支持链上 metadata
    // 使用硬编码兜底方案
    let symbol = get_known_token_symbol(mint);
    SYMBOL_CACHE.insert(*mint, symbol.clone());
    Ok(symbol)
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

/// 获取已知代币的 Symbol（硬编码兜底方案）
///
/// 当链上无法获取 symbol 时（如传统 SPL Token），使用此函数作为兜底
/// 适用于日志记录、调试显示等场景
pub fn get_known_token_symbol(mint: &Pubkey) -> String {
    if *mint == SOL_MINT {
        "SOL".to_string()
    } else if *mint == USDC_MINT {
        "USDC".to_string()
    } else if *mint == USDT_MINT {
        "USDT".to_string()
    } else if *mint == RAY_MINT {
        "RAY".to_string()
    } else {
        "".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_token_decimals_wsol() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let wsol = Pubkey::from_str_const("So11111111111111111111111111111111111111112");
        
        let decimals = get_token_decimals(&rpc, &wsol).await.unwrap();
        assert_eq!(decimals, 9);
    }

    #[tokio::test]
    async fn test_get_token_decimals_usdc() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let usdc = Pubkey::from_str_const("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        
        let decimals = get_token_decimals(&rpc, &usdc).await.unwrap();
        assert_eq!(decimals, 6);
    }

    #[tokio::test]
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

    #[tokio::test]
    async fn test_get_token_decimals_pumpfun() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        // Pump.fun 测试代币
        let pump = Pubkey::from_str_const("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn");

        let decimals = get_token_decimals(&rpc, &pump).await.unwrap();
        assert_eq!(decimals, 6, "Pump.fun token decimals should be 6");
        println!("Pump.fun token decimals: {}", decimals);
    }

    #[tokio::test]
    async fn test_decimals_cache_miss() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let wsol = Pubkey::from_str_const("So11111111111111111111111111111111111111112");

        // 清除缓存确保冷启动
        DECIMALS_CACHE.remove(&wsol);

        let decimals = get_token_decimals(&rpc, &wsol).await.unwrap();
        assert_eq!(decimals, 9);
    }

    #[tokio::test]
    async fn test_symbol_cache_miss() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let pump = Pubkey::from_str_const("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn");

        // 清除缓存确保冷启动
        SYMBOL_CACHE.remove(&pump);

        let symbol = get_token_symbol(&rpc, &pump).await.unwrap();
        assert!(!symbol.is_empty());
    }

    #[tokio::test]
    async fn test_decimals_cache_hit() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let usdc = Pubkey::from_str_const("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

        // 先触发一次缓存填充
        let _ = get_token_decimals(&rpc, &usdc).await.unwrap();

        // 第二次调用应该命中缓存
        let decimals1 = get_token_decimals(&rpc, &usdc).await.unwrap();
        let decimals2 = get_token_decimals(&rpc, &usdc).await.unwrap();

        assert_eq!(decimals1, 6);
        assert_eq!(decimals2, 6);
        assert_eq!(decimals1, decimals2);
    }

    #[tokio::test]
    async fn test_symbol_cache_hit() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        let pump = Pubkey::from_str_const("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn");

        // 先触发一次缓存填充
        let _ = get_token_symbol(&rpc, &pump).await.unwrap();

        // 第二次调用应该命中缓存
        let symbol1 = get_token_symbol(&rpc, &pump).await.unwrap();
        let symbol2 = get_token_symbol(&rpc, &pump).await.unwrap();

        assert!(!symbol1.is_empty());
        assert_eq!(symbol1, symbol2);
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
