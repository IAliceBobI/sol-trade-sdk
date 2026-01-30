//! 交易错误类型
//!
//! 定义了统一交易接口的错误类型

use crate::trading::factory::DexType;

/// 统一交易接口的错误类型
#[derive(Debug, thiserror::Error)]
pub enum TradingError {
    #[error("不支持的 DEX 类型: {0:?}")]
    UnsupportedDex(DexType),

    #[error("无效的参数: {0}")]
    InvalidParameters(String),

    #[error("Quote 计算失败: {0}")]
    QuoteFailed(String),

    #[error("模拟失败: {0}")]
    SimulationFailed(String),

    #[error("RPC 错误: {0}")]
    RpcError(#[from] reqwest::Error),

    #[error("序列化错误: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("交易构建错误: {0}")]
    TransactionBuildError(String),
}

/// 统一交易接口的 Result 类型
pub type Result<T> = std::result::Result<T, TradingError>;
