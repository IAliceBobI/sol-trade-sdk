//! Raydium CLMM 指令构建辅助函数
//!
//! 提取重复的逻辑以提高代码可维护性。

use anyhow::{anyhow, Result};
use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};

use crate::instruction::raydium_clmm::helpers::amount_with_slippage;
use crate::instruction::utils::raydium_clmm::{
    get_tick_array_bitmap_extension_pda, get_tick_array_pda,
};
use crate::trading::core::params::RaydiumClmmParams;

/// Sqrt price limits for slippage protection
pub const MIN_SQRT_PRICE_X64: u128 = 4295048016;
pub const MAX_SQRT_PRICE_X64: u128 = 79226673521066979257578248091;

/// Tick range constants
pub const MIN_TICK: i32 = -443636;
pub const MAX_TICK: i32 = 443636;
pub const TICK_ARRAY_SIZE: i32 = 60;

/// SwapV2 指令 discriminator
pub const SWAP_V2_DISCRIMINATOR: &[u8] = &[43, 4, 237, 11, 26, 201, 30, 98];

/// Tick arrays 信息
pub struct TickArraysInfo {
    pub tick_array_pdas: Vec<Pubkey>,
    pub tick_array_bitmap_extension_pda: Pubkey,
}

/// 获取 Swap 所需的 tick arrays
///
/// # 参数
/// * `pool_state` - Pool 状态地址
/// * `pool_state_tick_current` - Pool 当前 tick
/// * `pool_state_tick_spacing` - Pool tick spacing
/// * `zero_for_one` - 交易方向（true=token0->token1, false=token1->token0）
///
/// # 返回
/// 返回 TickArraysInfo 包含所有需要的 tick array 地址
pub fn get_swap_tick_arrays(
    pool_state: &Pubkey,
    pool_state_tick_current: i32,
    pool_state_tick_spacing: u16,
    zero_for_one: bool,
) -> Result<TickArraysInfo> {
    let pool_state_struct = crate::instruction::utils::raydium_clmm_types::PoolState {
        tick_current: pool_state_tick_current,
        tick_spacing: pool_state_tick_spacing,
        ..Default::default()
    };

    let mut tick_array_start_index =
        crate::instruction::utils::raydium_clmm::get_first_initialized_tick_array_start_index(
            &pool_state_struct,
            zero_for_one,
        );

    let mut tick_array_pdas = Vec::new();
    let (first_tick_array_pda, _) = get_tick_array_pda(pool_state, tick_array_start_index)?;
    tick_array_pdas.push(first_tick_array_pda);

    // 获取后续的 tick arrays（最多 5 个）
    let tick_spacing = pool_state_tick_spacing as i32;
    let ticks_per_array = tick_spacing * TICK_ARRAY_SIZE;

    for _ in 0..4 {
        tick_array_start_index = if zero_for_one {
            tick_array_start_index - ticks_per_array
        } else {
            tick_array_start_index + ticks_per_array
        };

        // 检查是否超出范围
        if (zero_for_one && tick_array_start_index < MIN_TICK)
            || (!zero_for_one && tick_array_start_index > MAX_TICK)
        {
            break;
        }

        if let Ok((tick_array_pda, _)) = get_tick_array_pda(pool_state, tick_array_start_index) {
            tick_array_pdas.push(tick_array_pda);
        }
    }

    let (tick_array_bitmap_extension_pda, _) =
        get_tick_array_bitmap_extension_pda(pool_state);

    Ok(TickArraysInfo {
        tick_array_pdas,
        tick_array_bitmap_extension_pda,
    })
}

