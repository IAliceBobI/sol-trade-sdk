// Raydium CPMM 辅助函数

use crate::{
    constants::{USDC_MINT, USDT_MINT, WSOL_TOKEN_ACCOUNT},
    instruction::utils::raydium_cpmm_types::PoolState,
};
use solana_sdk::pubkey::Pubkey;

use super::constants::{accounts, seeds};

/// 判断是否为 Hot Mint（主流桥接资产）
/// 当前包含：WSOL、USDC、USDT
pub fn is_hot_mint(mint: &Pubkey) -> bool {
    *mint == WSOL_TOKEN_ACCOUNT || *mint == USDC_MINT || *mint == USDT_MINT
}

/// 获取 Pool PDA
pub fn get_pool_pda(amm_config: &Pubkey, mint1: &Pubkey, mint2: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 4] =
        &[seeds::POOL_SEED, amm_config.as_ref(), mint1.as_ref(), mint2.as_ref()];
    let program_id: &Pubkey = &accounts::RAYDIUM_CPMM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

/// 获取 Vault PDA
pub fn get_vault_pda(pool_state: &Pubkey, mint: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 3] = &[seeds::POOL_VAULT_SEED, pool_state.as_ref(), mint.as_ref()];
    let program_id: &Pubkey = &accounts::RAYDIUM_CPMM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

/// 获取 Observation State PDA
pub fn get_observation_state_pda(pool_state: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 2] = &[seeds::OBSERVATION_STATE_SEED, pool_state.as_ref()];
    let program_id: &Pubkey = &accounts::RAYDIUM_CPMM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

/// 获取 Token Vault 账户地址
pub fn get_vault_account(
    pool_state: &Pubkey,
    token_mint: &Pubkey,
    protocol_params: &crate::trading::core::params::RaydiumCpmmParams,
) -> Pubkey {
    // 处理 SOL_TOKEN_ACCOUNT 和 WSOL_TOKEN_ACCOUNT 的别名关系
    // SOL_TOKEN_ACCOUNT (以 11 结尾) 和 WSOL_TOKEN_ACCOUNT (以 12 结尾) 在链上指向同一个代币
    // TradingClient 使用 SOL_TOKEN_ACCOUNT，但 Pool 使用 WSOL_TOKEN_ACCOUNT
    let normalized_mint = if *token_mint == crate::constants::SOL_TOKEN_ACCOUNT {
        &crate::constants::WSOL_TOKEN_ACCOUNT
    } else {
        token_mint
    };

    if protocol_params.base_mint == *normalized_mint && protocol_params.base_vault != Pubkey::default() {
        protocol_params.base_vault
    } else if protocol_params.quote_mint == *normalized_mint
        && protocol_params.quote_vault != Pubkey::default()
    {
        protocol_params.quote_vault
    } else {
        get_vault_pda(pool_state, normalized_mint).unwrap()
    }
}

/// 按 LP 供应量选择最佳池（CPMM 池没有交易量字段，使用 lp_supply 作为流动性指标）
///
/// 策略：
/// - 优先选择已激活且有流动性的池
/// - LP 供应量越大，说明流动性越好
pub fn select_best_pool_by_liquidity(pools: &[(Pubkey, PoolState)]) -> Option<(Pubkey, PoolState)> {
    if pools.is_empty() {
        return None;
    }

    if pools.len() == 1 {
        return pools.first().cloned();
    }

    // 优先选择已激活且有流动性的池
    let mut active_pools: Vec<_> = pools
        .iter()
        .filter(|(_, pool)| pool.status != 0 && pool.lp_supply > 0)
        .map(|(addr, pool)| (*addr, pool.clone()))
        .collect();

    if active_pools.is_empty() {
        // 如果全部池都不活跃，使用所有池
        active_pools = pools.to_vec();
    }

    // 按 LP 供应量排序
    active_pools.sort_by(|(_, pool_a), (_, pool_b)| {
        // 按 LP 供应量降序排序
        match pool_b.lp_supply.cmp(&pool_a.lp_supply) {
            std::cmp::Ordering::Equal => {
                // LP 供应量相同时，按开池时间排序（更早的池更成熟）
                pool_b.open_time.cmp(&pool_a.open_time)
            },
            other => other,
        }
    });

    // 返回 LP 供应量最高的池
    active_pools.into_iter().next()
}
