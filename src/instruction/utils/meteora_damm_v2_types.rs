use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct BaseFeeStruct {
    pub cliff_fee_numerator: u64,
    pub fee_scheduler_mode: u8,
    pub padding_0: [u8; 5],
    pub number_of_period: u16,
    pub period_frequency: u64,
    pub reduction_factor: u64,
    pub padding_1: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct DynamicFeeStruct {
    pub initialized: u8,
    pub padding: [u8; 7],
    pub max_volatility_accumulator: u32,
    pub variable_fee_control: u32,
    pub bin_step: u16,
    pub filter_period: u16,
    pub decay_period: u16,
    pub reduction_factor: u16,
    pub last_update_timestamp: u64,
    pub bin_step_u128: u128,
    pub sqrt_price_reference: u128,
    pub volatility_accumulator: u128,
    pub volatility_reference: u128,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct PoolFeesStruct {
    pub base_fee: BaseFeeStruct,
    pub protocol_fee_percent: u8,
    pub partner_fee_percent: u8,
    pub referral_fee_percent: u8,
    pub padding_0: [u8; 5],
    pub dynamic_fee: DynamicFeeStruct,
    pub padding_1: [u64; 2],
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct PoolMetrics {
    pub total_lp_a_fee: u128,
    pub total_lp_b_fee: u128,
    pub total_protocol_a_fee: u64,
    pub total_protocol_b_fee: u64,
    pub total_partner_a_fee: u64,
    pub total_partner_b_fee: u64,
    pub total_position: u64,
    pub padding: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct RewardInfo {
    pub initialized: u8,
    pub reward_token_flag: u8,
    pub padding_0: [u8; 6],
    pub padding_1: [u8; 8],
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub funder: Pubkey,
    pub reward_duration: u64,
    pub reward_duration_end: u64,
    pub reward_rate: u128,
    pub reward_per_token_stored: [u8; 32],
    pub last_update_time: u64,
    pub cumulative_seconds_with_empty_liquidity_reward: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pool {
    // ===== 元数据字段 =====
    /// DEX Program ID（从账户 owner 自动填充）
    pub program_id: Pubkey,

    /// DEX 协议名称（如 "meteora_damm_v2"）
    pub dex_name: String,

    /// DEX 显示名称（如 "Meteora DAMM V2"）
    pub dex_display_name: String,

    // ===== 链上数据字段 =====
    pub pool_fees: PoolFeesStruct,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub whitelisted_vault: Pubkey,
    pub partner: Pubkey,
    pub liquidity: u128,
    pub padding: u128,
    pub protocol_a_fee: u64,
    pub protocol_b_fee: u64,
    pub partner_a_fee: u64,
    pub partner_b_fee: u64,
    pub sqrt_min_price: u128,
    pub sqrt_max_price: u128,
    pub sqrt_price: u128,
    pub activation_point: u64,
    pub activation_type: u8,
    pub pool_status: u8,
    pub token_a_flag: u8,
    pub token_b_flag: u8,
    pub collect_fee_mode: u8,
    pub pool_type: u8,
    pub padding_0: [u8; 2],
    pub fee_a_per_liquidity: [u8; 32],
    pub fee_b_per_liquidity: [u8; 32],
    pub permanent_lock_liquidity: u128,
    pub metrics: PoolMetrics,
    pub padding_1: [u64; 10],
    pub reward_infos: [RewardInfo; 2],
}

/// 辅助结构体：只包含链上数据的字段（用于 Borsh 反序列化）
#[derive(BorshDeserialize)]
pub struct PoolDataOnly {
    pub pool_fees: PoolFeesStruct,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub whitelisted_vault: Pubkey,
    pub partner: Pubkey,
    pub liquidity: u128,
    pub padding: u128,
    pub protocol_a_fee: u64,
    pub protocol_b_fee: u64,
    pub partner_a_fee: u64,
    pub partner_b_fee: u64,
    pub sqrt_min_price: u128,
    pub sqrt_max_price: u128,
    pub sqrt_price: u128,
    pub activation_point: u64,
    pub activation_type: u8,
    pub pool_status: u8,
    pub token_a_flag: u8,
    pub token_b_flag: u8,
    pub collect_fee_mode: u8,
    pub pool_type: u8,
    pub padding_0: [u8; 2],
    pub fee_a_per_liquidity: [u8; 32],
    pub fee_b_per_liquidity: [u8; 32],
    pub permanent_lock_liquidity: u128,
    pub metrics: PoolMetrics,
    pub padding_1: [u64; 10],
    pub reward_infos: [RewardInfo; 2],
}

pub const POOL_SIZE: usize = 1104;

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
        Some(protocol) => (protocol.name().to_string(), protocol.display_name().to_string()),
        None => {
            // 未知 DEX，使用 fallback
            let fallback = "unknown".to_string();
            (fallback.clone(), fallback)
        },
    };

    // 3. 构建完整的 Pool
    Some(Pool {
        program_id,
        dex_name,
        dex_display_name,
        pool_fees: pool_data.pool_fees,
        token_a_mint: pool_data.token_a_mint,
        token_b_mint: pool_data.token_b_mint,
        token_a_vault: pool_data.token_a_vault,
        token_b_vault: pool_data.token_b_vault,
        whitelisted_vault: pool_data.whitelisted_vault,
        partner: pool_data.partner,
        liquidity: pool_data.liquidity,
        padding: pool_data.padding,
        protocol_a_fee: pool_data.protocol_a_fee,
        protocol_b_fee: pool_data.protocol_b_fee,
        partner_a_fee: pool_data.partner_a_fee,
        partner_b_fee: pool_data.partner_b_fee,
        sqrt_min_price: pool_data.sqrt_min_price,
        sqrt_max_price: pool_data.sqrt_max_price,
        sqrt_price: pool_data.sqrt_price,
        activation_point: pool_data.activation_point,
        activation_type: pool_data.activation_type,
        pool_status: pool_data.pool_status,
        token_a_flag: pool_data.token_a_flag,
        token_b_flag: pool_data.token_b_flag,
        collect_fee_mode: pool_data.collect_fee_mode,
        pool_type: pool_data.pool_type,
        padding_0: pool_data.padding_0,
        fee_a_per_liquidity: pool_data.fee_a_per_liquidity,
        fee_b_per_liquidity: pool_data.fee_b_per_liquidity,
        permanent_lock_liquidity: pool_data.permanent_lock_liquidity,
        metrics: pool_data.metrics,
        padding_1: pool_data.padding_1,
        reward_infos: pool_data.reward_infos,
    })
}
