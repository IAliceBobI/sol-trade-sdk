use crate::{
    common::spl_associated_token_account::get_associated_token_address_with_program_id,
    constants::{TOKEN_PROGRAM, USDC_MINT, USDT_MINT, WSOL_TOKEN_ACCOUNT},
    instruction::utils::pumpswap::constants::{accounts, seeds},
};
use solana_sdk::pubkey::Pubkey;

/// 判断是否为 Hot Mint（主流桥接资产）
/// 当前包含：WSOL、USDC、USDT
pub(crate) fn is_hot_mint(mint: &Pubkey) -> bool {
    *mint == WSOL_TOKEN_ACCOUNT || *mint == USDC_MINT || *mint == USDT_MINT
}

/// 从两个 mint 中识别出 quote mint（计价货币）
///
/// 优先级：
/// 1. USD 稳定币（USDC、USDT、USD1）优先级最高
/// 2. SOL/WSOL 次之
/// 3. 如果两个都不是 USD/SOL/WSOL，返回 None
///
/// # 参数
/// * `mint_a` - 第一个 mint 地址
/// * `mint_b` - 第二个 mint 地址
///
/// # 返回值
/// * `Some(quote_mint)` - 识别出的 quote mint
/// * `None` - 无法识别（两个都不是主流 quote asset）
pub fn identify_quote_mint(mint_a: &Pubkey, mint_b: &Pubkey) -> Option<Pubkey> {
    use crate::constants::{SOL_MINT, USDC_MINT, USDT_MINT, USD1_TOKEN_ACCOUNT, WSOL_TOKEN_ACCOUNT};

    // 定义优先级枚举
    #[derive(PartialEq, PartialOrd, Copy, Clone)]
    enum QuotePriority {
        None = 0,
        Sol = 1,
        Usd = 2,
    }

    // 获取单个 mint 的优先级
    fn get_priority(mint: &Pubkey) -> QuotePriority {
        if *mint == USDC_MINT || *mint == USDT_MINT || *mint == USD1_TOKEN_ACCOUNT {
            QuotePriority::Usd
        } else if *mint == SOL_MINT || *mint == WSOL_TOKEN_ACCOUNT {
            QuotePriority::Sol
        } else {
            QuotePriority::None
        }
    }

    // 比较优先级，返回更高的那个
    let priority_a = get_priority(mint_a);
    let priority_b = get_priority(mint_b);

    match (priority_a, priority_b) {
        (QuotePriority::None, QuotePriority::None) => None,
        (QuotePriority::Usd, QuotePriority::Usd) => Some(*mint_a), // 两者都是 USD，返回第一个
        (QuotePriority::Sol, QuotePriority::Sol) => Some(*mint_a), // 两者都是 SOL，返回第一个
        _ if priority_a >= priority_b => Some(*mint_a),            // mint_a 优先级更高或相等
        _ => Some(*mint_b),                                         // mint_b 优先级更高
    }
}

/// 按 LP 供应量选择最佳池（PumpSwap 池没有交易量字段，使用 lp_supply 作为流动性指标）
///
/// 策略：
/// - LP 供应量越大，说明流动性越好
pub(crate) fn select_best_pool_by_liquidity(
    pools: &[(Pubkey, super::Pool)],
) -> Option<(Pubkey, super::Pool)> {
    if pools.is_empty() {
        return None;
    }

    if pools.len() == 1 {
        return pools.first().cloned();
    }

    // 按 LP 供应量排序
    let mut sorted_pools = pools.to_vec();
    sorted_pools.sort_by(|(_, pool_a), (_, pool_b)| {
        // 按 LP 供应量降序排序
        pool_b.lp_supply.cmp(&pool_a.lp_supply)
    });

    // 返回 LP 供应量最高的池
    sorted_pools.into_iter().next()
}

pub(crate) fn coin_creator_vault_authority(coin_creator: Pubkey) -> Pubkey {
    let (pump_pool_authority, _) = Pubkey::find_program_address(
        &[b"creator_vault", &coin_creator.to_bytes()],
        &accounts::AMM_PROGRAM,
    );
    pump_pool_authority
}

pub(crate) fn coin_creator_vault_ata(coin_creator: Pubkey, quote_mint: Pubkey) -> Pubkey {
    let creator_vault_authority = coin_creator_vault_authority(coin_creator);

    get_associated_token_address_with_program_id(
        &creator_vault_authority,
        &quote_mint,
        &TOKEN_PROGRAM,
    )
}

pub(crate) fn fee_recipient_ata(fee_recipient: Pubkey, quote_mint: Pubkey) -> Pubkey {
    crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
        &fee_recipient,
        &quote_mint,
        &TOKEN_PROGRAM,
    )
}

pub(crate) fn get_user_volume_accumulator_pda(user: &Pubkey) -> Option<Pubkey> {
    crate::common::fast_fn::get_cached_pda(
        crate::common::fast_fn::PdaCacheKey::PumpSwapUserVolume(*user),
        || {
            let seeds: &[&[u8]; 2] = &[seeds::USER_VOLUME_ACCUMULATOR_SEED, user.as_ref()];
            let program_id: &Pubkey = &accounts::AMM_PROGRAM;
            let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
            pda.map(|pubkey| pubkey.0)
        },
    )
}

