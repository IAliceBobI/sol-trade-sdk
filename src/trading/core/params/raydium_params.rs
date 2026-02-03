use crate::common::{SolanaRpcClient, auto_mock_rpc::PoolRpcClient};
use crate::trading::common::get_multi_token_balances_with_client;
use solana_sdk::pubkey::Pubkey;

/// RaydiumCpmm protocol specific parameters
/// Configuration parameters specific to Raydium CPMM trading protocol
#[derive(Clone)]
pub struct RaydiumCpmmParams {
    /// Pool address
    pub pool_state: Pubkey,
    /// Amm config address
    pub amm_config: Pubkey,
    /// Base token mint address
    pub base_mint: Pubkey,
    /// Quote token mint address
    pub quote_mint: Pubkey,
    /// Base token reserve amount in the pool
    pub base_reserve: u64,
    /// Quote token reserve amount in the pool
    pub quote_reserve: u64,
    /// Base token vault address
    pub base_vault: Pubkey,
    /// Quote token vault address
    pub quote_vault: Pubkey,
    /// Base token program ID
    pub base_token_program: Pubkey,
    /// Quote token program ID
    pub quote_token_program: Pubkey,
    /// Observation state account
    pub observation_state: Pubkey,
}

impl RaydiumCpmmParams {
    pub fn from_trade(
        pool_state: Pubkey,
        amm_config: Pubkey,
        input_token_mint: Pubkey,
        output_token_mint: Pubkey,
        input_vault: Pubkey,
        output_vault: Pubkey,
        input_token_program: Pubkey,
        output_token_program: Pubkey,
        observation_state: Pubkey,
        base_reserve: u64,
        quote_reserve: u64,
    ) -> Self {
        Self {
            pool_state,
            amm_config,
            base_mint: input_token_mint,
            quote_mint: output_token_mint,
            base_reserve,
            quote_reserve,
            base_vault: input_vault,
            quote_vault: output_vault,
            base_token_program: input_token_program,
            quote_token_program: output_token_program,
            observation_state,
        }
    }

    pub async fn from_pool_address_by_rpc(
        rpc: &SolanaRpcClient,
        pool_address: &Pubkey,
    ) -> Result<Self, anyhow::Error> {
        let pool =
            crate::instruction::utils::raydium_cpmm::get_pool_by_address(rpc, pool_address).await?;
        let (token0_balance, token1_balance) =
            crate::instruction::utils::raydium_cpmm::get_pool_token_balances(
                rpc,
                pool_address,
                &pool.token0_mint,
                &pool.token1_mint,
            )
            .await?;
        Ok(Self {
            pool_state: *pool_address,
            amm_config: pool.amm_config,
            base_mint: pool.token0_mint,
            quote_mint: pool.token1_mint,
            base_reserve: token0_balance,
            quote_reserve: token1_balance,
            base_vault: pool.token0_vault,
            quote_vault: pool.token1_vault,
            base_token_program: pool.token0_program,
            quote_token_program: pool.token1_program,
            observation_state: pool.observation_key,
        })
    }
}

/// RaydiumCpmm protocol specific parameters
/// Configuration parameters specific to Raydium CPMM trading protocol
#[derive(Clone, Debug)]
pub struct RaydiumAmmV4Params {
    /// AMM pool address
    pub amm: Pubkey,
    /// Base token (coin) mint address
    pub coin_mint: Pubkey,
    /// Quote token (pc) mint address
    pub pc_mint: Pubkey,
    /// Pool's coin token account address
    pub token_coin: Pubkey,
    /// Pool's pc token account address
    pub token_pc: Pubkey,
    /// Current coin reserve amount in the pool
    pub coin_reserve: u64,
    /// Current pc reserve amount in the pool
    pub pc_reserve: u64,
}

impl RaydiumAmmV4Params {
    pub fn new(
        amm: Pubkey,
        coin_mint: Pubkey,
        pc_mint: Pubkey,
        token_coin: Pubkey,
        token_pc: Pubkey,
        coin_reserve: u64,
        pc_reserve: u64,
    ) -> Self {
        Self {
            amm,
            coin_mint,
            pc_mint,
            token_coin,
            token_pc,
            coin_reserve,
            pc_reserve,
        }
    }

