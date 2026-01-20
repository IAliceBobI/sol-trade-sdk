//! 调试 Raydium V4 交易数据
//!
//! 用于理解交易结构和 transfer 记录

use sol_trade_sdk::parser::DexParser;
use sol_trade_sdk::parser::transaction_adapter::TransactionAdapter;
use sol_trade_sdk::parser::types::DexProtocol;
use solana_rpc_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::UiTransactionEncoding;
use solana_commitment_config::CommitmentConfig;
use std::str::FromStr;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn debug_raydium_v4_transaction() {
    let rpc_url = "http://127.0.0.1:8899";
    let signature_str = "5tqpXeLDzBKXdWUrTXb5pApjhapj6PLZZLvcLFBsYUdGgtnW9MYTC7N16gF4GyVZHQgGZKApNRP3bAUckr7MdpJr";

    // 获取交易
    let client = RpcClient::new(rpc_url);
    let signature = Signature::from_str(signature_str).unwrap();

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Json),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let tx = client.get_transaction_with_config(&signature,config).unwrap();

    // 先打印原始JSON结构看看
    let tx_json = serde_json::to_string_pretty(&tx).unwrap();
    println!("=== 原始交易JSON (前2000行) ===");
    for (i, line) in tx_json.lines().take(2000).enumerate() {
        println!("{:04}: {}", i, line);
    }

    // 创建适配器
    let adapter = TransactionAdapter::from_encoded_transaction(
        &tx,
        tx.slot,
        tx.block_time,
    ).unwrap();

    println!("\n\n=== 交易基本信息 ===");
    println!("签名: {}", adapter.signature);
    println!("Slot: {}", adapter.slot);
    println!("时间戳: {}", adapter.timestamp);

    println!("\n=== 账户键 ===");
    println!("账户键数量: {}", adapter.account_keys.len());
    for (i, key) in adapter.account_keys.iter().enumerate() {
        println!("  [{}]: {}", i, key);
    }

    println!("\n=== 所有指令 ===");
    for (i, instr) in adapter.instructions.iter().enumerate() {
        println!("指令 {}: program={}", i, instr.program_id);
        println!("  账户数: {}", instr.accounts.len());
        println!("  数据长度: {}", instr.data.len());
        if !instr.data.is_empty() {
            println!("  数据第一字节: 0x{:02x}", instr.data[0]);
        }

        // 打印所有账户
        if instr.accounts.len() <= 20 {
            for (j, acc) in instr.accounts.iter().enumerate() {
                println!("    账户[{}]: {}", j, acc);
            }
        }
    }

    println!("\n=== 内部指令总数 ===");
    println!("内部指令数量: {}", adapter.inner_instructions.len());
    for (i, inner) in adapter.inner_instructions.iter().enumerate() {
        println!("内部指令[{}]: 外部={}, 内部={}, program={}",
            i, inner.outer_index, inner.inner_index, inner.instruction.program_id);

        // 打印 parsed_json
        if let Some(ref json) = inner.instruction.parsed_json {
            let json_str = serde_json::to_string_pretty(json).unwrap();
            println!("  JSON (前50行):");
            for (line_num, line) in json_str.lines().take(50).enumerate() {
                println!("    {}: {}", line_num, line);
            }
        }
    }

    // 找到 Raydium V4 的指令
    let program_id = DexProtocol::RaydiumV4.program_id().parse::<solana_sdk::pubkey::Pubkey>().unwrap();
    let raydium_instrs = adapter.get_instructions_by_program(&program_id);

    println!("\n=== Raydium V4 指令 ===");
    for instr in &raydium_instrs {
        println!("索引: {}", instr.index);
        println!("账户数: {}", instr.accounts.len());

        // 打印前几个账户
        for (i, acc) in instr.accounts.iter().take(20).enumerate() {
            println!("  账户[{}]: {}", i, acc);
        }

        // 获取这个指令的 transfer
        println!("\n该指令的 Transfer 记录:");
        let transfers = adapter.get_transfers_for_instruction(instr.index);

        println!("  Transfer 数量: {}", transfers.len());

        for (ti, transfer) in transfers.iter().enumerate() {
            println!("  Transfer[{}]:", ti);
            println!("    类型: {}", transfer.transfer_type);
            println!("    Mint: {}", transfer.mint);
            println!("    源: {}", transfer.source);
            println!("    目标: {}", transfer.destination);
            println!("    数量: {} (decimals={})", transfer.token_amount.amount, transfer.token_amount.decimals);
            println!("    Authority: {:?}", transfer.authority);

            // 打印余额变化
            if let Some(ref pre) = transfer.source_pre_balance {
                println!("    源前置余额: {}", pre.ui_amount_string);
            }
            if let Some(ref post) = transfer.source_balance {
                println!("    源后置余额: {}", post.ui_amount_string);
            }
            if let Some(ref pre) = transfer.destination_pre_balance {
                println!("    目标前置余额: {}", pre.ui_amount_string);
            }
            if let Some(ref post) = transfer.destination_balance {
                println!("    目标后置余额: {}", post.ui_amount_string);
            }
        }
    }

    // 尝试解析
    println!("\n=== 尝试解析 ===");
    let parser = DexParser::default();
    let result = parser.parse_transaction(signature_str).await;

    println!("解析结果:");
    println!("成功标志: {}", result.success);
    println!("交易数量: {}", result.trades.len());
    if let Some(err) = result.error {
        println!("错误: {}", err);
    }
}
