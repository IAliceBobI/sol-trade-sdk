// Quote 计算函数

use crate::common::SolanaRpcClient;
use crate::utils::calc::raydium_amm_v4::quote_exact_out as calc_quote_exact_out;
use crate::utils::calc::raydium_amm_v4_official::{SwapDirection, calculate_swap_with_fee};
use crate::utils::quote::{QuoteExactInParams, QuoteExactOutParams};
use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;

/// Quote an exact-in swap against a Raydium AMM V4 pool
///
/// 使用恒定乘积公式 (x * y = k) 计算预期输出金额
///
/// # Arguments
/// * `params` - Quote 参数，包含 pool_address、input_mint、output_mint、amount_in
///
/// # Examples
/// ```ignore
/// let params = QuoteExactInParams {
///     pool_address: pool_pubkey,
///     input_mint: coin_mint,
///     output_mint: pc_mint,
///     amount_in: 1_000_000,
/// };
/// let quote = quote_exact_in(&rpc, params).await?;
/// println!("预期输出: {} USDC", quote.amount_out);
/// ```
pub async fn quote_exact_in(
    rpc: &SolanaRpcClient,
    params: QuoteExactInParams,
) -> Result<crate::utils::quote::QuoteExactInResult, anyhow::Error> {
    // 1. 获取 Pool 状态
    let amm_info = super::pool_queries::get_pool_by_address(rpc, &params.pool_address).await?;

    // 2. 验证 input_mint 和 output_mint 是否在池子中
    let is_coin_in = params.input_mint == amm_info.coin_mint;
    let is_pc_in = params.input_mint == amm_info.pc_mint;

    if !is_coin_in && !is_pc_in {
        return Err(anyhow!(
            "Input mint {} not found in pool {} (coin={}, pc={})",
            params.input_mint,
            params.pool_address,
            amm_info.coin_mint,
            amm_info.pc_mint
        ));
    }

    let expected_output_mint = if is_coin_in {
        amm_info.pc_mint
    } else {
        amm_info.coin_mint
    };

    if params.output_mint != expected_output_mint {
        return Err(anyhow!(
            "Output mint mismatch: expected {}, got {}",
            expected_output_mint,
            params.output_mint
        ));
    }

    // 3. 获取实时储备余额
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

    // 4. 计算总储备金（扣除 PNL）- 这才是链上实际使用的储备金
    // 参考: math.rs:322-334 calc_total_without_pnl_no_orderbook
    let total_coin_without_pnl = coin_reserve
        .checked_sub(amm_info.out_put.need_take_pnl_coin)
        .ok_or_else(|| anyhow!("Subtraction overflow for coin reserve"))?;
    let total_pc_without_pnl = pc_reserve
        .checked_sub(amm_info.out_put.need_take_pnl_pc)
        .ok_or_else(|| anyhow!("Subtraction overflow for pc reserve"))?;

    // 5. 完全复制 Raydium 官方的计算逻辑
    // 源码: processor.rs:2393-2405

    // 确定方向：
    // 根据测试和 ray_log 验证：
    // - is_coin_in=false => 使用 PC 作为输入储备计算（PC->Coin 方向）
    // - is_coin_in=true  => 使用 Coin 作为输入储备计算（Coin->PC 方向）
    let swap_direction = if is_coin_in {
        SwapDirection::Coin2PC // 使用 coin 作为输入计算
    } else {
        SwapDirection::PC2Coin // 使用 pc 作为输入计算
    };

    // 使用链上的实际费用率
    let swap_fee_numerator = amm_info.fees.swap_fee_numerator;
    let swap_fee_denominator = amm_info.fees.swap_fee_denominator;

    // 调用官方计算函数（使用 total_without_pnl）
    let (swap_fee, amount_out) = calculate_swap_with_fee(
        params.amount_in,
        swap_fee_numerator,
        swap_fee_denominator,
        total_pc_without_pnl,   // total_pc_without_take_pnl
        total_coin_without_pnl, // total_coin_without_take_pnl
        swap_direction,
    );

    // 6. 返回统一格式
    Ok(crate::utils::quote::QuoteExactInResult {
        amount_out,
        fee_amount: swap_fee,
        price_impact_bps: None,
        extra_accounts_read: 2,
    })
}

