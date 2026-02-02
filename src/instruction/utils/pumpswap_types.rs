use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pool {
    // ===== 元数据字段 =====
    /// DEX Program ID（从账户 owner 自动填充）
    pub program_id: Pubkey,

    /// DEX 协议名称（如 "pumpswap"）
    pub dex_name: String,

    /// DEX 显示名称（如 "PumpSwap"）
    pub dex_display_name: String,

    // ===== 链上数据字段 =====
    pub pool_bump: u8,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub lp_supply: u64,
    pub coin_creator: Pubkey,
    pub is_mayhem_mode: bool,
}

/// 辅助结构体：只包含链上数据的字段（用于 Borsh 反序列化）
#[derive(BorshDeserialize)]
pub struct PoolDataOnly {
    pub pool_bump: u8,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub lp_supply: u64,
    pub coin_creator: Pubkey,
    pub is_mayhem_mode: bool,
}

pub const POOL_SIZE: usize = 1 + 2 + 32 * 6 + 8 + 32 + 1;

/// 修改后的 pool_decode：需要 program_id 参数
pub fn pool_decode(data: &[u8], program_id: Pubkey) -> Option<Pool> {
    if data.len() < POOL_SIZE {
        return None;
    }

    // 1. 反序列化链上数据部分
    let pool_data: PoolDataOnly = borsh::from_slice(&data[..POOL_SIZE]).ok()?;

    // 2. 从 DexProtocol 获取名称信息
    use crate::constants::dex_protocols::DexProtocol;
    let (dex_name, dex_display_name) = match DexProtocol::from_program_id(&program_id) {
        Some(protocol) => (
            protocol.name().to_string(),
            protocol.display_name().to_string(),
        ),
        None => {
            // 未知 DEX，使用 fallback
            let fallback = "unknown".to_string();
            (fallback.clone(), fallback)
        }
    };

    // 3. 构建完整的 Pool
    Some(Pool {
        program_id,
        dex_name,
        dex_display_name,
        pool_bump: pool_data.pool_bump,
        index: pool_data.index,
        creator: pool_data.creator,
        base_mint: pool_data.base_mint,
        quote_mint: pool_data.quote_mint,
        lp_mint: pool_data.lp_mint,
        pool_base_token_account: pool_data.pool_base_token_account,
        pool_quote_token_account: pool_data.pool_quote_token_account,
        lp_supply: pool_data.lp_supply,
        coin_creator: pool_data.coin_creator,
        is_mayhem_mode: pool_data.is_mayhem_mode,
    })
}
