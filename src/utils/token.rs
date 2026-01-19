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

/// Mint 账户信息（包含所有元数据）
#[derive(Clone, Debug)]
pub struct MintInfo {
    /// 代币精度
    pub decimals: u8,
    /// 代币符号
    pub symbol: String,
    /// 是否为 Token2022
    pub is_token2022: bool,
}

/// 全局 MintInfo 缓存（包含 decimals、symbol、is_token2022）
static MINT_INFO_CACHE: Lazy<DashMap<Pubkey, MintInfo>> =
    Lazy::new(|| DashMap::with_capacity(MAX_TOKEN_METADATA_CACHE_SIZE));

/// 获取缓存的 MintInfo，不存在则返回 None
pub fn get_cached_mint_info(mint: &Pubkey) -> Option<MintInfo> {
    MINT_INFO_CACHE.get(mint).map(|info| info.clone())
}

/// 获取 Mint 账户的完整信息（统一实现，支持 Token 和 Token2022）
///
/// 使用全局缓存减少 RPC 调用
pub async fn get_mint_info(
    rpc: &crate::common::SolanaRpcClient,
    mint: &Pubkey,
) -> Result<MintInfo> {
    let account = rpc.get_account(mint).await?;
    let is_token2022 = account.owner == spl_token_2022::ID;

    // 尝试解析为传统 Token 程序的 Mint
    if !is_token2022 {
        if let Ok(mint_account) = Mint::unpack(&account.data) {
            let info = MintInfo {
                decimals: mint_account.decimals,
                symbol: get_known_token_symbol(mint),
                is_token2022: false,
            };
            MINT_INFO_CACHE.insert(*mint, info.clone());
            return Ok(info);
        }
    }

    // 尝试解析为 Token2022 的 Mint
    if is_token2022 {
        use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
        use spl_token_2022::state::Mint as Mint2022;
        if let Ok(mint_account) = StateWithExtensions::<Mint2022>::unpack(&account.data) {
            let decimals = mint_account.base.decimals;

            // 尝试获取 TokenMetadata 中的 symbol
            let symbol = if let Ok(metadata) = mint_account
                .get_variable_len_extension::<spl_token_metadata_interface::state::TokenMetadata>()
            {
                let s = metadata.symbol.to_string();
                if s.is_empty() {
                    get_known_token_symbol(mint)
                } else {
                    s
                }
            } else {
                get_known_token_symbol(mint)
            };

            let info = MintInfo {
                decimals,
                symbol,
                is_token2022: true,
            };
            MINT_INFO_CACHE.insert(*mint, info.clone());
            return Ok(info);
        }
    }

    Err(anyhow::anyhow!(
        "无法解析 mint 账户数据: {} (数据长度: {}, owner: {})",
        mint,
        account.data.len(),
        account.owner
    ))
}

/// 获取代币精度（统一实现，支持 Token 和 Token2022）
///
/// 使用全局缓存减少 RPC 调用
pub async fn get_token_decimals(
    rpc: &crate::common::SolanaRpcClient,
    mint: &Pubkey,
) -> Result<u8> {
    // Fast path: 检查缓存
    if let Some(info) = get_cached_mint_info(mint) {
        return Ok(info.decimals);
    }

    // 缓存未命中，获取完整 MintInfo
    let info = get_mint_info(rpc, mint).await?;
    Ok(info.decimals)
}

/// 获取代币 Symbol（支持 Token 和 Token2022 Metadata Extension）
///
/// 使用全局缓存减少 RPC 调用
pub async fn get_token_symbol(
    rpc: &crate::common::SolanaRpcClient,
    mint: &Pubkey,
) -> Result<String> {
    // Fast path: 检查缓存
    if let Some(info) = get_cached_mint_info(mint) {
        return Ok(info.symbol);
    }

    // 缓存未命中，获取完整 MintInfo
    let info = get_mint_info(rpc, mint).await?;
    Ok(info.symbol)
}