/// Quote an exact-in swap against a Raydium AMM V4 pool (旧版接口，已废弃).
///
/// # Deprecated
///
/// 请使用新版本的 `quote_exact_in`，它使用 `QuoteExactInParams` 结构体参数。
#[deprecated(since = "4.1.0", note = "请使用 quote_exact_in(&rpc, QuoteExactInParams)")]
#[allow(dead_code)]
pub async fn quote_exact_in_legacy(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_in: u64,
    is_coin_in: bool,
) -> Result<crate::utils::quote::QuoteExactInResult, anyhow::Error> {
    let amm_info = super::pool_queries::get_pool_by_address(rpc, pool_address).await?;

    // 构建新版本的参数
    let (input_mint, output_mint) = if is_coin_in {
        (amm_info.coin_mint, amm_info.pc_mint)
    } else {
        (amm_info.pc_mint, amm_info.coin_mint)
    };

    let params = QuoteExactInParams {
        pool_address: *pool_address,
        input_mint,
        output_mint,
        amount_in,
    };

    quote_exact_in(rpc, params).await
}

/// Quote an exact-out swap against a Raydium AMM V4 pool.
///
/// 计算需要多少输入金额才能获得指定的输出金额。
///
/// # Arguments
///
/// * `params` - Quote 参数，包含 pool_address、input_mint、output_mint、amount_out
///
/// # Examples
///
/// ```ignore
/// let params = QuoteExactOutParams {
///     pool_address: pool_pubkey,
///     input_mint: coin_mint,
///     output_mint: pc_mint,
///     amount_out: 1_000_000,
/// };
/// let quote = quote_exact_out(&rpc, params).await?;
/// println!("需要输入: {} lamports", quote.amount_in);
/// ```
pub async fn quote_exact_out(
    rpc: &SolanaRpcClient,
    params: QuoteExactOutParams,
) -> Result<crate::utils::quote::QuoteExactOutResult, anyhow::Error> {
    // 1. 获取 Pool 状态
    let amm_info = super::pool_queries::get_pool_by_address(rpc, &params.pool_address).await?;

    // 2. 验证 input_mint 和 output_mint 是否在池子中
    let is_coin_in = params.input_mint == amm_info.coin_mint;
    let is_pc_in = params.input_mint == amm_info.pc_mint;

    if !is_coin_in && !is_pc_in {
        return Err(anyhow!(
            "Input mint {} not found in pool {} (coin={}, pc={})",
            params.input_mint,
            params.pool_address,
            amm_info.coin_mint,
            amm_info.pc_mint
        ));
    }

    let expected_output_mint = if is_coin_in {
        amm_info.pc_mint
    } else {
        amm_info.coin_mint
    };

    if params.output_mint != expected_output_mint {
        return Err(anyhow!(
            "Output mint mismatch: expected {}, got {}",
            expected_output_mint,
            params.output_mint
        ));
    }

    // 3. 获取实时储备余额
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

    // 4. 使用数学计算函数
    let result = calc_quote_exact_out(
        coin_reserve,
        pc_reserve,
        params.amount_out,
        is_coin_in,
    )
    .map_err(|e| anyhow!("Quote exact out failed: {}", e))?;

    // 5. 返回统一格式
    Ok(crate::utils::quote::QuoteExactOutResult {
        amount_in: result.amount_in,
        fee_amount: result.fee_amount,
        price_impact_bps: result.price_impact_bps,
        extra_accounts_read: 2,
    })
}

/// Quote an exact-out swap against a Raydium AMM V4 pool (旧版接口，已废弃).
///
/// # Deprecated
///
/// 请使用新版本的 `quote_exact_out`，它使用 `QuoteExactOutParams` 结构体参数。
#[deprecated(since = "4.1.0", note = "请使用 quote_exact_out(&rpc, QuoteExactOutParams)")]
#[allow(dead_code)]
pub async fn quote_exact_out_legacy(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_out: u64,
    is_coin_in: bool,
) -> Result<crate::utils::quote::QuoteExactOutResult, anyhow::Error> {
    let amm_info = super::pool_queries::get_pool_by_address(rpc, pool_address).await?;

    // 构建新版本的参数
    let (input_mint, output_mint) = if is_coin_in {
        (amm_info.coin_mint, amm_info.pc_mint)
    } else {
        (amm_info.pc_mint, amm_info.coin_mint)
    };

    let params = QuoteExactOutParams {
        pool_address: *pool_address,
        input_mint,
        output_mint,
        amount_out,
    };

    quote_exact_out(rpc, params).await
}
