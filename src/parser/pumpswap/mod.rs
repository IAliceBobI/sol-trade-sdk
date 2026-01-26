//! PumpSwap 交易解析器
//!
//! 参考 solana-dex-parser 的 PumpswapParser 实现

pub mod events;

use async_trait::async_trait;

use crate::parser::pumpswap::events::{parse_pumpswap_event, EventData, PumpswapEventType};
use crate::parser::{
    base_parser::{DexParserTrait, ParseError},
    transaction_adapter::TransactionAdapter,
    types::{DexProtocol, ParsedTradeInfo, TokenInfo, TradeType},
};

/// PumpSwap 解析器
pub struct PumpswapParser;

impl PumpswapParser {
    pub fn new() -> Self {
        Self
    }

    /// 解析买入事件为交易信息
    fn parse_buy_event(
        &self,
        event: &crate::parser::pumpswap::events::PumpswapBuyEvent,
        adapter: &TransactionAdapter,
    ) -> Result<ParsedTradeInfo, ParseError> {
        // 获取代币信息
        let input_mint = *adapter
            .get_token_mint(&event.user_quote_token_account)
            .ok_or(ParseError::MissingTokenInfo)?;

        // 对于新创建的账户，尝试从转账动作中查找 mint
        let output_mint = if let Some(mint) = adapter.get_token_mint(&event.user_base_token_account)
        {
            *mint
        } else {
            // 尝试从转账动作中查找
            let transfers = adapter.get_transfer_actions();
            let mut found_mint = None;

            for transfer in &transfers {
                if transfer.destination == event.user_base_token_account {
                    found_mint = Some(transfer.mint);
                    break;
                }
            }

            found_mint.ok_or(ParseError::MissingTokenInfo)?
        };

        let input_decimals =
            adapter.get_mint_decimals(&input_mint).ok_or(ParseError::MissingTokenInfo)?;

        let output_decimals =
            adapter.get_mint_decimals(&output_mint).ok_or(ParseError::MissingTokenInfo)?;

        // 计算实际数量（考虑精度）
        // 使用 quote_amount_in_with_lp_fee 作为输入金额，因为这是用户实际向池转账的金额
        // LP 费用包含在输入金额中（给流动性提供者），协议费用单独转账
        let input_amount =
            event.quote_amount_in_with_lp_fee as f64 / 10_f64.powi(input_decimals as i32);
        let output_amount = event.base_amount_out as f64 / 10_f64.powi(output_decimals as i32);

        let input_token = TokenInfo {
            mint: input_mint,
            amount: input_amount,
            amount_raw: event.quote_amount_in_with_lp_fee.to_string(),
            decimals: input_decimals,
            authority: Some(event.user),
            source: Some(event.user_quote_token_account),
            destination: None,
        };

        let output_token = TokenInfo {
            mint: output_mint,
            amount: output_amount,
            amount_raw: event.base_amount_out.to_string(),
            decimals: output_decimals,
            authority: Some(event.user),
            source: None,
            destination: Some(event.user_base_token_account),
        };

        // 手续费：只显示协议费用，因为 LP 费用已经包含在输入金额中
        let fee = if event.protocol_fee > 0 {
            Some(TokenInfo {
                mint: input_mint,
                amount: event.protocol_fee as f64 / 10_f64.powi(input_decimals as i32),
                amount_raw: event.protocol_fee.to_string(),
                decimals: input_decimals,
                authority: None,
                source: None,
                destination: None,
            })
        } else {
            None
        };

        Ok(ParsedTradeInfo {
            user: event.user,
            trade_type: TradeType::Buy,
            pool: event.pool,
            input_token,
            output_token,
            fee,
            fees: vec![],
            dex: DexProtocol::PumpSwap.name().to_string(),
            signature: adapter.signature.clone(),
            slot: adapter.slot,
            timestamp: adapter.timestamp,
        })
    }

