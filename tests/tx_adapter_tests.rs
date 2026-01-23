//! TransactionAdapter 集成测试 (Auto Mock 加速)
//!
//! 验证能从真实交易中正确提取 inner instructions 和 transferChecked
//!
//! 使用 Auto Mock 加速测试：
//! - 首次运行：从 RPC 获取并保存（约 1-2 秒）
//! - 后续运行：从缓存加载（约 0.01 秒）
//! - 速度提升：约 100-200 倍！

use solana_sdk::signature::Signature;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_transaction_status::UiTransactionEncoding;
use solana_commitment_config::CommitmentConfig;
use std::str::FromStr;
use sol_trade_sdk::{
    common::auto_mock_rpc::AutoMockRpcClient,
    parser::transaction_adapter::TransactionAdapter,
};

/// 测试：从 PumpSwap 买入交易中提取 transferChecked 指令 (Auto Mock 加速)
#[tokio::test]
async fn test_transaction_adapter_extract_transfer_checked() {
    println!("=== 测试：TransactionAdapter 提取 transferChecked 指令 (Auto Mock 加速) ===");

    let rpc_url = "http://127.0.0.1:8899";
    let signature_str = "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK";

    // 使用 Auto Mock RPC 客户端（使用独立命名空间）
    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("tx_adapter_tests".to_string())
    );

    let signature = Signature::from_str(signature_str)
        .expect("Failed to parse signature from string");

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let tx = auto_mock_client.get_transaction(&signature, config)
        .await
        .expect("Failed to get transaction from RPC");

    // 创建 TransactionAdapter
    let adapter = TransactionAdapter::from_encoded_transaction(&tx, tx.slot, tx.block_time)
        .expect("Failed to create TransactionAdapter from encoded transaction");

    // 验证基本信息
    assert_eq!(adapter.slot, 394648935);
    println!("✓ Slot 正确: {}", adapter.slot);
    println!("✓ 签名: {}", adapter.signature);

    // 验证账户密钥
    assert!(!adapter.account_keys.is_empty());
    println!("✓ 账户数量: {}", adapter.account_keys.len());
    println!("✓ 第一个账户: {}", adapter.account_keys[0]);

    // 验证内部指令
    assert!(!adapter.inner_instructions.is_empty());
    println!("✓ 内部指令数量: {}", adapter.inner_instructions.len());

    // 验证能找到 transferChecked 指令
    let transfer_checked = adapter.get_transfer_checked_instructions();
    assert!(!transfer_checked.is_empty());
    println!("✓ transferChecked 指令数量: {}", transfer_checked.len());

    // 打印找到的 transferChecked 指令
    for (i, ix) in transfer_checked.iter().enumerate() {
        println!("  [{}] 外部索引={}, 内部索引={}", i, ix.outer_index, ix.inner_index);
        if let Some(json) = &ix.instruction.parsed_json {
            println!("      程序ID: {}", json["programId"].as_str().unwrap_or(""));
            println!("      类型: {}", json["parsed"]["type"].as_str().unwrap_or(""));
            println!("      Token Program: {}", json["program"].as_str().unwrap_or(""));
        }
    }

    // 验证能找到 Token Program 的内部指令
    let token_program = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
    let token_instructions = adapter.get_inner_instructions_by_program(&token_program);
    assert!(!token_instructions.is_empty());
    println!("✓ Token Program 指令数量: {}", token_instructions.len());

    println!("✅ 测试通过");
    println!("💡 首次运行：从 RPC 获取并保存（约 1-2 秒）");
    println!("💡 后续运行：从缓存加载（约 0.01 秒）");
    println!("💡 速度提升：约 100-200 倍！");
}

/// 测试：提取所有转账类型的内部指令 (Auto Mock 加速)
#[tokio::test]
async fn test_transaction_adapter_extract_all_transfers() {
    println!("=== 测试：TransactionAdapter 提取所有转账类型指令 (Auto Mock 加速) ===");

    let rpc_url = "http://127.0.0.1:8899";
    let signature_str = "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK";

    // 使用 Auto Mock RPC 客户端（使用独立命名空间）
    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("tx_adapter_tests".to_string())
    );

    let signature = Signature::from_str(signature_str)
        .expect("Failed to parse signature from string");

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let tx = auto_mock_client.get_transaction(&signature, config)
        .await
        .expect("Failed to get transaction from RPC");

    let adapter = TransactionAdapter::from_encoded_transaction(&tx, tx.slot, tx.block_time).unwrap();

    // 获取所有转账类型的内部指令
    let all_transfers = adapter.get_all_transfer_instructions();
    println!("✓ 所有转账类型指令数量: {}", all_transfers.len());

    // 打印每个转账指令的详情
    for (i, ix) in all_transfers.iter().enumerate() {
        if let Some(json) = &ix.instruction.parsed_json {
            let instruction_type = json["parsed"]["type"].as_str().unwrap_or("unknown");
            let program = json["program"].as_str().unwrap_or("");
            println!("  [{}] 程序={}, 类型={}", i, program, instruction_type);

            // 如果是 transferChecked，打印详细信息
            if instruction_type == "transferChecked" {
                if let Some(info) = json["parsed"]["info"].as_object() {
                    println!("      mint: {:?}", info.get("mint"));
                    println!("      source: {:?}", info.get("source"));
                    println!("      destination: {:?}", info.get("destination"));
                    println!("      tokenAmount: {:?}", info.get("tokenAmount"));
                }
            }
        }
    }

    // 应该至少有一个 transferChecked
    assert!(all_transfers.iter().any(|ix| {
        if let Some(json) = &ix.instruction.parsed_json {
            json["parsed"]["type"].as_str() == Some("transferChecked")
        } else {
            false
        }
    }), "应该找到至少一个 transferChecked 指令");

    println!("✅ 测试通过");
    println!("💡 首次运行：从 RPC 获取并保存（约 1-2 秒）");
    println!("💡 后续运行：从缓存加载（约 0.01 秒）");
}

