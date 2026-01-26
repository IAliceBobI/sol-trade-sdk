use crate::{
    common::SolanaRpcClient,
    instruction::utils::bonk_types::{PoolState, pool_state_decode},
};
use anyhow::anyhow;
use solana_sdk::pubkey::Pubkey;

/// Constants used as seeds for deriving PDAs (Program Derived Addresses)
pub mod seeds {
    pub const POOL_SEED: &[u8] = b"bonkswappoolv1";
    pub const POOL_VAULT_SEED: &[u8] = b"pool_vault";
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};

    pub const AUTHORITY: Pubkey = pubkey!("WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh");
    pub const GLOBAL_CONFIG: Pubkey = pubkey!("6s1xP3hpbAfFoNtUNF8mfHsjr2Bd97JxFJRWLbL6aHuX");
    pub const USD1_GLOBAL_CONFIG: Pubkey = pubkey!("EPiZbnrThjyLnoQ6QQzkxeFqyL5uyg9RzNHHAudUPxBz");
    pub const EVENT_AUTHORITY: Pubkey = pubkey!("2DPAtwB8L12vrMRExbLuyGnC7n2J5LNoZQSejeQGpwkr");
    pub const BONK: Pubkey = pubkey!("BSwp6bEBihVLdqJRKGgzjcGLHkcTuzmSo1TQkHepzH8p");

    pub const PLATFORM_FEE_RATE: u128 = 100; // 1%
    pub const PROTOCOL_FEE_RATE: u128 = 25; // 0.25%
    pub const SHARE_FEE_RATE: u128 = 0; // 0%

    // META
    pub const AUTHORITY_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: AUTHORITY,
            is_signer: false,
            is_writable: false,
        };
    pub const GLOBAL_CONFIG_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: GLOBAL_CONFIG,
            is_signer: false,
            is_writable: false,
        };

    pub const USD1_GLOBAL_CONFIG_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: USD1_GLOBAL_CONFIG,
            is_signer: false,
            is_writable: false,
        };

    pub const EVENT_AUTHORITY_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta {
            pubkey: EVENT_AUTHORITY,
            is_signer: false,
            is_writable: false,
        };
    pub const BONK_META: solana_sdk::instruction::AccountMeta =
        solana_sdk::instruction::AccountMeta { pubkey: BONK, is_signer: false, is_writable: false };
}

pub const BUY_EXECT_IN_DISCRIMINATOR: [u8; 8] = [250, 234, 13, 123, 213, 156, 19, 236];
pub const SELL_EXECT_IN_DISCRIMINATOR: [u8; 8] = [149, 39, 222, 155, 211, 124, 152, 26];

// ==================== 缓存模块 ====================

const MAX_CACHE_SIZE: usize = 50_000;

pub(crate) mod bonk_cache {
    use super::*;
    use dashmap::DashMap;
    use once_cell::sync::Lazy;

    /// pool_address → PoolState 数据缓存
    pub(crate) static POOL_DATA_CACHE: Lazy<DashMap<Pubkey, PoolState>> =
        Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

    pub(crate) fn get_cached_pool_by_address(pool_address: &Pubkey) -> Option<PoolState> {
        POOL_DATA_CACHE.get(pool_address).map(|p| p.clone())
    }

    pub(crate) fn cache_pool_by_address(pool_address: &Pubkey, pool: &PoolState) {
        POOL_DATA_CACHE.insert(*pool_address, pool.clone());
    }

    pub(crate) fn clear_all() {
        POOL_DATA_CACHE.clear();
    }
}

pub async fn get_pool_by_address(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    // 1. 检查缓存
    if let Some(pool) = bonk_cache::get_cached_pool_by_address(pool_address) {
        return Ok(pool);
    }
    // 2. RPC 查询
    let account = rpc.get_account(pool_address).await?;
    if account.owner != accounts::BONK {
        return Err(anyhow!("Account is not owned by Bonk program"));
    }
    let pool_state = pool_state_decode(&account.data[8..])
        .ok_or_else(|| anyhow!("Failed to decode pool state"))?;
    // 3. 写入缓存
    bonk_cache::cache_pool_by_address(pool_address, &pool_state);
    Ok(pool_state)
}

pub async fn get_pool_by_address_force(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
) -> Result<PoolState, anyhow::Error> {
    bonk_cache::POOL_DATA_CACHE.remove(pool_address);
    get_pool_by_address(rpc, pool_address).await
}

