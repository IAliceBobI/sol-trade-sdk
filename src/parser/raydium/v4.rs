//! Raydium AMM V4 交易解析器
//!
//! 基于 Transfer 记录解析，不使用事件
//! 参考 solana-dex-parser 的 RaydiumParser 实现

use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;

use crate::parser::{
    transaction_adapter::TransactionAdapter,
    base_parser::{DexParserTrait, ParseError},
    types::{ParsedTradeInfo, TradeType, TokenInfo, DexProtocol},
    constants::discriminators::raydium_v4,
};

/// Transfer 记录
#[derive(Debug, Clone)]
struct TransferRecord {
    pub mint: Pubkey,
    pub source: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
    pub decimals: u8,
    pub authority: Option<Pubkey>,
}

/// Raydium V4 解析器
pub struct RaydiumV4Parser;

impl RaydiumV4Parser {
    pub fn new() -> Self {
        Self
    }

    /// 判断是否是 Swap 指令（排除流动性操作）
    fn is_swap_instruction(&self, data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }
        // Raydium V4 Swap 的 discriminator 是 9
        data[0] == raydium_v4::SWAP
    }

    /// 从账户列表中提取池地址
    /// Raydium V4: accounts[1] 是池地址
    fn extract_pool_address(&self, accounts: &[Pubkey]) -> Result<Pubkey, ParseError> {
        if accounts.len() < 15 {
            return Err(ParseError::MissingAccountData);
        }
        Ok(accounts[1])
    }

    /// 获取指令相关的 Transfer 记录
    fn get_transfers_for_instruction(
        &self,
        adapter: &TransactionAdapter,
        outer_index: usize,
    ) -> Result<Vec<TransferRecord>, ParseError> {
        let transfer_data_list = adapter.get_transfers_for_instruction(outer_index);

        // 将 TransferData 转换为 TransferRecord
        let mut records = Vec::new();
        for td in transfer_data_list {
            let amount: u64 = td.token_amount.amount.parse()
                .map_err(|_| ParseError::ParseFailed(format!("无效的数量: {}", td.token_amount.amount)))?;

            records.push(TransferRecord {
                mint: td.mint,
                source: td.source,
                destination: td.destination,
                amount,
                decimals: td.token_amount.decimals,
                authority: td.authority,
            });
        }

        Ok(records)
    }

    /// 从 Transfer 记录中提取唯一的代币
    fn extract_unique_tokens<'a>(&self, transfers: &'a [TransferRecord]) -> Vec<&'a TransferRecord> {
        let mut seen = std::collections::HashSet::new();
        let mut unique = Vec::new();

        for transfer in transfers {
            if seen.insert(transfer.mint) {
                unique.push(transfer);
            }
        }

        unique
    }

    /// 判断交易类型（买入/卖出）
    fn determine_trade_type<'a>(
        &self,
        user: Pubkey,
        transfers: &'a [TransferRecord],
    ) -> Result<(TradeType, &'a TransferRecord, &'a TransferRecord), ParseError> {
        // SOL 的 mint 地址
        let sol_mint = "So11111111111111111111111111111111111111112"
            .parse::<Pubkey>()
            .unwrap();

        // USDC 的 mint 地址 (常见的报价币)
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            .parse::<Pubkey>()
            .unwrap();

        let (input_transfer, output_transfer, trade_type) = if transfers.len() >= 2 {
            let t0 = &transfers[0];
            let t1 = &transfers[1];

            // 使用 source/destination 来判断交易方向
            // 检查 t0：如果 source 是用户，说明用户在转出 t0
            let t0_is_from_user = t0.source == user || t0.authority == Some(user);
            let t1_is_from_user = t1.source == user || t1.authority == Some(user);

            if t0_is_from_user && !t1_is_from_user {
                // t0 从用户转出，t1 转入用户
                // 如果转出的是 SOL 或 USDC，则是买入
                if t0.mint == sol_mint || t0.mint == usdc_mint {
                    (t0, t1, TradeType::Buy)  // 用 SOL/USDC 买入
                } else {
                    (t0, t1, TradeType::Sell) // 卖出代币获得其他
                }
            } else if t1_is_from_user && !t0_is_from_user {
                // t1 从用户转出，t0 转入用户
                if t1.mint == sol_mint || t1.mint == usdc_mint {
                    (t1, t0, TradeType::Buy)  // 用 SOL/USDC 买入
                } else {
                    (t1, t0, TradeType::Sell) // 卖出代币
                }
            } else {
                // 两者都是从用户转出（或都不是），尝试通过 mint 判断
                if t0.mint == sol_mint || t0.mint == usdc_mint {
                    (t0, t1, TradeType::Buy)
                } else if t1.mint == sol_mint || t1.mint == usdc_mint {
                    (t1, t0, TradeType::Sell)
                } else {
                    // 无法通过 SOL/USDC 判断，默认使用 t0 作为输入
                    (t0, t1, TradeType::Swap)
                }
            }
        } else {
            return Err(ParseError::ParseFailed("Transfer 记录不足".into()));
        };

        Ok((trade_type, input_transfer, output_transfer))
    }

    /// 从 Transfer 记录构建 TokenInfo
    fn transfer_to_tokeninfo(
        &self,
        transfer: &TransferRecord,
        user: Pubkey,
    ) -> TokenInfo {
        let amount = transfer.amount as f64 / 10_f64.powi(transfer.decimals as i32);

        TokenInfo {
            mint: transfer.mint,
            amount,
            amount_raw: transfer.amount.to_string(),
            decimals: transfer.decimals,
            authority: Some(user),
            source: Some(transfer.source),
            destination: Some(transfer.destination),
        }
    }
}

