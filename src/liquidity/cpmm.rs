//! Raydium CPMM 流动性管理
//!
//! 提供向 Raydium CPMM 池子添加流动性的功能

use crate::instruction::utils::raydium_cpmm_types::PoolState;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Raydium CPMM 程序 ID
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// Authority seed
pub const AUTH_SEED: &str = "vault_and_lp_mint_auth_seed";

/// Deposit 指令的 Anchor discriminator (8 字节)
const DEPOSIT_DISCRIMINATOR: [u8; 8] = [242, 35, 198, 137, 82, 225, 242, 182];

/// 舍入方向
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoundDirection {
    /// 向下舍入，1.9 => 1, 1.1 => 1, 1.5 => 1
    Floor,
    /// 向上舍入，1.9 => 2, 1.1 => 2, 1.5 => 2
    Ceiling,
}

/// LP token 转换结果
#[derive(Debug, Clone, PartialEq)]
pub struct TradingTokenResult {
    pub token_0_amount: u128,
    pub token_1_amount: u128,
}

/// CPMM Deposit 参数
#[derive(Clone, Debug)]
pub struct CpmmDepositParams {
    /// 池子状态账户地址
    pub pool_state: Pubkey,
    /// 用户的 LP 代币账户地址
    pub owner_lp_token: Pubkey,
    /// 用户的 token_0 账户地址
    pub token_0_account: Pubkey,
    /// 用户的 token_1 账户地址
    pub token_1_account: Pubkey,
    /// Token_0 金库地址
    pub token_0_vault: Pubkey,
    /// Token_1 金库地址
    pub token_1_vault: Pubkey,
    /// Token_0 mint 地址
    pub token_0_mint: Pubkey,
    /// Token_1 mint 地址
    pub token_1_mint: Pubkey,
    /// LP mint 地址
    pub lp_mint: Pubkey,
    /// 要铸造的 LP 代币数量
    pub lp_token_amount: u64,
    /// 最大 token_0 数量（滑点保护）
    pub maximum_token_0_amount: u64,
    /// 最大 token_1 数量（滑点保护）
    pub maximum_token_1_amount: u64,
    /// Token 程序 ID (Token 或 Token-2022)
    pub token_program: Pubkey,
}

/// 构建 CPMM Deposit 指令
///
/// # 参数
///
/// * `params` - Deposit 参数
/// * `owner` - 用户钱包地址（签名者）
///
/// # 返回
///
/// 返回构建好的 Instruction
pub fn build_deposit_instruction(params: CpmmDepositParams, owner: Pubkey) -> Instruction {
    let program_id = Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID)
        .expect("RAYDIUM_CPMM_PROGRAM_ID is a valid valid pubkey");

    // 派生 authority PDA
    let (authority, _bump) = Pubkey::find_program_address(&[AUTH_SEED.as_bytes()], &program_id);

    // 构建指令数据（使用 Anchor discriminator）
    let mut data = Vec::with_capacity(8 + 24); // discriminator (8) + 3 * u64 (24)
    data.extend_from_slice(&DEPOSIT_DISCRIMINATOR);
    data.extend_from_slice(&params.lp_token_amount.to_le_bytes());
    data.extend_from_slice(&params.maximum_token_0_amount.to_le_bytes());
    data.extend_from_slice(&params.maximum_token_1_amount.to_le_bytes());

    // 构建账户列表（按 Anchor CPI 账户顺序）
    let accounts = vec![
        // 0. owner (signer)
        AccountMeta::new_readonly(owner, true),
        // 1. authority (PDA)
        AccountMeta::new_readonly(authority, false),
        // 2. pool_state
        AccountMeta::new(params.pool_state, false),
        // 3. owner_lp_token
        AccountMeta::new(params.owner_lp_token, false),
        // 4. token_0_account
        AccountMeta::new(params.token_0_account, false),
        // 5. token_1_account
        AccountMeta::new(params.token_1_account, false),
        // 6. token_0_vault
        AccountMeta::new(params.token_0_vault, false),
        // 7. token_1_vault
        AccountMeta::new(params.token_1_vault, false),
        // 8. token_program
        AccountMeta::new_readonly(params.token_program, false),
        // 9. token_program_2022
        AccountMeta::new_readonly(spl_token_2022::id(), false),
        // 10. vault_0_mint
        AccountMeta::new_readonly(params.token_0_mint, false),
        // 11. vault_1_mint
        AccountMeta::new_readonly(params.token_1_mint, false),
        // 12. lp_mint
        AccountMeta::new(params.lp_mint, false),
    ];

    Instruction { program_id, accounts, data }
}

