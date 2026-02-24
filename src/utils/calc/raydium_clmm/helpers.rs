// Copyright (c) Raydium Foundation
// Licensed under Apache 2.0

//! CLMM swap 计算辅助函数

/// 在 tick arrays 中找到下一个初始化的 tick
pub(crate) fn find_next_initialized_tick(
    tick_arrays: &[(i32, Vec<(i32, i128, u128)>)],
    current_tick: i32,
    _tick_spacing: u16,
    zero_for_one: bool,
) -> Option<(i32, bool, i128)> {
    // 首先尝试找最接近的初始化 tick
    let mut best_tick: Option<(i32, bool, i128)> = None;

    for (_start_index, ticks) in tick_arrays {
        for &(tick, liquidity_net, liquidity_gross) in ticks {
            let is_initialized = liquidity_gross > 0;
            if !is_initialized {
                continue;
            }

            if zero_for_one {
                // token0 -> token1, 价格下降
                // 找小于等于当前 tick 的最大 tick
                if tick <= current_tick {
                    if best_tick.is_none() || tick > best_tick.as_ref().unwrap().0 {
                        best_tick = Some((tick, is_initialized, liquidity_net));
                    }
                }
            } else {
                // token1 -> token0, 价格上涨
                // 找大于当前 tick 的最小 tick
                if tick > current_tick {
                    if best_tick.is_none() || tick < best_tick.as_ref().unwrap().0 {
                        best_tick = Some((tick, is_initialized, liquidity_net));
                    }
                }
            }
        }
    }

    best_tick
}

/// 判断是否需要移动到下一个 tick array
pub(crate) fn needs_next_tick_array(
    current_tick: i32,
    tick_arrays: &[(i32, Vec<(i32, i128, u128)>)],
    current_idx: usize,
    tick_spacing: u16,
    zero_for_one: bool,
) -> bool {
    if current_idx >= tick_arrays.len() {
        return false;
    }

    let (start_index, _) = tick_arrays[current_idx];
    let ticks_in_array = 60 * (tick_spacing as i32);

    if zero_for_one {
        current_tick < start_index
    } else {
        current_tick >= start_index + ticks_in_array
    }
}
