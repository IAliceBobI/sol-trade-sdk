use crate::common::SolanaRpcClient;
use crate::common::bonding_curve::BondingCurveAccount;
use crate::instruction::utils::pumpfun::global_constants::MAYHEM_FEE_RECIPIENT;
use crate::utils::token::calculate_ata;
use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

/// PumpFun protocol specific parameters
/// Configuration parameters specific to PumpFun trading protocol
#[derive(Clone)]
pub struct PumpFunParams {
    pub bonding_curve: Arc<BondingCurveAccount>,
    pub associated_bonding_curve: Pubkey,
    pub creator_vault: Pubkey,
    pub token_program: Pubkey,
    /// Whether to close token account when selling, only effective during sell operations
    pub close_token_account_when_sell: Option<bool>,
}

impl PumpFunParams {
    pub fn immediate_sell(
        creator_vault: Pubkey,
        token_program: Pubkey,
        close_token_account_when_sell: bool,
    ) -> Self {
        Self {
            bonding_curve: Arc::new(BondingCurveAccount { ..Default::default() }),
            associated_bonding_curve: Pubkey::default(),
            creator_vault,
            close_token_account_when_sell: Some(close_token_account_when_sell),
            token_program,
        }
    }

    pub fn from_dev_trade(
        mint: Pubkey,
        token_amount: u64,
        max_sol_cost: u64,
        creator: Pubkey,
        bonding_curve: Pubkey,
        associated_bonding_curve: Pubkey,
        creator_vault: Pubkey,
        close_token_account_when_sell: Option<bool>,
        fee_recipient: Pubkey,
        token_program: Pubkey,
    ) -> Result<Self> {
        let is_mayhem_mode = fee_recipient == MAYHEM_FEE_RECIPIENT;
        let bonding_curve_account =
            BondingCurveAccount::from_dev_trade(crate::common::bonding_curve::DevTradeParams {
                bonding_curve,
                mint,
                dev_token_amount: token_amount,
                dev_sol_amount: max_sol_cost,
                creator,
                is_mayhem_mode,
            })?;
        Ok(Self {
            bonding_curve: Arc::new(bonding_curve_account),
            associated_bonding_curve,
            creator_vault,
            close_token_account_when_sell,
            token_program,
        })
    }

    pub fn from_trade(
        bonding_curve: Pubkey,
        associated_bonding_curve: Pubkey,
        mint: Pubkey,
        creator: Pubkey,
        creator_vault: Pubkey,
        virtual_token_reserves: u64,
        virtual_sol_reserves: u64,
        real_token_reserves: u64,
        real_sol_reserves: u64,
        close_token_account_when_sell: Option<bool>,
        fee_recipient: Pubkey,
        token_program: Pubkey,
    ) -> Result<Self> {
        let is_mayhem_mode = fee_recipient == MAYHEM_FEE_RECIPIENT;
        let bonding_curve =
            BondingCurveAccount::from_trade(crate::common::bonding_curve::TradeParams {
                bonding_curve,
                mint,
                creator,
                virtual_token_reserves,
                virtual_sol_reserves,
                real_token_reserves,
                real_sol_reserves,
                is_mayhem_mode,
            })?;
        Ok(Self {
            bonding_curve: Arc::new(bonding_curve),
            associated_bonding_curve,
            creator_vault,
            close_token_account_when_sell,
            token_program,
        })
    }

    pub async fn from_mint_by_rpc(
        rpc: &SolanaRpcClient,
        mint: &Pubkey,
    ) -> Result<Self, anyhow::Error> {
        let account =
            crate::instruction::utils::pumpfun::fetch_bonding_curve_account(rpc, mint).await?;
        let mint_account = rpc.get_account(mint).await?;
        let bonding_curve = BondingCurveAccount {
            discriminator: 0,
            account: account.1,
            virtual_token_reserves: account.0.virtual_token_reserves,
            virtual_sol_reserves: account.0.virtual_sol_reserves,
            real_token_reserves: account.0.real_token_reserves,
            real_sol_reserves: account.0.real_sol_reserves,
            token_total_supply: account.0.token_total_supply,
            complete: account.0.complete,
            creator: account.0.creator,
            is_mayhem_mode: account.0.is_mayhem_mode,
        };
        let associated_bonding_curve = calculate_ata(rpc, &bonding_curve.account, mint).await?;
        let creator_vault =
            crate::instruction::utils::pumpfun::get_creator_vault_pda(&bonding_curve.creator)
                .ok_or_else(|| anyhow::anyhow!("Creator vault PDA not found for creator"))?;
        Ok(Self {
            bonding_curve: Arc::new(bonding_curve),
            associated_bonding_curve,
            creator_vault,
            close_token_account_when_sell: None,
            token_program: mint_account.owner,
        })
    }
}
