//! 调试 CLMM 交易 V2

use sol_trade_sdk::parser::DexParser;

#[tokio::test]
async fn debug_clmm_transaction_v2() {
    let _parser = DexParser::default();
    let signature = "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU";

    // 手动获取交易
    use solana_rpc_client::rpc_client::RpcClient;
    use std::sync::Arc;
    use solana_client::rpc_config::RpcTransactionConfig;
    use solana_sdk::signature::Signature;
    use solana_transaction_status::{UiTransactionEncoding, EncodedConfirmedTransactionWithStatusMeta};
    use std::str::FromStr;

    let rpc_client = Arc::new(RpcClient::new("http://127.0.0.1:8899".to_string()));
    let sig = Signature::from_str(signature).unwrap();

    let tx: EncodedConfirmedTransactionWithStatusMeta = tokio::task::spawn_blocking(move || {
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            commitment: Some(solana_commitment_config::CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        rpc_client.get_transaction_with_config(&sig, config).unwrap()
    })
    .await
    .unwrap();

    let adapter = sol_trade_sdk::parser::transaction_adapter::TransactionAdapter::from_encoded_transaction(
        &tx, tx.slot, tx.block_time
    ).unwrap();

    // 查看 CLMM 程序的所有指令
    let clmm_program_id = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"
        .parse()
        .unwrap();

    println!("=== CLMM Program Instructions (Outer) ===");
    let instructions = adapter.get_instructions_by_program(&clmm_program_id);
    println!("Found {} CLMM outer instructions", instructions.len());

    // 查看内部指令
    println!("\n=== CLMM Inner Instructions ===");
    let inner_instructions = adapter.get_inner_instructions_by_program(&clmm_program_id);
    println!("Found {} CLMM inner instructions", inner_instructions.len());

    for (idx, inner_ix) in inner_instructions.iter().enumerate() {
        println!("\nInner Instruction {}:", idx);
        println!("  Outer index: {}", inner_ix.outer_index);
        println!("  Inner index: {}", inner_ix.inner_index);
        println!("  Data length: {}", inner_ix.instruction.data.len());
        if inner_ix.instruction.data.len() >= 8 {
            println!("  Discriminator (hex): {:02x?}", &inner_ix.instruction.data[0..8]);
        }
        println!("  Accounts: {}", inner_ix.instruction.accounts.len());
        println!("  Accounts[0]: {}", inner_ix.instruction.accounts.get(0).map(|a| a.to_string()).unwrap_or_default());
        println!("  Accounts[1]: {}", inner_ix.instruction.accounts.get(1).map(|a| a.to_string()).unwrap_or_default());
        println!("  Accounts[2]: {}", inner_ix.instruction.accounts.get(2).map(|a| a.to_string()).unwrap_or_default());
    }

    // 查看 Transfer 记录
    println!("\n=== Transfer Records ===");
    let transfers = adapter.get_transfer_actions();
    println!("Found {} transfer records", transfers.len());

    for (idx, transfer) in transfers.iter().enumerate() {
        println!("\nTransfer {}:", idx);
        println!("  Mint: {}", transfer.mint);
        println!("  Source: {}", transfer.source);
        println!("  Destination: {}", transfer.destination);
        println!("  Amount: {}", transfer.token_amount.ui_amount);
        println!("  Outer Index: {}", transfer.outer_index);
        println!("  Inner Index: {}", transfer.inner_index);
    }

    // 查看特定 outer index 的 transfer
    println!("\n=== Transfers for Outer Index 5 ===");
    let transfers_for_5 = adapter.get_transfers_for_instruction(5);
    println!("Found {} transfers for outer index 5", transfers_for_5.len());

    panic!("Debug complete");
}
