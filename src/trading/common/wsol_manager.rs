use crate::common::{
    fast_fn::create_associated_token_account_idempotent_fast,
    seed::{
        create_associated_token_account_use_seed,
        get_associated_token_address_with_program_id_use_seed,
    },
    spl_token::close_account,
};
use smallvec::SmallVec;
use solana_sdk::{instruction::Instruction, message::AccountMeta, pubkey::Pubkey};
use solana_system_interface::instruction::transfer;

#[inline]
pub fn handle_wsol(payer: &Pubkey, amount_in: u64) -> SmallVec<[Instruction; 3]> {
    let wsol_token_account =
        crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
            payer,
            &crate::constants::WSOL_TOKEN_ACCOUNT,
            &crate::constants::TOKEN_PROGRAM,
        );

    let mut insts = SmallVec::<[Instruction; 3]>::new();
    insts.extend(create_associated_token_account_idempotent_fast(
        &payer,
        &payer,
        &crate::constants::WSOL_TOKEN_ACCOUNT,
        &crate::constants::TOKEN_PROGRAM,
    ));
    insts.extend([
        transfer(payer, &wsol_token_account, amount_in),
        // sync_native
        Instruction {
            program_id: crate::constants::TOKEN_PROGRAM,
            accounts: vec![AccountMeta::new(wsol_token_account, false)],
            data: vec![17],
        },
    ]);

    insts
}

pub fn close_wsol(payer: &Pubkey) -> Vec<Instruction> {
    use std::sync::Arc;

    let wsol_token_account =
        crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
            &payer,
            &crate::constants::WSOL_TOKEN_ACCOUNT,
            &crate::constants::TOKEN_PROGRAM,
        );
    let arc_instructions = crate::common::fast_fn::get_cached_instructions(
        crate::common::fast_fn::InstructionCacheKey::CloseWsolAccount {
            payer: *payer,
            wsol_token_account,
        },
        || {
            vec![
                close_account(
                    &crate::constants::TOKEN_PROGRAM,
                    &wsol_token_account,
                    &payer,
                    &payer,
                    &[],
                )
                .unwrap(),
            ]
        },
    );

    // 🚀 性能优化：尝试零开销解包 Arc
    Arc::try_unwrap(arc_instructions).unwrap_or_else(|arc| (*arc).clone())
}

#[inline]
pub fn create_wsol_ata(payer: &Pubkey) -> Vec<Instruction> {
    create_associated_token_account_idempotent_fast(
        &payer,
        &payer,
        &crate::constants::WSOL_TOKEN_ACCOUNT,
        &crate::constants::TOKEN_PROGRAM,
    )
}

/// 只充值SOL到已存在的WSOL ATA（不创建账户）- 标准方式
#[inline]
pub fn wrap_sol_only(payer: &Pubkey, amount_in: u64) -> SmallVec<[Instruction; 2]> {
    let wsol_token_account =
        crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
            &payer,
            &crate::constants::WSOL_TOKEN_ACCOUNT,
            &crate::constants::TOKEN_PROGRAM,
        );

    let mut insts = SmallVec::<[Instruction; 2]>::new();
    insts.extend([
        transfer(&payer, &wsol_token_account, amount_in),
        // sync_native
        Instruction {
            program_id: crate::constants::TOKEN_PROGRAM,
            accounts: vec![AccountMeta::new(wsol_token_account, false)],
            data: vec![17],
        },
    ]);

    insts
}

