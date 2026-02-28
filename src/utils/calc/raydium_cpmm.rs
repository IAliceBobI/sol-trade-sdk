use crate::instruction::utils::raydium_cpmm::accounts::FEE_RATE_DENOMINATOR_VALUE;

/// Computes trading fee using ceiling division.
///
/// # Arguments
/// * `amount` - The amount to calculate fee for
/// * `fee_rate` - The fee rate to apply
///
/// # Returns
/// The calculated trading fee
#[inline(always)]
fn compute_trading_fee(amount: u64, fee_rate: u64) -> u64 {
    let numerator = (amount as u128) * (fee_rate as u128);
    numerator.div_ceil(FEE_RATE_DENOMINATOR_VALUE) as u64
}

/// Computes protocol or fund fee using floor division.
///
/// # Arguments
/// * `amount` - The amount to calculate fee for
/// * `fee_rate` - The fee rate to apply
///
/// # Returns
/// The calculated protocol or fund fee
#[inline(always)]
fn compute_protocol_fund_fee(amount: u64, fee_rate: u64) -> u64 {
    let numerator = (amount as u128) * (fee_rate as u128);
    (numerator / FEE_RATE_DENOMINATOR_VALUE) as u64
}

/// Computes creator fee using ceiling division.
///
/// # Arguments
/// * `amount` - The amount to calculate fee for
/// * `fee_rate` - The fee rate to apply
///
/// # Returns
/// The calculated creator fee
#[inline(always)]
fn compute_creator_fee_new(amount: u64, fee_rate: u64) -> u64 {
    let numerator = (amount as u128) * (fee_rate as u128);
    numerator.div_ceil(FEE_RATE_DENOMINATOR_VALUE) as u64
}

/// Parameters for computing swap amounts and fees.
#[derive(Debug, Clone)]
pub struct ComputeSwapParams {
    /// Whether the entire input amount is traded
    pub all_trade: bool,
    /// The input amount for the swap
    pub amount_in: u64,
    /// The expected output amount from the swap
    pub amount_out: u64,
    /// The minimum acceptable output amount (considering slippage_basis_points)
    pub min_amount_out: u64,
    /// The trading fee amount
    pub fee: u64,
}

/// Result of a swap calculation containing all relevant amounts and fees.
#[derive(Debug, Clone)]
pub struct SwapResult {
    /// The new amount in the input vault after the swap
    pub new_input_vault_amount: u64,
    /// The new amount in the output vault after the swap
    pub new_output_vault_amount: u64,
    /// The actual input amount used in the swap
    pub input_amount: u64,
    /// The actual output amount received from the swap
    pub output_amount: u64,
    /// The trading fee charged
    pub trade_fee: u64,
    /// The protocol fee charged
    pub protocol_fee: u64,
    /// The fund fee charged
    pub fund_fee: u64,
    /// The creator fee charged
    pub creator_fee: u64,
}

