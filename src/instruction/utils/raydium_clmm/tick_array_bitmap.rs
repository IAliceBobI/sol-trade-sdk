// Copyright (c) Raydium Foundation
// Licensed under Apache 2.0
// Source: https://github.com/raydium-io/raydium-clmm/programs/amm/src/libraries/tick_array_bit_map.rs
// 移植并适配为客户端使用

//! Tick Array Bitmap 辅助函数
//!
//! 用于查找已初始化的 tick array，支持向前和向后搜索。

use crate::instruction::utils::raydium_clmm::constants::TICKS_PER_ARRAY;
use crate::instruction::utils::raydium_clmm_types::PoolState;
use crate::utils::calc::clmm_math::big_num::U1024;

pub const TICK_ARRAY_BITMAP_SIZE: i32 = 512;

/// 获取 bitmap 中最大的 tick
pub fn max_tick_in_tickarray_bitmap(tick_spacing: u16) -> i32 {
    i32::from(tick_spacing) * TICKS_PER_ARRAY * TICK_ARRAY_BITMAP_SIZE
}

/// 检查当前 tick array 是否已初始化
///
/// # 参数
/// * `bitmap` - 1024 位 bitmap
/// * `tick_current` - 当前 tick
/// * `tick_spacing` - tick 间距
///
/// # 返回
/// (是否已初始化, tick array 起始索引)
pub fn check_current_tick_array_is_initialized(
    bitmap: U1024,
    tick_current: i32,
    tick_spacing: u16,
) -> Option<(bool, i32)> {
    // 检查边界
    if !(-443636..=443636).contains(&tick_current) {
        return None;
    }

    let multiplier = i32::from(tick_spacing) * TICKS_PER_ARRAY;
    let mut compressed = tick_current / multiplier + 512;
    if tick_current < 0 && tick_current % multiplier != 0 {
        // 向下取整（朝负无穷方向）
        compressed -= 1;
    }
    let bit_pos = compressed.unsigned_abs() as usize;

    // 检查当前位是否已设置
    let initialized = bitmap.bit(bit_pos);
    let start_index = (compressed - 512) * multiplier;

    Some((initialized, start_index))
}

/// 查找下一个已初始化的 tick array
///
/// # 参数
/// * `bitmap` - 1024 位 bitmap
/// * `last_tick_array_start_index` - 上一个 tick array 起始索引
/// * `tick_spacing` - tick 间距
/// * `zero_for_one` - 交易方向（true=向前/token0->token1, false=向后/token1->token0）
///
/// # 返回
/// (是否找到, 下一个 tick array 起始索引)
///
/// # 算法说明
///
/// 参考 Raydium CLMM 官方实现（programs/amm/src/libraries/tick_array_bit_map.rs）
pub fn next_initialized_tick_array_start_index(
    bitmap: U1024,
    last_tick_array_start_index: i32,
    tick_spacing: u16,
    zero_for_one: bool,
) -> (bool, i32) {
    // 验证 last_tick_array_start_index 是否有效
    let ticks_per_array = tick_count(tick_spacing);
    if last_tick_array_start_index % ticks_per_array != 0 {
        return (false, last_tick_array_start_index);
    }

    let tick_boundary = max_tick_in_tickarray_bitmap(tick_spacing);
    let next_tick_array_start_index = if zero_for_one {
        last_tick_array_start_index - ticks_per_array
    } else {
        last_tick_array_start_index + ticks_per_array
    };

    // 边界检查
    if next_tick_array_start_index < -tick_boundary || next_tick_array_start_index >= tick_boundary
    {
        return (false, last_tick_array_start_index);
    }

    let multiplier = i32::from(tick_spacing) * TICKS_PER_ARRAY;
    let mut compressed = next_tick_array_start_index / multiplier + 512;
    if next_tick_array_start_index < 0 && next_tick_array_start_index % multiplier != 0 {
        // 向下取整（朝负无穷方向）
        compressed -= 1;
    }
    let bit_pos = compressed.unsigned_abs() as usize;

    if zero_for_one {
        // tick 从高到低（向前搜索）
        // 查找 [0, bit_pos] 范围内的最高位（包括 bit_pos）
        // 如果 bit_pos 对应的 array 已初始化，返回它
        // 否则，返回 [0, bit_pos) 范围内的最高位

        // 先检查 bit_pos 本身
        if bitmap.bit(bit_pos) {
            let next_array_start_index = (bit_pos as i32 - 512) * multiplier;
            return (true, next_array_start_index);
        }

        // 如果 bit_pos 未初始化，查找 [0, bit_pos) 范围内的最高位
        if let Some(bit) = search_highest_bit_in_range(bitmap, 0, bit_pos) {
            let next_array_start_index = (bit as i32 - 512) * multiplier;
            (true, next_array_start_index)
        } else {
            // 未找到，返回边界
            (false, -tick_boundary)
        }
    } else {
        // tick 从低到高（向后搜索）
        // 查找 [bit_pos, 1024) 范围内的最低位（包括 bit_pos）
        // 如果 bit_pos 对应的 array 已初始化，返回它
        // 否则，返回 (bit_pos, 1024) 范围内的最低位

        // 先检查 bit_pos 本身
        if bitmap.bit(bit_pos) {
            let next_array_start_index = (bit_pos as i32 - 512) * multiplier;
            return (true, next_array_start_index);
        }

        // 如果 bit_pos 未初始化，查找 (bit_pos, 1024) 范围内的最低位
        if let Some(bit) = search_lowest_bit_in_range(bitmap, bit_pos + 1, 1024) {
            let next_array_start_index = (bit as i32 - 512) * multiplier;
            (true, next_array_start_index)
        } else {
            // 未找到，返回边界
            let boundary = tick_boundary - ticks_per_array;
            (false, boundary)
        }
    }
}