pub fn wrap_wsol_to_sol(payer: &Pubkey, amount: u64) -> Result<Vec<Instruction>, anyhow::Error> {
    let mut instructions = Vec::new();

    // 1. 创建 WSOL seed 账户（注意：如果账户已存在会失败）
    // 调用方应该先检查账户是否存在，如果存在则跳过此步骤
    let seed_account_instructions = create_associated_token_account_use_seed(
        payer,
        payer,
        &crate::constants::WSOL_TOKEN_ACCOUNT,
        &crate::constants::TOKEN_PROGRAM,
    )?;
    instructions.extend(seed_account_instructions);

    // 2. 获取 seed 账户的 ATA 地址
    let seed_ata_address = get_associated_token_address_with_program_id_use_seed(
        payer,
        &crate::constants::WSOL_TOKEN_ACCOUNT,
        &crate::constants::TOKEN_PROGRAM,
    )?;

    // 3. 获取用户的 WSOL ATA 地址
    let user_wsol_ata = crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
        &payer,
        &crate::constants::WSOL_TOKEN_ACCOUNT,
        &crate::constants::TOKEN_PROGRAM,
    );

    // 4. 添加从用户 WSOL ATA 转账到 seed ATA 的指令
    let transfer_instruction = crate::common::spl_token::transfer(
        &crate::constants::TOKEN_PROGRAM,
        &user_wsol_ata,
        &seed_ata_address,
        &payer,
        amount,
        &[],
    )?;
    instructions.push(transfer_instruction);

    // 5. 添加关闭 WSOL seed 账户的指令
    let close_instruction =
        close_account(&crate::constants::TOKEN_PROGRAM, &seed_ata_address, &payer, &payer, &[])?;
    instructions.push(close_instruction);

    Ok(instructions)
}