/// Performs a swap calculation based on input amount.
///
/// Calculates the output amount and all associated fees when swapping a specific input amount.
///
/// # Arguments
/// * `input_amount` - The amount of input tokens to swap
/// * `input_vault_amount` - Current amount in the input token vault
/// * `output_vault_amount` - Current amount in the output token vault
/// * `trade_fee_rate` - The trading fee rate
/// * `creator_fee_rate` - The creator fee rate
/// * `protocol_fee_rate` - The protocol fee rate
/// * `fund_fee_rate` - The fund fee rate
/// * `is_creator_fee_on_input` - Whether creator fee is charged on input tokens
///
/// # Returns
/// A `SwapResult` containing all swap calculations and fees
#[inline]
fn swap_base_input(
    input_amount: u64,
    input_vault_amount: u64,
    output_vault_amount: u64,
    trade_fee_rate: u64,
    creator_fee_rate: u64,
    protocol_fee_rate: u64,
    fund_fee_rate: u64,
    is_creator_fee_on_input: bool,
) -> SwapResult {
    let mut creator_fee = 0u64;

    // 根据 Raydium CPMM 官方实现修复
    // protocol_fee 和 fund_fee 应该从 trade_fee 计算，而不是从 input_amount
    let trade_fee = compute_trading_fee(input_amount, trade_fee_rate);
    let protocol_fee = compute_protocol_fund_fee(trade_fee, protocol_fee_rate);
    let fund_fee = compute_protocol_fund_fee(trade_fee, fund_fee_rate);

    // Creator fee 根据配置从输入或输出扣除
    let input_amount_less_fees = if is_creator_fee_on_input {
        creator_fee = compute_creator_fee_new(input_amount, creator_fee_rate);
        input_amount
            .saturating_sub(trade_fee)
            .saturating_sub(protocol_fee)
            .saturating_sub(fund_fee)
            .saturating_sub(creator_fee)
    } else {
        input_amount
            .saturating_sub(trade_fee)
            .saturating_sub(protocol_fee)
            .saturating_sub(fund_fee)
    };

    // 使用扣除费用后的金额进行恒定乘积计算
    let output_amount_swapped = ((output_vault_amount as u128)
        .saturating_mul(input_amount_less_fees as u128)
        / (input_vault_amount as u128).saturating_add(input_amount_less_fees as u128))
        as u64;

    // 如果 creator_fee 不从输入扣除，则从输出扣除
    let output_amount = if is_creator_fee_on_input {
        output_amount_swapped
    } else {
        creator_fee = compute_creator_fee_new(output_amount_swapped, creator_fee_rate);
        output_amount_swapped.saturating_sub(creator_fee)
    };

    SwapResult {
        new_input_vault_amount: input_vault_amount.saturating_add(input_amount_less_fees),
        new_output_vault_amount: output_vault_amount.saturating_sub(output_amount_swapped),
        input_amount,
        output_amount,
        trade_fee,
        protocol_fee,
        fund_fee,
        creator_fee,
    }
}

/// Computes swap parameters including amounts, fees, and slippage protection.
///
/// This function calculates the expected output amount, minimum output amount (with slippage),
/// and trading fees for a given input amount in a CPMM (Constant Product Market Maker) pool.
///
/// # Arguments
/// * `base_reserve` - The current reserve amount of the base token in the pool
/// * `quote_reserve` - The current reserve amount of the quote token in the pool
/// * `is_base_in` - Whether the input token is the base token (true) or quote token (false)
/// * `amount_in` - The amount of input tokens to swap
/// * `slippage_basis_points` - The acceptable slippage in basis points (e.g., 100 for 1%)
///
/// # Returns
/// A `ComputeSwapParams` struct containing all computed swap parameters
#[inline]
pub fn compute_swap_amount(
    base_reserve: u64,
    quote_reserve: u64,
    is_base_in: bool,
    amount_in: u64,
    slippage_basis_points: u64,
    trade_fee_rate: u64,
    protocol_fee_rate: u64,
    fund_fee_rate: u64,
) -> ComputeSwapParams {
    let (input_reserve, output_reserve) =
        if is_base_in { (base_reserve, quote_reserve) } else { (quote_reserve, base_reserve) };

    let swap_result = swap_base_input(
        amount_in,
        input_reserve,
        output_reserve,
        trade_fee_rate,
        0, // creator_fee_rate (CPMM 目前为 0)
        protocol_fee_rate,
        fund_fee_rate,
        true,
    );

    let min_amount_out = ((swap_result.output_amount as f64)
        * (1.0 - (slippage_basis_points as f64) / 10000.0)) as u64;

    let all_trade = swap_result.input_amount == amount_in;

    ComputeSwapParams {
        all_trade,
        amount_in,
        amount_out: swap_result.output_amount,
        min_amount_out,
        fee: swap_result.trade_fee,
    }
}

