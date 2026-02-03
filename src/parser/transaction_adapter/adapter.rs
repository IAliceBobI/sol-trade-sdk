//! 交易适配器 - 统一的交易数据访问层
//!
//! 参考 solana-dex-parser 的 TransactionAdapter 实现
//! 支持 Solana 3.0 的交易格式

use super::errors::AdapterError;
use super::parsers::{
    extract_account_keys, extract_instructions, extract_signature, extract_token_balances,
    parse_transfer_instruction_parsed, parse_transfer_instruction_raw,
};
use super::types::{
    InnerInstructionInfo, InstructionInfo, TransferData, token_program, token_program_2022,
};
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::HashMap;

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
    /// 代币余额变化映射 (token_account -> (pre_balance, post_balance))
    pub token_balance_changes: HashMap<Pubkey, (Option<UiTokenAmount>, Option<UiTokenAmount>)>,
    /// SPL Token Account -> Mint 映射
    pub spl_token_map: HashMap<Pubkey, Pubkey>,
    /// Mint -> 精度映射
    pub spl_decimals_map: HashMap<Pubkey, u8>,
    /// 指令列表
    pub instructions: Vec<InstructionInfo>,
    /// 内部指令列表
    pub inner_instructions: Vec<InnerInstructionInfo>,
    /// 内部指令的 JSON 表示（用于方便解析）
    pub inner_instructions_json: Vec<serde_json::Value>,
}

