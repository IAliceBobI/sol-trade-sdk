//! 指令 Discriminator 注册表
//!
//! 参考 solana-dex-parser 的 discriminator 系统
//! 使用 8 字节标识符精确识别指令类型

use std::collections::HashMap;

/// DEX 协议枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DexProtocol {
    PumpSwap,
    RaydiumClmm,
    RaydiumCpmm,
    RaydiumV4,
}

/// 指令类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionType {
    // Swap 相关
    Swap,
    Buy,
    Sell,

    // 流动性相关
    CreatePool,
    AddLiquidity,
    RemoveLiquidity,

    // 未知
    Unknown,
}

/// Discriminator 注册表
#[derive(Debug, Clone)]
pub struct DiscriminatorRegistry {
    /// 存储 (protocol, discriminator_bytes) -> InstructionType
    discriminators: HashMap<(DexProtocol, [u8; 8]), InstructionType>,
}

impl Default for DiscriminatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscriminatorRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            discriminators: HashMap::new(),
        };

        // 注册 Raydium CLMM discriminators
        // 来源: solana-dex-parser/src/constants/discriminators.ts
        registry.register_raydium_clmm();

        // 注册 PumpSwap discriminators
        registry.register_pumpswap();

        registry
    }

    /// 注册 Raydium CLMM 的 discriminators
    fn register_raydium_clmm(&mut self) {
        use InstructionType::*;

        // ADD_LIQUIDITY 操作
        let add_liquidity = [
            [135, 128, 47, 77, 15, 152, 240, 49], // openPosition
            [77, 184, 74, 214, 112, 86, 241, 199], // openPositionV2
            [77, 255, 174, 82, 125, 29, 201, 46],  // openPositionWithToken22Nft
            [46, 156, 243, 118, 13, 205, 251, 178], // increaseLiquidity
            [133, 29, 89, 223, 69, 238, 176, 10],  // increaseLiquidityV2
        ];

        for disc in add_liquidity {
            self.discriminators.insert(
                (DexProtocol::RaydiumClmm, disc),
                AddLiquidity
            );
        }

        // REMOVE_LIQUIDITY 操作
        let remove_liquidity = [
            [160, 38, 208, 111, 104, 91, 44, 1], // decreaseLiquidity
            [58, 127, 188, 62, 79, 82, 196, 96], // decreaseLiquidityV2
        ];

        for disc in remove_liquidity {
            self.discriminators.insert(
                (DexProtocol::RaydiumClmm, disc),
                RemoveLiquidity
            );
        }

        // CREATE 操作
        let create = [
            [233, 146, 209, 142, 207, 104, 64, 188], // createPool
        ];

        for disc in create {
            self.discriminators.insert(
                (DexProtocol::RaydiumClmm, disc),
                CreatePool
            );
        }
    }

    /// 注册 PumpSwap 的 discriminators
    fn register_pumpswap(&mut self) {
        use InstructionType::*;

        // BUY 操作
        let buy = [102, 6, 61, 18, 1, 218, 235, 234];
        self.discriminators.insert((DexProtocol::PumpSwap, buy), Buy);

        // SELL 操作
        let sell = [51, 230, 133, 164, 1, 127, 131, 173];
        self.discriminators.insert((DexProtocol::PumpSwap, sell), Sell);

        // 流动性操作
        let create_pool = [233, 146, 209, 142, 207, 104, 64, 188];
        self.discriminators.insert((DexProtocol::PumpSwap, create_pool), CreatePool);

        let add_liquidity = [242, 35, 198, 137, 82, 225, 242, 182];
        self.discriminators.insert((DexProtocol::PumpSwap, add_liquidity), AddLiquidity);

        let remove_liquidity = [183, 18, 70, 156, 148, 109, 161, 34];
        self.discriminators.insert((DexProtocol::PumpSwap, remove_liquidity), RemoveLiquidity);
    }

    /// 识别指令类型
    pub fn identify(&self, protocol: DexProtocol, data: &[u8]) -> InstructionType {
        if data.len() < 8 {
            return InstructionType::Unknown;
        }

        let mut key = [0u8; 8];
        key.copy_from_slice(&data[0..8]);

        self.discriminators
            .get(&(protocol, key))
            .copied()
            .unwrap_or(InstructionType::Unknown)
    }

    /// 判断是否是流动性操作（应该被 Swap 解析器排除）
    pub fn is_liquidity_discriminator(&self, protocol: DexProtocol, data: &[u8]) -> bool {
        let instr_type = self.identify(protocol, data);
        matches!(instr_type, InstructionType::CreatePool | InstructionType::AddLiquidity | InstructionType::RemoveLiquidity)
    }

    /// 判断是否是 Swap 操作（Buy/Sell）
    pub fn is_swap_discriminator(&self, protocol: DexProtocol, data: &[u8]) -> bool {
        let instr_type = self.identify(protocol, data);
        matches!(instr_type, InstructionType::Swap | InstructionType::Buy | InstructionType::Sell)
    }
}