/// 计算添加流动性所需的代币数量
///
/// 根据 LP token 数量和当前池子状态，计算需要提供的 token_0 和 token_1 数量
///
/// # 参数
///
/// * `lp_token_amount` - 要铸造的 LP 代币数量
/// * `pool_state` - 池子状态
/// * `token_0_vault_amount` - 当前 token_0 金库余额（不含手续费）
/// * `token_1_vault_amount` - 当前 token_1 金库余额（不含手续费）
///
/// # 返回
///
/// 返回 (token_0_amount, token_1_amount)
pub fn calculate_deposit_amounts(
    lp_token_amount: u64,
    pool_state: &PoolState,
    token_0_vault_amount: u64,
    token_1_vault_amount: u64,
) -> Option<(u64, u64)> {
    let lp_amount = u128::from(lp_token_amount);
    let lp_supply = u128::from(pool_state.lp_supply);
    let vault_0 = u128::from(token_0_vault_amount);
    let vault_1 = u128::from(token_1_vault_amount);

    if lp_supply == 0 {
        // 如果是首次添加流动性，使用固定比例（简化处理）
        return None;
    }

    // 使用恒定乘积公式计算 LP token 对应的交易代币数量
    let result = lp_tokens_to_trading_tokens(
        lp_amount,
        lp_supply,
        vault_0,
        vault_1,
        RoundDirection::Ceiling,
    )?;

    let token_0 = u64::try_from(result.token_0_amount).ok()?;
    let token_1 = u64::try_from(result.token_1_amount).ok()?;

    Some((token_0, token_1))
}

/// LP token 转换为交易代币（恒定乘积公式）
///
/// 使用 Uniswap v2 风格的恒定乘积公式计算 LP token 可以兑换多少交易代币
fn lp_tokens_to_trading_tokens(
    lp_token_amount: u128,
    lp_token_supply: u128,
    token_0_vault_amount: u128,
    token_1_vault_amount: u128,
    round_direction: RoundDirection,
) -> Option<TradingTokenResult> {
    let mut token_0_amount = lp_token_amount
        .checked_mul(token_0_vault_amount)?
        .checked_div(lp_token_supply)?;
    let mut token_1_amount = lp_token_amount
        .checked_mul(token_1_vault_amount)?
        .checked_div(lp_token_supply)?;

    let (token_0_amount, token_1_amount) = match round_direction {
        RoundDirection::Floor => (token_0_amount, token_1_amount),
        RoundDirection::Ceiling => {
            // 向上舍入：如果有余数且结果 > 0，则 +1
            let token_0_remainder = lp_token_amount
                .checked_mul(token_0_vault_amount)?
                .checked_rem(lp_token_supply)?;
            if token_0_remainder > 0 && token_0_amount > 0 {
                token_0_amount += 1;
            }

            let token_1_remainder = lp_token_amount
                .checked_mul(token_1_vault_amount)?
                .checked_rem(lp_token_supply)?;
            if token_1_remainder > 0 && token_1_amount > 0 {
                token_1_amount += 1;
            }

            (token_0_amount, token_1_amount)
        },
    };

    Some(TradingTokenResult { token_0_amount, token_1_amount })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_deposit_instruction() {
        let params = CpmmDepositParams {
            pool_state: Pubkey::new_unique(),
            owner_lp_token: Pubkey::new_unique(),
            token_0_account: Pubkey::new_unique(),
            token_1_account: Pubkey::new_unique(),
            token_0_vault: Pubkey::new_unique(),
            token_1_vault: Pubkey::new_unique(),
            token_0_mint: Pubkey::new_unique(),
            token_1_mint: Pubkey::new_unique(),
            lp_mint: Pubkey::new_unique(),
            lp_token_amount: 1_000_000,
            maximum_token_0_amount: 1_000_000_000,
            maximum_token_1_amount: 1_000_000_000,
            token_program: spl_token::id(),
        };

        let owner = Pubkey::new_unique();
        let instruction = build_deposit_instruction(params, owner);

        assert_eq!(instruction.program_id, Pubkey::from_str(RAYDIUM_CPMM_PROGRAM_ID).unwrap());
        // 验证 discriminator 正确
        assert_eq!(&instruction.data[0..8], &DEPOSIT_DISCRIMINATOR);
        assert_eq!(instruction.accounts.len(), 13);
    }

    #[test]
    fn test_lp_tokens_to_trading_tokens() {
        // 测试示例：池子有 2 个 token A 和 49 个 token B，LP 供应量为 10
        // 如果要铸造 5 个 LP token，应该得到 1 个 token A 和 25 个 token B
        let result = lp_tokens_to_trading_tokens(
            5,  // lp_token_amount
            10, // lp_token_supply
            2,  // token_0_vault_amount
            49, // token_1_vault_amount
            RoundDirection::Ceiling,
        );

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.token_0_amount, 1);
        assert_eq!(result.token_1_amount, 25);
    }

    #[test]
    fn test_calculate_deposit_amounts() {
        let pool_state = PoolState { lp_supply: 1000, ..Default::default() };

        let token_0_vault = 100_000;
        let token_1_vault = 50_000;

        // 要铸造 100 LP，应该得到 10% 的金库余额
        let result = calculate_deposit_amounts(100, &pool_state, token_0_vault, token_1_vault);

        assert!(result.is_some());
        let (token_0, token_1) = result.unwrap();
        assert_eq!(token_0, 10_000);
        assert_eq!(token_1, 5_000);
    }
}
