//! TradingClient WSOL 相关方法

use super::types::TradingClient;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

impl TradingClient {
    /// 将原生 SOL 包装为 wSOL（Wrapped SOL），用于 SPL 代币操作
    ///
    /// 此函数创建 wSOL 关联代币账户（如果不存在），
    /// 将指定数量的 SOL 转移到该账户，然后同步原生代币余额，
    /// 使 SOL 可用作交易操作中的 SPL 代币。
    ///
    /// # Arguments
    /// * `amount` - 要包装的 SOL 数量（lamports）
    ///
    /// # Returns
    /// * `Ok(String)` - 交易签名（如果成功）
    /// * `Err(anyhow::Error)` - 如果交易执行失败
    ///
    /// # Errors
    ///
    /// 此函数在以下情况下会返回错误：
    /// - SOL 余额不足以进行包装操作
    /// - wSOL 关联代币账户创建失败
    /// - 交易执行或确认失败
    /// - 网络或 RPC 错误
    pub async fn wrap_sol_to_wsol(&self, amount: u64) -> Result<String, anyhow::Error> {
        use crate::trading::common::wsol_manager::handle_wsol;

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let instructions = handle_wsol(&self.payer.pubkey(), amount);
        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey()));
        transaction.sign(&[&*self.payer], recent_blockhash);
        let signature = self.rpc.send_and_confirm_transaction(&transaction).await?;
        Ok(signature.to_string())
    }

    /// 关闭 wSOL 关联代币账户，并将剩余余额解包为原生 SOL
    ///
    /// 此函数关闭 wSOL 关联代币账户，自动将任何剩余的 wSOL 余额作为原生 SOL
    /// 转回账户所有者。这对于清理 wSOL 账户和在交易操作后回收包装的 SOL 很有用。
    ///
    /// # Returns
    /// * `Ok(String)` - 交易签名（如果成功）
    /// * `Err(anyhow::Error)` - 如果交易执行失败
    ///
    /// # Errors
    ///
    /// 此函数在以下情况下会返回错误：
    /// - wSOL 关联代币账户不存在
    /// - 由于权限不足，账户关闭失败
    /// - 交易执行或确认失败
    /// - 网络或 RPC 错误
    pub async fn close_wsol(&self) -> Result<String, anyhow::Error> {
        use crate::trading::common::wsol_manager::close_wsol;

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let instructions = close_wsol(&self.payer.pubkey());
        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey()));
        transaction.sign(&[&*self.payer], recent_blockhash);
        let signature = self.rpc.send_and_confirm_transaction(&transaction).await?;
        Ok(signature.to_string())
    }

    /// 创建 wSOL 关联代币账户（ATA），但不包装任何 SOL
    ///
    /// 此函数仅为付款人创建 wSOL 关联代币账户，
    /// 而不将任何 SOL 转移到其中。当你想要提前设置
    /// 账户基础设施但尚未承诺资金时，这很有用。
    ///
    /// # Returns
    /// * `Ok(String)` - 交易签名（如果成功）
    /// * `Err(anyhow::Error)` - 如果交易执行失败
    ///
    /// # Errors
    ///
    /// 此函数在以下情况下会返回错误：
    /// - wSOL ATA 账户已存在（幂等，将静默成功）
    /// - 交易执行或确认失败
    /// - 网络或 RPC 错误
    /// - 交易费用的 SOL 不足
    pub async fn create_wsol_ata(&self) -> Result<String, anyhow::Error> {
        use crate::trading::common::wsol_manager::create_wsol_ata;

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let instructions = create_wsol_ata(&self.payer.pubkey());

        // If instructions are empty, ATA already exists
        if instructions.is_empty() {
            return Err(anyhow::anyhow!("wSOL ATA already exists or no instructions needed"));
        }

        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey()));
        transaction.sign(&[&*self.payer], recent_blockhash);
        let signature = self.rpc.send_and_confirm_transaction(&transaction).await?;
        Ok(signature.to_string())
    }

    /// 将 WSOL 转换为 SOL，使用 seed 账户
    ///
    /// 这个函数实现以下步骤：
    /// 1. 使用 super::seed::create_associated_token_account_use_seed 创建 WSOL seed 账号
    /// 2. 使用 get_associated_token_address_with_program_id_use_seed 获取该账号的 ATA 地址
    /// 3. 添加从用户 WSOL ATA 转账到该 seed ATA 账号的指令
    /// 4. 添加关闭 WSOL seed 账号的指令
    ///
    /// # Arguments
    /// * `amount` - 要转换的 WSOL 数量（以 lamports 为单位）
    ///
    /// # Returns
    /// * `Ok(String)` - 交易签名
    /// * `Err(anyhow::Error)` - 如果交易执行失败
    ///
    /// # Errors
    ///
    /// 此函数在以下情况下会返回错误：
    /// - 用户 WSOL ATA 中余额不足
    /// - seed 账户创建失败
    /// - 转账指令执行失败
    /// - 交易执行或确认失败
    /// - 网络或 RPC 错误
    pub async fn wrap_wsol_to_sol(&self, amount: u64) -> Result<String, anyhow::Error> {
        use crate::common::seed::get_associated_token_address_with_program_id_use_seed;
        use crate::trading::common::wsol_manager::{
            wrap_wsol_to_sol as wrap_wsol_to_sol_internal, wrap_wsol_to_sol_without_create,
        };

        // 检查临时seed账户是否已存在
        let seed_ata_address = get_associated_token_address_with_program_id_use_seed(
            &self.payer.pubkey(),
            &crate::constants::WSOL_TOKEN_ACCOUNT,
            &crate::constants::TOKEN_PROGRAM,
        )?;

        let account_exists = self.rpc.get_account(&seed_ata_address).await.is_ok();

        let instructions = if account_exists {
            // 如果账户已存在，使用不创建账户的版本
            wrap_wsol_to_sol_without_create(&self.payer.pubkey(), amount)?
        } else {
            // 如果账户不存在，使用创建账户的版本
            wrap_wsol_to_sol_internal(&self.payer.pubkey(), amount)?
        };

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey()));
        transaction.sign(&[&*self.payer], recent_blockhash);
        let signature = self.rpc.send_and_confirm_transaction(&transaction).await?;
        Ok(signature.to_string())
    }
}
