// Raydium CPMM Quote 计算函数

use crate::{
    common::SolanaRpcClient,
    constants::{SOL_MINT, USDC_MINT, USDT_MINT, WSOL_TOKEN_ACCOUNT},
    instruction::utils::raydium_cpmm::{
        constants::DEFAULT_WSOL_USDT_CLMM_POOL, fee_queries, pool_queries,
    },
    instruction::utils::raydium_cpmm_types::PoolStateRaw,
    utils::price::raydium_cpmm::{price_base_in_quote, price_quote_in_base},
    utils::quote::{QuoteExactInParams, QuoteExactInResult, QuoteExactOutParams, QuoteExactOutResult},
};
use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;

/// 从 Pool 账户原始数据中读取 creator_fees
///
/// 注意：由于 Borsh 反序列化的限制，我们当前的 PoolState 结构体不包含 creator_fees 字段
/// 这个函数通过读取原始字节数据来获取 creator_fees
///
/// 参考：raydium-cp-swap/programs/cp-swap/src/states/pool.rs
#[allow(dead_code)]
async fn get_creator_fees_from_pool_data(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<(u64, u64), anyhow::Error> {
    let account = rpc.get_account(pool_address).await?;

    // 跳过 discriminator (8 bytes)
    let data: &[u8] = account.data.as_ref();
    if data.len() < 8 + 637 {
        return Ok((0, 0)); // 数据不完整，假设 creator_fees 为 0
    }

    // 使用 PoolStateRaw 解析（跳过 discriminator）
    if let Some(raw) = PoolStateRaw::from_bytes(&data[8..]) {
        let fees0 = raw.get_creator_fees_token0();
        let fees1 = raw.get_creator_fees_token1();
        Ok((fees0, fees1))
    } else {
        Ok((0, 0))
    }
}

/// Quote an exact-in swap against a Raydium CPMM pool.
///
/// # Arguments
///
/// * `params` - Quote 参数，包含 pool_address、input_mint、output_mint、amount_in
///
/// # Examples
///
/// ```ignore
/// let params = QuoteExactInParams {
///     pool_address: pool_pubkey,
///     input_mint: token0_mint,
///     output_mint: token1_mint,
///     amount_in: 1_000_000,
/// };
/// let quote = quote_exact_in(&rpc, params).await?;
/// ```
pub(crate) async fn quote_exact_in(
    rpc: &SolanaRpcClient,
    params: QuoteExactInParams,
) -> Result<QuoteExactInResult, anyhow::Error> {
    let pool_state = pool_queries::get_pool_by_address(rpc, &params.pool_address).await?;

    // 验证 input_mint 和 output_mint 是否在池子中
    let is_token0_in = params.input_mint == pool_state.token0_mint;
    let is_token1_in = params.input_mint == pool_state.token1_mint;

    if !is_token0_in && !is_token1_in {
        return Err(anyhow!(
            "Input mint {} not found in pool {} (token0={}, token1={})",
            params.input_mint,
            params.pool_address,
            pool_state.token0_mint,
            pool_state.token1_mint
        ));
    }

    let expected_output_mint = if is_token0_in {
        pool_state.token1_mint
    } else {
        pool_state.token0_mint
    };

    if params.output_mint != expected_output_mint {
        return Err(anyhow!(
            "Output mint mismatch: expected {}, got {}",
            expected_output_mint,
            params.output_mint
        ));
    }

    // 获取实际费率（从 amm_config 账户）
    let fees = fee_queries::get_amm_config_fees(rpc, &pool_state.amm_config).await?;

    let (token0_reserve, token1_reserve) = pool_queries::get_pool_token_balances(
        rpc,
        &params.pool_address,
        &pool_state.token0_mint,
        &pool_state.token1_mint,
    )
    .await?;

    // ⚠️ 重要：链上计算使用扣除累积手续费后的储备金
    // 参考：raydium-cp-swap/programs/cp-swap/src/states/pool.rs::vault_amount_without_fee
    // 需要从储备金中扣除：protocol_fees + fund_fees
    // 注意：暂时不考虑 creator_fees，因为它们通常为 0
    let token0_reserve_without_fees = token0_reserve
        .saturating_sub(pool_state.protocol_fees_token0)
        .saturating_sub(pool_state.fund_fees_token0);

    let token1_reserve_without_fees = token1_reserve
        .saturating_sub(pool_state.protocol_fees_token1)
        .saturating_sub(pool_state.fund_fees_token1);

    let q = crate::utils::calc::raydium_cpmm::compute_swap_amount(
        token0_reserve_without_fees,  // 使用扣除累积手续费后的储备金
        token1_reserve_without_fees,  // 使用扣除累积手续费后的储备金
        is_token0_in,
        params.amount_in,
        0,
        fees.trade_fee_rate,
        fees.protocol_fee_rate,
        fees.fund_fee_rate,
    );
    Ok(QuoteExactInResult {
        amount_out: q.amount_out,
        fee_amount: q.fee,
        price_impact_bps: None,
        extra_accounts_read: 2,
    })
}

/// Quote an exact-in swap against a Raydium CPMM pool (旧版接口，已废弃).
///
/// # Deprecated
///
/// 请使用新版本的 `quote_exact_in`，它使用 `QuoteExactInParams` 结构体参数。
///
/// - If `is_token0_in=true`: token0 -> token1
/// - If `is_token0_in=false`: token1 -> token0
#[deprecated(since = "4.1.0", note = "请使用 quote_exact_in(&rpc, QuoteExactInParams)")]
#[allow(dead_code)]
pub async fn quote_exact_in_legacy(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_in: u64,
    is_token0_in: bool,
) -> Result<QuoteExactInResult, anyhow::Error> {
    let pool_state = pool_queries::get_pool_by_address(rpc, pool_address).await?;

    // 构建新版本的参数
    let (input_mint, output_mint) = if is_token0_in {
        (pool_state.token0_mint, pool_state.token1_mint)
    } else {
        (pool_state.token1_mint, pool_state.token0_mint)
    };

    let params = QuoteExactInParams {
        pool_address: *pool_address,
        input_mint,
        output_mint,
        amount_in,
    };

    quote_exact_in(rpc, params).await
}

/// Quote an exact-out swap against a Raydium CPMM pool.
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
///     input_mint: token0_mint,
///     output_mint: token1_mint,
///     amount_out: 1_000_000,
/// };
/// let quote = quote_exact_out(&rpc, params).await?;
/// println!("需要输入: {} tokens", quote.amount_in);
/// ```
pub(crate) async fn quote_exact_out(
    rpc: &SolanaRpcClient,
    params: QuoteExactOutParams,
) -> Result<QuoteExactOutResult, anyhow::Error> {
    let pool_state = pool_queries::get_pool_by_address(rpc, &params.pool_address).await?;

    // 验证 input_mint 和 output_mint 是否在池子中
    let is_token0_in = params.input_mint == pool_state.token0_mint;
    let is_token1_in = params.input_mint == pool_state.token1_mint;

    if !is_token0_in && !is_token1_in {
        return Err(anyhow!(
            "Input mint {} not found in pool {} (token0={}, token1={})",
            params.input_mint,
            params.pool_address,
            pool_state.token0_mint,
            pool_state.token1_mint
        ));
    }

    let expected_output_mint = if is_token0_in {
        pool_state.token1_mint
    } else {
        pool_state.token0_mint
    };

    if params.output_mint != expected_output_mint {
        return Err(anyhow!(
            "Output mint mismatch: expected {}, got {}",
            expected_output_mint,
            params.output_mint
        ));
    }

    // 获取实际费率（从 amm_config 账户）
    let fees = fee_queries::get_amm_config_fees(rpc, &pool_state.amm_config).await?;

    let (token0_reserve, token1_reserve) = pool_queries::get_pool_token_balances(
        rpc,
        &params.pool_address,
        &pool_state.token0_mint,
        &pool_state.token1_mint,
    )
    .await?;

    // ⚠️ 重要：链上计算使用扣除累积手续费后的储备金
    // 参考：raydium-cp-swap/programs/cp-swap/src/states/pool.rs::vault_amount_without_fee
    // 注意：暂时不考虑 creator_fees，因为它们通常为 0
    let token0_reserve_without_fees = token0_reserve
        .saturating_sub(pool_state.protocol_fees_token0)
        .saturating_sub(pool_state.fund_fees_token0);

    let token1_reserve_without_fees = token1_reserve
        .saturating_sub(pool_state.protocol_fees_token1)
        .saturating_sub(pool_state.fund_fees_token1);

    let result = crate::utils::calc::raydium_cpmm::quote_exact_out(
        token0_reserve_without_fees,  // 使用扣除累积手续费后的储备金
        token1_reserve_without_fees,  // 使用扣除累积手续费后的储备金
        params.amount_out,
        is_token0_in,
        fees.trade_fee_rate,
        fees.protocol_fee_rate,
        fees.fund_fee_rate,
    )
    .map_err(|e| anyhow!("Quote exact out failed: {}", e))?;

    Ok(QuoteExactOutResult {
        amount_in: result.amount_in,
        fee_amount: result.fee_amount,
        price_impact_bps: result.price_impact_bps,
        extra_accounts_read: 2,
    })
}

/// Quote an exact-out swap against a Raydium CPMM pool (旧版接口，已废弃).
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
    is_token0_in: bool,
) -> Result<QuoteExactOutResult, anyhow::Error> {
    let pool_state = pool_queries::get_pool_by_address(rpc, pool_address).await?;

    // 构建新版本的参数
    let (input_mint, output_mint) = if is_token0_in {
        (pool_state.token0_mint, pool_state.token1_mint)
    } else {
        (pool_state.token1_mint, pool_state.token0_mint)
    };

    let params = QuoteExactOutParams {
        pool_address: *pool_address,
        input_mint,
        output_mint,
        amount_out,
    };

    quote_exact_out(rpc, params).await
}

