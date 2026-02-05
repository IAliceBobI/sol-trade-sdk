// Raydium CLMM 辅助函数

use solana_sdk::pubkey::Pubkey;

use crate::constants::{SOL_MINT, USDC_MINT, USDT_MINT};

use super::constants::{TICKS_PER_ARRAY, seeds};

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};
    pub const RAYDIUM_CLMM: Pubkey = pubkey!("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
}

/// 判断是否为 Hot Mint（主流桥接资产）
/// 当前包含：WSOL、USDC、USDT
pub(crate) fn is_hot_mint(mint: &Pubkey) -> bool {
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
pub(crate) fn get_tick_array_pda(
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
pub(crate) fn get_tick_array_bitmap_extension_pda(pool_id: &Pubkey) -> (Pubkey, u8) {
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
pub(crate) fn get_tick_array_start_index(tick_current: i32, tick_spacing: u16) -> i32 {
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
/// 使用完整的 bitmap 搜索逻辑，从 pool state 的 tick_array_bitmap 字段中查找
/// 在交易方向上第一个已初始化的 tick array。
///
/// # Arguments
/// * `pool_state` - Pool state
/// * `zero_for_one` - Swap direction (true = token0 -> token1, 向前搜索; false = token1 -> token0, 向后搜索)
///
/// # Returns
/// 第一个已初始化的 tick array 起始索引
///
/// # 算法说明
///
/// 参考 Raydium CLMM 官方实现（programs/amm/src/libraries/tick_array_bit_map.rs）：
///
/// 1. 将 PoolState 的 tick_array_bitmap ([u64; 16]) 转换为 U1024
/// 2. 计算当前 tick 所在的 array 起始索引
/// 3. 根据 zero_for_one 方向搜索下一个已初始化的 array：
///    - zero_for_one = true: 向前搜索（查找更低的 tick）
///    - zero_for_one = false: 向后搜索（查找更高的 tick）
/// 4. 如果找到，返回该 array 的起始索引；否则返回边界
///
/// # 实现状态
///
/// ✅ 已完成 - 移植自官方实现
///
/// 返回 `(bool, i32)` 元组：
/// - 第一个值表示"当前 tick array 是否已初始化"
/// - 第二个值是"应该使用的第一个 tick array 起始索引"
pub(crate) fn get_first_initialized_tick_array(
    pool_state: &crate::instruction::utils::raydium_clmm_types::PoolState,
    zero_for_one: bool,
) -> (bool, i32) {
    use crate::instruction::utils::raydium_clmm::tick_array_bitmap::{
        check_current_tick_array_is_initialized, next_initialized_tick_array_start_index,
        pool_bitmap_to_u1024,
    };

    // 将 PoolState 的 bitmap 转换为 U1024
    let bitmap = pool_bitmap_to_u1024(pool_state);

    // 首先检查当前 tick array 是否已初始化
    if let Some((is_initialized, start_index)) = check_current_tick_array_is_initialized(
        bitmap,
        pool_state.tick_current,
        pool_state.tick_spacing,
    ) {
        if is_initialized {
            // 当前 array 已初始化，直接使用
            return (true, start_index);
        }
    }

    // 当前 array 未初始化，查找下一个已初始化的 array
    let current_array_start =
        get_tick_array_start_index(pool_state.tick_current, pool_state.tick_spacing);

    let (found, next_array_start) = next_initialized_tick_array_start_index(
        bitmap,
        current_array_start,
        pool_state.tick_spacing,
        zero_for_one,
    );

    if found {
        (false, next_array_start)
    } else {
        // 未找到任何已初始化的 array，返回当前 array（虽然它未初始化）
        (false, current_array_start)
    }
}
