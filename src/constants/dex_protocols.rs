//! DEX 协议识别模块
//!
//! 提供统一的 DEX 协议枚举和识别功能

use solana_sdk::{pubkey, pubkey::Pubkey};

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
            DexProtocol::PumpFun => "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
            DexProtocol::PumpSwap => "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
            DexProtocol::Bonk => "BSwp6bEBihVLdqJRKGgzjcGLHkcTuzmSo1TQkHepzH8p",
            DexProtocol::RaydiumAmmV4 => "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
            DexProtocol::RaydiumClmm => "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
            DexProtocol::RaydiumCpmm => "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
            DexProtocol::RaydiumLaunchlab => "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj",
            DexProtocol::MeteoraDammV2 => "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG",
        }
    }

    /// 获取协议的 Program ID (Pubkey 格式)
    pub fn program_id_pubkey(&self) -> Pubkey {
        match self {
            DexProtocol::PumpFun => pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"),
            DexProtocol::PumpSwap => pubkey!("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"),
            DexProtocol::Bonk => pubkey!("BSwp6bEBihVLdqJRKGgzjcGLHkcTuzmSo1TQkHepzH8p"),
            DexProtocol::RaydiumAmmV4 => pubkey!("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"),
            DexProtocol::RaydiumClmm => pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"),
            DexProtocol::RaydiumCpmm => pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"),
            DexProtocol::RaydiumLaunchlab => pubkey!("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj"),
            DexProtocol::MeteoraDammV2 => pubkey!("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG"),
        }
    }

    /// 获取协议名称（不带空格，用于代码/数据库）
    pub fn name(&self) -> &'static str {
        match self {
            DexProtocol::PumpFun => "pumpfun",
            DexProtocol::PumpSwap => "pumpswap",
            DexProtocol::Bonk => "bonk",
            DexProtocol::RaydiumAmmV4 => "raydium_amm_v4",
            DexProtocol::RaydiumClmm => "raydium_clmm",
            DexProtocol::RaydiumCpmm => "raydium_cpmm",
            DexProtocol::RaydiumLaunchlab => "raydium_launchlab",
            DexProtocol::MeteoraDammV2 => "meteora_damm_v2",
        }
    }

    /// 获取协议显示名称（带空格，用于 UI 显示）
    pub fn display_name(&self) -> &'static str {
        match self {
            DexProtocol::PumpFun => "PumpFun",
            DexProtocol::PumpSwap => "PumpSwap",
            DexProtocol::Bonk => "Bonk",
            DexProtocol::RaydiumAmmV4 => "Raydium AMM V4",
            DexProtocol::RaydiumClmm => "Raydium CLMM",
            DexProtocol::RaydiumCpmm => "Raydium CPMM",
            DexProtocol::RaydiumLaunchlab => "Raydium LaunchLab",
            DexProtocol::MeteoraDammV2 => "Meteora DAMM V2",
        }
    }

    /// 从 Program ID 字符串解析协议
    pub fn from_program_id_str(program_id: &str) -> Option<Self> {
        match program_id {
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => Some(DexProtocol::PumpFun),
            "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => Some(DexProtocol::PumpSwap),
            "BSwp6bEBihVLdqJRKGgzjcGLHkcTuzmSo1TQkHepzH8p" => Some(DexProtocol::Bonk),
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => Some(DexProtocol::RaydiumAmmV4),
            "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => Some(DexProtocol::RaydiumClmm),
            "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => Some(DexProtocol::RaydiumCpmm),
            "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj" => Some(DexProtocol::RaydiumLaunchlab),
            "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG" => Some(DexProtocol::MeteoraDammV2),
            _ => None,
        }
    }

    /// 从 Program ID (Pubkey) 解析协议
    pub fn from_program_id(program_id: &Pubkey) -> Option<Self> {
        Self::from_program_id_str(&program_id.to_string())
    }

    /// 获取所有支持的协议列表
    pub fn all_protocols() -> &'static [DexProtocol] {
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
        for protocol in DexProtocol::all_protocols() {
            let id = protocol.program_id();
            let parsed = DexProtocol::from_program_id_str(id);
            assert_eq!(Some(*protocol), parsed, "Failed for {}", protocol.name());
        }
    }

    #[test]
    fn test_protocol_id_pubkey_roundtrip() {
        for protocol in DexProtocol::all_protocols() {
            let pubkey = protocol.program_id_pubkey();
            let parsed = DexProtocol::from_program_id(&pubkey);
            assert_eq!(Some(*protocol), parsed, "Failed for {}", protocol.name());
        }
    }

    #[test]
    fn test_unknown_program_id() {
        // 测试字符串版本
        let unknown_str = "Unknown1111111111111111111111111111111111111";
        assert!(DexProtocol::from_program_id_str(unknown_str).is_none());

        // 测试随机的有效 base58 字符串但不是已知 Program ID
        let random_valid_id = "1111111111111111111111111111111111111111";
        assert!(DexProtocol::from_program_id_str(random_valid_id).is_none());
    }

    #[test]
    fn test_all_protocols_unique() {
        let protocols = DexProtocol::all_protocols();
        let ids: Vec<_> = protocols.iter().map(|p| p.program_id()).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique_ids.len(), "Program IDs should be unique");
    }
}
