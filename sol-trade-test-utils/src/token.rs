//! Token 相关功能

use serde_json::Value;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::sync::Arc;

// 重新导出 SDK 的 MintInfo
pub use sol_trade_sdk::utils::token::MintInfo;

/// 查询 mint 的 decimals 和 token_program
pub async fn get_mint_info(rpc_client: &Arc<RpcClient>, mint: &Pubkey) -> Result<MintInfo, String> {
    sol_trade_sdk::utils::token::get_mint_info(rpc_client, mint)
        .await
        .map_err(|e| format!("获取 mint 信息失败: {}", e))
}

/// 将格式化的 amount 字符串（如 "1.22"）转换为原始单位（u64）
///
/// 使用精确的字符串解析和整数运算，避免 f64 精度问题
pub fn parse_formatted_amount(amount_str: &str, decimals: u8) -> Result<u64, String> {
    // 先尝试解析为原始单位格式（如 "2000000" 或 "2_000_000"）
    let cleaned_str = amount_str.replace('_', "").trim().to_string();

    // 检查是否包含小数点
    if cleaned_str.contains('.') {
        // 小数格式（如 "1.22"）- 使用精确的字符串解析
        let parts: Vec<&str> = cleaned_str.split('.').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid decimal format: '{}'", cleaned_str));
        }

        let integer_part = parts[0].trim();
        let decimal_part = parts[1].trim();

        // 解析整数部分
        let integer_value: u128 = if integer_part.is_empty() || integer_part == "0" {
            0
        } else {
            integer_part
                .parse()
                .map_err(|e| format!("Invalid integer part '{}': {}", integer_part, e))?
        };

        // 解析小数部分
        let decimal_value: u128 = if decimal_part.is_empty() {
            0
        } else {
            // 移除小数部分末尾的 0（不影响精度）
            let trimmed = decimal_part.trim_end_matches('0');
            if trimmed.is_empty() {
                0
            } else {
                trimmed
                    .parse()
                    .map_err(|e| format!("Invalid decimal part '{}': {}", trimmed, e))?
            }
        };

        // 计算小数部分的长度
        let decimal_len = decimal_part.len() as u32;

        // 计算: integer * 10^decimals + decimal * 10^(decimals - decimal_len)
        let multiplier = 10_u128.pow(u32::from(decimals));
        let decimal_multiplier = if decimal_len <= u32::from(decimals) {
            10_u128.pow(u32::from(decimals) - decimal_len)
        } else {
            return Err(format!(
                "Decimal part too long: {} digits (max {})",
                decimal_len, decimals
            ));
        };

        let result = integer_value
            .saturating_mul(multiplier)
            .saturating_add(decimal_value.saturating_mul(decimal_multiplier));

        // 检查是否溢出 u64
        if result > u64::MAX as u128 {
            return Err(format!("Amount too large: {} (exceeds u64::MAX)", cleaned_str));
        }

        Ok(result as u64)
    } else {
        // 原始单位格式 - 直接解析为 u64
        let amount_u64 = cleaned_str.parse::<u64>().map_err(|e| {
            format!(
                "Invalid amount format: {} (expected decimal like '1.22' or raw units like '2000000')",
                e
            )
        })?;
        Ok(amount_u64)
    }
}

/// 调用 surfnet_setTokenAccount RPC 方法设置代币余额
async fn call_surfnet_set_token_account(
    rpc_url: &str,
    owner: &str,
    mint: &str,
    amount: u64,
    token_program: Option<&str>,
) -> Result<(), String> {
    let http_client = reqwest::Client::new();

    // 手动构造 JSON 以避免大数字的精度问题
    // serde_json::json! 宏在处理大 u64 值时可能会使用 f64，导致精度丢失
    let token_program_json =
        if let Some(tp) = token_program { format!("\"{}\"", tp) } else { "null".to_string() };

    let request_body = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"surfnet_setTokenAccount","params":["{}","{}",{{"amount":{},"state":"initialized","closeAuthority":null,"delegate":null,"delegatedAmount":null}},{}]}}"#,
        owner, mint, amount, token_program_json
    );

    let response = http_client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .body(request_body)
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let response_text =
        response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;
    let response_json: Value =
        serde_json::from_str(&response_text).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    if let Some(error) = response_json.get("error") {
        return Err(format!("RPC error: {}", error));
    }

    Ok(())
}

