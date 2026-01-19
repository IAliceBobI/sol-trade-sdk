//! 交易解析器的核心数据类型定义

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// 交易类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TradeType {
    /// 买入交易
    Buy,
    /// 卖出交易
    Sell,
    /// 交换交易
    Swap,
}

/// 代币信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// 代币地址
    pub mint: Pubkey,
    /// 数量（UI格式，考虑了精度）
    pub amount: f64,
    /// 原始数量（未处理精度）
    pub amount_raw: String,
    /// 精度（小数位数）
    pub decimals: u8,
    /// 授权地址（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<Pubkey>,
    /// 源账户地址（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Pubkey>,
    /// 目标账户地址（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<Pubkey>,
}

impl TokenInfo {
    /// 从原始数量创建代币信息
    pub fn from_raw_amount(mint: Pubkey, amount_raw: u64, decimals: u8) -> Self {
        let amount = amount_raw as f64 / 10_f64.powi(decimals as i32);
        Self {
            mint,
            amount,
            amount_raw: amount_raw.to_string(),
            decimals,
            authority: None,
            source: None,
            destination: None,
        }
    }
}

/// 解析后的交易信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTradeInfo {
    /// 用户地址
    pub user: Pubkey,
    /// 交易类型
    pub trade_type: TradeType,
    /// 池地址
    pub pool: Pubkey,
    /// 输入代币
    pub input_token: TokenInfo,
    /// 输出代币
    pub output_token: TokenInfo,
    /// 手续费信息（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee: Option<TokenInfo>,
    /// 详细费用列表（可选）
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fees: Vec<FeeInfo>,
    /// DEX 协议名称
    pub dex: String,
    /// 交易签名
    pub signature: String,
    /// 区块槽位
    pub slot: u64,
    /// 时间戳
    pub timestamp: i64,
}

/// 费用信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeInfo {
    /// 费用代币
    pub token: TokenInfo,
    /// 费用类型（如：LP Fee, Protocol Fee）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_type: Option<String>,
}

/// 解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    /// 是否解析成功
    pub success: bool,
    /// 解析出的交易列表
    pub trades: Vec<ParsedTradeInfo>,
    /// 错误信息（如果解析失败）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 解析器配置
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// 是否包含详细日志
    pub verbose: bool,
    /// RPC 端点
    pub rpc_url: String,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            rpc_url: "http://127.0.0.1:8899".to_string(),
        }
    }
}

/// DEX 协议枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DexProtocol {
    PumpSwap,
    RaydiumV4,
    RaydiumClmm,
    RaydiumCpmm,
}

impl DexProtocol {
    /// 获取协议的程序ID
    pub fn program_id(&self) -> &'static str {
        match self {
            DexProtocol::PumpSwap => "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",
            DexProtocol::RaydiumV4 => "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
            DexProtocol::RaydiumClmm => "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
            DexProtocol::RaydiumCpmm => "CPMMoo8L3F4NbTegBCKVNunggL7H5ZQdwpK3bqdrh4jM",
        }
    }

    /// 获取协议名称
    pub fn name(&self) -> &'static str {
        match self {
            DexProtocol::PumpSwap => "PumpSwap",
            DexProtocol::RaydiumV4 => "Raydium V4",
            DexProtocol::RaydiumClmm => "Raydium CLMM",
            DexProtocol::RaydiumCpmm => "Raydium CPMM",
        }
    }

    /// 从程序ID解析协议
    pub fn from_program_id(program_id: &str) -> Option<Self> {
        match program_id {
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => Some(DexProtocol::PumpSwap),
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => Some(DexProtocol::RaydiumV4),
            "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => Some(DexProtocol::RaydiumClmm),
            "CPMMoo8L3F4NbTegBCKVNunggL7H5ZQdwpK3bqdrh4jM" => Some(DexProtocol::RaydiumCpmm),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_info_from_raw_amount() {
        let mint = Pubkey::new_unique();
        let amount_raw = 1_000_000u64;
        let decimals = 6u8;

        let token_info = TokenInfo::from_raw_amount(mint, amount_raw, decimals);

        assert_eq!(token_info.mint, mint);
        assert_eq!(token_info.amount, 1.0);
        assert_eq!(token_info.amount_raw, "1000000");
        assert_eq!(token_info.decimals, decimals);
    }
}
