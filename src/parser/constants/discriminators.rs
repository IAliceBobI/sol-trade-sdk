//! DEX 协议 Discriminator 常量
//!
//! 参考 solana-dex-parser 的 discriminator 定义

pub mod pumpswap {
    /// PumpSwap 事件 discriminator (16 字节)
    pub const BUY_EVENT: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, 103, 244, 82, 31, 44, 245, 119, 119
    ];

    pub const SELL_EVENT: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, 62, 47, 55, 10, 165, 3, 220, 42
    ];

    pub const CREATE_POOL_EVENT: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, 177, 49, 12, 210, 160, 118, 167, 116
    ];

    pub const ADD_LIQUIDITY_EVENT: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, 120, 248, 61, 83, 31, 142, 107, 144
    ];

    pub const REMOVE_LIQUIDITY_EVENT: [u8; 16] = [
        228, 69, 165, 46, 81, 203, 154, 29, 22, 9, 133, 26, 160, 44, 71, 192
    ];
}

pub mod raydium_v4 {
    /// Raydium V4 指令 discriminator (1 字节)
    pub const SWAP: u8 = 9;
    pub const ADD_LIQUIDITY: u8 = 1;
    pub const REMOVE_LIQUIDITY: u8 = 2;
    pub const CREATE_POOL: u8 = 0;
}

pub mod raydium_clmm {
    /// Raydium CLMM 指令 discriminator (8 字节)
    /// TODO: 需要查找实际值
    pub const SWAP: [u8; 8] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ];

    pub const ADD_LIQUIDITY: [u8; 8] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ];
}

pub mod raydium_cpmm {
    /// Raydium CPMM 指令 discriminator (8 字节)
    pub const SWAP: [u8; 8] = [
        0x8f, 0xbe, 0x5a, 0xda, 0xc4, 0x1e, 0x33, 0xde
    ];

    pub const CREATE_POOL: [u8; 8] = [
        175, 175, 109, 31, 13, 152, 155, 237 // initialize
    ];

    pub const ADD_LIQUIDITY: [u8; 8] = [
        242, 35, 198, 137, 82, 225, 242, 182 // deposit
    ];

    pub const REMOVE_LIQUIDITY: [u8; 8] = [
        183, 18, 70, 156, 148, 109, 161, 34 // withdraw
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pumpswap_discriminator_length() {
        assert_eq!(pumpswap::BUY_EVENT.len(), 16);
        assert_eq!(pumpswap::SELL_EVENT.len(), 16);
    }
}