/// 在 [start, end) 范围内查找最高位（最大的设置为 1 的位）
fn search_highest_bit_in_range(bitmap: U1024, start: usize, end: usize) -> Option<u32> {
    for i in (start..end).rev() {
        if bitmap.bit(i) {
            return Some(i as u32);
        }
    }
    None
}

/// 在 [start, end) 范围内查找最低位（最小的设置为 1 的位）
fn search_lowest_bit_in_range(bitmap: U1024, start: usize, end: usize) -> Option<u32> {
    for i in start..end {
        if bitmap.bit(i) {
            return Some(i as u32);
        }
    }
    None
}

/// 查找最高有效位（从高位到低位第一个非零位）
///
/// 返回最高有效位的位置（0-1023）
#[allow(dead_code)]
fn most_significant_bit(x: U1024) -> Option<u32> {
    if x.is_zero() {
        None
    } else {
        // U1024 有 1024 位
        // leading_zeros 返回前导零的数量
        // 最高位位置 = 1024 - leading_zeros - 1
        Some(1024 - x.leading_zeros() - 1)
    }
}

/// 查找最低有效位（从低位到高位第一个非零位）
///
/// 返回最低有效位的位置（0-1023）
#[allow(dead_code)]
fn least_significant_bit(x: U1024) -> Option<u32> {
    if x.is_zero() {
        None
    } else {
        // trailing_zeros 直接返回末尾零的数量，即最低位位置
        Some(x.trailing_zeros())
    }
}

/// 计算 tick array 中的 tick 数量
fn tick_count(tick_spacing: u16) -> i32 {
    i32::from(tick_spacing) * TICKS_PER_ARRAY
}

