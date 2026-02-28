// 辅助函数

use crate::constants::{SOL_MINT, USDC_MINT, USDT_MINT};
use crate::instruction::utils::raydium_amm_v4_types::AmmInfo;

/// 计算池子的有效交易量（基于 swap 数据）
/// - 如果包含 WSOL/USDC/USDT，只计算这些资产侧的交易量
/// - 否则计算两侧的总交易量
pub(crate) fn calculate_effective_volume(amm: &AmmInfo) -> u128 {
    // 检查 coin_mint 是否为 WSOL/USDC/USDT
    let coin_is_stable =
        amm.coin_mint == SOL_MINT || amm.coin_mint == USDC_MINT || amm.coin_mint == USDT_MINT;

    // 检查 pc_mint 是否为 WSOL/USDC/USDT
    let pc_is_stable =
        amm.pc_mint == SOL_MINT || amm.pc_mint == USDC_MINT || amm.pc_mint == USDT_MINT;

    if coin_is_stable && !pc_is_stable {
        // 只计算 coin 侧（WSOL/USDC/USDT）的交易量
        amm.out_put.swap_coin_in_amount.saturating_add(amm.out_put.swap_pc_out_amount)
    } else if pc_is_stable && !coin_is_stable {
        // 只计算 pc 侧（WSOL/USDC/USDT）的交易量
        amm.out_put.swap_pc_in_amount.saturating_add(amm.out_put.swap_coin_out_amount)
    } else {
        // 两侧都是稳定资产或都不是，计算总交易量
        amm.out_put
            .swap_coin_in_amount
            .saturating_add(amm.out_put.swap_pc_out_amount)
            .saturating_add(amm.out_put.swap_pc_in_amount)
            .saturating_add(amm.out_put.swap_coin_out_amount)
    }
}

/// 检查 pool 是否处于活跃状态
///
/// 只有活跃状态的 pool 才适合进行交易。
pub fn is_pool_active(amm_info: &AmmInfo) -> bool {
    amm_info.status == super::constants::pool_status::ACTIVE
}

/// 检查 pool 是否已禁用
///
/// 已禁用的 pool 不能进行交易。
#[allow(dead_code)]
pub(crate) fn is_pool_disabled(amm_info: &AmmInfo) -> bool {
    amm_info.status == super::constants::pool_status::DISABLED
}

/// 检查 pool 是否只能提现
///
/// 只能提现的 pool 不能进行交易，只能提取流动性。
#[allow(dead_code)]
pub(crate) fn is_pool_withdraw_only(amm_info: &AmmInfo) -> bool {
    amm_info.status == super::constants::pool_status::WITHDRAW_ONLY
}

/// 检查 pool 是否适合交易
///
/// 适合交易的 pool 必须是活跃状态。
pub fn is_pool_tradeable(amm_info: &AmmInfo) -> bool {
    is_pool_active(amm_info)
}

/// 按累计交易量选择最佳池（零网络开销）
///
/// 策略：
/// - 优先选择活跃状态的池
/// - 如果池子包含 WSOL/USDC/USDT，只计算这些稳定资产侧的累计交易量
/// - 否则计算两侧的总交易量
/// - 交易量越大，说明池子被实际使用越多，深度越可靠
pub(crate) fn select_best_pool_by_volume(
    pools: &[(solana_sdk::pubkey::Pubkey, AmmInfo)],
) -> Option<(solana_sdk::pubkey::Pubkey, AmmInfo)> {
    use solana_sdk::pubkey::Pubkey;

    if pools.is_empty() {
        return None;
    }

    if pools.len() == 1 {
        return Some(pools[0].clone());
    }

    // 优先选择活跃状态的池
    let mut active_pools: Vec<(Pubkey, AmmInfo)> = pools
        .iter()
        .filter(|(_, amm)| is_pool_tradeable(amm))
        .map(|(addr, amm)| (*addr, amm.clone()))
        .collect();

    if active_pools.is_empty() {
        // 如果全部池都不活跃，使用所有池
        active_pools = pools.to_vec();
    }

    // 按累计交易量排序
    active_pools.sort_by(|(_, amm_a), (_, amm_b)| {
        // 计算有效交易量（优先只看WSOL/USDC/USDT侧）
        let volume_a = calculate_effective_volume(amm_a);
        let volume_b = calculate_effective_volume(amm_b);

        // 按交易量降序排序
        match volume_b.cmp(&volume_a) {
            std::cmp::Ordering::Equal => {
                // 交易量相同时，按流动性排序
                amm_b.lp_amount.cmp(&amm_a.lp_amount)
            },
            other => other,
        }
    });

    // 返回交易量最高的池
    active_pools.into_iter().next()
}
