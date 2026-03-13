use crate::common::SolanaRpcClient;
use solana_sdk::pubkey::Pubkey;

/// MeteoraDammV2 protocol specific parameters
/// Configuration parameters specific to Meteora Damm V2 trading protocol
#[derive(Clone)]
pub struct MeteoraDammV2Params {
    pub pool: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_program: Pubkey,
    pub token_b_program: Pubkey,
}

impl MeteoraDammV2Params {
    pub fn new(
        pool: Pubkey,
        token_a_vault: Pubkey,
        token_b_vault: Pubkey,
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
        token_a_program: Pubkey,
        token_b_program: Pubkey,
    ) -> Self {
        Self {
            pool,
            token_a_vault,
            token_b_vault,
            token_a_mint,
            token_b_mint,
            token_a_program,
            token_b_program,
        }
    }

    pub async fn from_pool_address_by_rpc(
        rpc: &SolanaRpcClient,
        pool_address: &Pubkey,
    ) -> Result<Self, anyhow::Error> {
        let pool_data =
            crate::instruction::utils::meteora_damm_v2::get_pool_by_address(rpc, pool_address)
                .await?;

        // 🔧 获取 Token Program（从 Vault 账户的 owner）
        // Meteora DAMM V2 验证的是 Vault 账户的 owner 而不是 Mint 账户的 owner
        // 参考: ./temp/meteora/dlmm/damm-v2/programs/cp-amm/src/instructions/swap/ix_swap.rs
        // require!(token_a_vault.owner() == token_a_program.key(), ErrorCode::ConstraintTokenTokenProgram);
        let token_a_vault_account = rpc.get_account(&pool_data.token_a_vault).await?;
        let token_b_vault_account = rpc.get_account(&pool_data.token_b_vault).await?;
        let token_a_program = token_a_vault_account.owner;
        let token_b_program = token_b_vault_account.owner;

        Ok(Self {
            pool: *pool_address,
            token_a_vault: pool_data.token_a_vault,
            token_b_vault: pool_data.token_b_vault,
            token_a_mint: pool_data.token_a_mint,
            token_b_mint: pool_data.token_b_mint,
            token_a_program,
            token_b_program,
        })
    }
}