/// 设置测试账户的代币余额（使用 surfnet_setTokenAccount RPC）
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL
/// * `payer` - 测试账户
/// * `mint` - 代币 mint 地址
/// * `amount_formatted` - 格式化的金额（如 "1.22" 表示 1.22 个代币）
///
/// # 示例
/// ```ignore
/// // 设置测试账户的 JUP 余额为 100 JUP
/// set_token_balance(
///     &rpc,
///     "http://127.0.0.1:8899",
///     &payer,
///     &jup_mint,
///     "100",
/// ).await?;
/// ```
pub async fn set_token_balance(
    rpc_client: &Arc<RpcClient>,
    rpc_url: &str,
    payer: &Keypair,
    mint: &Pubkey,
    amount_formatted: &str,
) -> Result<(), String> {
    let payer_pubkey = payer.pubkey();

    // 查询 mint 信息
    let mint_info = get_mint_info(rpc_client, mint).await?;

    println!(
        "💰 设置代币余额: address={}, mint={}, amount={}",
        payer_pubkey, mint, amount_formatted
    );
    println!(
        "   查询到 mint info: decimals={}, token_program={}",
        mint_info.decimals, mint_info.token_program
    );

    // 解析格式化的金额
    let amount_u64 = parse_formatted_amount(amount_formatted, mint_info.decimals)?;

    println!(
        "   转换金额: {} -> {} raw units (decimals={})",
        amount_formatted, amount_u64, mint_info.decimals
    );

    // 计算 ATA 地址
    let ata_address =
        get_associated_token_address_with_program_id(&payer_pubkey, mint, &mint_info.token_program);

    println!("   计算 ATA 地址: {}", ata_address);

    // 调用 surfnet_setTokenAccount
    call_surfnet_set_token_account(
        rpc_url,
        &payer_pubkey.to_string(),
        &mint.to_string(),
        amount_u64,
        Some(&mint_info.token_program.to_string()),
    )
    .await?;

    println!("   ✅ 设置成功\n");

    Ok(())
}

/// Mint token 到指定账户（仅用于测试环境）
///
/// **⚠️ 重要**: 此函数仅用于测试环境！
/// - 只有当你拥有 mint authority 时才能使用
/// - 不要在主网或生产环境使用
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL（用于获取 blockhash）
/// * `mint_authority` - Mint authority 的 Keypair（必须有权 mint 此 token）
/// * `mint` - Token mint 地址
/// * `recipient` - 接收 token 的账户地址
/// * `amount` - 要 mint 的数量（raw units）
///
/// # 返回
/// * `Ok(())` - Mint 成功
/// * `Err(String)` - Mint 失败
pub async fn mint_token_to(
    rpc_client: &Arc<RpcClient>,
    _rpc_url: &str,
    mint_authority: &Keypair,
    mint: &Pubkey,
    recipient: &Pubkey,
    amount: u64,
) -> Result<(), String> {
    let mint_pubkey = mint_authority.pubkey();
    println!("💰 Minting token:");
    println!("   Mint: {}", mint);
    println!("   Mint Authority: {}", mint_pubkey);
    println!("   Recipient: {}", recipient);
    println!("   Amount: {} (raw units)", amount);

    // 1. 确保接收者的 ATA 存在
    let recipient_ata =
        get_associated_token_address_with_program_id(recipient, mint, &spl_token::id());

    // 检查 ATA 是否存在
    let ata_exists = rpc_client.get_token_account_balance(&recipient_ata).await.is_ok();

    let mut instructions = Vec::new();

    if !ata_exists {
        println!("   📝 创建 recipient ATA: {}", recipient_ata);
        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &mint_pubkey,
                recipient,
                mint,
                &spl_token::id(),
            );
        instructions.push(create_ata_ix);
    }

    // 2. 创建 MintTo 指令
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        mint,
        &recipient_ata,
        &mint_pubkey,
        &[&mint_pubkey],
        amount,
    )
    .map_err(|e| format!("创建 MintTo 指令失败: {}", e))?;

    instructions.push(mint_to_ix);

    // 3. 获取最新 blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| format!("获取 blockhash 失败: {}", e))?;

    // 4. 构造交易
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&mint_pubkey));
    transaction.sign(&[mint_authority], recent_blockhash);

    // 5. 发送交易
    let signature = rpc_client
        .send_transaction(&transaction)
        .await
        .map_err(|e| format!("发送交易失败: {}", e))?;

    println!("   ✅ Mint 交易发送成功: {}", signature);

    // 6. 等待交易确认
    println!("   ⏳ 等待交易确认...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 7. 检查交易状态
    match rpc_client.get_signature_status(&signature).await {
        Ok(Some(result)) => {
            if let Some(err) = result.err() {
                return Err(format!("交易执行失败: {:?}", err));
            }
            println!("   ✅ 交易确认成功\n");
        },
        _ => {
            println!("   ⚠️  无法确认交易状态，但交易已发送\n");
        },
    }

    Ok(())
}