    /// 解析卖出事件为交易信息
    fn parse_sell_event(
        &self,
        event: &crate::parser::pumpswap::events::PumpswapSellEvent,
        adapter: &TransactionAdapter,
    ) -> Result<ParsedTradeInfo, ParseError> {
        let input_mint = if let Some(mint) = adapter.get_token_mint(&event.user_base_token_account)
        {
            *mint
        } else {
            // 尝试从转账动作中查找
            let transfers = adapter.get_transfer_actions();
            let mut found_mint = None;

            for transfer in &transfers {
                if transfer.source == event.user_base_token_account {
                    found_mint = Some(transfer.mint);
                    break;
                }
            }

            found_mint.ok_or(ParseError::MissingTokenInfo)?
        };

        let output_mint = *adapter
            .get_token_mint(&event.user_quote_token_account)
            .ok_or(ParseError::MissingTokenInfo)?;

        let input_decimals =
            adapter.get_mint_decimals(&input_mint).ok_or(ParseError::MissingTokenInfo)?;

        let output_decimals =
            adapter.get_mint_decimals(&output_mint).ok_or(ParseError::MissingTokenInfo)?;

        let input_amount = event.base_amount_in as f64 / 10_f64.powi(input_decimals as i32);
        let output_amount = event.quote_amount_out as f64 / 10_f64.powi(output_decimals as i32);

        let input_token = TokenInfo {
            mint: input_mint,
            amount: input_amount,
            amount_raw: event.base_amount_in.to_string(),
            decimals: input_decimals,
            authority: Some(event.user),
            source: Some(event.user_base_token_account),
            destination: None,
        };

        let output_token = TokenInfo {
            mint: output_mint,
            amount: output_amount,
            amount_raw: event.quote_amount_out.to_string(),
            decimals: output_decimals,
            authority: Some(event.user),
            source: None,
            destination: Some(event.user_quote_token_account),
        };

        let fee = if event.lp_fee > 0 || event.protocol_fee > 0 {
            let total_fee = event.lp_fee + event.protocol_fee;
            Some(TokenInfo {
                mint: output_mint,
                amount: total_fee as f64 / 10_f64.powi(output_decimals as i32),
                amount_raw: total_fee.to_string(),
                decimals: output_decimals,
                authority: None,
                source: None,
                destination: None,
            })
        } else {
            None
        };

        Ok(ParsedTradeInfo {
            user: event.user,
            trade_type: TradeType::Sell,
            pool: event.pool,
            input_token,
            output_token,
            fee,
            fees: vec![],
            dex: DexProtocol::PumpSwap.name().to_string(),
            signature: adapter.signature.clone(),
            slot: adapter.slot,
            timestamp: adapter.timestamp,
        })
    }
}

#[async_trait]
impl DexParserTrait for PumpswapParser {
    async fn parse(
        &self,
        adapter: &TransactionAdapter,
    ) -> Result<Vec<ParsedTradeInfo>, ParseError> {
        let mut trades = Vec::new();

        // 获取 PumpSwap 程序的所有指令
        let program_id_str = DexProtocol::PumpSwap.program_id();
        let program_pubkey = program_id_str.parse().map_err(|_| {
            ParseError::UnsupportedProtocol("Invalid PumpSwap program ID".to_string())
        })?;

        // 从内部指令中查找事件（事件在 CPI 调用中发出）
        let inner_instructions = adapter.get_inner_instructions_by_program(&program_pubkey);

        // 解析每个指令中的事件
        for inner_ix in inner_instructions {
            // 尝试解析事件
            if let Some((event_type, event_data)) = parse_pumpswap_event(&inner_ix.instruction.data)
            {
                let trade = match event_type {
                    PumpswapEventType::Buy => {
                        if let EventData::Buy(ref buy_event) = event_data {
                            self.parse_buy_event(buy_event, adapter)?
                        } else {
                            continue;
                        }
                    }
                    PumpswapEventType::Sell => {
                        if let EventData::Sell(ref sell_event) = event_data {
                            self.parse_sell_event(sell_event, adapter)?
                        } else {
                            continue;
                        }
                    }
                    _ => continue, // 跳过其他事件类型
                };
                trades.push(trade);
            }
        }

        if trades.is_empty() {
            return Err(ParseError::ParseFailed("未找到有效的 PumpSwap 交易事件".to_string()));
        }

        Ok(trades)
    }

    fn protocol(&self) -> DexProtocol {
        DexProtocol::PumpSwap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pumpswap_parser_creation() {
        let parser = PumpswapParser::new();
        assert_eq!(parser.protocol(), DexProtocol::PumpSwap);
    }
}
