//! 调试 CPMM 交易

use sol_trade_sdk::parser::DexParser;

#[tokio::test]
async fn debug_cpmm_transaction() {
    let _parser = DexParser::default();
    let signature = "7Q5gThWgQkbSR6GSLVSAjo9x762DSuLQwg6ne6KKomjfWSho26Zmr7qfPQ7zzJk7sdTvHPqhW9grxaNzGhJgRrn";

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

    // 查看 CPMM 程序的所有指令
    let cpmm_program_id = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"
        .parse()
        .unwrap();

    println!("=== CPMM Program Instructions ===");
    let instructions = adapter.get_instructions_by_program(&cpmm_program_id);
    println!("Found {} CPMM instructions", instructions.len());

    for (idx, instr) in instructions.iter().enumerate() {
        println!("\nInstruction {}:", idx);
        println!("  Data length: {}", instr.data.len());
        if instr.data.len() >= 8 {
            println!("  Discriminator (hex): {:02x?}", &instr.data[0..8]);
        }
        println!("  Accounts: {}", instr.accounts.len());
    }

    // 查看内部指令
    let inner_instructions = adapter.get_inner_instructions_by_program(&cpmm_program_id);
    println!("\n=== CPMM Inner Instructions ===");
    println!("Found {} CPMM inner instructions", inner_instructions.len());

    for (idx, inner_ix) in inner_instructions.iter().enumerate() {
        println!("\nInner Instruction {}:", idx);
        println!("  Outer index: {}", inner_ix.outer_index);
        println!("  Data length: {}", inner_ix.instruction.data.len());
        if inner_ix.instruction.data.len() >= 8 {
            println!("  Discriminator (hex): {:02x?}", &inner_ix.instruction.data[0..8]);
        }
        println!("  Accounts: {}", inner_ix.instruction.accounts.len());
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
    }

    panic!("Debug complete");
}
