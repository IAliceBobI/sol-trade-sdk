//! TradingClient 辅助函数

use crate::TradeTokenType;

/// 获取输入代币的 mint 地址
pub fn get_input_mint(input_token_type: &TradeTokenType) -> solana_sdk::pubkey::Pubkey {
    match input_token_type {
        TradeTokenType::SOL => crate::SOL_TOKEN_ACCOUNT,
        TradeTokenType::WSOL => crate::WSOL_TOKEN_ACCOUNT,
        TradeTokenType::USDC => crate::USDC_TOKEN_ACCOUNT,
        TradeTokenType::USD1 => crate::USD1_TOKEN_ACCOUNT,
    }
}

/// 获取输出代币的 mint 地址
pub fn get_output_mint(output_type: &TradeTokenType) -> solana_sdk::pubkey::Pubkey {
    match output_type {
        TradeTokenType::SOL => crate::SOL_TOKEN_ACCOUNT,
        TradeTokenType::WSOL => crate::WSOL_TOKEN_ACCOUNT,
        TradeTokenType::USDC => crate::USDC_TOKEN_ACCOUNT,
        TradeTokenType::USD1 => crate::USD1_TOKEN_ACCOUNT,
    }
}

/// 检查 DEX 是否支持 quote
pub fn supports_quote(dex_type: &crate::DexType) -> bool {
    use crate::DexType;
    matches!(
        dex_type,
        DexType::RaydiumClmm | DexType::RaydiumCpmm | DexType::RaydiumAmmV4 | DexType::PumpSwap
    )
}
