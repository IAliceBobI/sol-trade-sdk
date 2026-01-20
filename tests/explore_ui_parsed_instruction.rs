//! 探索 Solana 3.0 UiParsedInstruction 结构
//!
//! 用于理解如何访问 ParsedInstruction 的 type 字段

use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_transaction_status::UiTransactionEncoding;
use solana_commitment_config::CommitmentConfig;
use std::str::FromStr;

#[test]
fn test_explore_ui_parsed_instruction_structure() {
    let rpc_url = "http://127.0.0.1:8899";
    let signature_str = "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK";

    let rpc_client = RpcClient::new(rpc_url);
    let signature = Signature::from_str(signature_str).unwrap();

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let tx = rpc_client.get_transaction_with_config(&signature, config).unwrap();

    // 打印类型
    println!("Transaction type: {:?}", std::any::type_name_of_val(&tx));
    println!("Transaction.transaction type: {:?}", std::any::type_name_of_val(&tx.transaction));

    // 尝试访问 inner_instructions
    if let Some(meta) = &tx.transaction.meta {
        if let Ok(inner_json) = serde_json::to_value(&meta.inner_instructions) {
            println!("inner_instructions JSON: {}", inner_json);

            // 尝试解析
            if let Ok(inner_vec) = serde_json::from_value::<Vec<serde_json::Value>>(inner_json) {
                for inner_set in inner_vec {
                    let index = inner_set["index"].as_u64().unwrap();
                    println!("Inner instruction set index: {}", index);

                    if let Some(instructions) = inner_set["instructions"].as_array() {
                        for ix_json in instructions {
                            println!("  Instruction: {}", ix_json);

                            // 检查是否有 parsed 字段
                            if let Some(parsed) = ix_json.get("parsed") {
                                println!("    Parsed: {}", parsed);
                                if let Some(type_val) = parsed.get("type") {
                                    println!("    Type: {}", type_val);
                                }
                            }

                            // 直接打印 JSON 结构查看
                            if let Some(program) = ix_json.get("program") {
                                println!("    Program: {}", program);
                            }
                            if let Some(parsed) = ix_json.get("parsed") {
                                if let Some(type_val) = parsed.get("type") {
                                    println!("    Type: {}", type_val);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
