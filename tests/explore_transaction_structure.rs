//! 探索 Solana 3.0 交易数据结构
//!
//! 用于理解如何从 EncodedConfirmedTransactionWithStatus 中提取数据

use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_transaction_status::{UiTransactionEncoding};
use solana_commitment_config::CommitmentConfig;
use std::str::FromStr;

#[test]
fn test_explore_transaction_structure() {
    // Step 1: 写测试 - 探索交易数据结构

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

    // 打印交易结构
    println!("Transaction slot: {}", tx.slot);
    println!("Transaction block_time: {:?}", tx.block_time);

    // 直接打印 transaction 的 debug 信息来了解类型
    println!("Transaction type: {:?}", std::any::type_name_of_val(&tx.transaction));

    // 尝试访问 meta
    if let Some(meta) = &tx.transaction.meta {
        println!("Meta exists");
        // inner_instructions 是 OptionSerializer 类型，需要特殊处理

        // 尝试序列化来访问 inner_instructions
        if let Ok(inner_instr) = serde_json::to_value(&meta.inner_instructions) {
            println!("Inner instructions JSON: {}", inner_instr);
        }
    }

    // 尝试访问 transaction
    println!("Transaction type: {:?}", std::any::type_name_of_val(&tx.transaction.transaction));
}
