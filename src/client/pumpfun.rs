//! TradingClient PumpFun 相关方法

use super::types::TradingClient;
use solana_sdk::message::Message;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::sync::Arc;

impl TradingClient {
    /// 在 PumpFun bonding curve 上创建新代币
    ///
    /// 此函数在 PumpFun 上创建新的 SPL 代币并初始化其 bonding curve。
    /// 您可以选择使用传统的 `create` 指令（Token program）或
    /// 较新的 `create_v2` 指令（Token2022，支持 Mayhem 模式）。
    ///
    /// # Arguments
    /// * `name` - 代币名称
    /// * `symbol` - 代币符号（最多 10 个字符）
    /// * `uri` - 元数据 URI（JSON 元数据 URL）
    /// * `use_v2` - 是否使用 create_v2（Token2022 + Mayhem 支持）。如果为 false，使用传统 create
    /// * `is_mayhem_mode` - 是否启用 Mayhem 模式（仅适用于 create_v2）
    ///
    /// # Returns
    /// * `Ok((Pubkey, String))` - 元组（mint 地址，交易签名）（如果成功）
    /// * `Err(anyhow::Error)` - 如果交易执行失败
    ///
    /// # Errors
    ///
    /// 此函数在以下情况下会返回错误：
    /// - 代币名称或符号为空
    /// - 符号超过 10 个字符
    /// - Mint 密钥对生成失败
    /// - 交易执行或确认失败
    /// - 网络或 RPC 错误
    pub async fn create_pumpfun_token(
        &self,
        name: String,
        symbol: String,
        uri: String,
        use_v2: bool,
        is_mayhem_mode: bool,
    ) -> Result<(solana_sdk::pubkey::Pubkey, String), anyhow::Error> {
        use crate::instruction::pumpfun::{CreateTokenParams, PumpFunInstructionBuilder};

        // 验证输入
        if name.trim().is_empty() {
            return Err(anyhow::anyhow!("Token name cannot be empty"));
        }
        if symbol.trim().is_empty() {
            return Err(anyhow::anyhow!("Token symbol cannot be empty"));
        }
        if symbol.len() > 10 {
            return Err(anyhow::anyhow!("Token symbol must be 10 characters or less"));
        }
        if use_v2 && is_mayhem_mode {
            // Mayhem 模式是实验性的和高风险的
            // 我们允许它，但在这里不强制任何限制
        }

        // 生成 mint 密钥对
        let mint = Arc::new(Keypair::new());

        // 构建 create 指令
        let create_params = CreateTokenParams {
            mint: mint.clone(),
            name,
            symbol,
            uri,
            creator: self.payer.pubkey(),
            use_v2,
            is_mayhem_mode,
        };

        let instruction = if use_v2 {
            PumpFunInstructionBuilder::build_create_v2_instruction(&create_params)?
        } else {
            PumpFunInstructionBuilder::build_create_instruction(&create_params)?
        };

        // 构建并发送交易
        // Reference: pumpfun-bonkfun-bot uses Transaction([payer, mint_keypair], message, recent_blockhash)
        // Signers order: payer first (as fee payer), then mint (as instruction signer)
        let recent_blockhash = self.rpc.get_latest_blockhash().await?;

        // 首先构建 message，然后创建带有签名者的交易
        // Reference: pumpfun-bonkfun-bot uses Transaction([payer, mint_keypair], message, recent_blockhash)
        // Signers order: payer first (as fee payer), then mint (as instruction signer)

        // 为什么需要 Message？
        // 在 Solana 中，Transaction 由两部分组成：
        // 1. Message: 包含交易的逻辑信息（指令、账户、fee payer、blockhash 等）
        // 2. signatures: 签名数组
        //
        // 为什么使用 Message::new() + Transaction::new_unsigned()？
        // - 需要精确控制签名者顺序：payer 作为 fee payer（必须在 message.account_keys[0]），
        //   mint 作为 instruction signer（在指令账户列表中标记为 signer）
        // - 如果使用 Transaction::new_with_payer()，签名顺序可能不符合要求
        //
        // 与 IDL 的关系：
        // - IDL 文件定义了程序的接口（指令名称、参数、账户结构），主要用于代码生成和接口定义
        // - 这里手动构建了 instruction（通过 build_create_instruction），不依赖 IDL 来创建 Message
        // - IDL 不直接参与运行时交易构建，Message 的创建使用的是 Solana SDK 的底层 API
        let message = Message::new(&[instruction], Some(&self.payer.pubkey()));

        // 使用正确的签名者顺序创建交易：[payer, mint]
        // payer is fee payer (first in message.account_keys), mint is instruction signer
        let mut transaction = Transaction::new_unsigned(message);

        // 签名交易：payer first (as fee payer)，then mint (as instruction signer)
        transaction.sign(&[&*self.payer, &*mint], recent_blockhash);

        let signature = self.rpc.send_and_confirm_transaction(&transaction).await?;

        Ok((mint.pubkey(), signature.to_string()))
    }
}
