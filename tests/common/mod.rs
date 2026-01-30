//! 测试公共模块
//!
//! 提供测试工具和辅助函数

pub mod proxy_http;

use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair,
};

/// 导入 WSOL 管理功能
use sol_trade_sdk::trading::common::wsol_manager;

/// 固定的测试模拟账户（已有 10 SOL 余额）
/// 注意：这个账户是预先创建并空投过的，不需要在测试中重复空投
///
/// 地址: 8be6dbPmZH1URHXyFTbY876QuVunrD8wTZhHGXjEdrvj
#[allow(dead_code)]
pub const SIMULATION_TEST_KEYPAIR: &str =
    "2cUyNj1YLguzrU89Xu2AcnGZD9qcNjEJo5QTg4tBs9foVXzLF3fBdBXiUdMmb867T9EK8FfKUQCH8FR5oD3bYVew";

/// 获取固定的模拟测试 Keypair
#[allow(dead_code)]
pub fn get_simulation_test_keypair() -> Keypair {
    Keypair::from_base58_string(SIMULATION_TEST_KEYPAIR)
}

/// 为测试账户空投 SOL 并循环等待到账
///
/// # 参数
/// * `rpc_url` - RPC URL
/// * `payer` - 账户公钥
/// * `amount_sol` - 空投的 SOL 数量
///
/// # 返回
/// * `Ok(())` - 空投成功
/// * `Err(String)` - 空投失败
#[allow(dead_code)]
pub async fn airdrop_and_wait(
    rpc_url: &str,
    payer: &Pubkey,
    amount_sol: u64,
) -> Result<(), String> {
    let client = RpcClient::new(rpc_url.to_string());
    let amount_lamports = amount_sol * LAMPORTS_PER_SOL;

    // 尝试空投
    println!("💰 空投 {} SOL 到测试账户...", amount_sol);
    match client.request_airdrop(payer, amount_lamports).await {
        Ok(sig) => {
            println!("✅ 空投成功，签名: {}", sig);
            // 循环等待余额到账
            println!("⏳ 等待余额到账...");
            let mut retries = 0;
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                match client.get_balance(payer).await {
                    Ok(balance) => {
                        if balance >= amount_lamports {
                            println!(
                                "✅ 余额已到账: {} lamports ({:.2} SOL)\n",
                                balance,
                                balance as f64 / 1_000_000_000.0
                            );
                            return Ok(());
                        }
                        retries += 1;
                        if retries > 20 {
                            return Err(format!("等待超时，当前余额: {} lamports", balance));
                        }
                    },
                    Err(e) => {
                        return Err(format!("查询余额失败: {}", e));
                    },
                }
            }
        },
        Err(e) => Err(format!("空投失败: {}", e)),
    }
}

