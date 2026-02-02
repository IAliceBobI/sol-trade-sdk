use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct AmmConfig {
    pub bump: u8,
    pub index: u16,
    pub owner: Pubkey,
    pub protocol_fee_rate: u32,
    pub trade_fee_rate: u32,
    pub tick_spacing: u16,
    pub fund_fee_rate: u32,
    pub padding_u32: u32,
    pub fund_owner: Pubkey,
    pub padding: [u64; 3],
}

pub const AMM_CONFIG_SIZE: usize = 8 + 1 + 2 + 32 + 4 + 4 + 2 + 4 + 4 + 32 + 24;

pub fn amm_config_decode(data: &[u8]) -> Option<AmmConfig> {
    if data.len() < AMM_CONFIG_SIZE {
        return None;
    }
    borsh::from_slice::<AmmConfig>(&data[8..]).ok()
}

#[derive(Clone, Debug, BorshDeserialize)]
pub struct TickState {
    pub tick: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_0_x64: u128,
    pub fee_growth_outside_1_x64: u128,
    pub reward_growths_outside_x64: [u128; 3],
    pub padding: [u32; 13],
}

#[derive(Clone, Debug, BorshDeserialize)]
pub struct TickArrayState {
    pub pool_id: Pubkey,
    pub start_tick_index: i32,
    pub ticks: [TickState; 60],
    pub initialized_tick_count: u8,
    pub recent_epoch: u64,
    pub padding: [u8; 107],
}

pub fn tick_array_state_decode(data: &[u8]) -> Option<TickArrayState> {
    if data.len() < 8 {
        return None;
    }
    borsh::from_slice::<TickArrayState>(&data[8..]).ok()
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct RewardInfo {
    pub reward_state: u8,
    pub open_time: u64,
    pub end_time: u64,
    pub last_update_time: u64,
    pub emissions_per_second_x64: u128,
    pub reward_total_emissioned: u64,
    pub reward_claimed: u64,
    pub token_mint: Pubkey,
    pub token_vault: Pubkey,
    pub authority: Pubkey,
    pub reward_growth_global_x64: u128,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolState {
    // ===== 元数据字段 =====
    /// DEX Program ID（从账户 owner 自动填充）
    pub program_id: Pubkey,

    /// DEX 协议名称（如 "raydium_clmm"）
    pub dex_name: String,

    /// DEX 显示名称（如 "Raydium CLMM"）
    pub dex_display_name: String,

    // ===== 链上数据字段 =====
    pub bump: [u8; 1],
    pub amm_config: Pubkey,
    pub owner: Pubkey,
    pub token_mint0: Pubkey,
    pub token_mint1: Pubkey,
    pub token_vault0: Pubkey,
    pub token_vault1: Pubkey,
    pub observation_key: Pubkey,
    pub mint_decimals0: u8,
    pub mint_decimals1: u8,
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
    pub padding3: u16,
    pub padding4: u16,
    pub fee_growth_global0_x64: u128,
    pub fee_growth_global1_x64: u128,
    pub protocol_fees_token0: u64,
    pub protocol_fees_token1: u64,
    pub swap_in_amount_token0: u128,
    pub swap_out_amount_token1: u128,
    pub swap_in_amount_token1: u128,
    pub swap_out_amount_token0: u128,
    pub status: u8,
    pub padding: [u8; 7],
    pub reward_infos: [RewardInfo; 3],
    pub tick_array_bitmap: [u64; 16],
    pub total_fees_token0: u64,
    pub total_fees_claimed_token0: u64,
    pub total_fees_token1: u64,
    pub total_fees_claimed_token1: u64,
    pub fund_fees_token0: u64,
    pub fund_fees_token1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub padding1: [u64; 24],
    pub padding2: [u64; 32],
}

/// 辅助结构体：只包含链上数据的字段（用于 Borsh 反序列化）
#[derive(BorshDeserialize)]
pub struct PoolStateDataOnly {
    pub bump: [u8; 1],
    pub amm_config: Pubkey,
    pub owner: Pubkey,
    pub token_mint0: Pubkey,
    pub token_mint1: Pubkey,
    pub token_vault0: Pubkey,
    pub token_vault1: Pubkey,
    pub observation_key: Pubkey,
    pub mint_decimals0: u8,
    pub mint_decimals1: u8,
    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,
    pub padding3: u16,
    pub padding4: u16,
    pub fee_growth_global0_x64: u128,
    pub fee_growth_global1_x64: u128,
    pub protocol_fees_token0: u64,
    pub protocol_fees_token1: u64,
    pub swap_in_amount_token0: u128,
    pub swap_out_amount_token1: u128,
    pub swap_in_amount_token1: u128,
    pub swap_out_amount_token0: u128,
    pub status: u8,
    pub padding: [u8; 7],
    pub reward_infos: [RewardInfo; 3],
    pub tick_array_bitmap: [u64; 16],
    pub total_fees_token0: u64,
    pub total_fees_claimed_token0: u64,
    pub total_fees_token1: u64,
    pub total_fees_claimed_token1: u64,
    pub fund_fees_token0: u64,
    pub fund_fees_token1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub padding1: [u64; 24],
    pub padding2: [u64; 32],
}

pub const POOL_STATE_SIZE: usize = 1536;

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

    // 3. 构建完整的 PoolState
    Some(PoolState {
        program_id,
        dex_name,
        dex_display_name,
        bump: pool_data.bump,
        amm_config: pool_data.amm_config,
        owner: pool_data.owner,
        token_mint0: pool_data.token_mint0,
        token_mint1: pool_data.token_mint1,
        token_vault0: pool_data.token_vault0,
        token_vault1: pool_data.token_vault1,
        observation_key: pool_data.observation_key,
        mint_decimals0: pool_data.mint_decimals0,
        mint_decimals1: pool_data.mint_decimals1,
        tick_spacing: pool_data.tick_spacing,
        liquidity: pool_data.liquidity,
        sqrt_price_x64: pool_data.sqrt_price_x64,
        tick_current: pool_data.tick_current,
        padding3: pool_data.padding3,
        padding4: pool_data.padding4,
        fee_growth_global0_x64: pool_data.fee_growth_global0_x64,
        fee_growth_global1_x64: pool_data.fee_growth_global1_x64,
        protocol_fees_token0: pool_data.protocol_fees_token0,
        protocol_fees_token1: pool_data.protocol_fees_token1,
        swap_in_amount_token0: pool_data.swap_in_amount_token0,
        swap_out_amount_token1: pool_data.swap_out_amount_token1,
        swap_in_amount_token1: pool_data.swap_in_amount_token1,
        swap_out_amount_token0: pool_data.swap_out_amount_token0,
        status: pool_data.status,
        padding: pool_data.padding,
        reward_infos: pool_data.reward_infos,
        tick_array_bitmap: pool_data.tick_array_bitmap,
        total_fees_token0: pool_data.total_fees_token0,
        total_fees_claimed_token0: pool_data.total_fees_claimed_token0,
        total_fees_token1: pool_data.total_fees_token1,
        total_fees_claimed_token1: pool_data.total_fees_claimed_token1,
        fund_fees_token0: pool_data.fund_fees_token0,
        fund_fees_token1: pool_data.fund_fees_token1,
        open_time: pool_data.open_time,
        recent_epoch: pool_data.recent_epoch,
        padding1: pool_data.padding1,
        padding2: pool_data.padding2,
    })
}
