//! PumpSwap TDD 测试
//!
//! 测试目标：验证能从真实交易中解析买入/卖出详情

use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_transaction_status::UiTransactionEncoding;
use solana_commitment_config::CommitmentConfig;
use std::str::FromStr;
use sol_trade_sdk::parser::{
    transaction_adapter::TransactionAdapter,
    pumpswap::PumpswapParser,
    base_parser::DexParserTrait,
    types::TradeType,
};

/// 测试：从 PumpSwap 买入交易中提取交易详情
#[test]
fn test_pumpswap_extract_buy_details() {
    // 使用之前测试的 PumpSwap 买入交易
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

    // 创建 TransactionAdapter
    let adapter = TransactionAdapter::from_encoded_transaction(&tx, tx.slot, tx.block_time).unwrap();

    println!("✓ 交易签名: {}", adapter.signature);
    println!("✓ Slot: {}", adapter.slot);
    println!("✓ 内部指令数量: {}", adapter.inner_instructions.len());

    // 检查 PumpSwap 指令的数据
    let pumpswap_program_id = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"
        .parse()
        .unwrap();

    let pumpswap_instructions = adapter.get_instructions_by_program(&pumpswap_program_id);
    println!("✓ 找到 {} 条外部 PumpSwap 指令", pumpswap_instructions.len());

    for (i, ix) in pumpswap_instructions.iter().enumerate() {
        println!("  [{}] 数据长度: {} bytes", i, ix.data.len());
        if ix.data.len() >= 16 {
            println!("      Discriminator (前8字节): {:?}", &ix.data[0..8]);
        }
    }

    // 也检查内部指令
    let pumpswap_inner_instructions = adapter.get_inner_instructions_by_program(&pumpswap_program_id);
    println!("✓ 找到 {} 条内部 PumpSwap 指令", pumpswap_inner_instructions.len());

    for (i, ix) in pumpswap_inner_instructions.iter().enumerate() {
        println!("  [{}] 外部={}, 内部={}, 数据长度: {} bytes",
            i, ix.outer_index, ix.inner_index, ix.instruction.data.len());
        if ix.instruction.data.len() >= 16 {
            println!("      完整 Discriminator (16字节): {:?}", &ix.instruction.data[0..16]);
        }
    }

    // 创建 PumpSwap 解析器
    let parser = PumpswapParser::new();

    // 使用 tokio runtime 运行异步解析
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(parser.parse(&adapter));

    match result {
        Ok(trades) => {
            println!("✓ 解析成功，找到 {} 笔交易", trades.len());

            for (i, trade) in trades.iter().enumerate() {
                println!("\n[{}] 交易详情:", i);
                println!("  用户: {}", trade.user);
                println!("  交易类型: {:?}", trade.trade_type);
                println!("  Pool: {}", trade.pool);
                println!("  DEX: {}", trade.dex);
                println!("  输入代币:");
                println!("    Mint: {}", trade.input_token.mint);
                println!("    数量: {} ({})", trade.input_token.amount, trade.input_token.amount_raw);
                println!("    精度: {}", trade.input_token.decimals);
                println!("  输出代币:");
                println!("    Mint: {}", trade.output_token.mint);
                println!("    数量: {} ({})", trade.output_token.amount, trade.output_token.amount_raw);
                println!("    精度: {}", trade.output_token.decimals);

                if let Some(fee) = &trade.fee {
                    println!("  手续费:");
                    println!("    Mint: {}", fee.mint);
                    println!("    数量: {} ({})", fee.amount, fee.amount_raw);
                }
            }

            // 验证解析结果
            assert!(!trades.is_empty(), "应该至少解析出一笔交易");

            let trade = &trades[0];

            // 验证基本信息
            assert_eq!(trade.trade_type, TradeType::Buy);
            assert_eq!(trade.dex, "PumpSwap");
            assert!(trade.input_token.amount > 0.0);
            assert!(trade.output_token.amount > 0.0);

            // 验证代币信息
            assert!(!trade.input_token.amount_raw.is_empty());
            assert!(!trade.output_token.amount_raw.is_empty());

            println!("\n✓ 所有验证通过！");
        }
        Err(e) => {
            panic!("解析失败: {:?}", e);
        }
    }
}

/// 测试：验证能找到 PumpSwap 程序的指令
#[test]
fn test_pumpswap_find_instructions() {
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
    let adapter = TransactionAdapter::from_encoded_transaction(&tx, tx.slot, tx.block_time).unwrap();

    // 先打印所有指令的程序ID
    println!("所有外部指令:");
    for (i, ix) in adapter.instructions.iter().enumerate() {
        println!("  [{}] 程序ID: {}", i, ix.program_id);
    }

    println!("\n所有内部指令:");
    for (i, ix) in adapter.inner_instructions.iter().enumerate() {
        println!("  [{}] 外部={}, 内部={}, 程序ID: {}",
            i, ix.outer_index, ix.inner_index, ix.instruction.program_id);
    }

    // 获取 PumpSwap 程序ID
    let pumpswap_program_id = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"
        .parse()
        .unwrap();

    // 查找 PumpSwap 程序的指令
    let instructions = adapter.get_instructions_by_program(&pumpswap_program_id);

    println!("✓ 找到 {} 条 PumpSwap 指令", instructions.len());

    for (i, ix) in instructions.iter().enumerate() {
        println!("  [{}] 程序ID: {}", i, ix.program_id);
        println!("      数据长度: {} bytes", ix.data.len());
        println!("      账户数量: {}", ix.accounts.len());

        // 尝试解析事件
        if let Some((event_type, _)) = sol_trade_sdk::parser::pumpswap::events::parse_pumpswap_event(&ix.data) {
            println!("      事件类型: {:?}", event_type);
        }
    }

    // 应该至少有一条 PumpSwap 指令
    assert!(!instructions.is_empty(), "应该找到至少一条 PumpSwap 指令");
}