/// 计算 ATA 地址（自动识别 Token Program）
///
/// 使用 fast_fn 的全局缓存（DashMap lock-free）减少重复计算，支持 MintInfo 缓存和 ATA 地址缓存
pub async fn calculate_ata(
    rpc: &crate::common::SolanaRpcClient,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Pubkey> {
    // 确定正确的 token_program
    let token_program = if let Some(info) = get_cached_mint_info(mint) {
        if info.is_token2022 {
            &crate::constants::TOKEN_PROGRAM_2022
        } else {
            &crate::constants::TOKEN_PROGRAM
        }
    } else {
        // 缓存未命中，获取 MintInfo
        let info = get_mint_info(rpc, mint).await?;
        if info.is_token2022 {
            &crate::constants::TOKEN_PROGRAM_2022
        } else {
            &crate::constants::TOKEN_PROGRAM
        }
    };

    // 使用 fast_fn 计算 ATA（自动缓存，lock-free DashMap）
    let ata = crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
        owner,
        mint,
        token_program,
    );

    Ok(ata)
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
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        let wsol = Pubkey::from_str_const("So11111111111111111111111111111111111111112");
        
        let decimals = get_token_decimals(&rpc, &wsol).await.unwrap();
        assert_eq!(decimals, 9);
    }

    #[tokio::test]
    async fn test_get_token_decimals_usdc() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        let usdc = Pubkey::from_str_const("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        
        let decimals = get_token_decimals(&rpc, &usdc).await.unwrap();
        assert_eq!(decimals, 6);
    }

    #[tokio::test]
    async fn test_get_token_symbol_pumpfun() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
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
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        // Pump.fun 测试代币
        let pump = Pubkey::from_str_const("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn");

        let decimals = get_token_decimals(&rpc, &pump).await.unwrap();
        assert_eq!(decimals, 6, "Pump.fun token decimals should be 6");
        println!("Pump.fun token decimals: {}", decimals);
    }

    #[tokio::test]
    async fn test_decimals_cache_miss() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        let wsol = Pubkey::from_str_const("So11111111111111111111111111111111111111112");

        // 清除缓存确保冷启动
        MINT_INFO_CACHE.remove(&wsol);

        let decimals = get_token_decimals(&rpc, &wsol).await.unwrap();
        assert_eq!(decimals, 9);
    }

    #[tokio::test]
    async fn test_symbol_cache_miss() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        let pump = Pubkey::from_str_const("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn");

        // 清除缓存确保冷启动
        MINT_INFO_CACHE.remove(&pump);

        let symbol = get_token_symbol(&rpc, &pump).await.unwrap();
        assert!(!symbol.is_empty());
    }

    #[tokio::test]
    async fn test_decimals_cache_hit() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
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
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        let pump = Pubkey::from_str_const("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn");

        // 先触发一次缓存填充
        let _ = get_token_symbol(&rpc, &pump).await.unwrap();

        // 第二次调用应该命中缓存
        let symbol1 = get_token_symbol(&rpc, &pump).await.unwrap();
        let symbol2 = get_token_symbol(&rpc, &pump).await.unwrap();

        assert!(!symbol1.is_empty());
        assert_eq!(symbol1, symbol2);
    }

    #[tokio::test]
    async fn test_get_mint_info_usdc() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        let usdc = Pubkey::from_str_const("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

        let info = get_mint_info(&rpc, &usdc).await.unwrap();
        assert_eq!(info.decimals, 6);
        assert!(!info.symbol.is_empty());
        assert!(!info.is_token2022, "USDC on mainnet is not Token2022");
    }

    #[tokio::test]
    async fn test_get_mint_info_pumpfun() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        let pump = Pubkey::from_str_const("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn");

        let info = get_mint_info(&rpc, &pump).await.unwrap();
        assert_eq!(info.decimals, 6);
        assert!(!info.symbol.is_empty());
        assert!(info.is_token2022, "Pump.fun tokens are Token2022");
    }

    #[tokio::test]
    async fn test_cached_mint_info() {
        use solana_client::nonblocking::rpc_client::RpcClient;
        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        let wsol = Pubkey::from_str_const("So11111111111111111111111111111111111111112");

        // 清除缓存确保冷启动
        MINT_INFO_CACHE.remove(&wsol);

        // 先获取 MintInfo
        let info1 = get_mint_info(&rpc, &wsol).await.unwrap();
        assert_eq!(info1.decimals, 9);

        // 使用 get_cached_mint_info 应该命中缓存
        let info2 = get_cached_mint_info(&wsol).unwrap();
        assert_eq!(info1.decimals, info2.decimals);
        assert_eq!(info1.symbol, info2.symbol);
        assert_eq!(info1.is_token2022, info2.is_token2022);
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
