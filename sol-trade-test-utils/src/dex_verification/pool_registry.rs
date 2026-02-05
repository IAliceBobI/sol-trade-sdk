//! Pool 配置注册表
//!
//! 集中管理所有测试用 Pool 的配置

use super::types::PoolConfig;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Raydium CPMM Pool 注册表
pub struct RaydiumCpmmPoolRegistry;

impl RaydiumCpmmPoolRegistry {
    /// PIPE-WSOL (Token/Token)
    ///
    /// - Pool: BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp
    /// - PIPE: 8ycz3kctoRb4LFrtoYG2r8tRyUYUeGf5Q16M2TEMp7A (Token Program)
    /// - WSOL: So11111111111111111111111111111111111111112 (Token Program)
    pub fn pipe_wsol() -> PoolConfig {
        PoolConfig::new(
            Pubkey::from_str("BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp").unwrap(),
            "PIPE-WSOL",
            Pubkey::from_str("8ycz3kctoRb4LFrtoYG2r8tRyUYUeGf5Q16M2TEMp7A").unwrap(),
            super::TokenProgramType::Token,
            Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            super::TokenProgramType::Token,
            10,
        )
    }

    /// USDC-PRTS (Token/Token2022 - 混合 Pool)
    ///
    /// - Pool: 7Cvz28TyKnGuL8GAtbsVFu1FJ3Po7A37Zc8JSJqkSPDp
    /// - USDC: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v (Token Program)
    /// - PRTS: 3PQkX8yfuxoe9kuBoLCEZoxzi9LG4w8Ci2JWWGNfPRTS (Token-2022 Program)
    pub fn usdc_prts() -> PoolConfig {
        PoolConfig::new(
            Pubkey::from_str("7Cvz28TyKnGuL8GAtbsVFu1FJ3Po7A37Zc8JSJqkSPDp").unwrap(),
            "USDC-PRTS",
            Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
            super::TokenProgramType::Token,
            Pubkey::from_str("3PQkX8yfuxoe9kuBoLCEZoxzi9LG4w8Ci2JWWGNfPRTS").unwrap(),
            super::TokenProgramType::Token2022,
            10,
        )
    }

    /// USDC-RING (Token/Token)
    ///
    /// - Pool: CVPpJXyiPNRgD3a8SjmXkC1cKdHtry1PF9BVG6dYoxjk
    /// - USDC: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v (Token Program)
    /// - RING: A3569FJtxQ9qstaE1ToZDt8uAwkTQyMRf8xy669DbUZz (Token Program)
    pub fn usdc_ring() -> PoolConfig {
        PoolConfig::new(
            Pubkey::from_str("CVPpJXyiPNRgD3a8SjmXkC1cKdHtry1PF9BVG6dYoxjk").unwrap(),
            "USDC-RING",
            Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
            super::TokenProgramType::Token,
            Pubkey::from_str("A3569FJtxQ9qstaE1ToZDt8uAwkTQyMRf8xy669DbUZz").unwrap(),
            super::TokenProgramType::Token,
            10,
        )
    }

    /// USDC-CIB (Token2022/Token2022)
    ///
    /// - Pool: GarGiGTMQrZyot44J9hc71NeGNeEaxnq3nefKxBruEsS
    /// - USDC: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v (Token Program)
    /// - CIB: GarGiGTMQrZyot44J9hc71NeGNeEaxnq3nefKxBruEsS (Token-2022 Program)
    pub fn usdc_cib() -> PoolConfig {
        PoolConfig::new(
            Pubkey::from_str("GarGiGTMQrZyot44J9hc71NeGNeEaxnq3nefKxBruEsS").unwrap(),
            "USDC-CIB",
            Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
            super::TokenProgramType::Token,
            Pubkey::from_str("GarGiGTMQrZyot44J9hc71NeGNeEaxnq3nefKxBruEsS").unwrap(),
            super::TokenProgramType::Token2022,
            10,
        )
    }
}

/// Raydium CLMM Pool 注册表
pub struct RaydiumClmmPoolRegistry;

impl RaydiumClmmPoolRegistry {
    /// USDT-WSOL (Token/Token)
    ///
    /// - Pool: ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6
    /// - USDT: Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB (Token Program)
    /// - WSOL: So11111111111111111111111111111111111111112 (Token Program)
    pub fn usdt_wsol() -> PoolConfig {
        PoolConfig::new(
            Pubkey::from_str("ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6").unwrap(),
            "USDT-WSOL",
            Pubkey::from_str("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB").unwrap(),
            super::TokenProgramType::Token,
            Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            super::TokenProgramType::Token,
            10,
        )
    }
}

/// Raydium AMM V4 Pool 注册表
pub struct RaydiumAmmV4PoolRegistry;

impl RaydiumAmmV4PoolRegistry {
    /// WSOL-USDC (Token/Token)
    ///
    /// - Pool: 58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2
    /// - WSOL: So11111111111111111111111111111111111111112 (Token Program)
    /// - USDC: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v (Token Program)
    pub fn wsol_usdc() -> PoolConfig {
        PoolConfig::new(
            Pubkey::from_str("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2").unwrap(),
            "WSOL-USDC",
            Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            super::TokenProgramType::Token,
            Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap(),
            super::TokenProgramType::Token,
            10,
        )
    }
}

/// PumpSwap Pool 注册表
pub struct PumpSwapPoolRegistry;

impl PumpSwapPoolRegistry {
    /// PUMP-WSOL (Token2022/Token - 混合 Pool)
    ///
    /// - Pool: 539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR
    /// - PUMP: pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn (Token-2022 Program)
    /// - WSOL: So11111111111111111111111111111111111111112 (Token Program)
    pub fn pump_wsol() -> PoolConfig {
        PoolConfig::new(
            Pubkey::from_str("539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR").unwrap(),
            "PUMP-WSOL",
            Pubkey::from_str("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn").unwrap(),
            super::TokenProgramType::Token2022,
            Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            super::TokenProgramType::Token,
            10,
        )
    }

    /// BONK-WSOL (Token/Token)
    ///
    /// - Pool: Dwczp92NX3ngbE2HeTUH4p5dcQxrpDF2AJMbW581gq1E
    /// - BONK: DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263 (Token Program)
    /// - WSOL: So11111111111111111111111111111111111111112 (Token Program)
    pub fn bonk_wsol() -> PoolConfig {
        PoolConfig::new(
            Pubkey::from_str("Dwczp92NX3ngbE2HeTUH4p5dcQxrpDF2AJMbW581gq1E").unwrap(),
            "BONK-WSOL",
            Pubkey::from_str("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263").unwrap(),
            super::TokenProgramType::Token,
            Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            super::TokenProgramType::Token,
            10,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpmm_pool_registry() {
        let pipe_wsol = RaydiumCpmmPoolRegistry::pipe_wsol();
        assert_eq!(pipe_wsol.pool_name, "PIPE-WSOL");
        assert!(!pipe_wsol.is_mixed_pool());
        assert!(!pipe_wsol.requires_token2022());

        let usdc_prts = RaydiumCpmmPoolRegistry::usdc_prts();
        assert_eq!(usdc_prts.pool_name, "USDC-PRTS");
        assert!(usdc_prts.is_mixed_pool());
        assert!(usdc_prts.requires_token2022());
    }
}
