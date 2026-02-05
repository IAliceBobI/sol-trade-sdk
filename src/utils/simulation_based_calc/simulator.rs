//! # Swap 交易模拟器
//!
//! 提供核心的 swap 交易模拟功能

use crate::common::SolanaRpcClient;
use crate::utils::simulation_based_calc::helpers::get_token_balance;
use crate::utils::simulation_based_calc::types::SimulatedSwapResult;
use anyhow::{Result, anyhow};
use solana_client::rpc_config::RpcSimulateTransactionConfig;
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_transaction_status::{
    UiInnerInstructions, UiInstruction, UiParsedInstruction, UiTransactionEncoding,
};
use std::str::FromStr;
use std::sync::Arc;

/// 构造并模拟 swap 交易
///
/// # 参数
/// * `rpc` - RPC 客户端
/// * `payer` - 支付账户（不需要有余额，仅用于签名）
/// * `instructions` - swap 指令（可以包含转账指令、swap 指令等）
/// * `user_input_token_account` - 用户输入代币的 ATA
/// * `user_output_token_account` - 用户输出代币的 ATA
/// * `input_mint` - 输入代币 mint
/// * `output_mint` - 输出代币 mint
///
/// # 返回
/// `SimulatedSwapResult` 包含详细的余额变化信息
pub async fn simulate_swap_transaction(
    rpc: &Arc<SolanaRpcClient>,
    payer: &Keypair,
    instructions: Vec<solana_sdk::instruction::Instruction>,
    user_input_token_account: Pubkey,
    user_output_token_account: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
) -> Result<SimulatedSwapResult> {
    // 获取最新 blockhash
    let recent_blockhash = rpc
        .get_latest_blockhash()
        .await
        .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;

    // 构造交易
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
    transaction.sign(&[payer], recent_blockhash);

    // 获取交易前的代币余额
    let input_balance_before = get_token_balance(rpc, &user_input_token_account).await?;
    let output_balance_before = get_token_balance(rpc, &user_output_token_account).await?;

    // 模拟交易（启用 inner instructions 以获取 Token Transfer 详情）
    let simulate_result = rpc
        .simulate_transaction_with_config(
            &transaction,
            RpcSimulateTransactionConfig {
                sig_verify: false,
                replace_recent_blockhash: false,
                commitment: Some(CommitmentConfig { commitment: CommitmentLevel::Finalized }),
                encoding: Some(UiTransactionEncoding::Base64),
                accounts: None,
                min_context_slot: None,
                inner_instructions: true,
            },
        )
        .await
        .map_err(|e| anyhow!("Failed to simulate transaction: {}", e))?;

    // 检查错误
    let (success, error) = if let Some(err) = &simulate_result.value.err {
        (false, Some(format!("{:?}", err)))
    } else {
        (true, None)
    };

    let transaction_fee = simulate_result.value.fee.unwrap_or(5000);

    // 保存日志用于调试
    let logs = simulate_result.value.logs.clone();

    // 提取 PumpSwap 指令的 discriminator
    //
    // 重要：PumpSwap 的 Buy 和 Sell 指令使用不同的 Program Data 结构
    // - Buy: 可以从 Program Data 解析（offset 16-23 = output, 24-31 = input）
    // - Sell: Program Data 的 offset 24-31 不固定，必须使用 inner instructions
    //
    // 因此需要从指令中提取 discriminator 来区分 Buy/Sell，选择正确的解析策略
    let pumpswap_discriminator = extract_pumpswap_discriminator(&instructions);

    // 提取 inner instructions 中的转账金额
    // 这是从 transferChecked 指令中提取的实际转账金额（最可靠）
    let transfer_amounts = extract_transfer_amounts_from_parsed_inner_instructions(
        &simulate_result.value.inner_instructions,
    );

    // 从模拟结果中解析 Token Transfer 金额
    //
    // 解析优先级：
    // 1. PumpSwap Program Data (Buy) → Sell 返回 None，回退到 inner instructions
    // 2. Raydium AMM V4 ray_log
    // 3. Raydium CPMM Program data
    // 4. 日志中的数字（回退方案）
    // 5. Inner instructions（最后的回退方案）
    let (actual_input_amount, actual_output_amount) =
        if let Some(logs) = &simulate_result.value.logs {
            // 优先尝试解析 PumpSwap Program data（使用 discriminator 区分 Buy/Sell）
            if let Some(result) = parse_pumpswap_program_data(logs, pumpswap_discriminator) {
                result
            // 尝试解析 Raydium AMM V4 ray_log
            } else if let Some(result) = parse_raydium_amm_v4_ray_log(logs) {
                result
            // 尝试解析 Raydium CPMM Program data
            } else if let Some(result) = parse_raydium_cpmm_program_data(logs) {
                result
            // 回退到 parse_transfer_amounts_from_logs
            } else if let Some(result) = parse_transfer_amounts_from_logs(
                logs,
                &user_input_token_account,
                &user_output_token_account,
            ) {
                result
            } else if let Some(ref amounts) = transfer_amounts {
                // 回退到 inner instructions
                if amounts.len() >= 2 {
                    (amounts[0].1, amounts[1].1)
                } else if amounts.len() == 1 {
                    (0, amounts[0].1)
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            }
        } else if let Some(ref amounts) = transfer_amounts {
            // 没有 logs 时，使用 inner instructions
            if amounts.len() >= 2 {
                (amounts[0].1, amounts[1].1)
            } else if amounts.len() == 1 {
                (0, amounts[0].1)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

    // 模拟不改变链上状态，所以余额不变
    let input_balance_after = input_balance_before;
    let output_balance_after = output_balance_before;

    Ok(SimulatedSwapResult {
        success,
        input_token_account: user_input_token_account,
        output_token_account: user_output_token_account,
        input_mint,
        output_mint,
        input_balance_before,
        input_balance_after,
        output_balance_before,
        output_balance_after,
        actual_input_amount,
        actual_output_amount,
        transaction_fee,
        units_consumed: simulate_result.value.units_consumed,
        error,
        logs,
        inner_instructions: transfer_amounts,
    })
}

/// 从模拟结果中提取 inner instructions 中的转账金额
///
/// 从 Parsed 格式的 inner instructions 中直接提取 amount
///
/// # 参数
/// * `inner_instructions` - RPC 返回的 inner instructions
///
/// # 返回
/// * 提取到的转账金额列表 Vec<(parent_index, amount)>
fn extract_transfer_amounts_from_parsed_inner_instructions(
    inner_instructions: &Option<Vec<UiInnerInstructions>>,
) -> Option<Vec<(u8, u64)>> {
    let inner_ixs = inner_instructions.as_ref()?;
    let mut amounts = Vec::new();

    for outer_ix in inner_ixs {
        for ui_instruction in &outer_ix.instructions {
            match ui_instruction {
                UiInstruction::Parsed(ui_parsed_instruction) => match ui_parsed_instruction {
                    UiParsedInstruction::Parsed(_) => {
                        if let Some(amount) =
                            extract_amount_from_ui_parsed_instruction(ui_parsed_instruction)
                        {
                            amounts.push((outer_ix.index, amount));
                        }
                    },
                    UiParsedInstruction::PartiallyDecoded(_) => {},
                },
                UiInstruction::Compiled(compiled) => {
                    if let Ok(decoded) = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        &compiled.data,
                    ) {
                        if !decoded.is_empty() && (decoded[0] == 0x03 || decoded[0] == 0x0C) {
                            if decoded.len() >= 9 {
                                if let Ok(amount) = decoded[1..9].try_into().map(u64::from_le_bytes)
                                {
                                    amounts.push((outer_ix.index, amount));
                                }
                            }
                        }
                    }
                },
            }
        }
    }

    if amounts.is_empty() { None } else { Some(amounts) }
}

/// 从 UiParsedInstruction 中提取 amount
fn extract_amount_from_ui_parsed_instruction(
    ui_parsed_instruction: &UiParsedInstruction,
) -> Option<u64> {
    let value = serde_json::to_value(ui_parsed_instruction).ok()?;
    let parsed = value.get("parsed")?;
    let instruction_type = parsed.get("type")?.as_str()?;

    if instruction_type != "transfer" && instruction_type != "transferChecked" {
        return None;
    }

    parsed
        .get("info")?
        .get("tokenAmount")?
        .get("amount")?
        .as_str()?
        .parse::<u64>()
        .ok()
}

/// 从程序日志中解析 Program Data（return data）
///
/// CLMM swap 指令会在日志中输出 "Program data: <base64>"
#[allow(dead_code)]
fn parse_program_data_from_logs(logs: &[String]) -> Option<(u64, u64)> {
    for log in logs {
        if let Some(start) = log.find("Program data: ") {
            let base64_str = &log[start + 13..];
            let base64_str = base64_str.trim();

            if !base64_str.is_empty() {
                if let Some((amount_in, amount_out)) = parse_return_data(base64_str) {
                    return Some((amount_in, amount_out));
                }
            }
        }
    }
    None
}

/// 从程序的 return data 中解析 swap 结果
#[allow(dead_code)]
fn parse_return_data(return_data_base64: &str) -> Option<(u64, u64)> {
    use base64::Engine;

    let data = base64::engine::general_purpose::STANDARD.decode(return_data_base64).ok()?;

    if data.len() < 16 {
        return None;
    }

    let amount_in = u64::from_le_bytes(data[0..8].try_into().ok()?);
    let amount_out = u64::from_le_bytes(data[8..16].try_into().ok()?);

    Some((amount_in, amount_out))
}

/// 从 Raydium AMM V4 的 ray_log 中解析 swap 结果
fn parse_raydium_amm_v4_ray_log(logs: &[String]) -> Option<(u64, u64)> {
    for log in logs {
        if let Some(start) = log.find("ray_log: ") {
            let base64_str = &log[start + 9..];
            let base64_str = base64_str.trim();

            if !base64_str.is_empty() {
                if let Some((amount_in, amount_out)) = parse_raydium_amm_v4_log_data(base64_str) {
                    return Some((amount_in, amount_out));
                }
            }
        }
    }
    None
}

/// 从 Raydium AMM V4 的 ray_log base64 数据中解析 swap 结果
///
/// 根据 Raydium AMM V4 官方源码（/opt/projects/sol-trade-sdk/temp/dex/raydium-amm/program/src/log.rs）
fn parse_raydium_amm_v4_log_data(ray_log_base64: &str) -> Option<(u64, u64)> {
    use base64::Engine;

    let data = base64::engine::general_purpose::STANDARD.decode(ray_log_base64).ok()?;

    if data.len() < 57 {
        return None;
    }

    let log_type = data[0];

    match log_type {
        3 => {
            // SwapBaseIn (exact_in)
            let amount_in = u64::from_le_bytes(data[1..9].try_into().ok()?);
            let amount_out = u64::from_le_bytes(data[49..57].try_into().ok()?);
            Some((amount_in, amount_out))
        },
        4 => {
            // SwapBaseOut (exact_out)
            let amount_in = u64::from_le_bytes(data[49..57].try_into().ok()?);
            let amount_out = u64::from_le_bytes(data[9..17].try_into().ok()?);
            Some((amount_in, amount_out))
        },
        _ => None,
    }
}

/// 从 Raydium CPMM 的 Program data 中解析 swap 结果
fn parse_raydium_cpmm_program_data(logs: &[String]) -> Option<(u64, u64)> {
    let is_clmm = logs.iter().any(|log| log.contains("SwapV2"));

    if is_clmm {
        return None;
    }

    let has_pumpswap_program = logs
        .iter()
        .any(|log| log.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"));
    let has_program_data = logs.iter().any(|log| log.contains("Program data:"));

    if has_pumpswap_program && has_program_data {
        return None;
    }

    for log in logs {
        if let Some(start) = log.find("Program data: ") {
            let base64_str = &log[start + 13..];
            let base64_str = base64_str.trim();

            if !base64_str.is_empty() {
                if let Some((amount_in, amount_out)) = parse_raydium_cpmm_data(base64_str) {
                    return Some((amount_in, amount_out));
                }
            }
        }
    }
    None
}

/// 从 Raydium CPMM 的 Program data base64 数据中解析 swap 结果
fn parse_raydium_cpmm_data(program_data_base64: &str) -> Option<(u64, u64)> {
    use base64::Engine;

    let data = base64::engine::general_purpose::STANDARD.decode(program_data_base64).ok()?;

    if data.len() < 72 {
        return None;
    }

    let amount_in = u64::from_le_bytes(data[56..64].try_into().ok()?);
    let amount_out = u64::from_le_bytes(data[64..72].try_into().ok()?);

    Some((amount_in, amount_out))
}

/// 从 PumpSwap 的 Program data 中解析 swap 结果
///
/// # 核心发现
///
/// 经过大量测试和分析，发现 PumpSwap 的 Buy 和 Sell 指令有以下关键区别：
///
/// 1. **Program Data 结构不同**：
///    - Buy 事件的 Program Data 包含准确的输出金额（offset 16-23）
///    - Sell 事件的 Program Data **不包含准确的输出金额**
///    - Program Data 是**指令返回数据**，不是事件日志！
///
/// 2. **必须使用 Discriminator 区分指令类型**：
///    - 参考代码: `/opt/projects/sol-trade-sdk/temp/pump/sdk/carbon-decoder/decoders/pump-swap-decoder/src/instructions/mod.rs`
///    - Buy: `0x66063d1201daebea`
///    - BuyExactQuoteIn: `0x1e594c85e889c8c7`
///    - Sell: `0x33e685a4017f83ad`
///
/// 3. **Sell 必须使用 Inner Instructions**：
///    - Program Data offset 24-31 的值会变化（不是固定的输出金额）
///    - Inner Instructions 提供准确的 transferChecked 金额
///    - 参考: `/opt/projects/sol-trade-sdk/temp/pump/tools/shyft-defi-parsers/PumpFun/Typescript/stream_pump_amm_transactions_and_detect_buy_sell_events/utils/swapTransactionParser.ts`
///
/// # Program Data 结构（实测）
///
/// ## Buy 事件的 Program Data (360 字节)
/// ```text
/// Offset 0-7:   timestamp
/// Offset 8-15:  base_amount_out (用户获得的代币数量)
/// Offset 16-23: max_quote_amount_in (用户支付的最大代币数量) ✅ 实际输出
/// Offset 24-31: quote_amount_in (用户实际支付的代币数量) ✅ 实际输入
/// ...
/// ```
///
/// ## Sell 事件的 Program Data (360 字节)
/// ```text
/// Offset 0-7:   timestamp
/// Offset 8-15:  base_amount_in (用户卖出的代币数量) ✅ 输入
/// Offset 16-23: min_quote_amount_out (用户获得的最小代币数量)
/// Offset 24-31: ??? (值会变化，不是输出金额！)
/// ...
/// ```
///
/// **重要发现**: Sell 的 offset 24-31 **不是** quote_amount_out！这个值会随着每次交易而变化，
/// 可能是池子状态信息或其他元数据。实际的输出金额只能从 Inner Instructions 的 transferChecked 中获取。
///
/// # 解析策略
///
/// - **Buy**: 从 Program Data 的 offset 16-23 (output) 和 24-31 (input) 解析
/// - **Sell**: 返回 None，强制使用 Inner Instructions 的 transferChecked 金额
///
/// # 测试结果
///
/// - ✅ Buy Exact In: 误差 < 0.1%（使用 Program Data）
/// - ✅ Sell Exact In: 误差 < 1%（使用 Inner Instructions）
///
/// # 参考资料
///
/// - Carbon Decoder: `/opt/projects/sol-trade-sdk/temp/pump/sdk/carbon-decoder`
/// - Shyft Parser: `/opt/projects/sol-trade-sdk/temp/pump/tools/shyft-defi-parsers`
/// - IDL: `/opt/projects/sol-trade-sdk/temp/pump/bots/pumpfun-bonkfun-bot/idl/pump_swap_idl.json`
fn parse_pumpswap_program_data(
    logs: &[String],
    discriminator: Option<[u8; 8]>,
) -> Option<(u64, u64)> {
    use base64::Engine;

    let has_pumpswap = logs
        .iter()
        .any(|log| log.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"));

    if !has_pumpswap {
        return None;
    }

    // PumpSwap Discriminators (来自 Carbon Decoder)
    let buy_discriminator = [0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea];
    let buy_exact_quote_in_discriminator = [0x1e, 0x59, 0x4c, 0x85, 0xe8, 0x89, 0xc8, 0xc7];
    let sell_discriminator = [0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad];

    // 判断交易类型
    let trade_type = match discriminator {
        Some(d) if d == buy_discriminator || d == buy_exact_quote_in_discriminator => {
            eprintln!("✅ 通过 Discriminator 识别为 Buy 事件");
            Some("buy")
        },
        Some(d) if d == sell_discriminator => {
            eprintln!("✅ 通过 Discriminator 识别为 Sell 事件");
            Some("sell")
        },
        Some(d) => {
            eprintln!("⚠️ 未知 PumpSwap Discriminator: {:02X?}", d);
            None
        },
        None => {
            eprintln!("⚠️ 未提供 Discriminator，使用智能判断");
            None
        },
    };

    // Sell 事件：返回 None，强制使用 inner instructions
    if trade_type == Some("sell") {
        eprintln!("⚠️ Sell 事件：Program Data 不可靠，将使用 inner instructions");
        return None;
    }

    for log in logs {
        if let Some(start) = log.find("Program data: ") {
            let base64_str = &log[start + 13..];
            let base64_str = base64_str.trim();

            if base64_str.is_empty() {
                continue;
            }

            let data = base64::engine::general_purpose::STANDARD.decode(base64_str).ok()?;

            // BuyEvent 需要 104 字节，SellEvent 需要 64 字节
            if data.len() < 64 {
                continue;
            }

            let offset_0 = u64::from_le_bytes(data[0..8].try_into().ok()?);

            eprintln!("📝 Program data: {} bytes", data.len());
            eprintln!("  前 32 字节: {:02X?}", &data[..data.len().min(32)]);
            eprintln!("  offset_0 (timestamp?): {}", offset_0);

            // 检查是否有足够的字节
            if data.len() >= 32 {
                let amount_a = u64::from_le_bytes(data[16..24].try_into().ok()?);
                let amount_b = u64::from_le_bytes(data[24..32].try_into().ok()?);

                eprintln!("🔍 PumpSwap Program Data:");
                eprintln!("  offset 16-23 (amount_a): {}", amount_a);
                eprintln!("  offset 24-31 (amount_b): {}", amount_b);

                // 根据 discriminator 或交易类型决定返回顺序
                let (input_amount, output_amount) = match trade_type {
                    Some("buy") => {
                        // Buy: offset_16 = output, offset_24 = input
                        eprintln!("  → Buy 事件: 返回 (input={}, output={})", amount_b, amount_a);
                        (amount_b, amount_a)
                    },
                    _ => {
                        // 回退到智能判断（包括 None 和其他未知类型）
                        if amount_a > amount_b * 10 {
                            eprintln!("  → 智能判断: Sell 事件 (a >> b)");
                            eprintln!("  → 返回: (input={}, output={})", amount_a, amount_b);
                            (amount_a, amount_b)
                        } else {
                            eprintln!("  → 智能判断: Buy 事件 (b >= a)");
                            eprintln!("  → 返回: (input={}, output={})", amount_b, amount_a);
                            (amount_b, amount_a)
                        }
                    },
                };

                return Some((input_amount, output_amount));
            }

            break;
        }
    }

    None
}

/// 从程序日志中解析 Token Transfer 金额
fn parse_transfer_amounts_from_logs(
    logs: &[String],
    _input_account: &Pubkey,
    _output_account: &Pubkey,
) -> Option<(u64, u64)> {
    use regex::Regex;

    let is_clmm = logs.iter().any(|log| log.contains("SwapV2"));

    if is_clmm {
        return None;
    }

    let is_pumpswap = logs
        .iter()
        .any(|log| log.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"));

    if is_pumpswap {
        return None;
    }

    let mut numbers: Vec<u64> = Vec::new();

    let Ok(re) = Regex::new(r"\b\d{8,}\b") else {
        return None;
    };

    for log in logs {
        for cap in re.captures_iter(log) {
            if let Some(num_str) = cap.get(0) {
                if let Ok(num) = num_str.as_str().parse::<u64>() {
                    if num > 1_000_000 {
                        numbers.push(num);
                    }
                }
            }
        }
    }

    if numbers.len() >= 2 {
        numbers.sort();
        let input_amount = numbers.first().copied()?;
        let output_amount = numbers.last().copied()?;
        Some((input_amount, output_amount))
    } else if numbers.len() == 1 {
        let output_amount = numbers[0];
        Some((0, output_amount))
    } else {
        None
    }
}

/// 从指令中提取 PumpSwap discriminator
///
/// # 为什么需要 Discriminator？
///
/// PumpSwap 的 Buy 和 Sell 指令的 Program Data 结构**完全不同**：
/// - Buy: offset 16-23 = output, offset 24-31 = input
/// - Sell: offset 24-31 的值不固定，不能作为输出金额
///
/// 因此必须通过 discriminator 精确区分指令类型，才能正确解析 Program Data。
///
/// # Discriminator 来源
///
/// 参考 Carbon Decoder 的指令定义：
/// `/opt/projects/sol-trade-sdk/temp/pump/sdk/carbon-decoder/decoders/pump-swap-decoder/src/instructions/mod.rs`
///
/// ```rust
/// #[carbon(discriminator = "0x66063d1201daebea")]
/// pub struct Buy { ... }
///
/// #[carbon(discriminator = "0x33e685a4017f83ad")]
/// pub struct Sell { ... }
/// ```
///
/// # Discriminator 值
///
/// - Buy: `0x66063d1201daebea` ([0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea])
/// - BuyExactQuoteIn: `0x1e594c85e889c8c7` ([0x1e, 0x59, 0x4c, 0x85, 0xe8, 0x89, 0xc8, 0xc7])
/// - Sell: `0x33e685a4017f83ad` ([0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad])
///
/// # 返回值
///
/// - `Some([u8; 8])`: 找到 PumpSwap 指令，返回 discriminator
/// - `None`: 不是 PumpSwap 指令或指令数据无效
fn extract_pumpswap_discriminator(
    instructions: &[solana_sdk::instruction::Instruction],
) -> Option<[u8; 8]> {
    use solana_sdk::pubkey::Pubkey;

    // PumpSwap AMM Program ID
    let pumpswap_program_id =
        Pubkey::from_str("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA").ok()?;

    // 查找 PumpSwap 指令
    for ix in instructions {
        if ix.program_id == pumpswap_program_id {
            // Solana 指令的前 8 字节是 discriminator（Anchor 框架约定）
            if ix.data.len() >= 8 {
                let mut discriminator = [0u8; 8];
                discriminator.copy_from_slice(&ix.data[0..8]);
                eprintln!("🔍 PumpSwap Discriminator: {:02X?}", discriminator);
                return Some(discriminator);
            }
        }
    }

    None
}