/// 转移 token 到指定账户
///
/// 从发送者的 ATA 转移 token 到接收者的 ATA
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL
/// * `payer` - 支付交易费用的账户（通常是发送者）
/// * `mint` - Token mint 地址
/// * `from` - 发送者账户地址
/// * `to` - 接收者账户地址
/// * `amount` - 要转移的数量（raw units）
///
/// # 返回
/// * `Ok(())` - 转移成功
/// * `Err(String)` - 转移失败
pub async fn transfer_token_to(
    rpc_client: &Arc<RpcClient>,
    _rpc_url: &str,
    payer: &Keypair,
    mint: &Pubkey,
    from: &Pubkey,
    to: &Pubkey,
    amount: u64,
) -> Result<(), String> {
    let payer_pubkey = payer.pubkey();
    println!("💸 转移 token:");
    println!("   Mint: {}", mint);
    println!("   From: {}", from);
    println!("   To: {}", to);
    println!("   Amount: {} (raw units)", amount);

    // 1. 计算发送者和接收者的 ATA
    let from_ata = get_associated_token_address_with_program_id(from, mint, &spl_token::id());
    let to_ata = get_associated_token_address_with_program_id(to, mint, &spl_token::id());

    // 2. 检查接收者 ATA 是否存在，不存在则创建
    let ata_exists = rpc_client.get_token_account_balance(&to_ata).await.is_ok();

    let mut instructions = Vec::new();

    if !ata_exists {
        println!("   📝 创建接收者 ATA: {}", to_ata);
        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer_pubkey,
                to,
                mint,
                &spl_token::id(),
            );
        instructions.push(create_ata_ix);
    }

    // 3. 创建 TransferChecked 指令
    let transfer_ix = spl_token::instruction::transfer_checked(
        &spl_token::id(),
        &from_ata,
        mint,
        &to_ata,
        from,
        &[],
        amount,
        0, // Decimals (从 mint 账户获取，但这里传 0 也可以)
    )
    .map_err(|e| format!("创建 TransferChecked 指令失败: {}", e))?;

    instructions.push(transfer_ix);

    // 4. 获取最新 blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| format!("获取 blockhash 失败: {}", e))?;

    // 5. 构造交易
    let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer_pubkey));
    transaction.sign(&[payer], recent_blockhash);

    // 6. 发送交易
    let signature = rpc_client
        .send_transaction(&transaction)
        .await
        .map_err(|e| format!("发送交易失败: {}", e))?;

    println!("   ✅ 转移交易发送成功: {}", signature);

    // 7. 等待交易确认
    println!("   ⏳ 等待交易确认...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 8. 检查交易状态
    match rpc_client.get_signature_status(&signature).await {
        Ok(Some(result)) => {
            if let Some(err) = result.err() {
                return Err(format!("交易执行失败: {:?}", err));
            }
            println!("   ✅ 交易确认成功\n");
        },
        _ => {
            println!("   ⚠️  无法确认交易状态，但交易已发送\n");
        },
    }

    Ok(())
}
