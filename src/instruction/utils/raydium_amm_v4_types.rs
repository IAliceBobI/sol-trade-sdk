//! Raydium AMM V4 (Raydium Liquidity Pool V4) 类型定义
//!
//! Raydium AMM V4 是 Raydium 的传统自动做市商（AMM）协议，使用恒定乘积公式（x * y = k）进行流动性提供和交易。
//!
//! ## 程序信息
//! - **程序名称**: Raydium Liquidity Pool V4
//! - **程序地址**: `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`
//! - **特性**: 集成 Serum 订单簿，支持限价单和市价单
//!
//! ## 费用结构
//! - **交易费**: 0.25% (25/10000)
//! - **Swap 费**: 0.25% (25/10000)
//! - **总费用**: 0.5%
//!
//! ## 类型定义
//! - `AmmInfo`: AMM pool 状态信息
//! - `Fees`: 费用配置
//! - `OutPutData`: 输出数据

use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct Fees {
    pub min_separate_numerator: u64,
    pub min_separate_denominator: u64,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub pnl_numerator: u64,
    pub pnl_denominator: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct OutPutData {
    pub need_take_pnl_coin: u64,
    pub need_take_pnl_pc: u64,
    pub total_pnl_pc: u64,
    pub total_pnl_coin: u64,
    pub pool_open_time: u64,
    pub punish_pc_amount: u64,
    pub punish_coin_amount: u64,
    pub orderbook_to_init_time: u64,
    pub swap_coin_in_amount: u128,
    pub swap_pc_out_amount: u128,
    pub swap_take_pc_fee: u64,
    pub swap_pc_in_amount: u128,
    pub swap_coin_out_amount: u128,
    pub swap_take_coin_fee: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct AmmInfo {
    pub status: u64,
    pub nonce: u64,
    pub order_num: u64,
    pub depth: u64,
    pub coin_decimals: u64,
    pub pc_decimals: u64,
    pub state: u64,
    pub reset_flag: u64,
    pub min_size: u64,
    pub vol_max_cut_ratio: u64,
    pub amount_wave: u64,
    pub coin_lot_size: u64,
    pub pc_lot_size: u64,
    pub min_price_multiplier: u64,
    pub max_price_multiplier: u64,
    pub sys_decimal_value: u64,
    pub fees: Fees,
    pub out_put: OutPutData,
    pub token_coin: Pubkey,
    pub token_pc: Pubkey,
    pub coin_mint: Pubkey,
    pub pc_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub open_orders: Pubkey,
    pub market: Pubkey,
    pub serum_dex: Pubkey,
    pub target_orders: Pubkey,
    pub withdraw_queue: Pubkey,
    pub token_temp_lp: Pubkey,
    pub amm_owner: Pubkey,
    pub lp_amount: u64,
    pub client_order_id: u64,
    pub padding: [u64; 2],
}

pub const AMM_INFO_SIZE: usize = 752;

pub fn amm_info_decode(data: &[u8]) -> Option<AmmInfo> {
    if data.len() < AMM_INFO_SIZE {
        return None;
    }
    borsh::from_slice::<AmmInfo>(&data[..AMM_INFO_SIZE]).ok()
}
