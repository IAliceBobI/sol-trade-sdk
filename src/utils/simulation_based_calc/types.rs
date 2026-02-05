//! # 模拟 Swap 交易结果类型定义

use solana_sdk::pubkey::Pubkey;

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
