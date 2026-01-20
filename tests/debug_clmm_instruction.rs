//! 调试 CLMM 交易指令数据

use std::str::FromStr;

#[tokio::test]
async fn debug_clmm_transaction() {
    let signature = "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU";

    // 我们需要手动获取交易来查看指令数据
    use solana_rpc_client::rpc_client::RpcClient;
    use std::sync::Arc;
    use solana_client::rpc_config::RpcTransactionConfig;
    use solana_sdk::signature::Signature;
    use solana_transaction_status::{UiTransactionEncoding, EncodedConfirmedTransactionWithStatusMeta};

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

    // 打印交易信息的 JSON
    let tx_value = serde_json::to_value(&tx).unwrap();

    println!("=== CLMM Transaction Debug ===");
    println!("Signature: {}", signature);
    println!("\n=== Instructions ===");

    if let Some(instructions) = tx_value["transaction"]["message"]["instructions"].as_array() {
        for (idx, ix) in instructions.iter().enumerate() {
            if let Some(program_id_index) = ix["programIdIndex"].as_u64() {
                println!("\nInstruction {}", idx);
                println!("  Program ID Index: {}", program_id_index);

                if let Some(accounts) = ix["accounts"].as_array() {
                    println!("  Accounts: {} accounts", accounts.len());
                }

                if let Some(data) = ix["data"].as_str() {
                    println!("  Data (base58): {}", data);

                    // 解码 data
                    if let Ok(decoded) = bs58::decode(data).into_vec() {
                        println!("  Data (hex): {:02X?}", decoded);

                        // 打印前 8 字节作为 discriminator
                        if decoded.len() >= 8 {
                            println!("  Discriminator (first 8 bytes): {:02X?}", &decoded[0..8]);
                        }
                    }
                }

                if let Some(parsed) = ix.get("parsed") {
                    println!("  Parsed: {}", parsed);
                }
            }
        }
    }

    // 查找 CLMM program ID
    let clmm_program_id = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
    println!("\n=== Looking for CLMM Program: {} ===", clmm_program_id);

    if let Some(account_keys) = tx_value["transaction"]["message"]["accountKeys"].as_array() {
        for (idx, key) in account_keys.iter().enumerate() {
            if let Some(key_str) = key.as_str() {
                if key_str == clmm_program_id {
                    println!("Found CLMM program at index {}", idx);
                }
            }
        }
    }

    panic!("Debug complete");
}
