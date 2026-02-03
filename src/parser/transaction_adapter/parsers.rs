//! 交易数据解析辅助函数

use super::errors::AdapterError;
use super::types::TokenAmount;
use solana_account_decoder::parse_token::UiTokenAmount;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedTransactionWithStatusMeta;
use std::collections::HashMap;
use std::str::FromStr;
use tracing::warn;

/// 提取签名
pub fn extract_signature(tx: &EncodedTransactionWithStatusMeta) -> Result<String, AdapterError> {
    let tx_value = serde_json::to_value(tx).map_err(|e| AdapterError::JsonError(e.to_string()))?;

    if let Some(signatures) = tx_value["transaction"]["signatures"].as_array()
        && let Some(first_sig) = signatures.first()
        && let Some(sig_str) = first_sig.as_str()
    {
        return Ok(sig_str.to_string());
    }

    Ok(String::new())
}

/// 提取账户密钥
pub fn extract_account_keys(
    tx: &EncodedTransactionWithStatusMeta,
) -> Result<Vec<Pubkey>, AdapterError> {
    let mut keys = Vec::new();

    let tx_value = serde_json::to_value(tx).map_err(|e| AdapterError::JsonError(e.to_string()))?;

    // 尝试多种可能的路径
    // 1. transaction.message.accountKeys (字符串数组)
    if let Some(account_keys) = tx_value["transaction"]["message"]["accountKeys"].as_array() {
        for key_value in account_keys {
            if let Some(key_str) = key_value.as_str() {
                if let Ok(pubkey) = Pubkey::from_str(key_str) {
                    keys.push(pubkey);
                }
            } else if let Some(key_str) = key_value["pubkey"].as_str()
                && let Ok(pubkey) = Pubkey::from_str(key_str)
            {
                keys.push(pubkey);
            }
        }
    }

    // 2. transaction.message.staticAccountKeys (备用)
    if keys.is_empty()
        && let Some(account_keys) =
            tx_value["transaction"]["message"]["staticAccountKeys"].as_array()
    {
        for key_value in account_keys {
            if let Some(key_str) = key_value.as_str()
                && let Ok(pubkey) = Pubkey::from_str(key_str)
            {
                keys.push(pubkey);
            }
        }
    }

    // 3. accountKeys (在 message 级别)
    if keys.is_empty()
        && let Some(account_keys) =
            tx_value["transaction"]["message"]["accountKeys"]["accountKeys"].as_array()
    {
        for key_value in account_keys {
            if let Some(key_str) = key_value.as_str()
                && let Ok(pubkey) = Pubkey::from_str(key_str)
            {
                keys.push(pubkey);
            }
        }
    }

    Ok(keys)
}