pub fn clear_pool_cache() {
    bonk_cache::clear_all();
}

pub fn get_amount_in_net(
    amount_in: u64,
    protocol_fee_rate: u128,
    platform_fee_rate: u128,
    share_fee_rate: u128,
) -> u64 {
    let amount_in_u128 = amount_in as u128;
    let protocol_fee = amount_in_u128 * protocol_fee_rate / 10000 ;
    let platform_fee = amount_in_u128 * platform_fee_rate / 10000 ;
    let share_fee = amount_in_u128 * share_fee_rate / 10000 ;
    amount_in_u128
        .checked_sub(protocol_fee)
        .unwrap()
        .checked_sub(platform_fee)
        .unwrap()
        .checked_sub(share_fee)
        .unwrap() as u64
}

pub fn get_amount_in(
    amount_out: u64,
    protocol_fee_rate: u128,
    platform_fee_rate: u128,
    share_fee_rate: u128,
    virtual_base: u128,
    virtual_quote: u128,
    real_base: u128,
    real_quote: u128,
    slippage_basis_points: u128,
) -> u64 {
    let amount_out_u128 = amount_out as u128;

    // Consider slippage, actual required output amount is higher
    let amount_out_with_slippage = amount_out_u128 * 10000 / (10000 - slippage_basis_points);

    let input_reserve = virtual_quote.checked_add(real_quote).unwrap();
    let output_reserve = virtual_base.checked_sub(real_base).unwrap();

    // Reverse calculate using AMM formula: amount_in_net = (amount_out * input_reserve) / (output_reserve - amount_out)
    let numerator = amount_out_with_slippage.checked_mul(input_reserve).unwrap();
    let denominator = output_reserve.checked_sub(amount_out_with_slippage).unwrap();
    let amount_in_net = numerator.checked_div(denominator).unwrap();

    // Calculate total fee rate
    let total_fee_rate = protocol_fee_rate + platform_fee_rate + share_fee_rate;

    let amount_in = amount_in_net * 10000 / (10000 - total_fee_rate);

    amount_in as u64
}

pub fn get_amount_out(
    amount_in: u64,
    protocol_fee_rate: u128,
    platform_fee_rate: u128,
    share_fee_rate: u128,
    virtual_base: u128,
    virtual_quote: u128,
    real_base: u128,
    real_quote: u128,
    slippage_basis_points: u128,
) -> u64 {
    let amount_in_u128 = amount_in as u128;
    let protocol_fee = amount_in_u128 * protocol_fee_rate / 10000 ;
    let platform_fee = amount_in_u128 * platform_fee_rate / 10000 ;
    let share_fee = amount_in_u128 * share_fee_rate / 10000 ;
    let amount_in_net = amount_in_u128
        .checked_sub(protocol_fee)
        .unwrap()
        .checked_sub(platform_fee)
        .unwrap()
        .checked_sub(share_fee)
        .unwrap();
    let input_reserve = virtual_quote.checked_add(real_quote).unwrap();
    let output_reserve = virtual_base.checked_sub(real_base).unwrap();
    let numerator = amount_in_net.checked_mul(output_reserve).unwrap();
    let denominator = input_reserve.checked_add(amount_in_net).unwrap();
    let mut amount_out = numerator.checked_div(denominator).unwrap();

    amount_out = amount_out - (amount_out * slippage_basis_points) / 10000;
    amount_out as u64
}

pub fn get_pool_pda(base_mint: &Pubkey, quote_mint: &Pubkey) -> Option<Pubkey> {
    crate::common::fast_fn::get_cached_pda(
        crate::common::fast_fn::PdaCacheKey::BonkPool(*base_mint, *quote_mint),
        || {
            let seeds: &[&[u8]; 3] = &[seeds::POOL_SEED, base_mint.as_ref(), quote_mint.as_ref()];
            let program_id: &Pubkey = &accounts::BONK;
            let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
            pda.map(|pubkey| pubkey.0)
        },
    )
}

pub fn get_vault_pda(pool_state: &Pubkey, mint: &Pubkey) -> Option<Pubkey> {
    crate::common::fast_fn::get_cached_pda(
        crate::common::fast_fn::PdaCacheKey::BonkVault(*pool_state, *mint),
        || {
            let seeds: &[&[u8]; 3] = &[seeds::POOL_VAULT_SEED, pool_state.as_ref(), mint.as_ref()];
            let program_id: &Pubkey = &accounts::BONK;
            let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
            pda.map(|pubkey| pubkey.0)
        },
    )
}

