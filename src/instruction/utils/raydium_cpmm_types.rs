use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct AmmConfig {
    pub bump: u8,
    pub disable_create_pool: bool,
    pub index: u16,
    pub trade_fee_rate: u64,
    pub protocol_fee_rate: u64,
    pub fund_fee_rate: u64,
    pub create_pool_fee: u64,
    pub protocol_owner: Pubkey,
    pub fund_owner: Pubkey,
    pub padding: [u64; 16],
}

pub const AMM_CONFIG_SIZE: usize = 228;

pub fn amm_config_decode(data: &[u8]) -> Option<AmmConfig> {
    if data.len() < AMM_CONFIG_SIZE {
        return None;
    }
    borsh::from_slice::<AmmConfig>(&data[..AMM_CONFIG_SIZE]).ok()
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolState {
    // ===== 元数据字段 =====
    /// DEX Program ID（从账户 owner 自动填充）
    pub program_id: Pubkey,

    /// DEX 协议名称（如 "raydium_cpmm"）
    pub dex_name: String,

    /// DEX 显示名称（如 "Raydium CPMM"）
    pub dex_display_name: String,

    // ===== 链上数据字段 =====
    pub amm_config: Pubkey,
    pub pool_creator: Pubkey,
    pub token0_vault: Pubkey,
    pub token1_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub token0_mint: Pubkey,
    pub token1_mint: Pubkey,
    pub token0_program: Pubkey,
    pub token1_program: Pubkey,
    pub observation_key: Pubkey,
    pub auth_bump: u8,
    pub status: u8,
    pub lp_mint_decimals: u8,
    pub mint0_decimals: u8,
    pub mint1_decimals: u8,
    pub lp_supply: u64,
    pub protocol_fees_token0: u64,
    pub protocol_fees_token1: u64,
    pub fund_fees_token0: u64,
    pub fund_fees_token1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub padding: [u64; 31],
}

/// 辅助结构体：只包含链上数据的字段（用于 Borsh 反序列化）
#[derive(BorshDeserialize)]
pub struct PoolStateDataOnly {
    pub amm_config: Pubkey,
    pub pool_creator: Pubkey,
    pub token0_vault: Pubkey,
    pub token1_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub token0_mint: Pubkey,
    pub token1_mint: Pubkey,
    pub token0_program: Pubkey,
    pub token1_program: Pubkey,
    pub observation_key: Pubkey,
    pub auth_bump: u8,
    pub status: u8,
    pub lp_mint_decimals: u8,
    pub mint0_decimals: u8,
    pub mint1_decimals: u8,
    pub lp_supply: u64,
    pub protocol_fees_token0: u64,
    pub protocol_fees_token1: u64,
    pub fund_fees_token0: u64,
    pub fund_fees_token1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub padding: [u64; 31],
}

pub const POOL_STATE_SIZE: usize = 629; // 不包含 discriminator 的数据大小（637 - 8 = 629）

/// 修改后的 pool_state_decode：需要 program_id 参数
pub fn pool_state_decode(data: &[u8], program_id: Pubkey) -> Option<PoolState> {
    if data.len() < POOL_STATE_SIZE {
        return None;
    }

    // 1. 反序列化链上数据部分
    let pool_data: PoolStateDataOnly = borsh::from_slice(&data[..POOL_STATE_SIZE]).ok()?;

    // 2. 从 DexProtocol 获取名称信息
    use crate::constants::dex_protocols::DexProtocol;
    let (dex_name, dex_display_name) = match DexProtocol::from_program_id(&program_id) {
        Some(protocol) => (protocol.name().to_string(), protocol.display_name().to_string()),
        None => {
            // 未知 DEX，使用 fallback
            let fallback = "unknown".to_string();
            (fallback.clone(), fallback)
        },
    };

    // 3. 构建完整的 PoolState
    Some(PoolState {
        program_id,
        dex_name,
        dex_display_name,
        amm_config: pool_data.amm_config,
        pool_creator: pool_data.pool_creator,
        token0_vault: pool_data.token0_vault,
        token1_vault: pool_data.token1_vault,
        lp_mint: pool_data.lp_mint,
        token0_mint: pool_data.token0_mint,
        token1_mint: pool_data.token1_mint,
        token0_program: pool_data.token0_program,
        token1_program: pool_data.token1_program,
        observation_key: pool_data.observation_key,
        auth_bump: pool_data.auth_bump,
        status: pool_data.status,
        lp_mint_decimals: pool_data.lp_mint_decimals,
        mint0_decimals: pool_data.mint0_decimals,
        mint1_decimals: pool_data.mint1_decimals,
        lp_supply: pool_data.lp_supply,
        protocol_fees_token0: pool_data.protocol_fees_token0,
        protocol_fees_token1: pool_data.protocol_fees_token1,
        fund_fees_token0: pool_data.fund_fees_token0,
        fund_fees_token1: pool_data.fund_fees_token1,
        open_time: pool_data.open_time,
        recent_epoch: pool_data.recent_epoch,
        padding: pool_data.padding,
    })
}
