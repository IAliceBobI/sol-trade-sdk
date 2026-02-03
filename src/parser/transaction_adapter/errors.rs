//! 交易适配器错误类型

use thiserror::Error;

/// 交易适配器错误
#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("交易数据无效")]
    InvalidTransactionData,

    #[error("指令数据解析失败: {0}")]
    InstructionParseError(String),

    #[error("余额数据缺失")]
    MissingBalanceData,

    #[error("Pubkey 解析失败: {0}")]
    PubkeyParseError(String),

    #[error("JSON 解析失败: {0}")]
    JsonError(String),

    #[error("数据解析错误: {0}")]
    DataError(#[from] anyhow::Error),
}