/// 初始化测试账户（一次性检查和设置）
///
/// 这个函数会：
/// 1. 检查账户余额是否足够，不足则空投
/// 2. 检查所需的 ATA 是否存在，不存在则创建并充值
/// 3. 对于 WSOL ATA，会自动 wrap SOL
/// 4. 真实执行交易，而不是模拟
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL（用于空投）
/// * `payer` - 测试账户 Keypair
/// * `mints_with_amounts` - 需要创建/充值的代币 Mint 地址及充值金额列表
/// * `min_balance_sol` - 最小余额要求（SOL）
///
/// # 返回
/// * `Ok(())` - 初始化成功
/// * `Err(String)` - 初始化失败
///
/// # 示例
/// ```ignore
/// // 创建并充值 WSOL ATA（0.001 SOL）和 JUP ATA
/// ensure_ata_with_balance(
///     &rpc,
///     &rpc_url,
///     &payer,
///     &[(wsol_mint, Some(1_000_000u64)), (jup_mint, None)],
///     1,
/// ).await?;
/// ```
#[allow(dead_code)]
pub async fn ensure_ata_with_balance(
    rpc_client: &sol_trade_sdk::common::SolanaRpcClient,
    rpc_url: &str,
    payer: &Keypair,
    mints_with_amounts: &[(Pubkey, Option<u64>)],
    min_balance_sol: u64,
) -> Result<(), String> {
    use solana_sdk::signature::Signer;
    use solana_sdk::transaction::Transaction;

    let payer_pubkey = payer.pubkey();
    let min_balance_lamports = min_balance_sol * LAMPORTS_PER_SOL;
    println!("🔧 初始化测试账户: {}", payer_pubkey);

    // ========================================
    // 步骤 1: 检查并空投 SOL（如果余额不足）
    // ========================================
    let balance = rpc_client
        .get_balance(&payer_pubkey)
        .await
        .map_err(|e| format!("查询余额失败: {}", e))?;

    if balance < min_balance_lamports {
        println!(
            "   💰 余额不足: {} lamports (需要至少 {} lamports)",
            balance, min_balance_lamports
        );
        println!("   💰 正在空投 {} SOL...", min_balance_sol);

        // 使用空投函数
        airdrop_and_wait(rpc_url, &payer_pubkey, min_balance_sol).await?;
    } else {
        println!(
            "   ✅ 余额充足: {} lamports ({:.2} SOL)",
            balance,
            balance as f64 / 1_000_000_000.0
        );
    }

    // ========================================
    // 步骤 2: 检查并创建/充值 ATA
    // ========================================
    let mut instructions = Vec::new();

    for (mint, wrap_amount) in mints_with_amounts {
        let ata_address =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &payer_pubkey,
                mint,
                &spl_token::id(),
            );

        // 检查 ATA 是否存在
        let ata_exists = rpc_client.get_token_account_balance(&ata_address).await.is_ok();

        if !ata_exists {
            println!("   📝 创建 ATA: {} (mint: {})", ata_address, mint);

            // 创建 ATA 指令
            let create_ix =
                spl_associated_token_account::instruction::create_associated_token_account(
                    &payer_pubkey,
                    &payer_pubkey,
                    mint,
                    &spl_token::id(),
                );
            instructions.push(create_ix);

            // 如果是 WSOL 且需要 wrap，添加 wrap SOL 指令
            if let Some(amount) = wrap_amount {
                if mint.to_string() == "So11111111111111111111111111111111111111112" {
                    println!("   💰 Wrap SOL: {} lamports -> WSOL ATA", amount);

                    // 使用 SDK 中现成的 wrap_sol_only 函数
                    let wrap_instructions = wsol_manager::wrap_sol_only(&payer_pubkey, *amount);
                    instructions.extend(wrap_instructions);
                }
            }
        } else {
            println!("   ✅ ATA 已存在: {} (mint: {})", ata_address, mint);

            // ATA 已存在，检查是否需要充值（仅对 WSOL）
            if let Some(amount) = wrap_amount {
                if mint.to_string() == "So11111111111111111111111111111111111111112" {
                    // 检查余额
                    match rpc_client.get_token_account_balance(&ata_address).await {
                        Ok(balance_info) => {
                            let current_balance = balance_info.amount.parse::<u64>().unwrap_or(0);
                            if current_balance < *amount {
                                println!(
                                    "   💰 充值 WSOL ATA: {} lamports",
                                    amount - current_balance
                                );

                                let topup_amount = amount - current_balance;

                                // 使用 SDK 中现成的 wrap_sol_only 函数
                                let wrap_instructions =
                                    wsol_manager::wrap_sol_only(&payer_pubkey, topup_amount);
                                instructions.extend(wrap_instructions);
                            } else {
                                println!("   ✅ WSOL 余额充足: {} lamports", current_balance);
                            }
                        },
                        Err(_) => {
                            // ATA 存在但查询余额失败，尝试充值
                            println!("   💰 充值 WSOL ATA: {} lamports", amount);

                            // 使用 SDK 中现成的 wrap_sol_only 函数
                            let wrap_instructions =
                                wsol_manager::wrap_sol_only(&payer_pubkey, *amount);
                            instructions.extend(wrap_instructions);
                        },
                    }
                }
            }
        }
    }

    // ========================================
    // 步骤 3: 执行交易（如果有需要执行的指令）
    // ========================================
    if !instructions.is_empty() {
        println!("   📤 执行 {} 条指令...", instructions.len());

        // 获取最新 blockhash
        let recent_blockhash = rpc_client
            .get_latest_blockhash()
            .await
            .map_err(|e| format!("获取 blockhash 失败: {}", e))?;

        // 构造交易
        let mut transaction = Transaction::new_with_payer(&instructions, Some(&payer_pubkey));
        transaction.sign(&[payer], recent_blockhash);

        // 发送交易
        let signature = rpc_client
            .send_transaction(&transaction)
            .await
            .map_err(|e| format!("发送交易失败: {}", e))?;

        println!("   ✅ 交易发送成功: {}", signature);

        // 等待交易确认
        println!("   ⏳ 等待交易确认...");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // 检查交易状态
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
    } else {
        println!("   ✅ 所有 ATA 已就绪，无需操作\n");
    }

    Ok(())
}

