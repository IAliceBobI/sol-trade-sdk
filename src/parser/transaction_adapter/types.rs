//! 交易适配器数据类型

use solana_account_decoder::parse_token::UiTokenAmount;
use solana_sdk::pubkey::Pubkey;
use serde_json::Value;

/// 指令信息
#[derive(Debug, Clone)]
pub struct InstructionInfo {
    /// 程序ID
    pub program_id: Pubkey,
    /// 账户列表
    pub accounts: Vec<Pubkey>,
    /// 指令数据（对于 parsed 指令可能是空的）
    pub data: Vec<u8>,
    /// 指令索引
    pub index: usize,
    /// Parsed 指令的 JSON 值（用于进一步解析）
    pub parsed_json: Option<Value>,
}

/// 内部指令信息
#[derive(Debug, Clone)]
pub struct InnerInstructionInfo {
    /// 外部指令索引
    pub outer_index: usize,
    /// 内部指令索引
    pub inner_index: usize,
    /// 指令信息
    pub instruction: InstructionInfo,
}

/// 代币数量信息
#[derive(Debug, Clone)]
pub struct TokenAmount {
    /// 原始数量
    pub amount: String,
    /// UI 格式数量
    pub ui_amount: f64,
    /// 精度
    pub decimals: u8,
}

/// 转账数据
#[derive(Debug, Clone)]
pub struct TransferData {
    /// 转账类型
    pub transfer_type: String,
    /// 程序ID
    pub program_id: Pubkey,
    /// 授权地址
    pub authority: Option<Pubkey>,
    /// 源账户
    pub source: Pubkey,
    /// 目标账户
    pub destination: Pubkey,
    /// Mint 地址
    pub mint: Pubkey,
    /// 代币数量
    pub token_amount: TokenAmount,
    /// 源账户余额（转账后）
    pub source_balance: Option<UiTokenAmount>,
    /// 源账户余额（转账前）
    pub source_pre_balance: Option<UiTokenAmount>,
    /// 目标账户余额（转账后）
    pub destination_balance: Option<UiTokenAmount>,
    /// 目标账户余额（转账前）
    pub destination_pre_balance: Option<UiTokenAmount>,
    /// 外部指令索引
    pub outer_index: usize,
    /// 内部指令索引
    pub inner_index: usize,
    /// 时间戳
    pub timestamp: i64,
    /// 交易签名
    pub signature: String,
}

/// Token Program ID
pub fn token_program() -> Pubkey {
    Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
}

/// Token 2022 Program ID
pub fn token_program_2022() -> Pubkey {
    Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
}
