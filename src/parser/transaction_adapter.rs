//! 交易适配器 - 统一的交易数据访问层

use solana_rpc_client::rpc_client::RpcClient;
use solana_rpc_client_api::response::RpcConfirmedTransactionWithStatus;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatus,
    UiTransactionEncoding,
};
use solana_account_decoder::parse_token::UiTokenAmount;
use std::collections::HashMap;

use super::types::TokenInfo;

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
    pub async fn from_confirmed_transaction(
        tx: &RpcConfirmedTransactionWithStatus,
        slot: u64,
        block_time: Option<i64>,
    ) -> Result<Self, AdapterError> {
        let meta = tx.transaction.meta.clone();

        let signature = tx.transaction.signatures.first()
            .map(|s| s.as_str().to_string())
            .unwrap_or_default();

        let timestamp = block_time.unwrap_or(0);

        // 解析代币余额变化
        let (token_balance_changes, spl_token_map, spl_decimals_map) =
            Self::parse_token_balances(&meta);

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

    /// 解析代币余额变化
    fn parse_token_balances(
        meta: &Option<solana_transaction_status::UiTransactionStatusMeta>,
    ) -> (
        HashMap<Pubkey, (Option<UiTokenAmount>, Option<UiTokenAmount>)>,
        HashMap<Pubkey, Pubkey>,
        HashMap<Pubkey, u8>,
    ) {
        let mut token_balance_changes: HashMap<Pubkey, (Option<UiTokenAmount>, Option<UiTokenAmount>) = HashMap::new();
        let mut spl_token_map: HashMap<Pubkey, Pubkey> = HashMap::new();
        let mut spl_decimals_map: HashMap<Pubkey, u8> = HashMap::new();

        if let Some(meta) = meta {
            // 解析转账前余额
            if let Some(pre_balances) = &meta.pre_token_balances {
                for balance in pre_balances.iter() {
                    if let Some(account_key) = &balance.account_address {
                        if let Ok(pubkey) = account_key.parse::<Pubkey>() {
                            token_balance_changes
                                .entry(pubkey)
                                .or_insert_with(|| (Some(balance.ui_token_amount.clone()), None));

                            // 保存 Mint 信息
                            if let Some(mint) = &balance.mint {
                                if let Ok(mint_pubkey) = mint.parse::<Pubkey>() {
                                    spl_token_map.insert(pubkey, mint_pubkey);
                                }
                            }
                        }
                    }
                }
            }

            // 解析转账后余额
            if let solana_transaction_status::option_serializer::OptionSerializer::Some(post_balances) = &meta.post_token_balances {
                for balance in post_balances.iter() {
                    if let Some(account_key) = &balance.account_address {
                        if let Ok(pubkey) = account_key.parse::<Pubkey>() {
                            token_balance_changes
                                .entry(pubkey)
                                .and_modify(|(pre, post): (Option<UiTokenAmount>, Option<UiTokenAmount>)| {
                                    if post.is_none() {
                                        *post = Some(balance.ui_token_amount.clone());
                                    }
                                });

                            // 保存 Mint 和精度信息
                            if let Some(mint) = &balance.mint {
                                if let Ok(mint_pubkey) = mint.parse::<Pubkey>() {
                                    spl_token_map.insert(pubkey, mint_pubkey);
                                    spl_decimals_map.insert(mint_pubkey, balance.ui_token_amount.decimals);
                                }
                            }
                        }
                    }
                }
            }
        }

        (token_balance_changes, spl_token_map, spl_decimals_map)
    }

    /// 简化版本的指令解析
    ///
    /// TODO: 完整实现需要解析 EncodedConfirmedTransactionWithStatus 的 message
    /// 当前返回空列表，等主解析器集成时再完善
    fn parse_instructions_simple() -> Result<(Vec<InstructionInfo>, Vec<InnerInstructionInfo>), AdapterError> {
        // 暂时返回空列表
        // 完整实现需要：
        // 1. 从 tx.transaction.message 中提取账户列表
        // 2. 解析 message.instructions 中的指令
        // 3. 解析 meta.innerInstructions 中的内部指令
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
        // 简化版本
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
