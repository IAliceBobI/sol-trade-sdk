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

pub(crate) fn get_global_volume_accumulator_pda() -> Option<Pubkey> {
    let seeds: &[&[u8]; 1] = &[seeds::GLOBAL_VOLUME_ACCUMULATOR_SEED];
    let program_id: &Pubkey = &accounts::AMM_PROGRAM;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

#[inline]
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