/// 从 PoolState 的 tick_array_bitmap 创建 U1024
pub fn pool_bitmap_to_u1024(pool_state: &PoolState) -> U1024 {
    U1024(pool_state.tick_array_bitmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_tick_in_tickarray_bitmap() {
        assert_eq!(max_tick_in_tickarray_bitmap(1), 60 * 512);
        assert_eq!(max_tick_in_tickarray_bitmap(8), 8 * 60 * 512);
        assert_eq!(max_tick_in_tickarray_bitmap(64), 64 * 60 * 512);
    }

    #[test]
    fn test_tick_count() {
        assert_eq!(tick_count(1), 60);
        assert_eq!(tick_count(8), 480);
        assert_eq!(tick_count(64), 3840);
    }

    #[test]
    fn test_check_current_tick_array_is_initialized() {
        // 创建一个 bitmap，只初始化了 array 0 (bit 512)
        let mut bitmap_arr = [0u64; 16];
        bitmap_arr[8] = 1; // bit 512 在数组中的位置（512/64 = 8）
        let bitmap = U1024(bitmap_arr);

        // 测试 tick 0（对应 array 0，已初始化）
        let result = check_current_tick_array_is_initialized(bitmap, 0, 1);
        assert!(result.is_some());
        let (initialized, start_index) = result.unwrap();
        assert!(initialized);
        assert_eq!(start_index, 0);

        // 测试 tick 600（对应 array 1，未初始化）
        let result = check_current_tick_array_is_initialized(bitmap, 600, 1);
        assert!(result.is_some());
        let (initialized, start_index) = result.unwrap();
        assert!(!initialized);
        assert_eq!(start_index, 600);
    }

    #[test]
    fn test_next_initialized_tick_array_forward() {
        // 创建全 1 的 bitmap
        let bitmap = U1024([u64::MAX; 16]);

        // 向前搜索（zero_for_one = true）：从较高的 tick 搜索较低的 tick
        // 从 tick 60 开始向前搜索，应该找到 tick 0（array 0）
        let (found, index) = next_initialized_tick_array_start_index(bitmap, 60, 1, true);
        println!("Forward from 60: found={}, index={}", found, index);
        assert!(found);
        assert_eq!(index, 0);

        // 从 tick 0 开始向前搜索，应该找到 tick -60（array -1）
        let (found, index) = next_initialized_tick_array_start_index(bitmap, 0, 1, true);
        println!("Forward from 0: found={}, index={}", found, index);
        assert!(found);
        assert_eq!(index, -60);
    }

    #[test]
    fn test_next_initialized_tick_array_backward() {
        // 创建全 1 的 bitmap
        let bitmap = U1024([u64::MAX; 16]);

        // 向后搜索（zero_for_one = false）：从较低的 tick 搜索较高的 tick
        // 从 tick 0 开始向后搜索，应该找到 tick 60（array 1）
        let (found, index) = next_initialized_tick_array_start_index(bitmap, 0, 1, false);
        assert!(found);
        assert_eq!(index, 60);

        // 从 tick 60 开始向后搜索，应该找到 tick 120（array 2）
        let (found, index) = next_initialized_tick_array_start_index(bitmap, 60, 1, false);
        assert!(found);
        assert_eq!(index, 120);
    }

    #[test]
    fn test_most_significant_bit() {
        // U1024([0, ..., 0, 1]) - 第 15 个 u64 的第 0 位被设置，位置是 15*64+0 = 960
        let bitmap = U1024([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let bit = most_significant_bit(bitmap);
        assert_eq!(bit, Some(960));

        // U1024([1, 0, ..., 0]) - 第 0 个 u64 的第 0 位被设置，位置是 0
        let bitmap = U1024([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let bit = most_significant_bit(bitmap);
        assert_eq!(bit, Some(0));

        // U1024([0x8000000000000000, 0, ..., 0]) - 第 0 个 u64 的第 63 位被设置，位置是 63
        let mut arr = [0u64; 16];
        arr[0] = 1u64 << 63;
        let bitmap = U1024(arr);
        let bit = most_significant_bit(bitmap);
        assert_eq!(bit, Some(63));

        // 最高位在 1023 位置（第 15 个 u64 的第 63 位）
        let mut arr = [0u64; 16];
        arr[15] = 1u64 << 63;
        let bitmap = U1024(arr);
        let bit = most_significant_bit(bitmap);
        assert_eq!(bit, Some(1023));
    }

    #[test]
    fn test_least_significant_bit() {
        let bitmap = U1024([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let bit = least_significant_bit(bitmap);
        assert_eq!(bit, Some(0));

        let bitmap = U1024([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        let bit = least_significant_bit(bitmap);
        assert_eq!(bit, Some(1024 - 64));
    }
}
