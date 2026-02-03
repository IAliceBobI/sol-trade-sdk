// Raydium CLMM 辅助函数

use solana_sdk::pubkey::Pubkey;

use crate::constants::{SOL_MINT, USDC_MINT, USDT_MINT};

use super::constants::{seeds, TICKS_PER_ARRAY};

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};
    pub const RAYDIUM_CLMM: Pubkey = pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
}

/// 判断是否为 Hot Mint（主流桥接资产）
/// 当前包含：WSOL、USDC、USDT
pub fn is_hot_mint(mint: &Pubkey) -> bool {
    *mint == SOL_MINT || *mint == USDC_MINT || *mint == USDT_MINT
}

/// Calculate tick array PDA
///
/// # Arguments
/// * `pool_id` - Pool state account address
/// * `start_tick_index` - Starting tick index for the tick array
///
/// # Returns
/// (tick_array_pda, bump)
///
/// Note: Reference implementation uses to_be_bytes() for tick index
pub fn get_tick_array_pda(
    pool_id: &Pubkey,
    start_tick_index: i32,
) -> Result<(Pubkey, u8), anyhow::Error> {
    let tick_index_bytes = start_tick_index.to_be_bytes(); // Use big-endian like reference implementation
    Pubkey::try_find_program_address(
        &[seeds::TICK_ARRAY_SEED, pool_id.as_ref(), &tick_index_bytes],
        &accounts::RAYDIUM_CLMM,
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to find tick array PDA"))
}

/// Calculate tick array bitmap extension PDA
///
/// # Arguments
/// * `pool_id` - Pool state account address
///
/// # Returns
/// (tick_array_bitmap_extension_pda, bump)
pub fn get_tick_array_bitmap_extension_pda(pool_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::POOL_TICK_ARRAY_BITMAP_SEED, pool_id.as_ref()],
        &accounts::RAYDIUM_CLMM,
    )
}

/// Calculate tick array start index from current tick and tick spacing
///
/// # Arguments
/// * `tick_current` - Current tick
/// * `tick_spacing` - Tick spacing
///
/// # Returns
/// Starting tick index for the tick array containing the current tick
///
/// Each tick array contains 60 ticks (TICKS_PER_ARRAY = 60)
/// Implementation matches Raydium SDK V2: TickUtils.getTickArrayStartIndexByTick
///
/// Formula: getTickArrayBitIndex(tickIndex, tickSpacing) * tickCount(tickSpacing)
/// where tickCount = TICK_ARRAY_SIZE * tickSpacing
pub fn get_tick_array_start_index(tick_current: i32, tick_spacing: u16) -> i32 {
    let tick_spacing_i32 = tick_spacing as i32;

    // Calculate ticks per array (tickCount)
    let ticks_in_array = TICKS_PER_ARRAY * tick_spacing_i32;

    // Calculate tick array bit index (getTickArrayBitIndex)
    // This is the array index, not the tick index
    let mut start_index: i32 = tick_current / ticks_in_array;

    // Handle negative ticks: round down towards negative infinity
    if tick_current < 0 && tick_current % ticks_in_array != 0 {
        start_index = ((start_index as f64).ceil() as i32) - 1;
    } else {
        start_index = (start_index as f64).floor() as i32;
    }

    // Convert bit index to tick index
    start_index * ticks_in_array
}

/// Find first initialized tick array from bitmap
///
/// This is a simplified version. In production, you should use the full bitmap logic
/// from the pool state's tick_array_bitmap field.
///
/// # Arguments
/// * `pool_state` - Pool state
/// * `_zero_for_one` - Swap direction (true = token0 -> token1)
///
/// # Returns
/// First initialized tick array start index, or falls back to current tick's array
pub fn get_first_initialized_tick_array_start_index(
    pool_state: &crate::instruction::utils::raydium_clmm_types::PoolState,
    _zero_for_one: bool,
) -> i32 {
    // TODO: Implement full bitmap search logic
    // For now, fall back to current tick's array
    get_tick_array_start_index(pool_state.tick_current, pool_state.tick_spacing)
}