/// Exact In Buy 方向的内部计算（用 quote 买 base）
///
/// 已知 quote 输入数量，计算能获得多少 base 输出。
///
/// # Arguments
///
/// * `base_reserve` - Pool 中 base token 的储备量
/// * `quote_reserve` - Pool 中 quote token 的储备量
/// * `quote_in` - 输入的 quote 数量（精确输入）
/// * `slippage_basis_points` - 滑点容忍度（基点，100 = 1%）
/// * `trade_fee_rate` - 交易费率
/// * `protocol_fee_rate` - 协议费率
/// * `fund_fee_rate` - 资金费率
///
/// # 返回
///
/// `ComputeSwapParams` 包含输出金额、最小输出金额和费用
pub fn buy_exact_in_internal(
    base_reserve: u64,
    quote_reserve: u64,
    quote_in: u64,
    slippage_basis_points: u64,
    trade_fee_rate: u64,
    protocol_fee_rate: u64,
    fund_fee_rate: u64,
) -> ComputeSwapParams {
    compute_swap_amount(
        base_reserve,
        quote_reserve,
        false, // is_base_in = false, 输入是 quote
        quote_in,
        slippage_basis_points,
        trade_fee_rate,
        protocol_fee_rate,
        fund_fee_rate,
    )
}

/// Exact In Sell 方向的内部计算（用 base 卖成 quote）
///
/// 已知 base 输入数量，计算能获得多少 quote 输出。
///
/// # Arguments
///
/// * `base_reserve` - Pool 中 base token 的储备量
/// * `quote_reserve` - Pool 中 quote token 的储备量
/// * `base_in` - 输入的 base 数量（精确输入）
/// * `slippage_basis_points` - 滑点容忍度（基点，100 = 1%）
/// * `trade_fee_rate` - 交易费率
/// * `protocol_fee_rate` - 协议费率
/// * `fund_fee_rate` - 资金费率
///
/// # 返回
///
/// `ComputeSwapParams` 包含输出金额、最小输出金额和费用
pub fn sell_exact_in_internal(
    base_reserve: u64,
    quote_reserve: u64,
    base_in: u64,
    slippage_basis_points: u64,
    trade_fee_rate: u64,
    protocol_fee_rate: u64,
    fund_fee_rate: u64,
) -> ComputeSwapParams {
    compute_swap_amount(
        base_reserve,
        quote_reserve,
        true, // is_base_in = true, 输入是 base
        base_in,
        slippage_basis_points,
        trade_fee_rate,
        protocol_fee_rate,
        fund_fee_rate,
    )
}

/// Result of an exact-out swap calculation
#[derive(Debug, Clone)]
pub struct QuoteExactOutResult {
    /// Required input amount (including fees)
    pub amount_in: u64,
    /// Fee amount charged
    pub fee_amount: u64,
    /// Price impact in basis points (optional)
    pub price_impact_bps: Option<u64>,
}