/// 批量创建 ATA 指令（如果不存在）
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `payer` - 支付账户（用于创建 ATA）
/// * `mints` - 代币 Mint 地址列表
/// * `owner` - ATA 所有者（默认为 payer）
///
/// # 返回
/// * 需要创建的 ATA 指令列表（只返回不存在的 ATA 的指令）
#[allow(dead_code)]
pub async fn create_ata_instructions_if_needed(
    rpc_client: &sol_trade_sdk::common::SolanaRpcClient,
    payer: &Pubkey,
    mints: &[Pubkey],
    owner: Option<&Pubkey>,
) -> Vec<Instruction> {
    let owner = owner.unwrap_or(payer);
    let mut instructions = Vec::new();

    for mint in mints {
        let ata_address =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                owner,
                mint,
                &spl_token::id(),
            );

        // 检查 ATA 是否存在
        if rpc_client.get_token_account_balance(&ata_address).await.is_err() {
            let create_ix =
                spl_associated_token_account::instruction::create_associated_token_account(
                    payer,
                    owner,
                    mint,
                    &spl_token::id(),
                );
            instructions.push(create_ix);
        }
    }

    instructions
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
///
/// # 示例
/// ```ignore
/// // 假设你有一个测试 token 的 mint authority
/// use solana_sdk::signer::Signer;
///
/// let mint_authority = Keypair::new(); // 测试用的 mint authority
/// let mint = Pubkey::from_str("...").unwrap(); // 测试 token mint
/// let recipient = pubkey(); // 接收地址
///
/// // Mint 1000 个 token（假设 decimals = 6）
/// mint_token_to(
///     &rpc,
///     "http://127.0.0.1:8899",
///     &mint_authority,
///     &mint,
///     &recipient,
///     1_000_000_000, // 1000 tokens (6 decimals)
/// ).await?;
/// ```
#[allow(dead_code)]
pub async fn mint_token_to(
    rpc_client: &sol_trade_sdk::common::SolanaRpcClient,
    _rpc_url: &str,
    mint_authority: &Keypair,
    mint: &Pubkey,
    recipient: &Pubkey,
    amount: u64,
) -> Result<(), String> {
    use solana_sdk::signature::Signer;
    use solana_sdk::transaction::Transaction;

    let mint_pubkey = mint_authority.pubkey();
    println!("💰 Minting token:");
    println!("   Mint: {}", mint);
    println!("   Mint Authority: {}", mint_pubkey);
    println!("   Recipient: {}", recipient);
    println!("   Amount: {} (raw units)", amount);

    // 1. 确保接收者的 ATA 存在
    let recipient_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        recipient,
        mint,
        &spl_token::id(),
    );

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
#[allow(dead_code)]
pub async fn transfer_token_to(
    rpc_client: &sol_trade_sdk::common::SolanaRpcClient,
    _rpc_url: &str,
    payer: &Keypair,
    mint: &Pubkey,
    from: &Pubkey,
    to: &Pubkey,
    amount: u64,
) -> Result<(), String> {
    use solana_sdk::signature::Signer;
    use solana_sdk::transaction::Transaction;

    let payer_pubkey = payer.pubkey();
    println!("💸 转移 token:");
    println!("   Mint: {}", mint);
    println!("   From: {}", from);
    println!("   To: {}", to);
    println!("   Amount: {} (raw units)", amount);

    // 1. 计算发送者和接收者的 ATA
    let from_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        from,
        mint,
        &spl_token::id(),
    );
    let to_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        to,
        mint,
        &spl_token::id(),
    );

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

// 重新导出常用的类型和函数