/// 获取任意 Token 在 Raydium CPMM 上的 USD 价格（通过 X-WSOL 池 + Raydium CLMM WSOL-USD 锚定池）
///
/// 价格计算路径：Token X -> WSOL -> USD
/// - 要求：存在一个 X-WSOL 的 CPMM 池，以及一个 Raydium CLMM 上的 WSOL-USDT/USDC 锚定池
pub async fn get_token_price_in_usd(
    rpc: &SolanaRpcClient,
    token_mint: &Pubkey,
    wsol_usd_clmm_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool = wsol_usd_clmm_pool_address.unwrap_or(&DEFAULT_WSOL_USDT_CLMM_POOL);

    // 稳定币自身的价格直接认为是 1 USD
    if *token_mint == USDC_MINT || *token_mint == USDT_MINT {
        return Ok(1.0);
    }

    // WSOL/SOL 的价格通过 Raydium CLMM 锚定池获取
    if *token_mint == SOL_MINT || *token_mint == WSOL_TOKEN_ACCOUNT {
        return crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await;
    }

    // 1. 在 CPMM 中找到 Token X 的最优池（优先 X-WSOL/USDC/USDT 对）
    let (pool_address, pool_best) = pool_queries::get_pool_by_mint(rpc, token_mint).await?;

    // 2. 为了价格实时性，对选中的池地址强制刷新一次 PoolState
    let pool = pool_queries::get_pool_by_address_force(rpc, &pool_address)
        .await
        .unwrap_or(pool_best);

    // 3. 判断池子配对类型
    let is_token0_x = pool.token0_mint == *token_mint;
    let is_token1_x = pool.token1_mint == *token_mint;

    let other_mint = if is_token0_x {
        pool.token1_mint
    } else if is_token1_x {
        pool.token0_mint
    } else {
        return Err(anyhow!(
            "CPMM Pool {} does not contain the target mint {}",
            pool_address,
            token_mint
        ));
    };

    // 支持三种池子类型：
    // 1. X-WSOL：需要通过 WSOL-USD 锚定池计算
    // 2. X-USDC/USDT：直接认为稳定币价格 = 1 USD
    // 3. 其他：暂不支持（需要多跳路由）
    if other_mint == USDC_MINT || other_mint == USDT_MINT {
        // X-稳定币池：直接计算 X 相对稳定币的价格
        // 获取实时余额
        let (token0_balance, token1_balance) = pool_queries::get_pool_token_balances(
            rpc,
            &pool_address,
            &pool.token0_mint,
            &pool.token1_mint,
        )
        .await?;

        let price_x_in_stable = if is_token0_x {
            // token0 = X, token1 = USDC/USDT
            price_base_in_quote(
                token0_balance,
                token1_balance,
                pool.mint0_decimals,
                pool.mint1_decimals,
            )
        } else {
            // token1 = X, token0 = USDC/USDT
            price_quote_in_base(
                token0_balance,
                token1_balance,
                pool.mint0_decimals,
                pool.mint1_decimals,
            )
        };

        if price_x_in_stable <= 0.0 {
            return Err(anyhow!(
                "Invalid price from X-Stable CPMM pool (<= 0): mint={}, pool={}",
                token_mint,
                pool_address
            ));
        }

        return Ok(price_x_in_stable); // 稳定币 = 1 USD
    }

    if other_mint != SOL_MINT && other_mint != WSOL_TOKEN_ACCOUNT {
        return Err(anyhow!(
            "Best CPMM pool for mint {} is paired with {} (not WSOL/USDC/USDT); multi-hop USD pricing is not supported yet",
            token_mint,
            other_mint
        ));
    }

    // X-WSOL 池：计算 X 相对 WSOL 的价格
    // 获取实时余额
    let (token0_balance, token1_balance) = pool_queries::get_pool_token_balances(
        rpc,
        &pool_address,
        &pool.token0_mint,
        &pool.token1_mint,
    )
    .await?;

    let price_x_in_wsol = if is_token0_x {
        // token0 = X, token1 = WSOL
        price_base_in_quote(
            token0_balance,
            token1_balance,
            pool.mint0_decimals,
            pool.mint1_decimals,
        )
    } else {
        // token1 = X, token0 = WSOL
        price_quote_in_base(
            token0_balance,
            token1_balance,
            pool.mint0_decimals,
            pool.mint1_decimals,
        )
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price on CPMM is invalid (<= 0)"));
    }

    // 4. 计算 WSOL 的 USD 价格
    let price_wsol_in_usd =
        crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}

