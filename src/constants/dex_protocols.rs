//! DEX 协议识别模块
//!
//! 提供统一的 DEX 协议枚举和识别功能

use solana_sdk::pubkey::Pubkey;

// ===== DEX 协议常量 =====

/// PumpFun 协议常量
pub const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
pub const PUMPFUN_NAME: &str = "pumpfun";
pub const PUMPFUN_DISPLAY_NAME: &str = "PumpFun";

/// PumpSwap 协议常量
pub const PUMPSWAP_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const PUMPSWAP_NAME: &str = "pumpswap";
pub const PUMPSWAP_DISPLAY_NAME: &str = "PumpSwap";

/// Bonk 协议常量
pub const BONK_PROGRAM_ID: &str = "BSwp6bEBihVLdqJRKGgzjcGLHkcTuzmSo1TQkHepzH8p";
pub const BONK_NAME: &str = "bonk";
pub const BONK_DISPLAY_NAME: &str = "Bonk";

/// Raydium AMM V4 协议常量
pub const RAYDIUM_AMM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const RAYDIUM_AMM_V4_NAME: &str = "raydium_amm_v4";
pub const RAYDIUM_AMM_V4_DISPLAY_NAME: &str = "Raydium AMM V4";

/// Raydium CLMM 协议常量
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
pub const RAYDIUM_CLMM_NAME: &str = "raydium_clmm";
pub const RAYDIUM_CLMM_DISPLAY_NAME: &str = "Raydium CLMM";

/// Raydium CPMM 协议常量
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
pub const RAYDIUM_CPMM_NAME: &str = "raydium_cpmm";
pub const RAYDIUM_CPMM_DISPLAY_NAME: &str = "Raydium CPMM";

/// Raydium LaunchLab 协议常量
pub const RAYDIUM_LAUNCHLAB_PROGRAM_ID: &str = "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj";
pub const RAYDIUM_LAUNCHLAB_NAME: &str = "raydium_launchlab";
pub const RAYDIUM_LAUNCHLAB_DISPLAY_NAME: &str = "Raydium LaunchLab";

/// Meteora DAMM V2 协议常量
pub const METEORA_DAMM_V2_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";
pub const METEORA_DAMM_V2_NAME: &str = "meteora_damm_v2";
pub const METEORA_DAMM_V2_DISPLAY_NAME: &str = "Meteora DAMM V2";

/// 支持的 DEX 协议
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DexProtocol {
    PumpFun,
    PumpSwap,
    Bonk,
    RaydiumAmmV4,
    RaydiumClmm,
    RaydiumCpmm,
    RaydiumLaunchlab,
    MeteoraDammV2,
}

