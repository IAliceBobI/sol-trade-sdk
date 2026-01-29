use solana_hash::Hash;
use solana_sdk::{
    instruction::Instruction, message::AddressLookupTableAccount,
    native_token::sol_str_to_lamports, pubkey::Pubkey, signature::Keypair, signer::Signer,
    transaction::VersionedTransaction,
};
use solana_system_interface::instruction::transfer;
use std::str::FromStr;
use std::sync::Arc;

use super::{
    compute_budget_manager::compute_budget_instructions,
    nonce_manager::{add_nonce_instruction, get_transaction_blockhash},
};
use crate::{
    common::{SolanaRpcClient, nonce_cache::DurableNonceInfo},
    trading::{
        MiddlewareManager,
        core::transaction_pool::{acquire_builder, release_builder},
    },
};

/// Build standard RPC transaction
pub async fn build_transaction(
    payer: Arc<Keypair>,
    _rpc: Option<Arc<SolanaRpcClient>>,
    unit_limit: u32,
    unit_price: u64,
    business_instructions: Vec<Instruction>,
    address_lookup_table_account: Option<AddressLookupTableAccount>,
    recent_blockhash: Option<Hash>,
    middleware_manager: Option<Arc<MiddlewareManager>>,
    protocol_name: &str,
    is_buy: bool,
    with_tip: bool,
    tip_account: &Pubkey,
    tip_amount: f64,
    durable_nonce: Option<DurableNonceInfo>,
    enable_jito_sandwich_protection: bool,
    // nonce_account: Option<Pubkey>,
    // current_nonce: Option<Hash>,
) -> Result<VersionedTransaction, anyhow::Error> {
    let mut instructions = Vec::with_capacity(business_instructions.len() + 5);

    // Add nonce instruction
    add_nonce_instruction(&mut instructions, payer.as_ref(), durable_nonce.clone())?;

    // Add Jito 三明治攻击防护（如果启用）
    //
    // ## 为什么需要添加？
    //
    // 当启用三明治防护时，我们需要在交易中添加一个以 `jitodontfront` 开头的账户。
    // 这样 Jito Block Engine 会确保包含这个账户的交易必须在 Bundle 的第一位。
    //
    // ## 如何工作？
    //
    // 我们通过添加一个只读账户（而不是指令）来实现防护。这是 Jito 官方推荐的方式：
    // > "add any valid Solana public key that starts with jitodontfront to any of the instructions"
    //
    // 注意：虽然文档说"添加到指令中"，但实际上是通过在交易中包含这个账户来实现的。
    // 最简单的方式是创建一个无操作指令来携带这个账户。
    //
    // ## 不启用防护时
    //
    // 交易结构：
    // [nonce_instruction, tip_transfer, compute_budget, business_instructions...]
    //
    // ## 启用防护时
    //
    // 交易结构：
    // [nonce_instruction, jitodontfront_marker, tip_transfer, compute_budget, business_instructions...]
    //
    // ## 性能影响
    //
    // - 交易大小：+32 bytes（一个 Pubkey）
    // - Compute Unit：几乎无影响（只读账户）
    // - 执行速度：无影响
    let dont_front_account = if enable_jito_sandwich_protection {
        // 使用默认的 jitodontfront 账户
        // 用户也可以通过 generate_dont_front_account(Some("custom_suffix")) 生成自定义账户
        Some(crate::swqos::jito::generate_dont_front_account(None))
    } else {
        None
    };

    // 如果启用了三明治防护，添加一个标记指令
    // 这个指令不执行任何操作，只是将 jitodontfront 账户包含在交易中
    if let Some(account_str) = dont_front_account {
        let account = account_str.parse::<Pubkey>().map_err(|e| {
            anyhow::anyhow!("无效的 jitodontfront 账户地址 '{}': {}", account_str, e)
        })?;

        // System Program ID
        let system_program_id = Pubkey::from_str("11111111111111111111111111111111")
            .map_err(|e| anyhow::anyhow!("无效的 System Program ID: {}", e))?;

        // 创建一个无操作指令来携带 jitodontfront 账户
        // 我们使用 System Program 的 transfer 指令，但转账金额为 0
        // 这样就是一个安全的无操作指令，包含 jitodontfront 账户
        instructions.push(Instruction {
            program_id: system_program_id,
            // 将 jitodontfront 账户添加为只读账户
            accounts: vec![
                solana_sdk::instruction::AccountMeta::new(payer.pubkey(), true), // payer (签名者)
                solana_sdk::instruction::AccountMeta::new_readonly(account, false), // jitodontfront (只读)
            ],
            data: vec![0, 0, 0, 0], // System Program 的 transfer 指令，但 8 字节的 u64 金额为 0
        });
    }

    // Add tip transfer instruction
    if with_tip && tip_amount > 0.0 {
        let tip_lamports = sol_str_to_lamports(&tip_amount.to_string())
            .ok_or_else(|| anyhow::anyhow!("无效的小费金额 '{}': 转换失败", tip_amount))?;
        instructions.push(transfer(&payer.pubkey(), tip_account, tip_lamports));
    }

    // Add compute budget instructions
    instructions.extend(compute_budget_instructions(unit_price, unit_limit));

    // Add business instructions
    instructions.extend(business_instructions);

    // Get blockhash for transaction
    let blockhash = get_transaction_blockhash(recent_blockhash, durable_nonce.clone())?;

    // Build transaction
    build_versioned_transaction(
        payer,
        instructions,
        address_lookup_table_account,
        blockhash,
        middleware_manager,
        protocol_name,
        is_buy,
    )
    .await
}

/// Low-level function for building versioned transactions
async fn build_versioned_transaction(
    payer: Arc<Keypair>,
    instructions: Vec<Instruction>,
    address_lookup_table_account: Option<AddressLookupTableAccount>,
    blockhash: Hash,
    middleware_manager: Option<Arc<MiddlewareManager>>,
    protocol_name: &str,
    is_buy: bool,
) -> Result<VersionedTransaction, anyhow::Error> {
    let full_instructions = match middleware_manager {
        Some(middleware_manager) => middleware_manager
            .apply_middlewares_process_full_instructions(
                instructions,
                protocol_name.to_string(),
                is_buy,
            )?,
        None => instructions,
    };

    // 使用预分配的交易构建器以降低延迟
    let mut builder = acquire_builder();

    let versioned_msg = builder.build_zero_alloc(
        &payer.pubkey(),
        &full_instructions,
        address_lookup_table_account,
        blockhash,
    );

    let msg_bytes = versioned_msg.serialize();
    let signature = payer
        .try_sign_message(&msg_bytes)
        .expect("交易签名失败：payer 密钥无效或消息序列化错误");
    let tx = VersionedTransaction { signatures: vec![signature], message: versioned_msg };

    // 归还构建器到池
    release_builder(builder);

    Ok(tx)
}
