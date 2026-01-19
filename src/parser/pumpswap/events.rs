//! PumpSwap 事件解析
//!
//! 参考 solana-dex-parser 的 PumpswapEventParser 实现

use solana_sdk::pubkey::Pubkey;
use crate::parser::{
    utils::BinaryReader,
    constants::discriminators::pumpswap,
};

/// PumpSwap 事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PumpswapEventType {
    Create,
    Buy,
    Sell,
    AddLiquidity,
    RemoveLiquidity,
}

/// PumpSwap 买入事件数据
#[derive(Debug, Clone)]
pub struct PumpswapBuyEvent {
    pub timestamp: i64,
    pub base_amount_out: u64,
    pub quote_amount_in: u64,
    pub quote_amount_in_with_lp_fee: u64,
    pub lp_fee: u64,
    pub protocol_fee: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
}

/// PumpSwap 卖出事件数据
#[derive(Debug, Clone)]
pub struct PumpswapSellEvent {
    pub timestamp: i64,
    pub base_amount_in: u64,
    pub quote_amount_out: u64,
    pub quote_amount_out_without_lp_fee: u64,
    pub lp_fee: u64,
    pub protocol_fee: u64,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
}

/// 事件数据枚举
#[derive(Debug, Clone)]
pub enum EventData {
    Buy(PumpswapBuyEvent),
    Sell(PumpswapSellEvent),
}

/// 解析 PumpSwap 事件
///
/// # 参数
/// - `data`: 完整事件数据（包含 16 字节 discriminator）
///
/// # 返回
/// 事件类型和解析后的数据
pub fn parse_pumpswap_event(data: &[u8]) -> Option<(PumpswapEventType, EventData)> {
    if data.len() < 16 {
        return None;
    }

    let discriminator = &data[0..16];

    // 匹配事件类型
    if discriminator == &pumpswap::BUY_EVENT {
        parse_buy_event(&data[16..])
            .map(|evt| (PumpswapEventType::Buy, EventData::Buy(evt)))
    } else if discriminator == &pumpswap::SELL_EVENT {
        parse_sell_event(&data[16..])
            .map(|evt| (PumpswapEventType::Sell, EventData::Sell(evt)))
    } else if discriminator == &pumpswap::CREATE_POOL_EVENT {
        parse_create_event(&data[16..])
            .map(|_| (PumpswapEventType::Create, EventData::Buy(create_dummy_buy())))
    } else {
        None
    }
}

/// 解析买入事件
fn parse_buy_event(data: &[u8]) -> Option<PumpswapBuyEvent> {
    let mut reader = BinaryReader::new(data.to_vec());

    let timestamp = reader.read_i64().ok()?;
    let base_amount_out = reader.read_u64().ok()?;
    let _max_quote_amount_in = reader.read_u64().ok()?;
    let _user_base_reserves = reader.read_u64().ok()?;
    let _user_quote_reserves = reader.read_u64().ok()?;
    let _pool_base_reserves = reader.read_u64().ok()?;
    let _pool_quote_reserves = reader.read_u64().ok()?;
    let quote_amount_in = reader.read_u64().ok()?;
    let _lp_fee_basis_points = reader.read_u64().ok()?;
    let lp_fee = reader.read_u64().ok()?;
    let _protocol_fee_basis_points = reader.read_u64().ok()?;
    let protocol_fee = reader.read_u64().ok()?;
    let _quote_amount_in_with_lp_fee = reader.read_u64().ok()?;
    let _user_quote_amount_in = reader.read_u64().ok()?;
    let pool = reader.read_pubkey().ok()?;
    let user = reader.read_pubkey().ok()?;
    let user_base_token_account = reader.read_pubkey().ok()?;
    let user_quote_token_account = reader.read_pubkey().ok()?;
    let _protocol_fee_recipient = reader.read_pubkey().ok()?;
    let _protocol_fee_recipient_token_account = reader.read_pubkey().ok()?;

    Some(PumpswapBuyEvent {
        timestamp,
        base_amount_out,
        quote_amount_in,
        quote_amount_in_with_lp_fee: quote_amount_in + lp_fee,
        lp_fee,
        protocol_fee,
        pool,
        user,
        user_base_token_account,
        user_quote_token_account,
    })
}

/// 解析卖出事件
fn parse_sell_event(data: &[u8]) -> Option<PumpswapSellEvent> {
    let mut reader = BinaryReader::new(data.to_vec());

    let timestamp = reader.read_i64().ok()?;
    let base_amount_in = reader.read_u64().ok()?;
    let _min_quote_amount_out = reader.read_u64().ok()?;
    let _user_base_reserves = reader.read_u64().ok()?;
    let _user_quote_reserves = reader.read_u64().ok()?;
    let _pool_base_reserves = reader.read_u64().ok()?;
    let _pool_quote_reserves = reader.read_u64().ok()?;
    let quote_amount_out = reader.read_u64().ok()?;
    let _lp_fee_basis_points = reader.read_u64().ok()?;
    let lp_fee = reader.read_u64().ok()?;
    let _protocol_fee_basis_points = reader.read_u64().ok()?;
    let protocol_fee = reader.read_u64().ok()?;
    let _quote_amount_out_without_lp_fee = reader.read_u64().ok()?;
    let _user_quote_amount_out = reader.read_u64().ok()?;
    let pool = reader.read_pubkey().ok()?;
    let user = reader.read_pubkey().ok()?;
    let user_base_token_account = reader.read_pubkey().ok()?;
    let user_quote_token_account = reader.read_pubkey().ok()?;
    let _protocol_fee_recipient = reader.read_pubkey().ok()?;
    let _protocol_fee_recipient_token_account = reader.read_pubkey().ok()?;

    Some(PumpswapSellEvent {
        timestamp,
        base_amount_in,
        quote_amount_out,
        quote_amount_out_without_lp_fee: quote_amount_out + lp_fee,
        lp_fee,
        protocol_fee,
        pool,
        user,
        user_base_token_account,
        user_quote_token_account,
    })
}

/// 解析创建池事件（简化版）
fn parse_create_event(_data: &[u8]) -> Option<()> {
    Some(())
}

/// 创建空的买入事件（用于占位）
fn create_dummy_buy() -> PumpswapBuyEvent {
    let data = pumpswap::BUY_EVENT.to_vec();
    let _ = data; // 暂时忽略，避免 unused 警告

    PumpswapBuyEvent {
        timestamp: 0,
        base_amount_out: 0,
        quote_amount_in: 0,
        quote_amount_in_with_lp_fee: 0,
        lp_fee: 0,
        protocol_fee: 0,
        pool: Pubkey::default(),
        user: Pubkey::default(),
        user_base_token_account: Pubkey::default(),
        user_quote_token_account: Pubkey::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discriminator_match() {
        assert_eq!(
            &pumpswap::BUY_EVENT[..16],
            &[228, 69, 165, 46, 81, 203, 154, 29, 103, 244, 82, 31, 44, 245, 119, 119]
        );
    }

    #[test]
    fn test_parse_empty_data() {
        let result = parse_pumpswap_event(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_insufficient_data() {
        let data = vec![1, 2, 3];  // 少于 16 字节
        let result = parse_pumpswap_event(&data);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_buy_event_discriminator_only() {
        // 只有 discriminator，没有事件数据
        let data = pumpswap::BUY_EVENT.to_vec();
        let result = parse_pumpswap_event(&data);
        // 应该能识别 discriminator，但解析事件数据会失败
        assert!(result.is_none());
    }
}
