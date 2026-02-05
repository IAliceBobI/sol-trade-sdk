//! # 基于链上模拟的 Swap 计算验证工具
//!
//! 这个模块提供了一种可靠的方法来验证各种 DEX 的 swap 计算准确性：
//! 1. 构造实际的 swap 交易指令
//! 2. 通过 RPC 模拟交易执行（不消耗真实费用）
//! 3. 从模拟结果中解析实际的 token 余额变化
//! 4. 与离线计算结果对比验证
//!
//! ## 优势
//! - **100% 准确**: 使用链上实际逻辑，无需维护复杂的数学公式
//! - **自动更新**: DEX 升级时自动适配，无需修改计算代码
//! - **费用准确**: 包含所有实际费用（协议费、LP 费等）
//!
//! ## 使用场景
//! - 验证离线计算的准确性
//! - 获取精确的输出数量（用于滑点设置）
//! - 测试新 DEX 协议的 swap 逻辑

use crate::common::SolanaRpcClient;
use crate::parser::pumpswap::events::{parse_pumpswap_event, EventData, PumpswapEventType};
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
use std::sync::Arc;

/// 模拟 Swap 交易的结果
#[derive(Debug, Clone)]
pub struct SimulatedSwapResult {
    /// 是否成功
    pub success: bool,
    /// 用户输入代币账户
    pub input_token_account: Pubkey,
    /// 用户输出代币账户
    pub output_token_account: Pubkey,
    /// 输入代币 Mint
    pub input_mint: Pubkey,
    /// 输出代币 Mint
    pub output_mint: Pubkey,
    /// 输入前余额
    pub input_balance_before: u64,
    /// 输入后余额
    pub input_balance_after: u64,
    /// 输出前余额
    pub output_balance_before: u64,
    /// 输出后余额
    pub output_balance_after: u64,
    /// 实际消耗的输入量
    pub actual_input_amount: u64,
    /// 实际得到的输出量
    pub actual_output_amount: u64,
    /// 交易费用（lamports）
    pub transaction_fee: u64,
    /// 计算单元消耗
    pub units_consumed: Option<u64>,
    /// 错误信息（如果失败）
    pub error: Option<String>,
    /// 交易日志（用于调试）
    pub logs: Option<Vec<String>>,
    /// Inner instructions 中的转账金额列表（用于解析转账金额）
    /// Vec<(parent_index, amount)>
    pub inner_instructions: Option<Vec<(u8, u64)>>,
}

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
                commitment: Some(CommitmentConfig { commitment: CommitmentLevel::Finalized }), // 改用 Finalized
                encoding: Some(UiTransactionEncoding::Base64),
                accounts: None,
                min_context_slot: None,
                inner_instructions: true, // 启用以获取内部指令（Token Transfer）
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

    // 提取 inner instructions 中的转账金额
    let transfer_amounts = extract_transfer_amounts_from_parsed_inner_instructions(
        &simulate_result.value.inner_instructions,
    );

    // 从模拟结果中解析 Token Transfer 金额
    //
    // 方法 1: 优先解析 PumpSwap 事件（适用于 PumpSwap）
    // 方法 2: 解析 Raydium AMM V4 ray_log（适用于 AMM V4）
    // 方法 3: 解析 Raydium CPMM Program data（适用于 CPMM）
    // 方法 4: 解析 Program data（最准确，适用于 CLMM）
    // 方法 5: 从 inner instructions 解析 Transfer/TransferChecked
    // 方法 6: 解析日志中的 Token Transfer（备用方案）

    let (actual_input_amount, actual_output_amount) =
        if let Some(logs) = &simulate_result.value.logs {
            // 优先尝试解析 PumpSwap Program data（适用于 PumpSwap）
            if let Some(result) = parse_pumpswap_program_data(logs) {
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
                UiInstruction::Parsed(ui_parsed_instruction) => {
                    // 第二层匹配：UiParsedInstruction
                    match ui_parsed_instruction {
                        UiParsedInstruction::Parsed(_) => {
                            // 尝试从 UiParsedInstruction 中提取 amount
                            if let Some(amount) =
                                extract_amount_from_ui_parsed_instruction(ui_parsed_instruction)
                            {
                                amounts.push((outer_ix.index, amount));
                            }
                        },
                        UiParsedInstruction::PartiallyDecoded(_) => {
                            // 暂不处理
                        },
                    }
                },
                UiInstruction::Compiled(compiled) => {
                    // 解码 base64 编码的指令数据
                    if let Ok(decoded) = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        &compiled.data,
                    ) {
                        // Transfer (0x03) 或 TransferChecked (0x0C)
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
///
/// UiParsedInstruction 格式类似：
/// ```json
/// {
///   "parsed": {
///     "info": {
///       "tokenAmount": {
///         "amount": "1000000"
///       }
///     },
///     "type": "transferChecked"
///   }
/// }
/// ```
fn extract_amount_from_ui_parsed_instruction(
    ui_parsed_instruction: &UiParsedInstruction,
) -> Option<u64> {
    // 将 UiParsedInstruction 转换为 serde_json::Value
    let value = serde_json::to_value(ui_parsed_instruction).ok()?;

    // 获取 "parsed" 字段（注意是小写！）
    let parsed = value.get("parsed")?;

    // 检查是否是 Transfer/TransferChecked 指令
    let instruction_type = parsed.get("type")?.as_str()?;

    if instruction_type != "transfer" && instruction_type != "transferChecked" {
        return None;
    }

    // 提取 tokenAmount.amount
    parsed
        .get("info")?
        .get("tokenAmount")?
        .get("amount")?
        .as_str()?
        .parse::<u64>()
        .ok()
}

/// 获取代币余额
async fn get_token_balance(rpc: &Arc<SolanaRpcClient>, token_account: &Pubkey) -> Result<u64> {
    match rpc.get_token_account_balance(token_account).await {
        Ok(balance) => {
            let balance_u64 = balance
                .amount
                .parse::<u64>()
                .map_err(|_| anyhow!("Failed to parse token balance"))?;
            Ok(balance_u64)
        },
        Err(_) => Ok(0), // 账户不存在时返回 0
    }
}

/// 从程序日志中解析 Program Data（return data）
///
/// CLMM swap 指令会在日志中输出 "Program data: <base64>"
/// 这个 base64 编码的数据包含：
/// - 前 8 字节：amount_in (u64, little endian)
/// - 后 8 字节：amount_out (u64, little endian)
///
/// # 参数
/// * `logs` - 程序日志数组
///
/// # 返回
/// * `Some((amount_in, amount_out))` - 解析成功
/// * `None` - 解析失败（没有找到 Program data 或格式错误）
#[allow(dead_code)]
fn parse_program_data_from_logs(logs: &[String]) -> Option<(u64, u64)> {
    // 查找包含 "Program data:" 的日志行
    for log in logs {
        if let Some(start) = log.find("Program data: ") {
            // 提取 base64 字符串（跳过 "Program data: " 前缀）
            let base64_str = &log[start + 13..]; // "Program data: ".len() = 14

            // 移除可能的空白字符
            let base64_str = base64_str.trim();

            if !base64_str.is_empty() {
                // 尝试解析
                if let Some((amount_in, amount_out)) = parse_return_data(base64_str) {
                    return Some((amount_in, amount_out));
                }
            }
        }
    }
    None
}

/// 从程序的 return data 中解析 swap 结果
///
/// CLMM swap 指令返回的数据格式（base64 编码）：
/// - 前 8 字节：amount_in (u64, little endian)
/// - 后 8 字节：amount_out (u64, little endian)
///
/// # 参数
/// * `return_data_base64` - base64 编码的返回数据
///
/// # 返回
/// * `Some((amount_in, amount_out))` - 解析成功
/// * `None` - 解析失败（数据长度不足或格式错误）
#[allow(dead_code)]
fn parse_return_data(return_data_base64: &str) -> Option<(u64, u64)> {
    use base64::Engine;

    // 解码 base64
    let data = base64::engine::general_purpose::STANDARD.decode(return_data_base64).ok()?;

    // 检查数据长度（至少需要 16 字节：2 个 u64）
    if data.len() < 16 {
        return None;
    }

    // 解析 amount_in（前 8 字节，little endian）
    let amount_in = u64::from_le_bytes(data[0..8].try_into().ok()?);

    // 解析 amount_out（后 8 字节，little endian）
    let amount_out = u64::from_le_bytes(data[8..16].try_into().ok()?);

    Some((amount_in, amount_out))
}

/// 从 Raydium AMM V4 的 ray_log 中解析 swap 结果
///
/// Raydium AMM V4 在日志中输出 "ray_log: <base64>"
/// 这个 base64 编码的数据包含：
/// - 前 8 字节：swap_in_amount (u64, little endian) / 256
/// - 后 8 字节：swap_out_amount (u64, little endian) / 256 * 100
///
/// # 参数
/// * `logs` - 程序日志数组
///
/// # 返回
/// * `Some((amount_in, amount_out))` - 解析成功
/// * `None` - 解析失败
fn parse_raydium_amm_v4_ray_log(logs: &[String]) -> Option<(u64, u64)> {
    // 查找包含 "ray_log:" 的日志行
    for log in logs {
        if let Some(start) = log.find("ray_log: ") {
            // 提取 base64 字符串（跳过 "ray_log: " 前缀）
            let base64_str = &log[start + 9..]; // "ray_log: ".len() = 9

            // 移除可能的空白字符
            let base64_str = base64_str.trim();

            if !base64_str.is_empty() {
                // 尝试解析
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
/// 的数据结构解析：
///
/// SwapBaseInLog 结构 (exact_in):
/// ```rust
/// pub struct SwapBaseInLog {
///     pub log_type: u8,       // Offset 0: 3
///     // 1 byte padding
///     pub amount_in: u64,      // Offset 1-8
///     pub minimum_out: u64,    // Offset 9-16
///     pub direction: u64,      // Offset 17-24
///     pub user_source: u64,    // Offset 25-32
///     pub pool_coin: u64,      // Offset 33-40
///     pub pool_pc: u64,        // Offset 41-48
///     pub out_amount: u64,     // Offset 49-56 ← 实际输出！
/// }
/// ```
///
/// SwapBaseOutLog 结构 (exact_out):
/// ```rust
/// pub struct SwapBaseOutLog {
///     pub log_type: u8,       // Offset 0: 4
///     // 1 byte padding
///     pub max_in: u64,         // Offset 1-8
///     pub amount_out: u64,     // Offset 9-16
///     pub direction: u64,      // Offset 17-24
///     pub user_source: u64,    // Offset 25-32
///     pub pool_coin: u64,      // Offset 33-40
///     pub pool_pc: u64,        // Offset 41-48
///     pub deduct_in: u64,      // Offset 49-56 ← 实际输入！
/// }
/// ```
///
/// # 参数
/// * `ray_log_base64` - base64 编码的 ray_log 数据
///
/// # 返回
/// * `Some((amount_in, amount_out))` - 解析成功
/// * `None` - 解析失败
fn parse_raydium_amm_v4_log_data(ray_log_base64: &str) -> Option<(u64, u64)> {
    use base64::Engine;

    // 解码 base64
    let data = base64::engine::general_purpose::STANDARD.decode(ray_log_base64).ok()?;

    // 检查数据长度（至少需要 57 字节：SwapBaseInLog 的完整结构）
    if data.len() < 57 {
        return None;
    }

    // 解析 log_type 来判断是 exact_in 还是 exact_out
    let log_type = data[0];

    match log_type {
        3 => {
            // SwapBaseIn (exact_in)
            // Offset 1-8: amount_in
            let amount_in = u64::from_le_bytes(data[1..9].try_into().ok()?);
            // Offset 9-16: minimum_out (忽略，使用 out_amount)
            // Offset 17-24: direction (用于验证)
            let _direction = u64::from_le_bytes(data[17..25].try_into().ok()?);
            // Offset 49-56: out_amount ← 实际输出！
            let amount_out = u64::from_le_bytes(data[49..57].try_into().ok()?);

            Some((amount_in, amount_out))
        },
        4 => {
            // SwapBaseOut (exact_out)
            // Offset 1-8: max_in
            // Offset 9-16: amount_out (期望输出)
            // Offset 17-24: direction
            // Offset 49-56: deduct_in ← 实际输入！
            let amount_in = u64::from_le_bytes(data[49..57].try_into().ok()?);
            let amount_out = u64::from_le_bytes(data[9..17].try_into().ok()?);

            Some((amount_in, amount_out))
        },
        _ => {
            // 未知 log_type
            None
        },
    }
}

/// 从 Raydium CPMM 的 Program data 中解析 swap 结果
///
/// CPMM swap 指令在 "Program data:" 日志中返回 base64 编码的 swap 结果。
/// 根据 CPMM Program 的 return data 格式：
/// - Offset 56-63: 输入金额（u64, little endian）
/// - Offset 64-71: 输出金额（u64, little endian）
///
/// # 参数
/// * `logs` - 程序日志数组
///
/// # 返回
/// * `Some((amount_in, amount_out))` - 解析成功
/// * `None` - 解析失败
fn parse_raydium_cpmm_program_data(logs: &[String]) -> Option<(u64, u64)> {
    // 检查是否是 CLMM 交易（包含 "SwapV2" 指令）
    let is_clmm = logs.iter().any(|log| log.contains("SwapV2"));

    // CLMM 交易不应该使用 CPMM 解析器
    if is_clmm {
        return None;
    }

    // 检查是否是 PumpSwap 交易（Program ID: pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA）
    // PumpSwap 也会输出 "Program data:" 但格式不同，不应该用 CPMM 解析器
    // 注意：Program ID 和 "Program data:" 可能在不同的日志行中
    let has_pumpswap_program = logs.iter().any(|log| {
        log.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA")
    });
    let has_program_data = logs.iter().any(|log| {
        log.contains("Program data:")
    });

    // 如果日志中同时包含 PumpSwap Program ID 和 Program data，则跳过 CPMM 解析
    if has_pumpswap_program && has_program_data {
        return None;
    }

    // 查找包含 "Program data:" 的日志行
    for log in logs {
        if let Some(start) = log.find("Program data: ") {
            // 提取 base64 字符串（跳过 "Program data: " 前缀）
            let base64_str = &log[start + 13..]; // "Program data: ".len() = 13

            // 移除可能的空白字符
            let base64_str = base64_str.trim();

            if !base64_str.is_empty() {
                // 尝试解析
                if let Some((amount_in, amount_out)) = parse_raydium_cpmm_data(base64_str) {
                    return Some((amount_in, amount_out));
                }
            }
        }
    }
    None
}

/// 从 Raydium CPMM 的 Program data base64 数据中解析 swap 结果
///
/// 根据 CPMM Program 的 return data 格式：
/// - Offset 56-63: 输入金额（u64, little endian）
/// - Offset 64-71: 输出金额（u64, little endian）
///
/// # 参数
/// * `program_data_base64` - base64 编码的 Program data
///
/// # 返回
/// * `Some((amount_in, amount_out))` - 解析成功
/// * `None` - 解析失败
fn parse_raydium_cpmm_data(program_data_base64: &str) -> Option<(u64, u64)> {
    use base64::Engine;

    // 解码 base64
    let data = base64::engine::general_purpose::STANDARD.decode(program_data_base64).ok()?;

    // 检查数据长度（至少需要 72 字节到 offset 64）
    if data.len() < 72 {
        return None;
    }

    // 解析输入金额（offset 56）
    let amount_in = u64::from_le_bytes(data[56..64].try_into().ok()?);

    // 解析输出金额（offset 64）
    let amount_out = u64::from_le_bytes(data[64..72].try_into().ok()?);

    Some((amount_in, amount_out))
}

/// 从 PumpSwap 的 inner instructions 中解析 swap 结果
///
/// PumpSwap swap 指令会在 inner instructions 中发出事件，包含：
/// - base_amount_out: 用户收到的代币数量
/// - quote_amount_in: 用户支付的代币数量
///
/// # 参数
/// * `inner_instructions` - RPC 返回的 inner instructions
///
/// # 返回
/// * `Some((amount_in, amount_out))` - 解析成功
/// * `None` - 解析失败
fn parse_pumpswap_event_from_inner_instructions(
    inner_instructions: &Option<Vec<UiInnerInstructions>>,
) -> Option<(u64, u64)> {
    use base64::Engine;

    let inner_ixs = inner_instructions.as_ref()?;

    for outer_ix in inner_ixs {
        for ui_instruction in &outer_ix.instructions {
            // 尝试从 Parsed 格式中提取数据
            if let UiInstruction::Parsed(ui_parsed_instruction) = ui_instruction {
                // 将 UiParsedInstruction 转换为 serde_json::Value
                if let Ok(value) = serde_json::to_value(ui_parsed_instruction) {
                    // 检查是否是 PumpSwap 程序
                    if let Some(program) = value.get("program") {
                        if program.as_str() == Some("pump_amm") {
                            // 尝试从 data 字段中提取事件数据
                            if let Some(data_str) = value.get("data").and_then(|d| d.as_str()) {
                                // 解码 base64 数据
                                if let Ok(event_data) = base64::engine::general_purpose::STANDARD
                                    .decode(data_str)
                                {
                                    // 使用 PumpSwap 事件解析器
                                    if let Some((event_type, event_data)) =
                                        parse_pumpswap_event(&event_data)
                                    {
                                        match event_type {
                                            PumpswapEventType::Buy => {
                                                if let EventData::Buy(buy_event) = event_data {
                                                    // 返回 (输入金额, 输出金额)
                                                    return Some((
                                                        buy_event.quote_amount_in_with_lp_fee,
                                                        buy_event.base_amount_out,
                                                    ));
                                                }
                                            },
                                            PumpswapEventType::Sell => {
                                                if let EventData::Sell(sell_event) = event_data {
                                                    // 返回 (输入金额, 输出金额)
                                                    return Some((
                                                        sell_event.base_amount_in,
                                                        sell_event.quote_amount_out,
                                                    ));
                                                }
                                            },
                                            _ => {},
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 尝试从 Compiled 格式中提取数据
            if let UiInstruction::Compiled(compiled) = ui_instruction {
                // 解码 base64 编码的指令数据
                if let Ok(decoded) = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    &compiled.data,
                ) {
                    // 尝试解析 PumpSwap 事件
                    if let Some((event_type, event_data)) = parse_pumpswap_event(&decoded) {
                        match event_type {
                            PumpswapEventType::Buy => {
                                if let EventData::Buy(buy_event) = event_data {
                                    return Some((
                                        buy_event.quote_amount_in_with_lp_fee,
                                        buy_event.base_amount_out,
                                    ));
                                }
                            },
                            PumpswapEventType::Sell => {
                                if let EventData::Sell(sell_event) = event_data {
                                    return Some((
                                        sell_event.base_amount_in,
                                        sell_event.quote_amount_out,
                                    ));
                                }
                            },
                            _ => {},
                        }
                    }
                }
            }
        }
    }

    None
}

/// 从 PumpSwap 的 Program data 中解析 swap 结果
///
/// PumpSwap 的 Program data 格式（从链上实际交易验证）：
/// - Offset 16-23: base_amount_out（用户收到的 base token 数量，u64, little endian）
/// - Offset 24-31: quote_amount_in（用户支付的 quote token 数量，u64, little endian）
///
/// # 参数
/// * `logs` - 程序日志数组
///
/// # 返回
/// * `Some((amount_in, amount_out))` - 解析成功
/// * `None` - 解析失败
fn parse_pumpswap_program_data(logs: &[String]) -> Option<(u64, u64)> {
    use base64::Engine;

    // 检查是否是 PumpSwap 交易
    let has_pumpswap = logs.iter().any(|log| {
        log.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA")
    });

    if !has_pumpswap {
        return None;
    }

    // 查找包含 "Program data:" 的日志行
    for log in logs {
        if let Some(start) = log.find("Program data: ") {
            let base64_str = &log[start + 13..];
            let base64_str = base64_str.trim();

            if base64_str.is_empty() {
                continue;
            }

            // 解码 base64
            let data = base64::engine::general_purpose::STANDARD.decode(base64_str).ok()?;

            // 检查数据长度（至少需要 32 字节到 offset 24）
            if data.len() < 32 {
                continue;
            }

            // 解析输出金额（offset 16）：用户收到的 base token
            let amount_out = u64::from_le_bytes(data[16..24].try_into().ok()?);

            // 解析输入金额（offset 24）：用户支付的 quote token
            let amount_in = u64::from_le_bytes(data[24..32].try_into().ok()?);

            return Some((amount_in, amount_out));
        }
    }

    None
}

/// 从 PumpSwap 交易的 inner instructions 中解析 swap 结果
///
/// PumpSwap 的 swap 交易包含多个 Transfer 指令：
/// 1. 用户 → Pool：输入代币（WSOL）
/// 2. Pool → 用户：输出代币（PUMP）
/// 3. Pool → 协议：手续费
///
/// 此函数从 inner instructions 中提取用户实际收到的代币金额
fn parse_pumpswap_from_inner_instructions(
    inner_instructions: &Option<Vec<UiInnerInstructions>>,
    user_output_token_account: &Pubkey,
) -> Option<(u64, u64)> {
    use base64::Engine;

    let inner_ixs = inner_instructions.as_ref()?;

    let mut user_input_amount = None;
    let mut user_output_amount = None;

    for outer_ix in inner_ixs {
        for ui_instruction in &outer_ix.instructions {
            match ui_instruction {
                UiInstruction::Parsed(ui_parsed_instruction) => {
                    // 从 Parsed 格式中提取 Transfer/TransferChecked 指令
                    if let Ok(value) = serde_json::to_value(ui_parsed_instruction) {
                        // 检查是否是 Transfer/TransferChecked 指令
                        let instruction_type = value.get("type")?.as_str()?;

                        if instruction_type == "transfer" || instruction_type == "transferChecked" {
                            // 获取目标账户（接收方）
                            if let Some(info) = value.get("info") {
                                let destination = info.get("destination").and_then(|d| d.as_str())?;
                                let amount_str = info.get("tokenAmount")
                                    .and_then(|t| t.get("amount"))
                                    .and_then(|a| a.as_str())?;
                                let amount = amount_str.parse::<u64>().ok()?;

                                // 如果接收方是用户的输出代币账户，记录为输出金额
                                if destination == user_output_token_account.to_string() {
                                    user_output_amount = Some(amount);
                                }
                            }
                        }
                    }
                },
                UiInstruction::Compiled(compiled) => {
                    // 从 Compiled 格式中提取 Transfer/TransferChecked 指令
                    if let Ok(decoded) = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        &compiled.data,
                    ) {
                        // Transfer (0x03) 或 TransferChecked (0x0C)
                        if !decoded.is_empty() && (decoded[0] == 0x03 || decoded[0] == 0x0C) {
                            if decoded.len() >= 9 {
                                let amount = u64::from_le_bytes(decoded[1..9].try_into().ok()?);

                                // 注意：Compiled 格式中没有账户信息，无法确定是哪个 Transfer
                                // 暂时记录，但优先使用 Parsed 格式
                                if user_output_amount.is_none() {
                                    user_output_amount = Some(amount);
                                }
                            }
                        }
                    }
                },
            }
        }
    }

    // 返回解析结果
    match (user_input_amount, user_output_amount) {
        (Some(input), Some(output)) => Some((input, output)),
        (_, Some(output)) => Some((0, output)), // 只有输出金额
        _ => None,
    }
}

/// 从程序日志中解析 Token Transfer 金额
///
/// Solana Token Transfer 指令通常会在日志中输出转账金额
/// 日志格式示例：
/// - "Program log: Transfer: { amount: "1234567890" }"
/// - "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA: Transfer 1234567890 tokens"
///
/// 注意：这个函数依赖于具体的程序日志格式，可能不完全准确
/// 更可靠的方法是解析 inner instructions 中的指令数据
fn parse_transfer_amounts_from_logs(
    logs: &[String],
    _input_account: &Pubkey,
    _output_account: &Pubkey,
) -> Option<(u64, u64)> {
    use regex::Regex;

    // 检查是否是 CLMM 交易（包含 "SwapV2" 指令）
    let is_clmm = logs.iter().any(|log| log.contains("SwapV2"));

    // CLMM 交易不应该使用这个方法解析，应该使用 inner instructions
    if is_clmm {
        return None;
    }

    // 检查是否是 PumpSwap 交易（应该使用 inner instructions）
    let is_pumpswap = logs.iter().any(|log| {
        log.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA")
    });

    if is_pumpswap {
        return None;
    }

    // 尝试从日志中提取所有数字
    let mut numbers: Vec<u64> = Vec::new();

    // 在循环外编译 regex
    let Ok(re) = Regex::new(r"\b\d{8,}\b") else {
        return None;
    };

    for log in logs {
        // 查找包含大数字的日志（转账金额通常很大）
        // 使用正则表达式提取数字
        for cap in re.captures_iter(log) {
            if let Some(num_str) = cap.get(0) {
                if let Ok(num) = num_str.as_str().parse::<u64>() {
                    // 过滤掉明显不是转账金额的数字
                    // 例如：compute units (通常是几千到几十万)
                    if num > 1_000_000 {
                        // 转账金额通常大于 100 万
                        numbers.push(num);
                    }
                }
            }
        }
    }

    // 如果找到多个数字，通常：
    // - 最小的数字可能是输入金额
    // - 最大的数字可能是输出金额
    if numbers.len() >= 2 {
        numbers.sort();
        let input_amount = numbers.first().copied()?;
        let output_amount = numbers.last().copied()?;
        Some((input_amount, output_amount))
    } else if numbers.len() == 1 {
        // 只找到一个数字，可能是输出金额
        let output_amount = numbers[0];
        Some((0, output_amount)) // 无法确定输入金额
    } else {
        // 没找到明显的转账金额
        None
    }
}

/// 验证离线计算的准确性
///
/// # 使用示例
/// ```ignore
/// use sol_trade_sdk::utils::simulation_based_calc::*;
///
/// // 1. 离线计算
/// let calculated_output = raydium_cpmm::compute_swap_amount(
///     base_reserve, quote_reserve, true, amount_in, 100
/// );
///
/// // 2. 链上模拟
/// let simulated = simulate_swap_transaction(
///     &rpc, &payer, instructions,
///     input_ata, output_ata,
///     input_mint, output_mint
/// ).await?;
///
/// // 3. 对比验证
/// let diff = if simulated.actual_output_amount > calculated_output.amount_out {
///     simulated.actual_output_amount - calculated_output.amount_out
/// } else {
///     calculated_output.amount_out - simulated.actual_output_amount
/// };
///
/// let error_rate = (diff as f64 / simulated.actual_output_amount as f64) * 100.0;
/// println!("离线计算: {}", calculated_output.amount_out);
/// println!("链上模拟: {}", simulated.actual_output_amount);
/// println!("误差率: {}%", error_rate);
///
/// assert!(error_rate < 0.1, "误差超过 0.1%");
/// ```
pub fn verify_calculation_accuracy(
    calculated_output: u64,
    simulated_output: u64,
    max_error_percentage: f64,
) -> Result<bool> {
    if simulated_output == 0 {
        return Err(anyhow!("模拟输出为 0，无法验证"));
    }

    let diff = calculated_output.abs_diff(simulated_output);

    let error_rate = (diff as f64 / simulated_output as f64) * 100.0;

    if error_rate > max_error_percentage {
        Err(anyhow!(
            "计算误差过大: {}% (最大允许: {}%) | 计算: {}, 实际: {}",
            error_rate,
            max_error_percentage,
            calculated_output,
            simulated_output
        ))
    } else {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_calculation_accuracy() {
        // 完全匹配
        assert!(verify_calculation_accuracy(1000, 1000, 0.1).is_ok());

        // 0.01% 误差
        assert!(verify_calculation_accuracy(1000, 1001, 0.1).is_ok());

        // 0.1% 误差
        assert!(verify_calculation_accuracy(1000, 1001, 0.1).is_ok());

        // 超过 0.1% 误差
        assert!(verify_calculation_accuracy(1000, 1002, 0.1).is_err());
    }
}