impl TransactionAdapter {
    /// 从 EncodedConfirmedTransactionWithStatusMeta 创建适配器
    pub fn from_encoded_transaction(
        encoded_tx: &EncodedConfirmedTransactionWithStatusMeta,
        slot: u64,
        block_time: Option<i64>,
    ) -> Result<Self, AdapterError> {
        let tx_with_meta = &encoded_tx.transaction;

        // 获取签名
        let signature = extract_signature(tx_with_meta)?;

        let timestamp = block_time.unwrap_or(0);

        // 提取账户密钥
        let account_keys = extract_account_keys(tx_with_meta)?;

        // 提取代币余额变化
        let (token_balance_changes, spl_token_map, spl_decimals_map) =
            extract_token_balances(tx_with_meta, &account_keys)?;

        // 提取指令
        let (instructions, inner_instructions, inner_instructions_json) =
            extract_instructions(tx_with_meta, &account_keys)?;

        Ok(Self {
            signature,
            slot,
            timestamp,
            account_keys,
            token_balance_changes,
            spl_token_map,
            spl_decimals_map,
            instructions,
            inner_instructions,
            inner_instructions_json,
        })
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

    /// 获取指定程序ID的所有外部指令
    pub fn get_instructions_by_program(&self, program_id: &Pubkey) -> Vec<&InstructionInfo> {
        self.instructions.iter().filter(|ix| &ix.program_id == program_id).collect()
    }

    /// 获取指定程序ID的所有内部指令
    pub fn get_inner_instructions_by_program(
        &self,
        program_id: &Pubkey,
    ) -> Vec<&InnerInstructionInfo> {
        self.inner_instructions
            .iter()
            .filter(|ix| &ix.instruction.program_id == program_id)
            .collect()
    }

    /// 获取所有 transferChecked 类型的内部指令
    pub fn get_transfer_checked_instructions(&self) -> Vec<&InnerInstructionInfo> {
        self.inner_instructions
            .iter()
            .filter(|ix| {
                if let Some(json) = &ix.instruction.parsed_json {
                    json["parsed"]["type"]
                        .as_str()
                        .map(|t| t == "transferChecked" || t == "transfer")
                        .unwrap_or(false)
                } else {
                    false
                }
            })
            .collect()
    }

    /// 获取所有转账类型的内部指令（扩展版）
    pub fn get_all_transfer_instructions(&self) -> Vec<&InnerInstructionInfo> {
        let token_program_id = token_program();
        let token_2022_program_id = token_program_2022();

        self.inner_instructions
            .iter()
            .filter(|ix| {
                // 必须是 Token Program 或 Token-2022 Program 的指令
                if ix.instruction.program_id != token_program_id
                    && ix.instruction.program_id != token_2022_program_id
                {
                    return false;
                }

                // 尝试从 parsed 字段判断
                if let Some(json) = &ix.instruction.parsed_json
                    && let Some(t) = json["parsed"]["type"].as_str()
                {
                    return matches!(
                        t,
                        "transfer"
                            | "transferChecked"
                            | "mintTo"
                            | "mintToChecked"
                            | "burn"
                            | "burnChecked"
                    );
                }

                // 如果没有 parsed 字段，尝试从指令数据判断
                if !ix.instruction.data.is_empty() {
                    let discriminator = ix.instruction.data[0];
                    matches!(discriminator, 3 | 12)
                } else {
                    false
                }
            })
            .collect()
    }

    /// 获取所有转账动作
    ///
    /// 解析所有 transfer/transferChecked/mintTo/burn 类型的指令，返回结构化的转账数据
    pub fn get_transfer_actions(&self) -> Vec<TransferData> {
        let mut transfers = Vec::new();

        for ix in self.get_all_transfer_instructions() {
            if let Some(json) = &ix.instruction.parsed_json
                && let Ok(transfer_data) =
                    self.parse_transfer_instruction(json, ix.outer_index, ix.inner_index)
            {
                transfers.push(transfer_data);
            }
        }

        transfers
    }

    /// 解析单个转账指令
    fn parse_transfer_instruction(
        &self,
        json: &serde_json::Value,
        outer_index: usize,
        inner_index: usize,
    ) -> Result<TransferData, AdapterError> {
        // 尝试从 parsed 字段解析
        if let Some(parsed_type) = json["parsed"]["type"].as_str() {
            let program_id_str = json["programId"]
                .as_str()
                .ok_or_else(|| AdapterError::InstructionParseError("缺少 programId".to_string()))?;

            let program_id = Pubkey::from_str_const(program_id_str);

            let info = &json["parsed"]["info"];

            return parse_transfer_instruction_parsed(
                info,
                parsed_type,
                program_id,
                &self.spl_token_map,
                &self.spl_decimals_map,
                &self.token_balance_changes,
                self.signature.clone(),
                self.timestamp,
                outer_index,
                inner_index,
            );
        }

        // 如果没有 parsed 字段，尝试从原始数据解析
        let inner_instr = self
            .inner_instructions
            .iter()
            .find(|ix| ix.outer_index == outer_index && ix.inner_index == inner_index)
            .ok_or_else(|| {
                AdapterError::InstructionParseError("找不到对应的内部指令".to_string())
            })?;

        parse_transfer_instruction_raw(
            &inner_instr.instruction.accounts,
            &inner_instr.instruction.data,
            inner_instr.instruction.program_id,
            &self.spl_token_map,
            &self.spl_decimals_map,
            &self.token_balance_changes,
            self.signature.clone(),
            self.timestamp,
            outer_index,
            inner_index,
        )
    }

    /// 获取特定指令相关的 Transfer 记录
    ///
    /// # 参数
    /// - `outer_index`: 外部指令索引
    ///
    /// # 返回
    /// 该外部指令内的所有 Transfer 记录
    pub fn get_transfers_for_instruction(&self, outer_index: usize) -> Vec<TransferData> {
        self.get_transfer_actions()
            .into_iter()
            .filter(|t| t.outer_index == outer_index)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_info_structure() {
        let program_id = Pubkey::new_unique();
        let account1 = Pubkey::new_unique();
        let account2 = Pubkey::new_unique();

        let instruction = InstructionInfo {
            program_id,
            accounts: vec![account1, account2],
            data: vec![1, 2, 3, 4],
            index: 0,
            parsed_json: None,
        };

        assert_eq!(instruction.program_id, program_id);
        assert_eq!(instruction.accounts.len(), 2);
        assert_eq!(instruction.accounts[0], account1);
        assert_eq!(instruction.accounts[1], account2);
        assert_eq!(instruction.data, vec![1, 2, 3, 4]);
        assert_eq!(instruction.index, 0);
    }

    #[test]
    fn test_inner_instruction_info_structure() {
        let program_id = Pubkey::new_unique();
        let account = Pubkey::new_unique();

        let instruction = InstructionInfo {
            program_id,
            accounts: vec![account],
            data: vec![1, 2, 3],
            index: 0,
            parsed_json: None,
        };

        let inner_instruction =
            InnerInstructionInfo { outer_index: 1, inner_index: 0, instruction };

        assert_eq!(inner_instruction.outer_index, 1);
        assert_eq!(inner_instruction.inner_index, 0);
        assert_eq!(inner_instruction.instruction.program_id, program_id);
    }
}
