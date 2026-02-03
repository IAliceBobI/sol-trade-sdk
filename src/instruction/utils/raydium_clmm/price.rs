// Raydium CLMM 价格计算函数

use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;

use crate::{
    common::auto_mock_rpc::PoolRpcClient,
    constants::{SOL_MINT, USDC_MINT, USDT_MINT},
    utils::price::raydium_clmm::{price_token0_in_token1, price_token1_in_token0},
};

use super::{
    constants::DEFAULT_WSOL_USDT_CLMM_POOL,
    pool_queries::{get_pool_by_address_force, get_pool_by_mint},
};

/// 获取 WSOL 的 USD 价格（泛型版本，支持 Auto Mock）
///
/// # Arguments
/// * `rpc` - RPC 客户端（支持 AutoMockRpcClient）
/// * `wsol_usd_pool_address` - WSOL-USDT/USDC CLMM 池地址（例如你提供的 USDT-WSOL 池）
pub async fn get_wsol_price_in_usd_with_client<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    wsol_usd_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool = wsol_usd_pool_address.unwrap_or(&DEFAULT_WSOL_USDT_CLMM_POOL);

    // 强制刷新：每次调用都重新从链上读取池状态，避免价格缓存
    let pool_state = get_pool_by_address_force(rpc, wsol_usd_pool).await?;

    // 只支持 WSOL <-> USDC/USDT 的稳定币池
    let is_token0_sol = pool_state.token_mint0 == SOL_MINT;
    let is_token1_sol = pool_state.token_mint1 == SOL_MINT;
    let is_token0_stable =
        pool_state.token_mint0 == USDC_MINT || pool_state.token_mint0 == USDT_MINT;
    let is_token1_stable =
        pool_state.token_mint1 == USDC_MINT || pool_state.token_mint1 == USDT_MINT;

    let price_wsol_in_stable = if is_token0_sol && is_token1_stable {
        // token0 = WSOL, token1 = USDC/USDT
        price_token0_in_token1(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    } else if is_token1_sol && is_token0_stable {
        // token1 = WSOL, token0 = USDC/USDT
        price_token1_in_token0(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    } else {
        return Err(anyhow!(
            "WSOL-USD anchor pool must be a SOL<->USDC/USDT CLMM pool, got {:?} / {:?}",
            pool_state.token_mint0,
            pool_state.token_mint1
        ));
    };

    if price_wsol_in_stable <= 0.0 {
        return Err(anyhow!("Invalid WSOL price from anchor pool (<= 0)"));
    }

    // 默认认为 USDC / USDT ~= 1 USD
    Ok(price_wsol_in_stable)
}

/// 获取任意 Token 在 Raydium CLMM 上的 USD 价格（支持 PoolRpcClient）
///
/// 与 `get_token_price_in_usd` 功能相同，但接受 `PoolRpcClient` trait 参数，
/// 支持 `AutoMockRpcClient` 进行测试加速。
///
/// 价格计算路径：Token X -> WSOL -> USD
/// - 要求：存在一个 X-WSOL 的 CLMM 池（Hot 对），以及一个 WSOL-USDT/USDC 锚定池
pub async fn get_token_price_in_usd<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    token_mint: &Pubkey,
    wsol_usd_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool = wsol_usd_pool_address.unwrap_or(&DEFAULT_WSOL_USDT_CLMM_POOL);

    // 稳定币自身的价格直接认为是 1 USD
    if *token_mint == USDC_MINT || *token_mint == USDT_MINT {
        return Ok(1.0);
    }

    // WSOL/SOL 的价格直接来自锚定池
    if *token_mint == SOL_MINT {
        return get_wsol_price_in_usd_with_client(rpc, Some(wsol_usd_pool)).await;
    }

    // 1. 先在 CLMM 中找到 Token X 的最优池（优先 X-WSOL/USDC/USDT 对）
    let (pool_address, pool_state_best) = get_pool_by_mint(rpc, token_mint).await?;

    // 2. 为了价格实时性，对选中的池地址强制刷新一次 PoolState
    let pool_state = get_pool_by_address_force(rpc, &pool_address).await.unwrap_or(pool_state_best);

    // 3. 判断池子配对类型
    let is_token0_x = pool_state.token_mint0 == *token_mint;
    let is_token1_x = pool_state.token_mint1 == *token_mint;

    let other_mint = if is_token0_x {
        pool_state.token_mint1
    } else if is_token1_x {
        pool_state.token_mint0
    } else {
        return Err(anyhow!(
            "Pool {} does not contain the target mint {}",
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
        let price_x_in_stable = if is_token0_x {
            price_token0_in_token1(
                pool_state.sqrt_price_x64,
                pool_state.mint_decimals0,
                pool_state.mint_decimals1,
            )
        } else {
            price_token1_in_token0(
                pool_state.sqrt_price_x64,
                pool_state.mint_decimals0,
                pool_state.mint_decimals1,
            )
        };

        if price_x_in_stable <= 0.0 {
            return Err(anyhow!(
                "Invalid price from X-Stable pool (<= 0): mint={}, pool={}",
                token_mint,
                pool_address
            ));
        }

        return Ok(price_x_in_stable); // 稳定币 = 1 USD
    }

    if other_mint != SOL_MINT {
        return Err(anyhow!(
            "Best CLMM pool for mint {} is paired with {} (not WSOL/USDC/USDT); multi-hop USD pricing is not supported yet",
            token_mint,
            other_mint
        ));
    }

    // 4. X-WSOL 池：计算 X 相对 WSOL 的价格
    let price_x_in_wsol = if is_token0_x {
        // token0 = X, token1 = WSOL
        price_token0_in_token1(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    } else {
        // token1 = X, token0 = WSOL
        price_token1_in_token0(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price is invalid (<= 0)"));
    }

    // 5. 计算 WSOL 的 USD 价格
    let price_wsol_in_usd = get_wsol_price_in_usd_with_client(rpc, Some(wsol_usd_pool)).await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}

/// 获取任意 Token 在 Raydium CLMM 上的 USD 价格（直接传入池地址，支持 PoolRpcClient）
///
/// 与 `get_token_price_in_usd_with_pool` 功能相同，但接受 `PoolRpcClient` trait 参数，
/// 支持 `AutoMockRpcClient` 进行测试加速。
///
/// 与 `get_token_price_in_usd` 的区别：
/// - 此函数要求调用者已知 X-WSOL 池地址，直接传入，避免 `get_pool_by_mint` 的查找开销
/// - 适用于高频调用、已缓存池地址的场景
///
/// # Arguments
/// * `rpc` - 实现 PoolRpcClient trait 的 RPC 客户端（支持 AutoMockRpcClient）
/// * `token_mint` - Token X 的 mint 地址
/// * `x_wsol_pool_address` - Token X 与 WSOL 配对的 CLMM 池地址
/// * `wsol_usd_pool_address` - WSOL-USDT/USDC 锚定池地址（可选，默认使用 DEFAULT_WSOL_USDT_CLMM_POOL）
pub async fn get_token_price_in_usd_with_pool<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    token_mint: &Pubkey,
    x_wsol_pool_address: &Pubkey,
    wsol_usd_pool_address: Option<&Pubkey>,
) -> Result<f64, anyhow::Error> {
    let wsol_usd_pool = wsol_usd_pool_address.unwrap_or(&DEFAULT_WSOL_USDT_CLMM_POOL);

    // 稳定币自身的价格直接认为是 1 USD
    if *token_mint == USDC_MINT || *token_mint == USDT_MINT {
        return Ok(1.0);
    }

    // WSOL/SOL 的价格直接来自锚定池
    if *token_mint == SOL_MINT {
        return get_wsol_price_in_usd_with_client(rpc, Some(wsol_usd_pool)).await;
    }

    // 1. 直接强制刷新指定的 X-WSOL 池（跳过查找步骤）
    let pool_state = get_pool_by_address_force(rpc, x_wsol_pool_address).await?;

    // 2. 判断池子配对类型
    let is_token0_x = pool_state.token_mint0 == *token_mint;
    let is_token1_x = pool_state.token_mint1 == *token_mint;

    let other_mint = if is_token0_x {
        pool_state.token_mint1
    } else if is_token1_x {
        pool_state.token_mint0
    } else {
        return Err(anyhow!(
            "Provided pool {} does not contain the target mint {}",
            x_wsol_pool_address,
            token_mint
        ));
    };

    // 支持三种池子类型：
    // 1. X-WSOL：需要通过 WSOL-USD 锚定池计算
    // 2. X-USDC/USDT：直接认为稳定币价格 = 1 USD
    // 3. 其他：不支持
    if other_mint == USDC_MINT || other_mint == USDT_MINT {
        // X-稳定币池：直接计算 X 相对稳定币的价格
        let price_x_in_stable = if is_token0_x {
            price_token0_in_token1(
                pool_state.sqrt_price_x64,
                pool_state.mint_decimals0,
                pool_state.mint_decimals1,
            )
        } else {
            price_token1_in_token0(
                pool_state.sqrt_price_x64,
                pool_state.mint_decimals0,
                pool_state.mint_decimals1,
            )
        };

        if price_x_in_stable <= 0.0 {
            return Err(anyhow!(
                "Invalid price from X-Stable pool (<= 0): mint={}, pool={}",
                token_mint,
                x_wsol_pool_address
            ));
        }

        return Ok(price_x_in_stable); // 稳定币 = 1 USD
    }

    if other_mint != SOL_MINT {
        return Err(anyhow!(
            "Provided pool {} is paired with {} (not WSOL/USDC/USDT); multi-hop USD pricing is not supported yet",
            x_wsol_pool_address,
            other_mint
        ));
    }

    // 3. X-WSOL 池：计算 X 相对 WSOL 的价格
    let price_x_in_wsol = if is_token0_x {
        // token0 = X, token1 = WSOL
        price_token0_in_token1(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    } else {
        // token1 = X, token0 = WSOL
        price_token1_in_token0(
            pool_state.sqrt_price_x64,
            pool_state.mint_decimals0,
            pool_state.mint_decimals1,
        )
    };

    if price_x_in_wsol <= 0.0 {
        return Err(anyhow!("Computed X/WSOL price is invalid (<= 0)"));
    }

    // 4. 计算 WSOL 的 USD 价格
    let price_wsol_in_usd = get_wsol_price_in_usd_with_client(rpc, Some(wsol_usd_pool)).await?;

    Ok(price_x_in_wsol * price_wsol_in_usd)
}