#[async_trait]
impl DexParserTrait for RaydiumV4Parser {
    async fn parse(&self, adapter: &TransactionAdapter) -> Result<Vec<ParsedTradeInfo>, ParseError> {
        let mut trades = Vec::new();

        let program_id_str = DexProtocol::RaydiumV4.program_id();
        let program_pubkey = program_id_str.parse()
            .map_err(|_| ParseError::UnsupportedProtocol("Invalid Raydium V4 program ID".to_string()))?;

        let instructions = adapter.get_instructions_by_program(&program_pubkey);

        for instr in instructions {
            // 过滤非 Swap 指令
            if !self.is_swap_instruction(&instr.data) {
                continue;
            }

            // 获取 Transfer 记录
            let transfers = self.get_transfers_for_instruction(adapter, instr.index)?;

            // 至少需要 2 个 transfer
            if transfers.len() < 2 {
                continue;
            }

            // 提取池地址
            let pool = self.extract_pool_address(&instr.accounts)?;

            // 从 Transfer 的 authority 中提取用户地址
            // 而不是从账户列表中提取,因为账户列表中的用户地址可能不准确
            let user = transfers.iter()
                .find_map(|t| t.authority)
                .ok_or(ParseError::ParseFailed("无法从 Transfer 记录中提取用户地址".into()))?;

            // 提取唯一代币
            let unique_transfers = self.extract_unique_tokens(&transfers);
            if unique_transfers.len() < 2 {
                continue;
            }

            // 判断交易类型
            let (trade_type, input_transfer, output_transfer) =
                self.determine_trade_type(user, &transfers)?;

            // 构建 TokenInfo
            let input_token = self.transfer_to_tokeninfo(input_transfer, user);
            let output_token = self.transfer_to_tokeninfo(output_transfer, user);

            // 检测费用（第3个 transfer）
            let fee = if transfers.len() > 2 {
                Some(self.transfer_to_tokeninfo(&transfers[2], user))
            } else {
                None
            };

            trades.push(ParsedTradeInfo {
                user,
                trade_type,
                pool,
                input_token,
                output_token,
                fee,
                fees: vec![],
                dex: DexProtocol::RaydiumV4.name().to_string(),
                signature: adapter.signature.clone(),
                slot: adapter.slot,
                timestamp: adapter.timestamp,
            });
        }

        if trades.is_empty() {
            return Err(ParseError::ParseFailed(
                "未找到有效的 Raydium V4 交易".to_string(),
            ));
        }

        Ok(trades)
    }

    fn protocol(&self) -> DexProtocol {
        DexProtocol::RaydiumV4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raydium_v4_parser_creation() {
        let parser = RaydiumV4Parser::new();
        assert_eq!(parser.protocol(), DexProtocol::RaydiumV4);
    }

    #[test]
    fn test_is_swap_instruction() {
        let parser = RaydiumV4Parser::new();

        // Swap 指令
        assert!(parser.is_swap_instruction(&[raydium_v4::SWAP]));

        // 非 Swap 指令
        assert!(!parser.is_swap_instruction(&[raydium_v4::ADD_LIQUIDITY]));
        assert!(!parser.is_swap_instruction(&[raydium_v4::REMOVE_LIQUIDITY]));
    }
}