/// 内部函数：Exact Out 计算（通用）
///
/// 计算获得指定输出金额所需的输入金额。
fn quote_exact_out_internal(
    base_reserve: u64,
    quote_reserve: u64,
    amount_out: u64,
    is_base_in: bool,
    trade_fee_rate: u64,
    protocol_fee_rate: u64,
    fund_fee_rate: u64,
) -> Result<QuoteExactOutResult, String> {
    let (reserve_in, reserve_out) =
        if is_base_in { (base_reserve, quote_reserve) } else { (quote_reserve, base_reserve) };

    // 流动性检查
    if amount_out >= reserve_out {
        return Err(format!(
            "Insufficient liquidity: requested={}, available={}",
            amount_out, reserve_out
        ));
    }

    // 恒定乘积公式: (reserve_in + amount_in) * (reserve_out - amount_out) = reserve_in * reserve_out
    // 反解: amount_in = (reserve_in * amount_out) / (reserve_out - amount_out)

    let numerator = (reserve_in as u128)
        .checked_mul(amount_out as u128)
        .ok_or_else(|| "Calculation overflow in numerator".to_string())?;

    let denominator = (reserve_out as u128)
        .checked_sub(amount_out as u128)
        .ok_or_else(|| "Invalid reserve calculation".to_string())?;

    let amount_in = numerator
        .checked_div(denominator)
        .ok_or_else(|| "Calculation overflow in division".to_string())? as u64;

    // 计算手续费 - protocol_fee 和 fund_fee 从 trade_fee 计算
    let trade_fee = compute_trading_fee(amount_in, trade_fee_rate);
    let protocol_fee = compute_protocol_fund_fee(trade_fee, protocol_fee_rate);
    let fund_fee = compute_protocol_fund_fee(trade_fee, fund_fee_rate);
    let creator_fee = compute_creator_fee_new(amount_in, 0); // CPMM 目前 creator_fee 为 0

    let total_fee = trade_fee
        .saturating_add(protocol_fee)
        .saturating_add(fund_fee)
        .saturating_add(creator_fee);

    let total_amount_in = amount_in
        .checked_add(total_fee)
        .ok_or_else(|| "Total amount calculation overflow".to_string())?;

    // 计算价格影响
    let price_impact_bps = if reserve_out > 0 {
        let impact = (amount_out as u128)
            .checked_mul(10_000u128)
            .and_then(|p| p.checked_div(reserve_out as u128))
            .unwrap_or(0);
        Some(impact as u64)
    } else {
        None
    };

    Ok(QuoteExactOutResult {
        amount_in: total_amount_in,
        fee_amount: total_fee,
        price_impact_bps,
    })
}

/// Exact Out Buy 方向的内部计算（用 quote 买 base）
///
/// 已知想要获得的 base 数量，计算需要多少 quote 作为输入。
///
/// # Arguments
///
/// * `base_reserve` - Pool 中 base token 的储备量
/// * `quote_reserve` - Pool 中 quote token 的储备量
/// * `base_out` - 想要获得的 base 数量（精确输出）
/// * `trade_fee_rate` - 交易费率
/// * `protocol_fee_rate` - 协议费率
/// * `fund_fee_rate` - 资金费率
///
/// # 返回
///
/// `QuoteExactOutResult` 包含所需输入金额和费用
pub fn buy_exact_out_internal(
    base_reserve: u64,
    quote_reserve: u64,
    base_out: u64,
    trade_fee_rate: u64,
    protocol_fee_rate: u64,
    fund_fee_rate: u64,
) -> Result<QuoteExactOutResult, String> {
    quote_exact_out_internal(
        base_reserve,
        quote_reserve,
        base_out,
        false, // is_base_in = false, 输入是 quote，输出是 base
        trade_fee_rate,
        protocol_fee_rate,
        fund_fee_rate,
    )
}

/// Exact Out Sell 方向的内部计算（用 base 卖成 quote）
///
/// 已知想要获得的 quote 数量，计算需要多少 base 作为输入。
///
/// # Arguments
///
/// * `base_reserve` - Pool 中 base token 的储备量
/// * `quote_reserve` - Pool 中 quote token 的储备量
/// * `quote_out` - 想要获得的 quote 数量（精确输出）
/// * `trade_fee_rate` - 交易费率
/// * `protocol_fee_rate` - 协议费率
/// * `fund_fee_rate` - 资金费率
///
/// # 返回
///
/// `QuoteExactOutResult` 包含所需输入金额和费用
pub fn sell_exact_out_internal(
    base_reserve: u64,
    quote_reserve: u64,
    quote_out: u64,
    trade_fee_rate: u64,
    protocol_fee_rate: u64,
    fund_fee_rate: u64,
) -> Result<QuoteExactOutResult, String> {
    quote_exact_out_internal(
        base_reserve,
        quote_reserve,
        quote_out,
        true, // is_base_in = true, 输入是 base，输出是 quote
        trade_fee_rate,
        protocol_fee_rate,
        fund_fee_rate,
    )
}
