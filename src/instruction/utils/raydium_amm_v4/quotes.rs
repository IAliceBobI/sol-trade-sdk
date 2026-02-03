// Quote 计算函数

use crate::common::SolanaRpcClient;
use crate::utils::calc::raydium_amm_v4::{
    compute_swap_amount, quote_exact_out as calc_quote_exact_out,
};
use crate::utils::calc::raydium_amm_v4_official::{calculate_swap_with_fee, SwapDirection};
use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;

/// Quote an exact-in swap against a Raydium AMM V4 pool
///
/// 使用恒定乘积公式 (x * y = k) 计算预期输出金额
///
/// # Arguments
/// * `rpc` - Solana RPC 客户端
/// * `pool_address` - AMM V4 Pool 地址
/// * `amount_in` - 输入代币数量（最小单位）
/// * `is_coin_in` - true: coin -> pc, false: pc -> coin
///
/// # Returns
/// 返回 `QuoteExactInResult`，包含输出金额、手续费等
///
/// # Example
/// ```ignore
/// let quote = quote_exact_in(&rpc, &pool, 1_000_000, true).await?;
/// println!("预期输出: {} USDC", quote.amount_out);
/// ```
pub async fn quote_exact_in(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_in: u64,
    is_coin_in: bool,
) -> Result<crate::utils::quote::QuoteExactInResult, anyhow::Error> {
    // 1. 获取 Pool 状态
    let amm_info = super::pool_queries::get_pool_by_address(rpc, pool_address).await?;

    // 2. 获取实时储备余额
    let coin_balance = rpc.get_token_account_balance(&amm_info.token_coin).await?;
    let pc_balance = rpc.get_token_account_balance(&amm_info.token_pc).await?;

    let coin_reserve = coin_balance
        .amount
        .parse::<u64>()
        .map_err(|_| anyhow!("Failed to parse coin reserve"))?;
    let pc_reserve = pc_balance
        .amount
        .parse::<u64>()
        .map_err(|_| anyhow!("Failed to parse pc reserve"))?;

    // 3. 完全复制 Raydium 官方的计算逻辑
    // 源码: processor.rs:2393-2405

    // 确定方向：
    // 根据测试和 ray_log 验证：
    // - is_coin_in=false => 使用 PC 作为输入储备计算（PC->Coin 方向）
    // - is_coin_in=true  => 使用 Coin 作为输入储备计算（Coin->PC 方向）
    let swap_direction = if is_coin_in {
        SwapDirection::Coin2PC  // 使用 coin 作为输入计算
    } else {
        SwapDirection::PC2Coin   // 使用 pc 作为输入计算
    };

    // 使用链上的实际费用率
    let swap_fee_numerator = amm_info.fees.swap_fee_numerator;
    let swap_fee_denominator = amm_info.fees.swap_fee_denominator;

    // 调用官方计算函数
    let (swap_fee, amount_out) = calculate_swap_with_fee(
        amount_in,
        swap_fee_numerator,
        swap_fee_denominator,
        pc_reserve,          // total_pc_without_take_pnl
        coin_reserve,        // total_coin_without_take_pnl
        swap_direction,
    );

    // 4. 返回统一格式
    Ok(crate::utils::quote::QuoteExactInResult {
        amount_out,
        fee_amount: swap_fee,
        price_impact_bps: None,
        extra_accounts_read: 2,
    })
}

/// Quote an exact-out swap against a Raydium AMM V4 pool.
///
/// Calculates the required input amount to obtain a specific output amount.
///
/// # Arguments
///
/// * `rpc` - RPC client
/// * `pool_address` - Pool address
/// * `amount_out` - Desired output amount (in smallest units)
/// * `is_coin_in` - true if coin token is the input, false if PC token is the input
///
/// # Returns
///
/// Returns `QuoteExactOutResult` containing the required input amount and fees
///
/// # Example
/// ```ignore
/// let quote = quote_exact_out(&rpc, &pool, 1_000_000, true).await?;
/// println!("需要输入: {} lamports", quote.amount_in);
/// ```
pub async fn quote_exact_out(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_out: u64,
    is_coin_in: bool,
) -> Result<crate::utils::quote::QuoteExactOutResult, anyhow::Error> {
    // 1. 获取 Pool 状态
    let amm_info = super::pool_queries::get_pool_by_address(rpc, pool_address).await?;

    // 2. 获取实时储备余额
    let coin_balance = rpc.get_token_account_balance(&amm_info.token_coin).await?;
    let pc_balance = rpc.get_token_account_balance(&amm_info.token_pc).await?;

    let coin_reserve = coin_balance
        .amount
        .parse::<u64>()
        .map_err(|_| anyhow!("Failed to parse coin reserve"))?;
    let pc_reserve = pc_balance
        .amount
        .parse::<u64>()
        .map_err(|_| anyhow!("Failed to parse pc reserve"))?;

    // 3. 使用数学计算函数
    let result = calc_quote_exact_out(coin_reserve, pc_reserve, amount_out, is_coin_in)
        .map_err(|e| anyhow!("Quote exact out failed: {}", e))?;

    // 4. 返回统一格式
    Ok(crate::utils::quote::QuoteExactOutResult {
        amount_in: result.amount_in,
        fee_amount: result.fee_amount,
        price_impact_bps: result.price_impact_bps,
        extra_accounts_read: 2,
    })
}