impl DexProtocol {
    /// 获取协议的 Program ID
    pub fn program_id(&self) -> &'static str {
        match self {
            DexProtocol::PumpFun => PUMPFUN_PROGRAM_ID,
            DexProtocol::PumpSwap => PUMPSWAP_PROGRAM_ID,
            DexProtocol::Bonk => BONK_PROGRAM_ID,
            DexProtocol::RaydiumAmmV4 => RAYDIUM_AMM_V4_PROGRAM_ID,
            DexProtocol::RaydiumClmm => RAYDIUM_CLMM_PROGRAM_ID,
            DexProtocol::RaydiumCpmm => RAYDIUM_CPMM_PROGRAM_ID,
            DexProtocol::RaydiumLaunchlab => RAYDIUM_LAUNCHLAB_PROGRAM_ID,
            DexProtocol::MeteoraDammV2 => METEORA_DAMM_V2_PROGRAM_ID,
        }
    }

    /// 获取协议的 Program ID (Pubkey 格式)
    pub fn program_id_pubkey(&self) -> Pubkey {
        self.program_id().parse().expect("Invalid program ID")
    }

    /// 获取协议名称（不带空格，用于代码/数据库）
    pub fn name(&self) -> &'static str {
        match self {
            DexProtocol::PumpFun => PUMPFUN_NAME,
            DexProtocol::PumpSwap => PUMPSWAP_NAME,
            DexProtocol::Bonk => BONK_NAME,
            DexProtocol::RaydiumAmmV4 => RAYDIUM_AMM_V4_NAME,
            DexProtocol::RaydiumClmm => RAYDIUM_CLMM_NAME,
            DexProtocol::RaydiumCpmm => RAYDIUM_CPMM_NAME,
            DexProtocol::RaydiumLaunchlab => RAYDIUM_LAUNCHLAB_NAME,
            DexProtocol::MeteoraDammV2 => METEORA_DAMM_V2_NAME,
        }
    }

    /// 获取协议显示名称（带空格，用于 UI 显示）
    pub fn display_name(&self) -> &'static str {
        match self {
            DexProtocol::PumpFun => PUMPFUN_DISPLAY_NAME,
            DexProtocol::PumpSwap => PUMPSWAP_DISPLAY_NAME,
            DexProtocol::Bonk => BONK_DISPLAY_NAME,
            DexProtocol::RaydiumAmmV4 => RAYDIUM_AMM_V4_DISPLAY_NAME,
            DexProtocol::RaydiumClmm => RAYDIUM_CLMM_DISPLAY_NAME,
            DexProtocol::RaydiumCpmm => RAYDIUM_CPMM_DISPLAY_NAME,
            DexProtocol::RaydiumLaunchlab => RAYDIUM_LAUNCHLAB_DISPLAY_NAME,
            DexProtocol::MeteoraDammV2 => METEORA_DAMM_V2_DISPLAY_NAME,
        }
    }

    /// 从 DEX 名称解析协议（不带空格，用于代码/数据库）
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            PUMPFUN_NAME => Some(DexProtocol::PumpFun),
            PUMPSWAP_NAME => Some(DexProtocol::PumpSwap),
            BONK_NAME => Some(DexProtocol::Bonk),
            RAYDIUM_AMM_V4_NAME => Some(DexProtocol::RaydiumAmmV4),
            RAYDIUM_CLMM_NAME => Some(DexProtocol::RaydiumClmm),
            RAYDIUM_CPMM_NAME => Some(DexProtocol::RaydiumCpmm),
            RAYDIUM_LAUNCHLAB_NAME => Some(DexProtocol::RaydiumLaunchlab),
            METEORA_DAMM_V2_NAME => Some(DexProtocol::MeteoraDammV2),
            _ => None,
        }
    }

    /// 从 Program ID (Pubkey) 解析协议
    pub fn from_program_id(program_id: &Pubkey) -> Option<Self> {
        match program_id.to_string().as_str() {
            PUMPFUN_PROGRAM_ID => Some(DexProtocol::PumpFun),
            PUMPSWAP_PROGRAM_ID => Some(DexProtocol::PumpSwap),
            BONK_PROGRAM_ID => Some(DexProtocol::Bonk),
            RAYDIUM_AMM_V4_PROGRAM_ID => Some(DexProtocol::RaydiumAmmV4),
            RAYDIUM_CLMM_PROGRAM_ID => Some(DexProtocol::RaydiumClmm),
            RAYDIUM_CPMM_PROGRAM_ID => Some(DexProtocol::RaydiumCpmm),
            RAYDIUM_LAUNCHLAB_PROGRAM_ID => Some(DexProtocol::RaydiumLaunchlab),
            METEORA_DAMM_V2_PROGRAM_ID => Some(DexProtocol::MeteoraDammV2),
            _ => None,
        }
    }

    /// 从 Program ID 字符串解析协议
    pub fn from_program_id_str(program_id: &str) -> Option<Self> {
        match program_id {
            PUMPFUN_PROGRAM_ID => Some(DexProtocol::PumpFun),
            PUMPSWAP_PROGRAM_ID => Some(DexProtocol::PumpSwap),
            BONK_PROGRAM_ID => Some(DexProtocol::Bonk),
            RAYDIUM_AMM_V4_PROGRAM_ID => Some(DexProtocol::RaydiumAmmV4),
            RAYDIUM_CLMM_PROGRAM_ID => Some(DexProtocol::RaydiumClmm),
            RAYDIUM_CPMM_PROGRAM_ID => Some(DexProtocol::RaydiumCpmm),
            RAYDIUM_LAUNCHLAB_PROGRAM_ID => Some(DexProtocol::RaydiumLaunchlab),
            METEORA_DAMM_V2_PROGRAM_ID => Some(DexProtocol::MeteoraDammV2),
            _ => None,
        }
    }

    /// 获取所有支持的协议列表
    pub fn all() -> &'static [DexProtocol] {
        &[
            DexProtocol::PumpFun,
            DexProtocol::PumpSwap,
            DexProtocol::Bonk,
            DexProtocol::RaydiumAmmV4,
            DexProtocol::RaydiumClmm,
            DexProtocol::RaydiumCpmm,
            DexProtocol::RaydiumLaunchlab,
            DexProtocol::MeteoraDammV2,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_id_roundtrip() {
        for protocol in DexProtocol::all() {
            let id = protocol.program_id();
            let parsed = DexProtocol::from_program_id_str(id);
            assert_eq!(Some(*protocol), parsed, "Failed for {}", protocol.name());
        }
    }

    #[test]
    fn test_protocol_id_pubkey_roundtrip() {
        for protocol in DexProtocol::all() {
            let pubkey = protocol.program_id_pubkey();
            let parsed = DexProtocol::from_program_id(&pubkey);
            assert_eq!(Some(*protocol), parsed, "Failed for {}", protocol.name());
        }
    }

    #[test]
    fn test_name_roundtrip() {
        for protocol in DexProtocol::all() {
            let name = protocol.name();
            let parsed = DexProtocol::from_name(name);
            assert_eq!(Some(*protocol), parsed, "Failed for {}", name);
        }
    }

    #[test]
    fn test_unknown_program_id() {
        let unknown_str = "Unknown1111111111111111111111111111111111111";
        assert!(DexProtocol::from_program_id_str(unknown_str).is_none());

        let random_valid_id = "1111111111111111111111111111111111111111";
        assert!(DexProtocol::from_program_id_str(random_valid_id).is_none());
    }

    #[test]
    fn test_unknown_name() {
        assert!(DexProtocol::from_name("unknown_dex").is_none());
        assert!(DexProtocol::from_name("").is_none());
    }

    #[test]
    fn test_all_protocols_unique() {
        let protocols = DexProtocol::all();
        let ids: Vec<_> = protocols.iter().map(|p| p.program_id()).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique_ids.len(), "Program IDs should be unique");
    }

    #[test]
    fn test_all_names_unique() {
        let protocols = DexProtocol::all();
        let names: Vec<_> = protocols.iter().map(|p| p.name()).collect();
        let unique_names: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(names.len(), unique_names.len(), "Names should be unique");
    }
}