/// 提取代币余额变化
pub fn extract_token_balances(
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

    let tx_value = serde_json::to_value(tx).map_err(|e| AdapterError::JsonError(e.to_string()))?;

    let meta = &tx_value["meta"];

    // 提取 pre token balances
    if let Some(pre_balances) = meta["preTokenBalances"].as_array() {
        for balance in pre_balances {
            let account_index = if let Some(idx_u8) = balance["accountIndex"].as_u64() {
                idx_u8 as usize
            } else if let Some(idx_u8) = balance["accountIndex"].as_u64() {
                idx_u8 as usize
            } else {
                continue;
            };

            if account_index < account_keys.len() {
                let account = account_keys[account_index];

                if let Some(mint_str) = balance["mint"].as_str()
                    && let Ok(mint) = Pubkey::from_str(mint_str)
                {
                    spl_token_map.insert(account, mint);

                    if let Some(ui_amount) = balance.get("uiTokenAmount") {
                        let decimals = ui_amount["decimals"]
                            .as_u64()
                            .ok_or_else(|| anyhow::anyhow!("Missing decimals field"))?
                            as u8;
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

                if let Some(mint_str) = balance["mint"].as_str()
                    && let Ok(mint) = Pubkey::from_str(mint_str)
                {
                    spl_token_map.insert(account, mint);

                    if let Some(ui_amount) = balance.get("uiTokenAmount") {
                        let decimals = ui_amount["decimals"]
                            .as_u64()
                            .ok_or_else(|| anyhow::anyhow!("Missing decimals field"))?
                            as u8;
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

    Ok((token_balance_changes, spl_token_map, spl_decimals_map))
}

/// 从 parsed 格式解析转账指令
pub fn parse_transfer_instruction_parsed(
    info: &serde_json::Value,
    transfer_type: &str,
    program_id: Pubkey,
    spl_token_map: &HashMap<Pubkey, Pubkey>,
    spl_decimals_map: &HashMap<Pubkey, u8>,
    token_balance_changes: &HashMap<Pubkey, (Option<UiTokenAmount>, Option<UiTokenAmount>)>,
    signature: String,
    timestamp: i64,
    outer_index: usize,
    inner_index: usize,
) -> Result<super::types::TransferData, AdapterError> {
    // 解析 source 和 destination
    let source_str = info
        .get("source")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AdapterError::InstructionParseError("缺少 source".to_string()))?;

    let destination_str = info
        .get("destination")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AdapterError::InstructionParseError("缺少 destination".to_string()))?;

    let source =
        Pubkey::from_str(source_str).map_err(|e| AdapterError::PubkeyParseError(e.to_string()))?;

    let destination = Pubkey::from_str(destination_str)
        .map_err(|e| AdapterError::PubkeyParseError(e.to_string()))?;

    // 解析 mint
    let mint = if let Some(mint_str) = info.get("mint").and_then(|v| v.as_str()) {
        Pubkey::from_str(mint_str).map_err(|e| AdapterError::PubkeyParseError(e.to_string()))?
    } else {
        spl_token_map
            .get(&source)
            .or_else(|| spl_token_map.get(&destination))
            .copied()
            .ok_or_else(|| {
                AdapterError::InstructionParseError(
                    "无法从 source/destination 推断 mint".to_string(),
                )
            })?
    };

    // 解析 decimals
    let decimals = spl_decimals_map.get(&mint).copied().ok_or_else(|| {
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
    let authority = info
        .get("authority")
        .and_then(|v| v.as_str())
        .and_then(|s| Pubkey::from_str(s).ok());

    // 获取余额信息
    let source_balance =
        token_balance_changes.get(&source).and_then(|(pre, _)| pre.as_ref().cloned());
    let source_pre_balance =
        token_balance_changes.get(&source).and_then(|(pre, _)| pre.as_ref().cloned());
    let destination_balance = token_balance_changes
        .get(&destination)
        .and_then(|(_, post)| post.as_ref().cloned());
    let destination_pre_balance = token_balance_changes
        .get(&destination)
        .and_then(|(_, post)| post.as_ref().cloned());

    Ok(super::types::TransferData {
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
        timestamp,
        signature,
    })
}

/// 从原始数据解析转账指令
pub fn parse_transfer_instruction_raw(
    accounts: &[Pubkey],
    data: &[u8],
    program_id: Pubkey,
    spl_token_map: &HashMap<Pubkey, Pubkey>,
    spl_decimals_map: &HashMap<Pubkey, u8>,
    token_balance_changes: &HashMap<Pubkey, (Option<UiTokenAmount>, Option<UiTokenAmount>)>,
    signature: String,
    timestamp: i64,
    outer_index: usize,
    inner_index: usize,
) -> Result<super::types::TransferData, AdapterError> {
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
            let authority = Some(accounts[2]);

            let mint = spl_token_map
                .get(&source)
                .or_else(|| spl_token_map.get(&destination))
                .copied()
                .ok_or_else(|| AdapterError::InstructionParseError("无法推断 mint".to_string()))?;

            (source, mint, destination, authority)
        },
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
        },
        _ => {
            return Err(AdapterError::InstructionParseError(format!(
                "未知的指令 discriminator: {}",
                discriminator
            )));
        },
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

    let decimals = spl_decimals_map.get(&mint).copied().ok_or_else(|| {
        AdapterError::InstructionParseError(format!("找不到 mint {} 的精度", mint))
    })?;

    let token_amount = TokenAmount {
        amount: amount.to_string(),
        ui_amount: amount as f64 / 10_f64.powi(decimals as i32),
        decimals,
    };

    // 获取余额信息
    let source_balance =
        token_balance_changes.get(&source).and_then(|(pre, _)| pre.as_ref().cloned());
    let source_pre_balance =
        token_balance_changes.get(&source).and_then(|(pre, _)| pre.as_ref().cloned());
    let destination_balance = token_balance_changes
        .get(&destination)
        .and_then(|(_, post)| post.as_ref().cloned());
    let destination_pre_balance = token_balance_changes
        .get(&destination)
        .and_then(|(_, post)| post.as_ref().cloned());

    let transfer_type = match discriminator {
        3 => "transfer",
        12 => "transferChecked",
        _ => "unknown",
    };

    Ok(super::types::TransferData {
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
        timestamp,
        signature,
    })
}

/// 提取指令和内部指令
pub fn extract_instructions(
    tx: &EncodedTransactionWithStatusMeta,
    account_keys: &[Pubkey],
) -> Result<
    (
        Vec<super::types::InstructionInfo>,
        Vec<super::types::InnerInstructionInfo>,
        Vec<serde_json::Value>,
    ),
    AdapterError,
> {
    let mut instructions = Vec::new();
    let mut inner_instructions = Vec::new();
    let mut inner_instructions_json = Vec::new();

    let tx_value = serde_json::to_value(tx).map_err(|e| AdapterError::JsonError(e.to_string()))?;

    // 提取外部指令
    if let Some(ixs) = tx_value["transaction"]["message"]["instructions"].as_array() {
        for (idx, ix_value) in ixs.iter().enumerate() {
            let program_id = if let Some(program_id_index) = ix_value["programIdIndex"].as_u64() {
                let index = program_id_index as usize;
                if index < account_keys.len() {
                    account_keys[index]
                } else {
                    continue;
                }
            } else if let Some(program_id_str) = ix_value["programId"].as_str() {
                if let Ok(pid) = Pubkey::from_str(program_id_str) {
                    pid
                } else {
                    continue;
                }
            } else {
                continue;
            };

            // 解析账户列表
            let accounts = if let Some(accounts_arr) = ix_value["accounts"].as_array() {
                accounts_arr
                    .iter()
                    .filter_map(|acc| {
                        if let Some(index) = acc.as_u64() {
                            let idx = index as usize;
                            if idx < account_keys.len() { Some(account_keys[idx]) } else { None }
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
            let data = if let Some(data_str) = ix_value["data"].as_str() {
                bs58::decode(data_str)
                    .into_vec()
                    .inspect_err(|e| warn!("指令数据 base58 解析失败 (指令索引 {}): {}", idx, e))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            instructions.push(super::types::InstructionInfo {
                program_id,
                accounts,
                data,
                index: idx,
                parsed_json: Some(ix_value.clone()),
            });
        }
    }

    // 提取内部指令
    if let Some(inner_instrs) = tx_value["meta"]["innerInstructions"].as_array() {
        for inner_set in inner_instrs {
            inner_instructions_json.push(inner_set.clone());

            let outer_index = inner_set["index"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("Missing inner instruction index"))?
                as usize;

            if let Some(instructions_arr) = inner_set["instructions"].as_array() {
                for (inner_idx, ix_json) in instructions_arr.iter().enumerate() {
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
                        bs58::decode(data_str)
                            .into_vec()
                            .inspect_err(|e| {
                                warn!(
                                    "内部指令数据 base58 解析失败 (外部索引 {}, 内部索引 {}): {}",
                                    outer_index, inner_idx, e
                                )
                            })
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };

                    inner_instructions.push(super::types::InnerInstructionInfo {
                        outer_index,
                        inner_index: inner_idx,
                        instruction: super::types::InstructionInfo {
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