/// 测试：提取代币余额变化 (Auto Mock 加速)
#[tokio::test]
async fn test_transaction_adapter_token_balances() {
    println!("=== 测试：TransactionAdapter 提取代币余额变化 (Auto Mock 加速) ===");

    let rpc_url = "http://127.0.0.1:8899";
    let signature_str = "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK";

    // 使用 Auto Mock RPC 客户端（使用独立命名空间）
    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("tx_adapter_tests".to_string())
    );

    let signature = Signature::from_str(signature_str)
        .expect("Failed to parse signature from string");

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let tx = auto_mock_client.get_transaction(&signature, config)
        .await
        .expect("Failed to get transaction from RPC");

    let adapter = TransactionAdapter::from_encoded_transaction(&tx, tx.slot, tx.block_time).unwrap();

    // 验证代币余额变化
    println!("✓ 代币余额变化数量: {}", adapter.token_balance_changes.len());

    // 打印有余额变化的账户
    for (account, (pre, post)) in &adapter.token_balance_changes {
        println!("账户 {}", account);
        if let Some(pre) = pre {
            println!("  Pre: {} ({:?})", pre.amount, pre.ui_amount);
        }
        if let Some(post) = post {
            println!("  Post: {} ({:?})", post.amount, post.ui_amount);
        }
    }

    // 验证 spl_token_map
    println!("✓ Token Account -> Mint 映射数量: {}", adapter.spl_token_map.len());

    // 验证 spl_decimals_map
    println!("✓ Mint -> Decimals 映射数量: {}", adapter.spl_decimals_map.len());

    // 应该有一些代币余额变化
    assert!(!adapter.token_balance_changes.is_empty() || !adapter.spl_token_map.is_empty());

    println!("✅ 测试通过");
    println!("💡 首次运行：从 RPC 获取并保存（约 1-2 秒）");
    println!("💡 后续运行：从缓存加载（约 0.01 秒）");
}

/// 测试：提取转账动作 (Auto Mock 加速)
#[tokio::test]
async fn test_transaction_adapter_get_transfer_actions() {
    println!("=== 测试：TransactionAdapter 提取转账动作 (Auto Mock 加速) ===");

    let rpc_url = "http://127.0.0.1:8899";
    let signature_str = "5GCZ3TR31aDRP9LZxznKPBux86jWDyCxt1noCAAhX43d6Cmtqi8HvK6oHErq7DBr9j5KRcqeYumW2wHt5qJG1tQK";

    // 使用 Auto Mock RPC 客户端（使用独立命名空间）
    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("tx_adapter_tests".to_string())
    );

    let signature = Signature::from_str(signature_str)
        .expect("Failed to parse signature from string");

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let tx = auto_mock_client.get_transaction(&signature, config)
        .await
        .expect("Failed to get transaction from RPC");

    let adapter = TransactionAdapter::from_encoded_transaction(&tx, tx.slot, tx.block_time).unwrap();

    // 获取所有转账动作
    let transfers = adapter.get_transfer_actions();
    println!("✓ 转账动作数量: {}", transfers.len());

    // 打印每个转账的详情
    for (i, transfer) in transfers.iter().enumerate() {
        println!("\n[{}] 转账详情:", i);
        println!("  类型: {}", transfer.transfer_type);
        println!("  程序ID: {}", transfer.program_id);
        println!("  Mint: {}", transfer.mint);
        println!("  Source: {}", transfer.source);
        println!("  Destination: {}", transfer.destination);
        println!("  数量: {} ({} decimals)", transfer.token_amount.ui_amount, transfer.token_amount.decimals);
        println!("  原始数量: {}", transfer.token_amount.amount);
        if let Some(auth) = &transfer.authority {
            println!("  Authority: {}", auth);
        }
        if let Some(pre) = &transfer.source_pre_balance {
            println!("  Source Pre: {} ({:?})", pre.amount, pre.ui_amount);
        }
        if let Some(post) = &transfer.source_balance {
            println!("  Source Post: {} ({:?})", post.amount, post.ui_amount);
        }
        if let Some(pre) = &transfer.destination_pre_balance {
            println!("  Destination Pre: {} ({:?})", pre.amount, pre.ui_amount);
        }
        if let Some(post) = &transfer.destination_balance {
            println!("  Destination Post: {} ({:?})", post.amount, post.ui_amount);
        }
    }

    // 应该至少有 3 个转账
    assert!(transfers.len() >= 3, "应该找到至少 3 个转账，实际找到 {}", transfers.len());

    // 验证第一个转账的详细信息
    let first_transfer = &transfers[0];
    assert_eq!(first_transfer.transfer_type, "transferChecked");
    assert_eq!(first_transfer.token_amount.decimals, 9);
    assert!(first_transfer.token_amount.ui_amount > 0.0);

    println!("✅ 测试通过");
    println!("💡 首次运行：从 RPC 获取并保存（约 1-2 秒）");
    println!("💡 后续运行：从缓存加载（约 0.01 秒）");
    println!("💡 速度提升：约 100-200 倍！");
}
