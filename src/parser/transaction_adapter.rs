//! 交易适配器 - 统一的交易数据访问层
//!
//! 参考 solana-dex-parser 的 TransactionAdapter 实现
//! 支持 Solana 3.0 的交易格式

use solana_account_decoder::parse_token::UiTokenAmount;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta, EncodedTransactionWithStatusMeta,
};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::warn;

/// 交易适配器错误
#[derive(Debug, thiserror::Error)]
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
}

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
    pub parsed_json: Option<serde_json::Value>,
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
        let signature = Self::extract_signature(tx_with_meta)?;

        let timestamp = block_time.unwrap_or(0);

        // 提取账户密钥
        let account_keys = Self::extract_account_keys(tx_with_meta)?;

        // 提取代币余额变化
        let (token_balance_changes, spl_token_map, spl_decimals_map) =
            Self::extract_token_balances(tx_with_meta, &account_keys)?;

        // 提取指令
        let (instructions, inner_instructions, inner_instructions_json) =
            Self::extract_instructions(tx_with_meta, &account_keys)?;

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

    /// 提取签名
    fn extract_signature(tx: &EncodedTransactionWithStatusMeta) -> Result<String, AdapterError> {
        // 从 transaction 中提取签名
        // 使用 JSON 序列化来避免复杂的类型匹配
        let tx_value =
            serde_json::to_value(tx).map_err(|e| AdapterError::JsonError(e.to_string()))?;

        if let Some(signatures) = tx_value["transaction"]["signatures"].as_array() {
            if let Some(first_sig) = signatures.first() {
                if let Some(sig_str) = first_sig.as_str() {
                    return Ok(sig_str.to_string());
                }
            }
        }

        // 备选方案：尝试直接访问
        Ok(String::new())
    }

    /// 提取账户密钥
    fn extract_account_keys(
        tx: &EncodedTransactionWithStatusMeta,
    ) -> Result<Vec<Pubkey>, AdapterError> {
        let mut keys = Vec::new();

        // 使用 JSON 方式提取账户密钥
        let tx_value =
            serde_json::to_value(tx).map_err(|e| AdapterError::JsonError(e.to_string()))?;

        // 尝试多种可能的路径
        // 1. transaction.message.accountKeys (字符串数组)
        if let Some(account_keys) = tx_value["transaction"]["message"]["accountKeys"].as_array() {
            for key_value in account_keys {
                // 尝试作为字符串
                if let Some(key_str) = key_value.as_str() {
                    if let Ok(pubkey) = Pubkey::from_str(key_str) {
                        keys.push(pubkey);
                    }
                }
                // 尝试作为对象 (有 pubkey 字段)
                else if let Some(key_str) = key_value["pubkey"].as_str() {
                    if let Ok(pubkey) = Pubkey::from_str(key_str) {
                        keys.push(pubkey);
                    }
                }
            }
        }

        // 2. transaction.message.staticAccountKeys (备用)
        if keys.is_empty() {
            if let Some(account_keys) =
                tx_value["transaction"]["message"]["staticAccountKeys"].as_array()
            {
                for key_value in account_keys {
                    if let Some(key_str) = key_value.as_str() {
                        if let Ok(pubkey) = Pubkey::from_str(key_str) {
                            keys.push(pubkey);
                        }
                    }
                }
            }
        }

        // 3. accountKeys (在 message 级别)
        if keys.is_empty() {
            if let Some(account_keys) =
                tx_value["transaction"]["message"]["accountKeys"]["accountKeys"].as_array()
            {
                for key_value in account_keys {
                    if let Some(key_str) = key_value.as_str() {
                        if let Ok(pubkey) = Pubkey::from_str(key_str) {
                            keys.push(pubkey);
                        }
                    }
                }
            }
        }

        Ok(keys)
    }

    /// 提取代币余额变化
    fn extract_token_balances(
        tx: &EncodedTransactionWithStatusMeta,
        account_keys: &[Pubkey],
    ) -> Result<
        (
            HashMap<Pubkey, (Option<UiTokenAmount>, Option<UiTokenAmount>)>,
            HashMap<Pubkey, Pubkey>,
            HashMap<Pubkey, u8>,
        ),
        AdapterError,
    > {
        let mut token_balance_changes = HashMap::new();
        let mut spl_token_map = HashMap::new();
        let mut spl_decimals_map = HashMap::new();

        let tx_value =
            serde_json::to_value(tx).map_err(|e| AdapterError::JsonError(e.to_string()))?;

        let meta = &tx_value["meta"];

        // 提取 pre token balances
        if let Some(pre_balances) = meta["preTokenBalances"].as_array() {
            for balance in pre_balances {
                // accountIndex 可能是 u8 或 u64
                let account_index = if let Some(idx_u8) = balance["accountIndex"].as_u64() {
                    idx_u8 as usize
                } else if let Some(idx_u8) = balance["accountIndex"].as_u64() {
                    idx_u8 as usize
                } else {
                    continue;
                };

                if account_index < account_keys.len() {
                    let account = account_keys[account_index];

                    if let Some(mint_str) = balance["mint"].as_str() {
                        if let Ok(mint) = Pubkey::from_str(mint_str) {
                            spl_token_map.insert(account, mint);

                            // 解析 UiTokenAmount
                            if let Some(ui_amount) = balance.get("uiTokenAmount") {
                                let decimals = ui_amount["decimals"].as_u64().unwrap() as u8;
                                spl_decimals_map.insert(mint, decimals);

                                let token_amount = UiTokenAmount {
                                    amount: ui_amount["amount"].as_str().unwrap_or("0").to_string(),
                                    decimals,
                                    ui_amount: ui_amount["uiAmount"].as_f64().or(Some(0.0)),
                                    ui_amount_string: ui_amount["uiAmountString"]
                                        .as_str()
                                        .unwrap_or("0")
                                        .to_string(),
                                };

                                token_balance_changes
                                    .entry(account)
                                    .or_insert_with(|| (Some(token_amount.clone()), None))
                                    .0 = Some(token_amount.clone());
                            }
                        }
                    }
                }
            }
        }

        // 提取 post token balances
        if let Some(post_balances) = meta["postTokenBalances"].as_array() {
            for balance in post_balances {
                let account_index = if let Some(idx_u8) = balance["accountIndex"].as_u64() {
                    idx_u8 as usize
                } else if let Some(idx_u8) = balance["accountIndex"].as_u64() {
                    idx_u8 as usize
                } else {
                    continue;
                };

                if account_index < account_keys.len() {
                    let account = account_keys[account_index];

                    if let Some(mint_str) = balance["mint"].as_str() {
                        if let Ok(mint) = Pubkey::from_str(mint_str) {
                            spl_token_map.insert(account, mint);

                            if let Some(ui_amount) = balance.get("uiTokenAmount") {
                                let decimals = ui_amount["decimals"].as_u64().unwrap() as u8;
                                spl_decimals_map.insert(mint, decimals);

                                let token_amount = UiTokenAmount {
                                    amount: ui_amount["amount"].as_str().unwrap_or("0").to_string(),
                                    decimals,
                                    ui_amount: ui_amount["uiAmount"].as_f64().or(Some(0.0)),
                                    ui_amount_string: ui_amount["uiAmountString"]
                                        .as_str()
                                        .unwrap_or("0")
                                        .to_string(),
                                };

                                token_balance_changes
                                    .entry(account)
                                    .or_insert_with(|| (None, Some(token_amount.clone())))
                                    .1 = Some(token_amount.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok((token_balance_changes, spl_token_map, spl_decimals_map))
    }

    /// 提取指令和内部指令
    fn extract_instructions(
        tx: &EncodedTransactionWithStatusMeta,
        account_keys: &[Pubkey],
    ) -> Result<
        (Vec<InstructionInfo>, Vec<InnerInstructionInfo>, Vec<serde_json::Value>),
        AdapterError,
    > {
        let mut instructions = Vec::new();
        let mut inner_instructions = Vec::new();
        let mut inner_instructions_json = Vec::new();

        let tx_value =
            serde_json::to_value(tx).map_err(|e| AdapterError::JsonError(e.to_string()))?;

        // 提取外部指令
        if let Some(ixs) = tx_value["transaction"]["message"]["instructions"].as_array() {
            for (idx, ix_value) in ixs.iter().enumerate() {
                // Solana 新格式使用 programIdIndex (指向 accountKeys 的索引)
                // 旧格式可能使用直接的 programId 字符串

                let program_id = if let Some(program_id_index) = ix_value["programIdIndex"].as_u64()
                {
                    // 新格式：通过索引获取 programId
                    let index = program_id_index as usize;
                    if index < account_keys.len() {
                        account_keys[index]
                    } else {
                        continue; // 索引越界，跳过此指令
                    }
                } else if let Some(program_id_str) = ix_value["programId"].as_str() {
                    // 旧格式：直接解析字符串
                    if let Ok(pid) = Pubkey::from_str(program_id_str) {
                        pid
                    } else {
                        continue;
                    }
                } else {
                    continue; // 无法解析 programId
                };

                // 解析账户列表 - 新格式使用索引数组
                let accounts = if let Some(accounts_arr) = ix_value["accounts"].as_array() {
                    accounts_arr
                        .iter()
                        .filter_map(|acc| {
                            // 尝试作为索引解析
                            if let Some(index) = acc.as_u64() {
                                let idx = index as usize;
                                if idx < account_keys.len() {
                                    Some(account_keys[idx])
                                } else {
                                    None
                                }
                            } else if let Some(acc_str) = acc.as_str() {
                                // 尝试作为字符串解析
                                Pubkey::from_str(acc_str).ok()
                            } else {
                                None
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                };

                // 解析 data
                let data = if let Some(data_str) = ix_value["data"].as_str() {
                    bs58::decode(data_str)
                        .into_vec()
                        .inspect_err(|e| {
                            warn!("指令数据 base58 解析失败 (指令索引 {}): {}", idx, e)
                        })
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };

                instructions.push(InstructionInfo {
                    program_id,
                    accounts,
                    data,
                    index: idx,
                    parsed_json: Some(ix_value.clone()),
                });
            }
        }

        // 提取内部指令（meta 和 transaction 是同级的）
        if let Some(inner_instrs) = tx_value["meta"]["innerInstructions"].as_array() {
            for inner_set in inner_instrs {
                inner_instructions_json.push(inner_set.clone());

                let outer_index = inner_set["index"].as_u64().unwrap() as usize;

                if let Some(instructions_arr) = inner_set["instructions"].as_array() {
                    for (inner_idx, ix_json) in instructions_arr.iter().enumerate() {
                        // 同样处理 programIdIndex
                        let program_id =
                            if let Some(program_id_index) = ix_json["programIdIndex"].as_u64() {
                                let index = program_id_index as usize;
                                if index < account_keys.len() {
                                    account_keys[index]
                                } else {
                                    continue;
                                }
                            } else if let Some(program_id_str) = ix_json["programId"].as_str() {
                                if let Ok(pid) = Pubkey::from_str(program_id_str) {
                                    pid
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            };

                        // 解析账户列表
                        let accounts = if let Some(accounts_arr) = ix_json["accounts"].as_array() {
                            accounts_arr
                                .iter()
                                .filter_map(|acc| {
                                    if let Some(index) = acc.as_u64() {
                                        let idx = index as usize;
                                        if idx < account_keys.len() {
                                            Some(account_keys[idx])
                                        } else {
                                            None
                                        }
                                    } else if let Some(acc_str) = acc.as_str() {
                                        Pubkey::from_str(acc_str).ok()
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        } else {
                            Vec::new()
                        };

                        // 解析 data
                        let data = if let Some(data_str) = ix_json["data"].as_str() {
                            bs58::decode(data_str).into_vec()
                                .inspect_err(|e| warn!("内部指令数据 base58 解析失败 (外部索引 {}, 内部索引 {}): {}", outer_index, inner_idx, e))
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        };

                        inner_instructions.push(InnerInstructionInfo {
                            outer_index,
                            inner_index: inner_idx,
                            instruction: InstructionInfo {
                                program_id,
                                accounts,
                                data,
                                index: outer_index,
                                parsed_json: Some(ix_json.clone()),
                            },
                        });
                    }
                }
            }
        }

        Ok((instructions, inner_instructions, inner_instructions_json))
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
                    // 检查 parsed.type 是否为 transferChecked
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
        let token_program_id =
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".parse::<Pubkey>().unwrap();

        let token_2022_program_id =
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".parse::<Pubkey>().unwrap();

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
                if let Some(json) = &ix.instruction.parsed_json {
                    if let Some(t) = json["parsed"]["type"].as_str() {
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
                }

                // 如果没有 parsed 字段，尝试从指令数据判断
                // Token Program 指令的 discriminator:
                // - Transfer: 3
                // - TransferChecked: 12
                // - (参见 Token Program 的指令定义)
                if !ix.instruction.data.is_empty() {
                    let discriminator = ix.instruction.data[0];
                    // Transfer (3) 和 TransferChecked (12) 是最常见的
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
            if let Some(json) = &ix.instruction.parsed_json {
                if let Ok(transfer_data) =
                    self.parse_transfer_instruction(json, ix.outer_index, ix.inner_index)
                {
                    transfers.push(transfer_data);
                }
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

            let program_id = Pubkey::from_str(program_id_str)
                .map_err(|e| AdapterError::PubkeyParseError(e.to_string()))?;

            return self.parse_transfer_instruction_parsed(
                json,
                outer_index,
                inner_index,
                parsed_type,
                program_id,
            );
        }

        // 如果没有 parsed 字段，尝试从原始数据解析
        // 从 inner_instructions 获取 program_id
        let inner_instr = self
            .inner_instructions
            .iter()
            .find(|ix| ix.outer_index == outer_index && ix.inner_index == inner_index)
            .ok_or_else(|| {
                AdapterError::InstructionParseError("找不到对应的内部指令".to_string())
            })?;

        self.parse_transfer_instruction_raw(
            json,
            outer_index,
            inner_index,
            inner_instr.instruction.program_id,
        )
    }

    /// 从 parsed 格式解析转账指令
    fn parse_transfer_instruction_parsed(
        &self,
        json: &serde_json::Value,
        outer_index: usize,
        inner_index: usize,
        transfer_type: &str,
        program_id: Pubkey,
    ) -> Result<TransferData, AdapterError> {
        // 获取 info 字段
        let info = json["parsed"]["info"]
            .as_object()
            .ok_or_else(|| AdapterError::InstructionParseError("缺少 info 字段".to_string()))?;

        // 解析 source 和 destination
        let source_str = info
            .get("source")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AdapterError::InstructionParseError("缺少 source".to_string()))?;

        let destination_str = info
            .get("destination")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AdapterError::InstructionParseError("缺少 destination".to_string()))?;

        let source = Pubkey::from_str(source_str)
            .map_err(|e| AdapterError::PubkeyParseError(e.to_string()))?;

        let destination = Pubkey::from_str(destination_str)
            .map_err(|e| AdapterError::PubkeyParseError(e.to_string()))?;

        // 解析 mint
        let mint = if let Some(mint_str) = info.get("mint").and_then(|v| v.as_str()) {
            Pubkey::from_str(mint_str).map_err(|e| AdapterError::PubkeyParseError(e.to_string()))?
        } else {
            self.spl_token_map
                .get(&source)
                .or_else(|| self.spl_token_map.get(&destination))
                .copied()
                .ok_or_else(|| {
                    AdapterError::InstructionParseError(format!(
                        "无法从 source/destination 推断 mint"
                    ))
                })?
        };

        // 解析 decimals
        let decimals = self.spl_decimals_map.get(&mint).copied().ok_or_else(|| {
            AdapterError::InstructionParseError(format!("找不到 mint {} 的精度", mint))
        })?;

        // 解析 tokenAmount
        let token_amount = if let Some(token_amount_json) = info.get("tokenAmount") {
            TokenAmount {
                amount: token_amount_json["amount"].as_str().unwrap_or("0").to_string(),
                ui_amount: token_amount_json["uiAmount"].as_f64().unwrap_or(0.0),
                decimals,
            }
        } else {
            let amount_str = info.get("amount").and_then(|v| v.as_str()).unwrap_or("0");
            let amount_u64 = amount_str.parse::<u64>().unwrap_or(0);
            let ui_amount = amount_u64 as f64 / 10_f64.powi(decimals as i32);
            TokenAmount { amount: amount_str.to_string(), ui_amount, decimals }
        };

        // 解析 authority
        let authority =
            info.get("authority").and_then(|v| v.as_str()).and_then(|s| Pubkey::from_str(s).ok());

        // 获取余额信息
        let source_balance =
            self.token_balance_changes.get(&source).and_then(|(pre, _)| pre.as_ref().cloned());
        let source_pre_balance =
            self.token_balance_changes.get(&source).and_then(|(pre, _)| pre.as_ref().cloned());
        let destination_balance = self
            .token_balance_changes
            .get(&destination)
            .and_then(|(_, post)| post.as_ref().cloned());
        let destination_pre_balance = self
            .token_balance_changes
            .get(&destination)
            .and_then(|(_, post)| post.as_ref().cloned());

        Ok(TransferData {
            transfer_type: transfer_type.to_string(),
            program_id,
            authority,
            source,
            destination,
            mint,
            token_amount,
            source_balance,
            source_pre_balance,
            destination_balance,
            destination_pre_balance,
            outer_index,
            inner_index,
            timestamp: self.timestamp,
            signature: self.signature.clone(),
        })
    }

    /// 从原始数据解析转账指令
    fn parse_transfer_instruction_raw(
        &self,
        _json: &serde_json::Value,
        outer_index: usize,
        inner_index: usize,
        program_id: Pubkey,
    ) -> Result<TransferData, AdapterError> {
        // 从外部获取账户信息
        // 通过找到对应的 InnerInstructionInfo
        let inner_instr = self
            .inner_instructions
            .iter()
            .find(|ix| ix.outer_index == outer_index && ix.inner_index == inner_index)
            .ok_or_else(|| {
                AdapterError::InstructionParseError("找不到对应的内部指令".to_string())
            })?;

        let accounts = &inner_instr.instruction.accounts;
        let data = &inner_instr.instruction.data;

        if data.is_empty() {
            return Err(AdapterError::InstructionParseError("指令数据为空".to_string()));
        }

        let discriminator = data[0];

        // Transfer (3): [source, destination, owner]
        // TransferChecked (12): [source, mint, destination, owner]
        let (source, mint, destination, authority) = match discriminator {
            3 => {
                // Transfer
                if accounts.len() < 3 {
                    return Err(AdapterError::InstructionParseError(
                        "Transfer 指令账户不足".to_string(),
                    ));
                }
                let source = accounts[0];
                let destination = accounts[1];
                // owner 是 authority
                let authority = Some(accounts[2]);

                // 从 spl_token_map 获取 mint
                let mint = self
                    .spl_token_map
                    .get(&source)
                    .or_else(|| self.spl_token_map.get(&destination))
                    .copied()
                    .ok_or_else(|| {
                        AdapterError::InstructionParseError("无法推断 mint".to_string())
                    })?;

                (source, mint, destination, authority)
            }
            12 => {
                // TransferChecked
                if accounts.len() < 4 {
                    return Err(AdapterError::InstructionParseError(
                        "TransferChecked 指令账户不足".to_string(),
                    ));
                }
                let source = accounts[0];
                let mint = accounts[1];
                let destination = accounts[2];
                let authority = Some(accounts[3]);

                (source, mint, destination, authority)
            }
            _ => {
                return Err(AdapterError::InstructionParseError(format!(
                    "未知的指令 discriminator: {}",
                    discriminator
                )));
            }
        };

        // 解析 amount (从偏移 1 开始，8 字节)
        if data.len() < 9 {
            return Err(AdapterError::InstructionParseError("指令数据长度不足".to_string()));
        }

        let amount_bytes = &data[1..9];
        let amount = u64::from_le_bytes(
            amount_bytes
                .try_into()
                .map_err(|_| AdapterError::InstructionParseError("无法解析 amount".to_string()))?,
        );

        // 获取 decimals
        let decimals = self.spl_decimals_map.get(&mint).copied().ok_or_else(|| {
            AdapterError::InstructionParseError(format!("找不到 mint {} 的精度", mint))
        })?;

        let token_amount = TokenAmount {
            amount: amount.to_string(),
            ui_amount: amount as f64 / 10_f64.powi(decimals as i32),
            decimals,
        };

        // 获取余额信息
        let source_balance =
            self.token_balance_changes.get(&source).and_then(|(pre, _)| pre.as_ref().cloned());
        let source_pre_balance =
            self.token_balance_changes.get(&source).and_then(|(pre, _)| pre.as_ref().cloned());
        let destination_balance = self
            .token_balance_changes
            .get(&destination)
            .and_then(|(_, post)| post.as_ref().cloned());
        let destination_pre_balance = self
            .token_balance_changes
            .get(&destination)
            .and_then(|(_, post)| post.as_ref().cloned());

        let transfer_type = match discriminator {
            3 => "transfer",
            12 => "transferChecked",
            _ => "unknown",
        };

        Ok(TransferData {
            transfer_type: transfer_type.to_string(),
            program_id,
            authority,
            source,
            destination,
            mint,
            token_amount,
            source_balance,
            source_pre_balance,
            destination_balance,
            destination_pre_balance,
            outer_index,
            inner_index,
            timestamp: self.timestamp,
            signature: self.signature.clone(),
        })
    }

    /// 获取特定指令相关的 Transfer 记录
    ///
    /// # 参数
    /// - `outer_index`: 外部指令索引
    ///
    /// # 返回
    /// 该外部指令内的所有 Transfer 记录
    pub fn get_transfers_for_instruction(&self, outer_index: usize) -> Vec<TransferData> {
        self.get_transfer_actions().into_iter().filter(|t| t.outer_index == outer_index).collect()
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
