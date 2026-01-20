//! 详细探索 Raydium V4 交易解析过程

use sol_trade_sdk::parser::transaction_adapter::TransactionAdapter;
use sol_trade_sdk::parser::types::DexProtocol;
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_transaction_status::{UiTransactionEncoding};
use solana_commitment_config::CommitmentConfig;
use std::str::FromStr;

#[test]
fn test_explore_raydium_v4_detailed() {
    let rpc_url = "http://127.0.0.1:8899";
    let signature_str = "5tqpXeLDzBKXdWUrTXb5pApjhapj6PLZZLvcLFBsYUdGgtnW9MYTC7N16gF4GyVZHQgGZKApNRP3bAUckr7MdpJr";

    let rpc_client = RpcClient::new(rpc_url);
    let signature = Signature::from_str(signature_str).unwrap();

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let tx = rpc_client.get_transaction_with_config(&signature, config).unwrap();
    let slot = tx.slot;
    let block_time = tx.block_time;

    // 创建适配器
    let adapter = TransactionAdapter::from_encoded_transaction(&tx, slot, block_time).unwrap();

    println!("=== 基本信息 ===");
    println!("签名: {}", adapter.signature);
    println!("Slot: {}", adapter.slot);
    println!("账户数量: {}", adapter.account_keys.len());

    println!("\n=== 外部指令 ===");
    for (i, instr) in adapter.instructions.iter().enumerate() {
        println!("指令 {}: {:?}", i, instr.program_id);
        println!("  数据长度: {}", instr.data.len());
        if !instr.data.is_empty() {
            println!("  第一个字节: {} (0x{:02x})", instr.data[0], instr.data[0]);
        }
        println!("  账户数量: {}", instr.accounts.len());
    }

    let raydium_v4_program_id = DexProtocol::RaydiumV4.program_id().parse().unwrap();

    println!("\n=== Raydium V4 指令 ===");
    let v4_instructions = adapter.get_instructions_by_program(&raydium_v4_program_id);
    println!("找到 {} 个 Raydium V4 指令", v4_instructions.len());
    for instr in v4_instructions {
        println!("  指令索引: {}", instr.index);
        println!("  第一个字节: {} (应该是 9)", instr.data.get(0).unwrap_or(&0));
    }

    println!("\n=== Inner Instructions ===");
    println!("总共 {} 个 inner instructions", adapter.inner_instructions.len());
    for ix in &adapter.inner_instructions {
        println!("  [{}-{}] 程序: {:?}", ix.outer_index, ix.inner_index, ix.instruction.program_id);
        if let Some(json) = &ix.instruction.parsed_json {
            if let Some(type_str) = json["parsed"]["type"].as_str() {
                println!("    类型: {}", type_str);
            }
        }
    }

    println!("\n=== Transfer Instructions ===");
    let transfer_instructions = adapter.get_all_transfer_instructions();
    println!("找到 {} 个 transfer 指令", transfer_instructions.len());
    for ix in &transfer_instructions {
        println!("  [{}-{}] 程序: {:?}", ix.outer_index, ix.inner_index, ix.instruction.program_id);
        if let Some(json) = &ix.instruction.parsed_json {
            if let Some(type_str) = json["parsed"]["type"].as_str() {
                println!("    类型: {}", type_str);
            }
        }
    }

    println!("\n=== Transfer Actions ===");
    let transfer_actions = adapter.get_transfer_actions();
    println!("找到 {} 个 transfer actions", transfer_actions.len());
    for (i, action) in transfer_actions.iter().enumerate() {
        println!("  Transfer {}:", i);
        println!("    类型: {}", action.transfer_type);
        println!("    Mint: {}", action.mint);
        println!("    Source: {}", action.source);
        println!("    Destination: {}", action.destination);
        println!("    Authority: {:?}", action.authority);
        println!("    Amount: {}", action.token_amount.ui_amount);
        println!("    Outer Index: {}", action.outer_index);
    }

    // 打印用户地址
    println!("\n=== 用户地址 ===");
    // accounts[14] 是用户钱包
    if adapter.account_keys.len() > 14 {
        println!("用户钱包 (accounts[14]): {}", adapter.account_keys[14]);
    }

    println!("\n=== 外部指令 0 的 Transfer ===");
    let transfers_for_instr_0 = adapter.get_transfers_for_instruction(0);
    println!("找到 {} 个 transfer", transfers_for_instr_0.len());

    // 验证：Raydium V4 的 SWAP discriminator 是 9
    println!("\n=== 验证 ===");
    let swap_discriminator = 9u8;
    println!("Raydium V4 SWAP discriminator: {}", swap_discriminator);
}