pub fn get_platform_associated_account(platform_config: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 2] =
        &[platform_config.as_ref(), crate::constants::WSOL_TOKEN_ACCOUNT.as_ref()];
    let program_id: &Pubkey = &accounts::BONK;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

pub fn get_creator_associated_account(creator: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 2] = &[creator.as_ref(), crate::constants::WSOL_TOKEN_ACCOUNT.as_ref()];
    let program_id: &Pubkey = &accounts::BONK;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    #[test]
    fn test_get_pool_pda_returns_valid_pda() {
        // Test with known mint addresses
        let base_mint = Pubkey::from_str("EPeUFDgHRxs9xxEPVaL6kfGQvCon7jmAWKVUHuux1Tpz").unwrap();
        let quote_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

        let result = get_pool_pda(&base_mint, &quote_mint);

        // Should return Some(Pubkey)
        assert!(result.is_some());

        let pool_pda = result.unwrap();

        println!("计算得到的池子地址: {}", pool_pda);

        // 验证池子地址是否正确
        let expected_pool =
            Pubkey::from_str("HytrL5NCP55DyJJtD8Rx7BTKw59ZPZx558GuGW2AP2od").unwrap();
        println!("期望的池子地址: {}", expected_pool);

        assert_eq!(pool_pda, expected_pool, "池子地址不匹配!");

        // 同时验证PDA推导是否正确
        let seeds: &[&[u8]; 3] = &[seeds::POOL_SEED, base_mint.as_ref(), quote_mint.as_ref()];
        let (derived_pda, _bump) = Pubkey::find_program_address(seeds, &accounts::BONK);
        println!("通过find_program_address推导的地址: {}", derived_pda);

        assert_eq!(pool_pda, derived_pda, "PDA推导结果不一致!");
    }

    #[tokio::test]
    async fn test_fetch_pool_state() {
        // Test pool address provided by user
        let pool_address =
            Pubkey::from_str("5UUBHfBssdFDtqFrcuPYA8xvYftYwYWEawucDuAH45KX").unwrap();

        // Use public Solana RPC endpoint
        let rpc_url = "http://127.0.0.1:8899";
        let rpc = RpcClient::new(rpc_url.to_string());

        // Call get_pool_by_address
        let result = get_pool_by_address(&rpc, &pool_address).await;

        // Verify the result
        assert!(result.is_ok(), "Failed to fetch pool state: {:?}", result.err());

        let pool_state = result.unwrap();

        // Print pool state for verification
        println!("Pool State fetched successfully!");
        println!("  Token X: {}", pool_state.token_x);
        println!("  Token Y: {}", pool_state.token_y);
        println!("  Token X Reserve: {}", pool_state.token_x_reserve.v);
        println!("  Token Y Reserve: {}", pool_state.token_y_reserve.v);
        println!("  Price: {}", pool_state.price.v);
        println!("  LP Fee: {}", pool_state.lp_fee.v);
        println!("  Buyback Fee: {}", pool_state.buyback_fee.v);
        println!("  Project Fee: {}", pool_state.project_fee.v);
        println!("  Mercanti Fee: {}", pool_state.mercanti_fee.v);
        println!("  Const K: {}", pool_state.const_k.v);
        println!("  Farm Count: {}", pool_state.farm_count);
        println!("  Bump: {}", pool_state.bump);

        // Verify basic field constraints
        assert!(pool_state.token_x_reserve.v > 0, "Token X reserve should be positive");
        assert!(pool_state.token_y_reserve.v > 0, "Token Y reserve should be positive");
        assert!(!pool_state.token_x.eq(&Pubkey::default()), "Token X should not be zero");
        assert!(!pool_state.token_y.eq(&Pubkey::default()), "Token Y should not be zero");
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_pool_state_owner_validation() {
        // Test that get_pool_by_address validates the program owner
        let invalid_address = Pubkey::from_str("11111111111111111111111111111111").unwrap(); // System program

        let rpc_url = "https://api.mainnet-beta.solana.com";
        let rpc = RpcClient::new(rpc_url.to_string());

        let result = get_pool_by_address(&rpc, &invalid_address).await;

        // Should fail because system program is not the BONK program
        assert!(result.is_err(), "Expected error for invalid program owner");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Bonk program"), "Error should mention Bonk program");
    }
}
