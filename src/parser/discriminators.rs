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
        let mut registry = Self { discriminators: HashMap::new() };

        // 注册 Raydium CLMM discriminators
        // 来源: solana-dex-parser/src/constants/discriminators.ts
        registry.register_raydium_clmm();

        // 注册 PumpSwap discriminators
        registry.register_pumpswap();

        // 注册 Raydium CPMM discriminators
        registry.register_raydium_cpmm();

        // 注册 Raydium V4 discriminators
        registry.register_raydium_v4();

        registry
    }

    /// 注册 Raydium CLMM 的 discriminators
    fn register_raydium_clmm(&mut self) {
        use InstructionType::*;

        // ADD_LIQUIDITY 操作
        let add_liquidity = [
            [135, 128, 47, 77, 15, 152, 240, 49],   // openPosition
            [77, 184, 74, 214, 112, 86, 241, 199],  // openPositionV2
            [77, 255, 174, 82, 125, 29, 201, 46],   // openPositionWithToken22Nft
            [46, 156, 243, 118, 13, 205, 251, 178], // increaseLiquidity
            [133, 29, 89, 223, 69, 238, 176, 10],   // increaseLiquidityV2
        ];

        for disc in add_liquidity {
            self.discriminators.insert((DexProtocol::RaydiumClmm, disc), AddLiquidity);
        }

        // REMOVE_LIQUIDITY 操作
        let remove_liquidity = [
            [160, 38, 208, 111, 104, 91, 44, 1], // decreaseLiquidity
            [58, 127, 188, 62, 79, 82, 196, 96], // decreaseLiquidityV2
        ];

        for disc in remove_liquidity {
            self.discriminators.insert((DexProtocol::RaydiumClmm, disc), RemoveLiquidity);
        }

        // CREATE 操作
        let create = [
            [233, 146, 209, 142, 207, 104, 64, 188], // createPool
        ];

        for disc in create {
            self.discriminators.insert((DexProtocol::RaydiumClmm, disc), CreatePool);
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
        self.discriminators
            .insert((DexProtocol::PumpSwap, remove_liquidity), RemoveLiquidity);
    }

    /// 注册 Raydium CPMM 的 discriminators
    fn register_raydium_cpmm(&mut self) {
        use InstructionType::*;

        // SWAP 操作 (8 字节 discriminator)
        let swap = [0x8f, 0xbe, 0x5a, 0xda, 0xc4, 0x1e, 0x33, 0xde];
        self.discriminators.insert((DexProtocol::RaydiumCpmm, swap), Swap);

        // 流动性操作
        let create_pool = [175, 175, 109, 31, 13, 152, 155, 237];
        self.discriminators.insert((DexProtocol::RaydiumCpmm, create_pool), CreatePool);

        let add_liquidity = [242, 35, 198, 137, 82, 225, 242, 182];
        self.discriminators
            .insert((DexProtocol::RaydiumCpmm, add_liquidity), AddLiquidity);

        let remove_liquidity = [183, 18, 70, 156, 148, 109, 161, 34];
        self.discriminators
            .insert((DexProtocol::RaydiumCpmm, remove_liquidity), RemoveLiquidity);
    }

    /// 注册 Raydium V4 的 discriminators
    fn register_raydium_v4(&mut self) {
        use InstructionType::*;

        // V4 使用 1 字节 discriminator，为了兼容系统，我们转换为 8 字节（前 1 字节有效）
        // SWAP 操作 (discriminator = 9)
        let swap = [9, 0, 0, 0, 0, 0, 0, 0];
        self.discriminators.insert((DexProtocol::RaydiumV4, swap), Swap);

        // 流动性操作
        let add_liquidity = [1, 0, 0, 0, 0, 0, 0, 0];
        self.discriminators
            .insert((DexProtocol::RaydiumV4, add_liquidity), AddLiquidity);

        let remove_liquidity = [2, 0, 0, 0, 0, 0, 0, 0];
        self.discriminators
            .insert((DexProtocol::RaydiumV4, remove_liquidity), RemoveLiquidity);

        let create_pool = [0, 0, 0, 0, 0, 0, 0, 0];
        self.discriminators.insert((DexProtocol::RaydiumV4, create_pool), CreatePool);
    }

    /// 识别指令类型
    pub fn identify(&self, protocol: DexProtocol, data: &[u8]) -> InstructionType {
        // Raydium V4 使用 1 字节 discriminator，其他协议使用 8 字节
        let min_len = if matches!(protocol, DexProtocol::RaydiumV4) { 1 } else { 8 };

        if data.len() < min_len {
            return InstructionType::Unknown;
        }

        let mut key = [0u8; 8];
        let copy_len = std::cmp::min(data.len(), 8);
        key[0..copy_len].copy_from_slice(&data[0..copy_len]);

        self.discriminators
            .get(&(protocol, key))
            .copied()
            .unwrap_or(InstructionType::Unknown)
    }

    /// 判断是否是流动性操作（应该被 Swap 解析器排除）
    pub fn is_liquidity_discriminator(&self, protocol: DexProtocol, data: &[u8]) -> bool {
        let instr_type = self.identify(protocol, data);
        matches!(
            instr_type,
            InstructionType::CreatePool
                | InstructionType::AddLiquidity
                | InstructionType::RemoveLiquidity
        )
    }

    /// 判断是否是 Swap 操作（Buy/Sell）
    pub fn is_swap_discriminator(&self, protocol: DexProtocol, data: &[u8]) -> bool {
        let instr_type = self.identify(protocol, data);
        matches!(instr_type, InstructionType::Swap | InstructionType::Buy | InstructionType::Sell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 Raydium CLMM discriminator 识别
    #[test]
    fn test_discriminator_registry_clmm() {
        let registry = DiscriminatorRegistry::default();

        // openPosition - 应该被识别为流动性操作
        let open_position = [135, 128, 47, 77, 15, 152, 240, 49];
        assert!(registry.is_liquidity_discriminator(DexProtocol::RaydiumClmm, &open_position));

        // 非 registered discriminator - 不应该是流动性操作
        let unknown = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        assert!(!registry.is_liquidity_discriminator(DexProtocol::RaydiumClmm, &unknown));
    }

    /// 测试 PumpSwap discriminator 识别
    #[test]
    fn test_discriminator_registry_pumpswap() {
        let registry = DiscriminatorRegistry::default();

        // BUY 操作
        let buy = [102, 6, 61, 18, 1, 218, 235, 234];
        assert!(registry.is_swap_discriminator(DexProtocol::PumpSwap, &buy));
        assert!(!registry.is_liquidity_discriminator(DexProtocol::PumpSwap, &buy));

        // SELL 操作
        let sell = [51, 230, 133, 164, 1, 127, 131, 173];
        assert!(registry.is_swap_discriminator(DexProtocol::PumpSwap, &sell));

        // 创建池子 - 应该是流动性操作
        let create_pool = [233, 146, 209, 142, 207, 104, 64, 188];
        assert!(registry.is_liquidity_discriminator(DexProtocol::PumpSwap, &create_pool));
        assert!(!registry.is_swap_discriminator(DexProtocol::PumpSwap, &create_pool));
    }

    /// 测试 Raydium CPMM discriminator 识别
    #[test]
    fn test_discriminator_registry_raydium_cpmm() {
        let registry = DiscriminatorRegistry::default();

        // SWAP 操作
        let swap = [0x8f, 0xbe, 0x5a, 0xda, 0xc4, 0x1e, 0x33, 0xde];
        assert!(registry.is_swap_discriminator(DexProtocol::RaydiumCpmm, &swap));
        assert!(!registry.is_liquidity_discriminator(DexProtocol::RaydiumCpmm, &swap));

        // 添加流动性 - 应该是流动性操作
        let add_liquidity = [242, 35, 198, 137, 82, 225, 242, 182];
        assert!(registry.is_liquidity_discriminator(DexProtocol::RaydiumCpmm, &add_liquidity));
    }

    /// 测试 Raydium V4 discriminator 识别（1字节 discriminator）
    #[test]
    fn test_discriminator_registry_raydium_v4() {
        let registry = DiscriminatorRegistry::default();

        // SWAP 操作（discriminator = 9）
        let swap = [9, 0, 0, 0, 0, 0, 0, 0];
        assert!(registry.is_swap_discriminator(DexProtocol::RaydiumV4, &swap));

        // 添加流动性操作（discriminator = 1）
        let add_liquidity = [1, 0, 0, 0, 0, 0, 0, 0];
        assert!(registry.is_liquidity_discriminator(DexProtocol::RaydiumV4, &add_liquidity));
    }

    /// 测试未知 discriminator
    #[test]
    fn test_unknown_discriminator() {
        let registry = DiscriminatorRegistry::default();

        let unknown = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];

        // 对于所有协议，未知的 discriminator 都应该返回 Unknown
        assert!(!registry.is_swap_discriminator(DexProtocol::RaydiumClmm, &unknown));
        assert!(!registry.is_liquidity_discriminator(DexProtocol::RaydiumClmm, &unknown));

        assert_eq!(registry.identify(DexProtocol::PumpSwap, &unknown), InstructionType::Unknown);
    }

    /// 测试数据长度不足的情况
    #[test]
    fn test_insufficient_data_length() {
        let registry = DiscriminatorRegistry::default();

        // 空数据
        let empty = [];
        assert_eq!(registry.identify(DexProtocol::RaydiumClmm, &empty), InstructionType::Unknown);

        // 数据长度不足 8 字节（对于非 V4 协议）
        let short = [0x01, 0x02, 0x03];
        assert_eq!(registry.identify(DexProtocol::RaydiumCpmm, &short), InstructionType::Unknown);

        // V4 协议只需要 1 字节
        let v4_short = [9];
        assert_eq!(registry.identify(DexProtocol::RaydiumV4, &v4_short), InstructionType::Swap);
    }
}
