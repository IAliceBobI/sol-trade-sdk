//! 交易适配器 - 统一的交易数据访问层

use solana_sdk::pubkey::Pubkey;
use solana_account_decoder::parse_token::UiTokenAmount;
use std::collections::HashMap;


/// 交易适配器错误
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("交易数据无效")]
    InvalidTransactionData,
    #[error("指令数据解析失败: {0}")]
    InstructionParseError(String),
    #[error("余额数据缺失")]
    MissingBalanceData,
}

/// 指令信息
#[derive(Debug, Clone)]
pub struct InstructionInfo {
    /// 程序ID
    pub program_id: Pubkey,
    /// 账户列表
    pub accounts: Vec<Pubkey>,
    /// 指令数据
    pub data: Vec<u8>,
    /// 指令索引
    pub index: usize,
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

/// 交易适配器
///
/// 统一封装不同格式的交易数据，提供一致的访问接口
#[derive(Debug, Clone)]
pub struct TransactionAdapter {
    /// 交易签名
    pub signature: String,
    /// 区块槽位
    pub slot: u64,
    /// 时间戳
    pub timestamp: i64,
    /// 账户公钥列表
    pub account_keys: Vec<Pubkey>,
    /// 代币余额变化映射
    pub token_balance_changes: HashMap<Pubkey, (Option<UiTokenAmount>, Option<UiTokenAmount>)>,
    /// SPL Token 信息映射
    pub spl_token_map: HashMap<Pubkey, Pubkey>,
    /// SPL Token 精度映射
    pub spl_decimals_map: HashMap<Pubkey, u8>,
    /// 指令列表
    pub instructions: Vec<InstructionInfo>,
    /// 内部指令列表
    pub inner_instructions: Vec<InnerInstructionInfo>,
}

impl TransactionAdapter {
    /// 从已确认的交易创建适配器
    pub async fn from_confirmed_transaction<T>(
        _tx: &T,
        slot: u64,
        block_time: Option<i64>,
    ) -> Result<Self, AdapterError>
    where
        T: std::any::Any + Send + Sync,
    {
        // TODO: 完整实现需要正确解析交易数据
        // 当前返回简化版本

        let signature = String::new();  // TODO: 从交易中提取

        let timestamp = block_time.unwrap_or(0);

        // 解析代币余额变化（简化版）
        let (token_balance_changes, spl_token_map, spl_decimals_map) =
            Self::parse_token_balances();

        // 解析指令（简化版）
        let (instructions, inner_instructions) = Self::parse_instructions_simple()?;

        Ok(Self {
            signature,
            slot,
            timestamp,
            account_keys: vec![],  // TODO: 从交易中提取
            token_balance_changes,
            spl_token_map,
            spl_decimals_map,
            instructions,
            inner_instructions,
        })
    }

    /// 解析代币余额变化（简化版）
    ///
    /// TODO: 完整实现需要正确处理 Solana 3.0 的 UiTransactionTokenBalance 类型
    fn parse_token_balances() -> (
        HashMap<Pubkey, (Option<UiTokenAmount>, Option<UiTokenAmount>)>,
        HashMap<Pubkey, Pubkey>,
        HashMap<Pubkey, u8>,
    ) {
        // TODO: 完整实现代币余额解析
        // 当前返回空映射
        (
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
        )
    }

    /// 简化版本的指令解析
    ///
    /// TODO: 完整实现需要解析交易消息
    fn parse_instructions_simple() -> Result<(Vec<InstructionInfo>, Vec<InnerInstructionInfo>), AdapterError> {
        // 暂时返回空列表
        Ok((vec![], vec![]))
    }

    /// 获取指定账户的代币余额变化
    pub fn get_token_balance_change(
        &self,
        account: &Pubkey,
    ) -> Option<&(Option<UiTokenAmount>, Option<UiTokenAmount>)> {
        self.token_balance_changes.get(account)
    }

    /// 获取 Token Account 对应的 Mint
    pub fn get_token_mint(&self, token_account: &Pubkey) -> Option<&Pubkey> {
        self.spl_token_map.get(token_account)
    }

    /// 获取 Mint 的精度
    pub fn get_mint_decimals(&self, mint: &Pubkey) -> Option<u8> {
        self.spl_decimals_map.get(mint).copied()
    }

    /// 获取指定程序ID的所有指令
    pub fn get_instructions_by_program(&self, _program_id: &Pubkey) -> Vec<&InstructionInfo> {
        // 简化版本，返回所有指令
        self.instructions.iter().collect()
    }

    /// 获取指定程序ID的所有内部指令
    pub fn get_inner_instructions_by_program(
        &self,
        _program_id: &Pubkey,
    ) -> Vec<&InnerInstructionInfo> {
        self.inner_instructions.iter().collect()
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
