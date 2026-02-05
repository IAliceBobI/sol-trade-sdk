//! # 代币余额查询辅助函数

use crate::common::SolanaRpcClient;
use anyhow::{Result, anyhow};
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

/// 获取代币余额
pub async fn get_token_balance(rpc: &Arc<SolanaRpcClient>, token_account: &Pubkey) -> Result<u64> {
    match rpc.get_token_account_balance(token_account).await {
        Ok(balance) => {
            let balance_u64 = balance
                .amount
                .parse::<u64>()
                .map_err(|_| anyhow!("Failed to parse token balance"))?;
            Ok(balance_u64)
        },
        Err(_) => Ok(0), // 账户不存在时返回 0
    }
}
