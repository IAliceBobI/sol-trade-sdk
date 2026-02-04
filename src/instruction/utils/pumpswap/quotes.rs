use crate::{
    common::{SolanaRpcClient, auto_mock_rpc::PoolRpcClient},
    constants::{SOL_MINT, USDC_MINT, USDT_MINT, WSOL_TOKEN_ACCOUNT},
    utils::price::pumpswap::{price_base_in_quote, price_quote_in_base},
    utils::quote::{QuoteExactInParams, QuoteExactOutParams},
};
use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;

use super::constants::DEFAULT_WSOL_USDT_CLMM_POOL;
use super::pool_queries::{get_pool_by_address, get_pool_by_address_force, get_token_balances};

/// Quote an exact-in swap against a PumpSwap pool.
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
///     input_mint: base_mint,
///     output_mint: quote_mint,
///     amount_in: 1_000_000,
/// };
/// let quote = quote_exact_in(&rpc, params).await?;
/// ```
pub(crate) async fn quote_exact_in(
    rpc: &SolanaRpcClient,
    params: QuoteExactInParams,
) -> Result<crate::utils::quote::QuoteExactInResult, anyhow::Error> {
    let pool = get_pool_by_address(rpc, &params.pool_address).await?;

    // 验证 input_mint 和 output_mint 是否在池子中
    let is_base_in = params.input_mint == pool.base_mint;
    let is_quote_in = params.input_mint == pool.quote_mint;

    if !is_base_in && !is_quote_in {
        return Err(anyhow!(
            "Input mint {} not found in pool {} (base={}, quote={})",
            params.input_mint,
            params.pool_address,
            pool.base_mint,
            pool.quote_mint
        ));
    }

    let expected_output_mint = if is_base_in {
        pool.quote_mint
    } else {
        pool.base_mint
    };

    if params.output_mint != expected_output_mint {
        return Err(anyhow!(
            "Output mint mismatch: expected {}, got {}",
            expected_output_mint,
            params.output_mint
        ));
    }

    let (base_reserve, quote_reserve) = get_token_balances(&pool, rpc).await?;

    if is_base_in {
        // base -> quote
        let r = crate::utils::calc::pumpswap::sell_base_input_internal(
            params.amount_in,
            0,
            base_reserve,
            quote_reserve,
            &pool.coin_creator,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        // fee in output token space is less helpful; we expose fee in input token units when possible.
        // For base->quote we don't have an input-fee field; return 0 here for now.
        Ok(crate::utils::quote::QuoteExactInResult {
            amount_out: r.ui_quote,
            fee_amount: 0,
            price_impact_bps: None,
            extra_accounts_read: 2, // two token accounts
        })
    } else {
        // quote -> base
        let r = crate::utils::calc::pumpswap::buy_quote_input_internal(
            params.amount_in,
            0,
            base_reserve,
            quote_reserve,
            &pool.coin_creator,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        // fee in input token units: amount_in - effective_quote (without fees)
        let fee_amount = params.amount_in.saturating_sub(r.internal_quote_without_fees);
        Ok(crate::utils::quote::QuoteExactInResult {
            amount_out: r.base,
            fee_amount,
            price_impact_bps: None,
            extra_accounts_read: 2,
        })
    }
}

/// Quote an exact-in swap against a PumpSwap pool (旧版接口，已废弃).
///
/// # Deprecated
///
/// 请使用新版本的 `quote_exact_in`，它使用 `QuoteExactInParams` 结构体参数。
#[deprecated(since = "4.1.0", note = "请使用 quote_exact_in(&rpc, QuoteExactInParams)")]
pub async fn quote_exact_in_legacy(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_in: u64,
    is_base_in: bool,
) -> Result<crate::utils::quote::QuoteExactInResult, anyhow::Error> {
    let pool = get_pool_by_address(rpc, pool_address).await?;

    // 构建新版本的参数
    let (input_mint, output_mint) = if is_base_in {
        (pool.base_mint, pool.quote_mint)
    } else {
        (pool.quote_mint, pool.base_mint)
    };

    let params = QuoteExactInParams {
        pool_address: *pool_address,
        input_mint,
        output_mint,
        amount_in,
    };

    quote_exact_in(rpc, params).await
}

/// Quote an exact-out swap against a PumpSwap pool.
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
///     input_mint: base_mint,
///     output_mint: quote_mint,
///     amount_out: 1_000_000,
/// };
/// let quote = quote_exact_out(&rpc, params).await?;
/// ```
pub(crate) async fn quote_exact_out(
    rpc: &SolanaRpcClient,
    params: QuoteExactOutParams,
) -> Result<crate::utils::quote::QuoteExactOutResult, anyhow::Error> {
    let pool = get_pool_by_address(rpc, &params.pool_address).await?;

    // 验证 input_mint 和 output_mint 是否在池子中
    let is_base_in = params.input_mint == pool.base_mint;
    let is_quote_in = params.input_mint == pool.quote_mint;

    if !is_base_in && !is_quote_in {
        return Err(anyhow!(
            "Input mint {} not found in pool {} (base={}, quote={})",
            params.input_mint,
            params.pool_address,
            pool.base_mint,
            pool.quote_mint
        ));
    }

    let expected_output_mint = if is_base_in {
        pool.quote_mint
    } else {
        pool.base_mint
    };

    if params.output_mint != expected_output_mint {
        return Err(anyhow!(
            "Output mint mismatch: expected {}, got {}",
            expected_output_mint,
            params.output_mint
        ));
    }

    let (base_reserve, quote_reserve) = get_token_balances(&pool, rpc).await?;

    if is_base_in {
        // base -> quote (卖出 base，获得指定数量的 quote)
        // 使用 sell_quote_input_internal 进行逆向计算
        let r = crate::utils::calc::pumpswap::sell_quote_input_internal(
            params.amount_out,
            0, // slippage 在 quote 中不计算
            base_reserve,
            quote_reserve,
            &pool.coin_creator,
        )
        .map_err(|e| anyhow::anyhow!(e))?;

        // 🔧 补偿精度误差：增加 0.1% 的缓冲以应对整数除法精度损失
        // 这确保实际输出不会因为精度问题低于期望输出
        let buffer = r.base / 1000; // 0.1%
        let amount_in_with_buffer = r.base.saturating_add(buffer.max(1));

        // 计算费用：简单处理，返回 0（因为不同精度代币难以准确表示）
        Ok(crate::utils::quote::QuoteExactOutResult {
            amount_in: amount_in_with_buffer,
            fee_amount: 0,
            price_impact_bps: None,
            extra_accounts_read: 2,
        })
    } else {
        // quote -> base (买入 base，获得指定数量的 base)
        // 使用 buy_base_input_internal 进行逆向计算
        let r = crate::utils::calc::pumpswap::buy_base_input_internal(
            params.amount_out,
            0, // slippage 在 quote 中不计算
            base_reserve,
            quote_reserve,
            &pool.coin_creator,
        )
        .map_err(|e| anyhow::anyhow!(e))?;

        // 费用包含在 ui_quote 中，需要减去 internal_quote_amount
        let fee_amount = r.ui_quote.saturating_sub(r.internal_quote_amount);

        Ok(crate::utils::quote::QuoteExactOutResult {
            amount_in: r.ui_quote,
            fee_amount,
            price_impact_bps: None,
            extra_accounts_read: 2,
        })
    }
}

/// Quote an exact-out swap against a PumpSwap pool (旧版接口，已废弃).
///
/// # Deprecated
///
/// 请使用新版本的 `quote_exact_out`，它使用 `QuoteExactOutParams` 结构体参数。
#[deprecated(since = "4.1.0", note = "请使用 quote_exact_out(&rpc, QuoteExactOutParams)")]
pub async fn quote_exact_out_legacy(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_out: u64,
    is_base_in: bool,
) -> Result<crate::utils::quote::QuoteExactOutResult, anyhow::Error> {
    let pool = get_pool_by_address(rpc, pool_address).await?;

    // 构建新版本的参数
    let (input_mint, output_mint) = if is_base_in {
        (pool.base_mint, pool.quote_mint)
    } else {
        (pool.quote_mint, pool.base_mint)
    };

    let params = QuoteExactOutParams {
        pool_address: *pool_address,
        input_mint,
        output_mint,
        amount_out,
    };

    quote_exact_out(rpc, params).await
}

/// 获取任意 Token 在 PumpSwap 上的 USD 价格（通过 X-WSOL 池 + Raydium CLMM WSOL-USD 锚定池）
///
/// 价格计算路径：Token X -> WSOL -> USD
/// - 要求：存在一个 X-WSOL 的 PumpSwap 池，以及一个 Raydium CLMM 上的 WSOL-USDT/USDC 锚定池
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

    // 1. 在 PumpSwap 中找到 Token X 的最优池（优先 X-WSOL/USDC/USDT 对）
    let (pool_address, pool_best) = super::pool_queries::get_pool_by_mint(rpc, token_mint).await?;

    // 2. 为了价格实时性，对选中的池地址强制刷新一次 Pool
    let pool = get_pool_by_address_force(rpc, &pool_address).await.unwrap_or(pool_best);

    // 3. 只处理 X-WSOL 对（X 是任意 token，另一侧必须是 WSOL_TOKEN_ACCOUNT）
    let is_base_x = pool.base_mint == *token_mint && pool.quote_mint == WSOL_TOKEN_ACCOUNT;
    let is_quote_x = pool.quote_mint == *token_mint && pool.base_mint == WSOL_TOKEN_ACCOUNT;

    if !is_base_x && !is_quote_x {
        return Err(anyhow!(
            "Best PumpSwap pool for mint {} is not paired with WSOL; USD pricing via WSOL is not supported yet",
            token_mint
        ));
    }

    // 4. 获取池子实时余额
    let (base_reserve, quote_reserve) = get_token_balances(&pool, rpc).await?;

    // 5. 获取两侧代币精度
    let base_decimals = crate::utils::token::get_token_decimals(rpc, &pool.base_mint).await?;
    let quote_decimals = crate::utils::token::get_token_decimals(rpc, &pool.quote_mint).await?;

    // 6. 计算 X 相对 WSOL 的价格
    let price_x_in_wsol = if is_base_x {
        // base = X, quote = WSOL
        price_base_in_quote(base_reserve, quote_reserve, base_decimals, quote_decimals)
    } else {
        // quote = X, base = WSOL
        price_quote_in_base(base_reserve, quote_reserve, base_decimals, quote_decimals)
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price on PumpSwap is invalid (<= 0)"));
    }

    // 7. 获取 WSOL 的 USD 价格（通过 Raydium CLMM 锚定池）
    let price_wsol_in_usd =
        crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}

/// 获取任意 Token 在 PumpSwap 上的 USD 价格（支持 Auto Mock）
///
/// 支持 PoolRpcClient trait，可以接受 AutoMockRpcClient 或标准 RpcClient。
/// 此函数要求调用者已知 X-WSOL 池地址，直接传入，避免 `get_pool_by_mint` 的查找开销。
/// 适用于高频调用、已缓存池地址的场景。
///
/// # Arguments
/// * `rpc` - 实现了 PoolRpcClient 的 RPC 客户端（支持 AutoMockRpcClient 或标准 RpcClient）
/// * `token_mint` - Token X 的 mint 地址
/// * `x_wsol_pool_address` - Token X 与 WSOL 配对的 PumpSwap 池地址
/// * `wsol_usd_clmm_pool_address` - Raydium CLMM 上的 WSOL-USDT/USDC 锚定池地址
pub async fn get_token_price_in_usd_with_pool<T: PoolRpcClient + ?Sized>(
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

    // 1. 直接强制刷新指定的 X-WSOL 池（跳过查找步骤）
    let pool = get_pool_by_address_force(rpc, x_wsol_pool_address).await?;

    // 2. 只处理 X-WSOL 对（X 是任意 token，另一侧必须是 WSOL_TOKEN_ACCOUNT）
    let is_base_x = pool.base_mint == *token_mint && pool.quote_mint == WSOL_TOKEN_ACCOUNT;
    let is_quote_x = pool.quote_mint == *token_mint && pool.base_mint == WSOL_TOKEN_ACCOUNT;

    if !is_base_x && !is_quote_x {
        return Err(anyhow!(
            "Provided PumpSwap pool {} is not paired with WSOL; USD pricing via WSOL is not supported yet",
            x_wsol_pool_address
        ));
    }

    // 3. 获取池子实时余额
    let (base_reserve, quote_reserve) = get_token_balances(&pool, rpc).await?;

    // 4. 获取两侧代币精度
    let base_decimals =
        crate::utils::token::get_token_decimals_with_client(rpc, &pool.base_mint).await?;
    let quote_decimals =
        crate::utils::token::get_token_decimals_with_client(rpc, &pool.quote_mint).await?;

    // 5. 计算 X 相对 WSOL 的价格
    let price_x_in_wsol = if is_base_x {
        // base = X, quote = WSOL
        price_base_in_quote(base_reserve, quote_reserve, base_decimals, quote_decimals)
    } else {
        // quote = X, base = WSOL
        price_quote_in_base(base_reserve, quote_reserve, base_decimals, quote_decimals)
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price on PumpSwap is invalid (<= 0)"));
    }

    // 6. 获取 WSOL 的 USD 价格（通过 Raydium CLMM 锚定池）
    let price_wsol_in_usd =
        crate::instruction::utils::raydium_clmm::get_wsol_price_in_usd_with_client(
            rpc,
            Some(wsol_usd_pool),
        )
        .await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试：USDC/USDT 价格分支（不依赖真实 RPC，只要函数能返回 1.0 即可）
    #[tokio::test]
    async fn test_get_token_price_in_usd_stable_tokens() {
        use solana_client::nonblocking::rpc_client::RpcClient;

        let rpc = RpcClient::new("http://127.0.0.1:8899".to_string());
        let dummy_anchor_pool = Pubkey::new_unique();

        let usdc_price = get_token_price_in_usd(&rpc, &USDC_MINT, Some(&dummy_anchor_pool))
            .await
            .unwrap();
        let usdt_price = get_token_price_in_usd(&rpc, &USDT_MINT, Some(&dummy_anchor_pool))
            .await
            .unwrap();

        assert_eq!(usdc_price, 1.0);
        assert_eq!(usdt_price, 1.0);
    }
}
