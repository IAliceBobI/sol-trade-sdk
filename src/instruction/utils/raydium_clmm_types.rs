use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct PoolState {
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

pub fn pool_state_decode(data: &[u8]) -> Option<PoolState> {
    if data.len() < POOL_STATE_SIZE {
        return None;
    }
    borsh::from_slice::<PoolState>(&data[..POOL_STATE_SIZE]).ok()
}