/// 获取任意 Token 在 Raydium CPMM 上的 USD 价格（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
/// 此函数要求调用者已知 X-WSOL 池地址，直接传入，避免 `get_pool_by_mint` 的查找开销。
/// 适用于高频调用、已缓存池地址的场景。
///
/// # Arguments
/// * `rpc` - 实现了 PoolRpcClient 的 RPC 客户端（支持 AutoMockRpcClient 或标准 RpcClient）
/// * `token_mint` - Token X 的 mint 地址
/// * `x_wsol_pool_address` - Token X 与 WSOL 配对的 CPMM 池地址
/// * `wsol_usd_clmm_pool_address` - Raydium CLMM 上的 WSOL-USDT/USDC 锚定池地址
pub async fn get_token_price_in_usd_with_pool<
    T: crate::common::auto_mock_rpc::PoolRpcClient + ?Sized,
>(
    rpc: &T,
    token_mint: &Pubkey,
    x_wsol_pool_address: &Pubkey,
    wsol_usd_clmm_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool = wsol_usd_clmm_pool_address.unwrap_or(&DEFAULT_WSOL_USDT_CLMM_POOL);

    // 稳定币自身的价格直接认为是 1 USD
    if *token_mint == USDC_MINT || *token_mint == USDT_MINT {
        return Ok(1.0);
    }

    // WSOL/SOL 的价格通过 Raydium CLMM 锚定池获取
    if *token_mint == SOL_MINT || *token_mint == WSOL_TOKEN_ACCOUNT {
        return crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await;
    }

    // 1. 获取指定的 X-WSOL 池（跳过查找步骤）
    let pool = pool_queries::get_pool_by_address(rpc, x_wsol_pool_address).await?;

    // 2. 判断池子配对类型
    let is_token0_x = pool.token0_mint == *token_mint;
    let is_token1_x = pool.token1_mint == *token_mint;

    let other_mint = if is_token0_x {
        pool.token1_mint
    } else if is_token1_x {
        pool.token0_mint
    } else {
        return Err(anyhow!(
            "Provided CPMM pool {} does not contain the target mint {}",
            x_wsol_pool_address,
            token_mint
        ));
    };

    // 支持三种池子类型：
    // 1. X-WSOL：需要通过 WSOL-USD 锚定池计算
    // 2. X-USDC/USDT：直接认为稳定币价格 = 1 USD
    // 3. 其他：暂不支持（需要多跳路由）
    if other_mint == USDC_MINT || other_mint == USDT_MINT {
        // X-稳定币池：直接计算 X 相对稳定币的价格
        // 获取实时余额
        let (token0_balance, token1_balance) = pool_queries::get_pool_token_balances(
            rpc,
            x_wsol_pool_address,
            &pool.token0_mint,
            &pool.token1_mint,
        )
        .await?;

        let price_x_in_stable = if is_token0_x {
            // token0 = X, token1 = USDC/USDT
            price_base_in_quote(
                token0_balance,
                token1_balance,
                pool.mint0_decimals,
                pool.mint1_decimals,
            )
        } else {
            // token1 = X, token0 = USDC/USDT
            price_quote_in_base(
                token0_balance,
                token1_balance,
                pool.mint0_decimals,
                pool.mint1_decimals,
            )
        };

        if price_x_in_stable <= 0.0 {
            return Err(anyhow!(
                "Invalid price from X-Stable CPMM pool (<= 0): mint={}, pool={}",
                token_mint,
                x_wsol_pool_address
            ));
        }

        return Ok(price_x_in_stable); // 稳定币 = 1 USD
    }

    if other_mint != SOL_MINT && other_mint != WSOL_TOKEN_ACCOUNT {
        return Err(anyhow!(
            "Provided CPMM pool {} is paired with {} (not WSOL/USDC/USDT); multi-hop USD pricing is not supported yet",
            x_wsol_pool_address,
            other_mint
        ));
    }

    // 3. X-WSOL 池：计算 X 相对 WSOL 的价格
    // 获取实时余额
    let (token0_balance, token1_balance) = pool_queries::get_pool_token_balances(
        rpc,
        x_wsol_pool_address,
        &pool.token0_mint,
        &pool.token1_mint,
    )
    .await?;

    let price_x_in_wsol = if is_token0_x {
        // token0 = X, token1 = WSOL
        price_base_in_quote(
            token0_balance,
            token1_balance,
            pool.mint0_decimals,
            pool.mint1_decimals,
        )
    } else {
        // token1 = X, token0 = WSOL
        price_quote_in_base(
            token0_balance,
            token1_balance,
            pool.mint0_decimals,
            pool.mint1_decimals,
        )
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price on CPMM is invalid (<= 0)"));
    }

    // 4. 计算 WSOL 的 USD 价格
    let price_wsol_in_usd =
        crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}