/// 将 WSOL 转换为 SOL（仅转账和关闭，不创建账户）
/// 用于当临时seed账户已存在的情况
pub fn wrap_wsol_to_sol_without_create(
    payer: &Pubkey,
    amount: u64,
) -> Result<Vec<Instruction>, anyhow::Error> {
    let mut instructions = Vec::new();

    // 1. 获取 seed 账户的 ATA 地址
    let seed_ata_address = get_associated_token_address_with_program_id_use_seed(
        payer,
        &crate::constants::WSOL_TOKEN_ACCOUNT,
        &crate::constants::TOKEN_PROGRAM,
    )?;

    // 2. 获取用户的 WSOL ATA 地址
    let user_wsol_ata = crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
        &payer,
        &crate::constants::WSOL_TOKEN_ACCOUNT,
        &crate::constants::TOKEN_PROGRAM,
    );

    // 3. 添加从用户 WSOL ATA 转账到 seed ATA 的指令
    let transfer_instruction = crate::common::spl_token::transfer(
        &crate::constants::TOKEN_PROGRAM,
        &user_wsol_ata,
        &seed_ata_address,
        payer,
        amount,
        &[],
    )?;
    instructions.push(transfer_instruction);

    // 4. 添加关闭 WSOL seed 账户的指令
    let close_instruction =
        close_account(&crate::constants::TOKEN_PROGRAM, &seed_ata_address, &payer, &payer, &[])?;
    instructions.push(close_instruction);

    Ok(instructions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{SYSTEM_PROGRAM, TOKEN_PROGRAM, WSOL_TOKEN_ACCOUNT};

    #[test]
    fn test_handle_wsol_instructions_count() {
        let payer = &Pubkey::new_unique();
        let amount_in = 1_000_000;

        let instructions = handle_wsol(payer, amount_in);

        // 应该生成3条指令：创建ATA、转账、sync_native
        assert_eq!(instructions.len(), 3);
    }

    #[test]
    fn test_handle_wsol_ata_address() {
        let payer = &Pubkey::new_unique();
        let amount_in = 1_000_000;

        let expected_ata =
            crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
                payer,
                &WSOL_TOKEN_ACCOUNT,
                &TOKEN_PROGRAM,
            );

        let instructions = handle_wsol(payer, amount_in);

        // 第一条指令应该是创建 ATA
        let create_ata_instruction = &instructions[0];
        assert_eq!(create_ata_instruction.program_id, spl_associated_token_account::ID);

        // 转账指令的目标应该是 WSOL ATA
        let transfer_instruction = &instructions[1];
        assert_eq!(transfer_instruction.accounts.len(), 2);
        assert_eq!(transfer_instruction.accounts[1].pubkey, expected_ata);

        // sync_native 指令的目标应该是 WSOL ATA
        let sync_instruction = &instructions[2];
        assert_eq!(sync_instruction.program_id, TOKEN_PROGRAM);
        assert_eq!(sync_instruction.accounts[0].pubkey, expected_ata);
        assert_eq!(sync_instruction.data, vec![17]); // sync_native 的 opcode
    }

    #[test]
    fn test_close_wsol_instructions_count() {
        let payer = &Pubkey::new_unique();
        let instructions = close_wsol(payer);

        // 应该生成1条指令：关闭账户
        assert_eq!(instructions.len(), 1);
    }

    #[test]
    fn test_create_wsol_ata() {
        let payer = &Pubkey::new_unique();
        let instructions = create_wsol_ata(payer);

        // 应该生成1条指令：创建 ATA
        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0].program_id, spl_associated_token_account::ID);
    }

    #[test]
    fn test_wrap_sol_only_instructions_count() {
        let payer = &Pubkey::new_unique();
        let amount_in = 1_000_000;

        let instructions = wrap_sol_only(payer, amount_in);

        // 应该生成2条指令：转账、sync_native（不创建ATA）
        assert_eq!(instructions.len(), 2);
    }

    #[test]
    fn test_wrap_sol_only_no_create_ata() {
        let payer = &Pubkey::new_unique();
        let amount_in = 1_000_000;

        let instructions = wrap_sol_only(payer, amount_in);

        // 第一条应该是系统转账，不是创建 ATA
        let transfer_instruction = &instructions[0];
        assert_ne!(transfer_instruction.program_id, spl_associated_token_account::ID);
        assert_eq!(transfer_instruction.program_id, SYSTEM_PROGRAM);
    }

    #[test]
    fn test_handle_wsol_amount_transfer() {
        let payer = &Pubkey::new_unique();
        let amount_in = 2_500_000_000; // 2.5 SOL

        let instructions = handle_wsol(payer, amount_in);

        // 检查转账指令的金额
        let transfer_instruction = &instructions[1];
        // 转账指令的数据包含 lamports 金额
        let lamports = u64::from_le_bytes(
            transfer_instruction.data[4..12]
                .try_into()
                .expect("should have 8 bytes for lamports"),
        );
        assert_eq!(lamports, amount_in);
    }

    #[test]
    fn test_sync_native_opcode() {
        let payer = &Pubkey::new_unique();
        let instructions = handle_wsol(payer, 1_000_000);

        let sync_instruction = &instructions[2];
        // sync_native 的 opcode 是 17
        assert_eq!(sync_instruction.data, vec![17]);
    }

    #[test]
    fn test_wsol_address_pda_derivation() {
        // 测试 WSOL ATA 地址推导是否正确
        let payer = Pubkey::new_from_array([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]);
        let wsol_ata = crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
            &payer,
            &WSOL_TOKEN_ACCOUNT,
            &TOKEN_PROGRAM,
        );

        // 验证推导出的地址不是零地址
        assert_ne!(wsol_ata, Pubkey::default());
        assert_ne!(wsol_ata, payer);
    }

    /// 验证主网地址的 WSOL ATA 计算
    /// 预期 ATA: F7hCHiC6gZLqufNag1ytn4a34S22nvjEbwgH7qbnjuvG
    #[test]
    fn test_wsol_ata_for_mainnet_address() {
        // 主网地址: 2QfBNK2WDwSLoUQRb1zAnp3KM12N9hQ8q6ApwUMnWW2T
        let payer = "2QfBNK2WDwSLoUQRb1zAnp3KM12N9hQ8q6ApwUMnWW2T"
            .parse::<Pubkey>()
            .expect("Invalid payer address");

        // 计算 WSOL ATA
        let wsol_ata = crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
            &payer,
            &WSOL_TOKEN_ACCOUNT,
            &TOKEN_PROGRAM,
        );

        // 预期 ATA 地址: F7hCHiC6gZLqufNag1ytn4a34S22nvjEbwgH7qbnjuvG
        let expected_ata: Pubkey = "F7hCHiC6gZLqufNag1ytn4a34S22nvjEbwgH7qbnjuvG"
            .parse()
            .expect("Invalid expected ATA address");

        println!("Payer: {}", payer);
        println!("WSOL Mint: {}", WSOL_TOKEN_ACCOUNT);
        println!("Expected ATA: {}", expected_ata);
        println!("Calculated ATA: {}", wsol_ata);

        assert_eq!(wsol_ata, expected_ata, "WSOL ATA 计算不匹配!");
    }
}
