//! TransactionAdapter 测试
//!
//! 测试交易适配器的数据解析功能

use sol_trade_sdk::parser::transaction_adapter::TransactionAdapter;
use solana_sdk::pubkey::Pubkey;

#[test]
fn test_transaction_adapter_creation() {
    // Step 1: 写测试 - 验证 TransactionAdapter 能创建基本结构

    // 创建一个空的适配器（使用占位符数据）
    let adapter = TransactionAdapter {
        signature: "test_signature".to_string(),
        slot: 12345,
        timestamp: 1234567890,
        account_keys: vec![],
        token_balance_changes: std::collections::HashMap::new(),
        spl_token_map: std::collections::HashMap::new(),
        spl_decimals_map: std::collections::HashMap::new(),
        instructions: vec![],
        inner_instructions: vec![],
    };

    // Step 2: 验证基本字段
    assert_eq!(adapter.signature, "test_signature");
    assert_eq!(adapter.slot, 12345);
    assert_eq!(adapter.timestamp, 1234567890);
    assert!(adapter.account_keys.is_empty());
    assert!(adapter.instructions.is_empty());
    assert!(adapter.inner_instructions.is_empty());
}

#[test]
fn test_instruction_info_structure() {
    // Step 1: 写测试 - 验证 InstructionInfo 结构

    use sol_trade_sdk::parser::transaction_adapter::InstructionInfo;

    let program_id = Pubkey::new_unique();
    let account1 = Pubkey::new_unique();
    let account2 = Pubkey::new_unique();

    let instruction = InstructionInfo {
        program_id,
        accounts: vec![account1, account2],
        data: vec![1, 2, 3, 4],
        index: 0,
    };

    // Step 2: 验证结构
    assert_eq!(instruction.program_id, program_id);
    assert_eq!(instruction.accounts.len(), 2);
    assert_eq!(instruction.accounts[0], account1);
    assert_eq!(instruction.accounts[1], account2);
    assert_eq!(instruction.data, vec![1, 2, 3, 4]);
    assert_eq!(instruction.index, 0);
}