#[allow(dead_code)]
pub(crate) fn get_global_volume_accumulator_pda() -> Option<Pubkey> {
    let seeds: &[&[u8]; 1] = &[seeds::GLOBAL_VOLUME_ACCUMULATOR_SEED];
    let program_id: &Pubkey = &accounts::AMM_PROGRAM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

#[inline]
#[allow(dead_code)]
pub(crate) fn get_fee_config_pda() -> Option<Pubkey> {
    let seeds: &[&[u8]; 2] = &[seeds::FEE_CONFIG_SEED, accounts::AMM_PROGRAM.as_ref()];
    let program_id: &Pubkey = &accounts::FEE_PROGRAM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

/// Calculate the canonical PumpSwap pool PDA for a mint that was migrated from PumpFun
///
/// Canonical pools are created by the PumpFun migrate instruction and use:
/// - pool_index = [0, 0] (CANONICAL_POOL_INDEX)
/// - pool_authority = PDA("pool-authority", mint) under PumpFun program
/// - pool = PDA("pool", [0, 0], pool_authority, mint, wsol_mint) under PumpSwap AMM program
pub(crate) fn calculate_canonical_pool_pda(mint: &Pubkey) -> Option<(Pubkey, Pubkey)> {
    use crate::constants::WSOL_TOKEN_ACCOUNT;
    use crate::instruction::utils::pumpfun::accounts::PUMPFUN;

    // Calculate pool_authority PDA (seeds: "pool-authority" + mint, program: PumpFun)
    let (pool_authority, _) =
        Pubkey::try_find_program_address(&[b"pool-authority", mint.as_ref()], &PUMPFUN)?;

    // Calculate pool PDA (seeds: "pool" + [0, 0] + pool_authority + mint + wsol_mint, program: PumpSwap AMM)
    let pool_index = [0u8, 0u8];
    let wsol_mint = WSOL_TOKEN_ACCOUNT; // WSOL mint address
    let (pool, _) = Pubkey::try_find_program_address(
        &[b"pool", &pool_index, pool_authority.as_ref(), mint.as_ref(), wsol_mint.as_ref()],
        &accounts::AMM_PROGRAM,
    )?;

    Some((pool, pool_authority))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{SOL_MINT, USDC_MINT, USDT_MINT, USD1_TOKEN_ACCOUNT, WSOL_TOKEN_ACCOUNT};

    #[test]
    fn test_identify_quote_mint_usdc_vs_wsol() {
        // USDC vs WSOL - 应该返回 USDC
        let result = identify_quote_mint(&USDC_MINT, &WSOL_TOKEN_ACCOUNT);
        assert_eq!(result, Some(USDC_MINT));
    }

    #[test]
    fn test_identify_quote_mint_usdt_vs_sol() {
        // USDT vs SOL - 应该返回 USDT
        let result = identify_quote_mint(&USDT_MINT, &SOL_MINT);
        assert_eq!(result, Some(USDT_MINT));
    }

    #[test]
    fn test_identify_quote_mint_sol_vs_random() {
        // SOL vs 任意 token - 应该返回 SOL
        let random_token = Pubkey::new_unique();
        let result = identify_quote_mint(&WSOL_TOKEN_ACCOUNT, &random_token);
        assert_eq!(result, Some(WSOL_TOKEN_ACCOUNT));
    }

    #[test]
    fn test_identify_quote_mint_usdc_vs_random() {
        // USDC vs 任意 token - 应该返回 USDC
        let random_token = Pubkey::new_unique();
        let result = identify_quote_mint(&USDC_MINT, &random_token);
        assert_eq!(result, Some(USDC_MINT));
    }

    #[test]
    fn test_identify_quote_mint_both_random() {
        // 两个非主流 token - 应该返回 None
        let token_a = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();
        let result = identify_quote_mint(&token_a, &token_b);
        assert_eq!(result, None);
    }

    #[test]
    fn test_identify_quote_mint_both_usd() {
        // USDC vs USDT - 应该返回第一个（USDC）
        let result = identify_quote_mint(&USDC_MINT, &USDT_MINT);
        assert_eq!(result, Some(USDC_MINT));
    }

    #[test]
    fn test_identify_quote_mint_both_sol() {
        // SOL vs WSOL - 应该返回第一个（SOL）
        let result = identify_quote_mint(&SOL_MINT, &WSOL_TOKEN_ACCOUNT);
        assert_eq!(result, Some(SOL_MINT));
    }

    #[test]
    fn test_identify_quote_mint_reverse_order() {
        // 反向顺序测试
        let random_token = Pubkey::new_unique();

        // WSOL vs random
        let result1 = identify_quote_mint(&WSOL_TOKEN_ACCOUNT, &random_token);
        assert_eq!(result1, Some(WSOL_TOKEN_ACCOUNT));

        // random vs WSOL
        let result2 = identify_quote_mint(&random_token, &WSOL_TOKEN_ACCOUNT);
        assert_eq!(result2, Some(WSOL_TOKEN_ACCOUNT));
    }

    #[test]
    fn test_identify_quote_mint_usd1_vs_sol() {
        // USD1 vs SOL - 应该返回 USD1
        let result = identify_quote_mint(&USD1_TOKEN_ACCOUNT, &SOL_MINT);
        assert_eq!(result, Some(USD1_TOKEN_ACCOUNT));
    }

    #[test]
    fn test_identify_quote_mint_usd1_vs_random() {
        // USD1 vs 任意 token - 应该返回 USD1
        let random_token = Pubkey::new_unique();
        let result = identify_quote_mint(&USD1_TOKEN_ACCOUNT, &random_token);
        assert_eq!(result, Some(USD1_TOKEN_ACCOUNT));
    }

    #[test]
    fn test_identify_quote_mint_usdc_vs_usd1() {
        // USDC vs USD1 - 应该返回第一个（USDC）
        let result = identify_quote_mint(&USDC_MINT, &USD1_TOKEN_ACCOUNT);
        assert_eq!(result, Some(USDC_MINT));
    }
}