    /// 从 AMM 地址通过 RPC 获取参数（泛型版本，支持 Auto Mock）
    pub async fn from_amm_address_by_rpc_with_client<T: PoolRpcClient + ?Sized>(
        rpc: &T,
        amm: Pubkey,
    ) -> Result<Self, anyhow::Error> {
        let amm_info =
            crate::instruction::utils::raydium_amm_v4::get_pool_by_address(rpc, &amm).await?;
        let (coin_reserve, pc_reserve) =
            get_multi_token_balances_with_client(rpc, &amm_info.token_coin, &amm_info.token_pc)
                .await?;
        Ok(Self {
            amm,
            coin_mint: amm_info.coin_mint,
            pc_mint: amm_info.pc_mint,
            token_coin: amm_info.token_coin,
            token_pc: amm_info.token_pc,
            coin_reserve,
            pc_reserve,
        })
    }

    /// 从 AMM 地址通过 RPC 获取参数（便捷封装）
    pub async fn from_amm_address_by_rpc(
        rpc: &SolanaRpcClient,
        amm: Pubkey,
    ) -> Result<Self, anyhow::Error> {
        Self::from_amm_address_by_rpc_with_client(rpc, amm).await
    }
}

/// RaydiumClmm protocol specific parameters
/// Configuration parameters specific to Raydium CLMM trading protocol
#[derive(Clone)]
pub struct RaydiumClmmParams {
    /// Pool state address
    pub pool_state: Pubkey,
    /// AMM config address
    pub amm_config: Pubkey,
    /// Token0 mint address
    pub token0_mint: Pubkey,
    /// Token1 mint address
    pub token1_mint: Pubkey,
    /// Token0 vault address
    pub token0_vault: Pubkey,
    /// Token1 vault address
    pub token1_vault: Pubkey,
    /// Observation state account address
    pub observation_state: Pubkey,
    /// Token0 decimals
    pub token0_decimals: u8,
    /// Token1 decimals
    pub token1_decimals: u8,
    /// Token0 program ID
    pub token0_program: Pubkey,
    /// Token1 program ID
    pub token1_program: Pubkey,
}

impl RaydiumClmmParams {
    pub fn new(
        pool_state: Pubkey,
        amm_config: Pubkey,
        token0_mint: Pubkey,
        token1_mint: Pubkey,
        token0_vault: Pubkey,
        token1_vault: Pubkey,
        observation_state: Pubkey,
        token0_decimals: u8,
        token1_decimals: u8,
        token0_program: Pubkey,
        token1_program: Pubkey,
    ) -> Self {
        Self {
            pool_state,
            amm_config,
            token0_mint,
            token1_mint,
            token0_vault,
            token1_vault,
            observation_state,
            token0_decimals,
            token1_decimals,
            token0_program,
            token1_program,
        }
    }

    pub async fn from_pool_address_by_rpc(
        rpc: &SolanaRpcClient,
        pool_address: &Pubkey,
    ) -> Result<Self, anyhow::Error> {
        let pool_state =
            crate::instruction::utils::raydium_clmm::get_pool_by_address(rpc, pool_address).await?;

        // 获取 Token Program（使用缓存，避免重复 RPC 调用）
        // Mint 的 owner 永远不变，首次查询后会永久缓存
        let token0_program =
            crate::utils::token::get_token_program_with_cache(rpc, &pool_state.token_mint0).await?;
        let token1_program =
            crate::utils::token::get_token_program_with_cache(rpc, &pool_state.token_mint1).await?;

        // Observation state is stored in pool_state.observation_key

        Ok(Self {
            pool_state: *pool_address,
            amm_config: pool_state.amm_config,
            token0_mint: pool_state.token_mint0,
            token1_mint: pool_state.token_mint1,
            token0_vault: pool_state.token_vault0,
            token1_vault: pool_state.token_vault1,
            observation_state: pool_state.observation_key,
            token0_decimals: pool_state.mint_decimals0,
            token1_decimals: pool_state.mint_decimals1,
            token0_program,
            token1_program,
        })
    }
}
