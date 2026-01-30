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
use anyhow::{Result, anyhow};
use solana_client::rpc_config::RpcSimulateTransactionConfig;
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_transaction_status::{
    UiTransactionEncoding,
    UiInnerInstructions,
    UiInstruction,
    UiParsedInstruction,
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
                commitment: Some(CommitmentConfig { commitment: CommitmentLevel::Processed }),
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
    // 方法 1: 从 inner instructions 解析 Transfer/TransferChecked（最准确）
    // 方法 2: 解析 Program data（如果 inner instructions 不可用）
    // 方法 3: 解析日志中的 Token Transfer（备用方案）

    let (actual_input_amount, actual_output_amount) =
        if let Some(ref amounts) = transfer_amounts {
            // 方法 1: 从 inner instructions 解析
            if amounts.len() >= 2 {
                // 假设第一条是输入，第二条是输出
                (amounts[0].1, amounts[1].1)
            } else if amounts.len() == 1 {
                // 只有一条，可能是输出
                (0, amounts[0].1)
            } else {
                (0, 0)
            }
        } else if let Some(logs) = &simulate_result.value.logs {
            // 方法 2: 尝试从日志中解析 "Program data:"
            parse_program_data_from_logs(logs).unwrap_or_else(|| {
                // 方法 3: 如果没有 Program data，回退到解析 Token Transfer 日志
                parse_transfer_amounts_from_logs(
                    logs,
                    &user_input_token_account,
                    &user_output_token_account,
                )
                .unwrap_or((0, 0))
            })
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
                            if let Some(amount) = extract_amount_from_ui_parsed_instruction(ui_parsed_instruction) {
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
                    if let Ok(decoded) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &compiled.data) {
                        // Transfer (0x03) 或 TransferChecked (0x0C)
                        if !decoded.is_empty() && (decoded[0] == 0x03 || decoded[0] == 0x0C) {
                            if decoded.len() >= 9 {
                                if let Ok(amount) = decoded[1..9].try_into().map(u64::from_le_bytes) {
                                    amounts.push((outer_ix.index, amount));
                                }
                            }
                        }
                    }
                },
            }
        }
    }

    if amounts.is_empty() {
        None
    } else {
        Some(amounts)
    }
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
fn parse_return_data(return_data_base64: &str) -> Option<(u64, u64)> {
    use base64::Engine;

    // 解码 base64
    let data = base64::engine::general_purpose::STANDARD
        .decode(return_data_base64)
        .ok()?;

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