/// 构建 Swap 指令数据
///
/// # 参数
/// * `swap_amount` - Swap 金额
/// * `other_threshold` - 其他金额阈值
/// * `sqrt_price_limit_x64` - Sqrt 价格限制
/// * `is_base_input` - 是否为固定输入模式
pub fn build_swap_instruction_data(
    swap_amount: u64,
    other_threshold: u64,
    sqrt_price_limit_x64: u128,
    is_base_input: u8,
) -> Vec<u8> {
    let mut data = vec![0u8; 41];
    data[0..8].copy_from_slice(SWAP_V2_DISCRIMINATOR);
    data[8..16].copy_from_slice(&swap_amount.to_le_bytes());
    data[16..24].copy_from_slice(&other_threshold.to_le_bytes());
    data[24..40].copy_from_slice(&sqrt_price_limit_x64.to_le_bytes());
    data[40] = is_base_input;
    data
}

/// 构建 SwapV2 指令的账户列表
///
/// # 参数
/// * `payer` - 付款人公钥
/// * `protocol_params` - CLMM 协议参数
/// * `input_token_account` - 输入代币账户
/// * `output_token_account` - 输出代币账户
/// * `input_mint` - 输入 mint
/// * `output_mint` - 输出 mint
/// * `tick_arrays_info` - Tick arrays 信息
/// * `is_token0_in` - token0 是否为输入
pub fn build_swap_account_metas(
    payer: Pubkey,
    protocol_params: &RaydiumClmmParams,
    input_token_account: Pubkey,
    output_token_account: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
    tick_arrays_info: &TickArraysInfo,
    is_token0_in: bool,
) -> Vec<AccountMeta> {
    // 获取 vaults
    let (input_vault, output_vault) = if is_token0_in {
        (protocol_params.token0_vault, protocol_params.token1_vault)
    } else {
        (protocol_params.token1_vault, protocol_params.token0_vault)
    };

    let mut account_metas = vec![
        AccountMeta::new_readonly(payer, true), // 0. Payer (signer, readonly)
        AccountMeta::new_readonly(protocol_params.amm_config, false), // 1. Amm Config (readonly)
        AccountMeta::new(protocol_params.pool_state, false), // 2. Pool State (writable)
        AccountMeta::new(input_token_account, false), // 3. Input Token Account (writable)
        AccountMeta::new(output_token_account, false), // 4. Output Token Account (writable)
        AccountMeta::new(input_vault, false),         // 5. Input Vault (writable)
        AccountMeta::new(output_vault, false),        // 6. Output Vault (writable)
        AccountMeta::new(protocol_params.observation_state, false), // 7. Observation State (writable)
        AccountMeta::new_readonly(crate::constants::TOKEN_PROGRAM, false), // 8. Token Program (readonly)
        AccountMeta::new_readonly(crate::constants::TOKEN_2022_PROGRAM, false), // 9. Token 2022 Program (readonly)
        AccountMeta::new_readonly(crate::constants::MEMO_PROGRAM, false), // 10. Memo Program (readonly)
        AccountMeta::new_readonly(input_mint, false), // 11. Input Mint (readonly)
        AccountMeta::new_readonly(output_mint, false), // 12. Output Mint (readonly)
    ];

    // remainingAccounts: exTickArrayBitmap (readonly for SwapV2) + tickArrays (writable)
    account_metas.push(AccountMeta::new_readonly(
        tick_arrays_info.tick_array_bitmap_extension_pda,
        false,
    )); // 13. TickArray Bitmap Extension (readonly)

    // 添加额外的 tick arrays（全部 writable）
    for tick_array_pda in &tick_arrays_info.tick_array_pdas {
        account_metas.push(AccountMeta::new(*tick_array_pda, false));
    }

    account_metas
}

/// 计算 exact_out 模式的滑点后的输入金额
///
/// # 参数
/// * `input_amount` - 原始输入金额
/// * `slippage` - 滑点（基点）
/// * `is_round_up` - 是否向上取整
pub fn calculate_slippage_amount(input_amount: u64, slippage: u64, is_round_up: bool) -> u64 {
    if input_amount > 0 {
        amount_with_slippage(input_amount, slippage as u16, is_round_up)
    } else {
        0
    }
}

