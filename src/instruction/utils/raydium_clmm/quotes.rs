// Raydium CLMM Quote 计算函数

use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;

use crate::{common::SolanaRpcClient, instruction::utils::raydium_clmm_types::amm_config_decode};

use super::{
    helpers::get_tick_array_start_index,
    pool_queries::{get_pool_by_address, get_tick_arrays},
};

/// Quote an exact-in swap against a Raydium CLMM pool.
///
/// IMPORTANT: This implementation currently assumes the swap does **not** cross initialized ticks
/// (i.e. stays within the current tick). It still reads the current tick array account to
/// validate availability and for future extension, but does not yet decode tick liquidity nets.
///
/// - `zero_for_one=true`: token0 -> token1
/// - `zero_for_one=false`: token1 -> token0
pub async fn quote_exact_in(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_in: u64,
    zero_for_one: bool,
) -> Result<crate::utils::quote::QuoteExactInResult, anyhow::Error> {
    let pool_state = get_pool_by_address(rpc, pool_address).await?;

    // 获取费率
    let amm_config = rpc.get_account(&pool_state.amm_config).await?;
    let config = amm_config_decode(&amm_config.data)
        .ok_or_else(|| anyhow!("Failed to decode amm config"))?;
    let fee_rate = config.trade_fee_rate as u32;

    // 获取 tick arrays（使用完整版计算）
    let current_tick_array_start =
        get_tick_array_start_index(pool_state.tick_current, pool_state.tick_spacing);

    // 计算需要获取的 tick arrays（当前 + 可能的下一个）
    let mut start_indices = vec![current_tick_array_start];
    if zero_for_one {
        // token0 -> token1，价格下降，需要向左获取
        start_indices.push(current_tick_array_start - (pool_state.tick_spacing as i32 * 60));
    } else {
        // token1 -> token0，价格上涨，需要向右获取
        start_indices.push(current_tick_array_start + (pool_state.tick_spacing as i32 * 60));
    }

    // 获取 tick arrays
    let tick_array_states = get_tick_arrays(rpc, pool_address, &start_indices).await?;

    // 转换为完整计算所需的格式
    type TickData = (i32, Vec<(i32, i128, u128)>);
    let mut tick_arrays: Vec<TickData> = Vec::new();

    for (start_index, tick_array_state) in tick_array_states {
        let ticks: Vec<(i32, i128, u128)> = tick_array_state
            .ticks
            .iter()
            .cloned()
            .filter_map(|tick_state| {
                // 只返回已初始化的 tick
                if tick_state.liquidity_gross > 0 {
                    Some((tick_state.tick, tick_state.liquidity_net, tick_state.liquidity_gross))
                } else {
                    None
                }
            })
            .collect();
        tick_arrays.push((start_index, ticks));
    }

    // 使用完整版 CLMM 计算
    let result = crate::utils::calc::raydium_clmm::calculate_swap_amount_with_tick_arrays(
        amount_in,
        pool_state.sqrt_price_x64,
        pool_state.liquidity,
        pool_state.tick_current,
        pool_state.tick_spacing,
        fee_rate,
        zero_for_one,
        &tick_arrays,
    )
    .map_err(|e| anyhow!("CLMM calculation failed: {}", e))?;

    Ok(crate::utils::quote::QuoteExactInResult {
        amount_out: result.amount_out,
        fee_amount: result.fee_amount,
        price_impact_bps: None, // TODO: 使用执行价格 vs spot 价格计算
        extra_accounts_read: tick_arrays.len(),
    })
}

/// Quote an exact-out swap against a Raydium CLMM pool (完整版本)
///
/// 使用完整的 tick array 遍历算法，支持大额交易和跨 tick 边界。
///
/// IMPORTANT: 此实现使用完整的 tick array 遍历，支持跨 tick 边界交易。
///
/// - `zero_for_one=true`: token0 -> token1 (卖出 token0)
/// - `zero_for_one=false`: token1 -> token0 (买入 token0)
///
/// # Arguments
///
/// * `rpc` - RPC 客户端
/// * `pool_address` - CLMM Pool 地址
/// * `amount_out` - 期望的输出金额（固定）
/// * `zero_for_one` - 交易方向
///
/// # Returns
///
/// 返回 `QuoteExactOutResult` 包含所需的输入金额和手续费
pub async fn quote_exact_out(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_out: u64,
    zero_for_one: bool,
) -> Result<crate::utils::calc::raydium_clmm::QuoteExactOutResult, anyhow::Error> {
    let pool_state = get_pool_by_address(rpc, pool_address).await?;

    // 获取费率
    let amm_config = rpc.get_account(&pool_state.amm_config).await?;
    let config = amm_config_decode(&amm_config.data)
        .ok_or_else(|| anyhow!("Failed to decode amm config"))?;
    let fee_rate = config.trade_fee_rate as u32;

    // 获取 tick arrays（使用完整版计算）
    let current_tick_array_start =
        get_tick_array_start_index(pool_state.tick_current, pool_state.tick_spacing);

    // 计算需要获取的 tick arrays（当前 + 可能的下一个）
    let mut start_indices = vec![current_tick_array_start];
    if zero_for_one {
        // token0 -> token1，价格下降，需要向左获取
        start_indices.push(current_tick_array_start - (pool_state.tick_spacing as i32 * 60));
    } else {
        // token1 -> token0，价格上涨，需要向右获取
        start_indices.push(current_tick_array_start + (pool_state.tick_spacing as i32 * 60));
    }

    // 获取 tick arrays
    let tick_array_states = get_tick_arrays(rpc, pool_address, &start_indices).await?;

    // 转换为完整计算所需的格式
    type TickData = (i32, Vec<(i32, i128, u128)>);
    let mut tick_arrays: Vec<TickData> = Vec::new();

    for (start_index, tick_array_state) in tick_array_states {
        let ticks: Vec<(i32, i128, u128)> = tick_array_state
            .ticks
            .iter()
            .cloned()
            .filter_map(|tick_state| {
                // 只返回已初始化的 tick
                if tick_state.liquidity_gross > 0 {
                    Some((tick_state.tick, tick_state.liquidity_net, tick_state.liquidity_gross))
                } else {
                    None
                }
            })
            .collect();
        tick_arrays.push((start_index, ticks));
    }

    // 使用完整版 CLMM exact_out 计算
    let result = crate::utils::calc::raydium_clmm::quote_exact_out(
        amount_out,
        pool_state.sqrt_price_x64,
        pool_state.liquidity,
        pool_state.tick_current,
        pool_state.tick_spacing,
        fee_rate,
        zero_for_one,
        &tick_arrays,
    )
    .map_err(|e| anyhow!("CLMM exact_out calculation failed: {}", e))?;

    Ok(result)
}
