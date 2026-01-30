//! 测试公共模块
//!
//! 提供测试工具和辅助函数

pub mod proxy_http;

use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    instruction::Instruction,
};

/// 固定的测试模拟账户（已有 10 SOL 余额）
/// 注意：这个账户是预先创建并空投过的，不需要在测试中重复空投
///
/// 地址: 8be6dbPmZH1URHXyFTbY876QuVunrD8wTZhHGXjEdrvj
#[allow(dead_code)]
pub const SIMULATION_TEST_KEYPAIR: &str = "2cUyNj1YLguzrU89Xu2AcnGZD9qcNjEJo5QTg4tBs9foVXzLF3fBdBXiUdMmb867T9EK8FfKUQCH8FR5oD3bYVew";

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
                            println!("✅ 余额已到账: {} lamports ({:.2} SOL)\n", balance, balance as f64 / 1_000_000_000.0);
                            return Ok(());
                        }
                        retries += 1;
                        if retries > 20 {
                            return Err(format!("等待超时，当前余额: {} lamports", balance));
                        }
                    },
                    Err(e) => {
                        return Err(format!("查询余额失败: {}", e));
                    }
                }
            }
        },
        Err(e) => {
            Err(format!("空投失败: {}", e))
        }
    }
}

/// 初始化测试账户（一次性检查和设置）
///
/// 这个函数会：
/// 1. 检查账户余额是否足够，不足则空投
/// 2. 检查所需的 ATA 是否存在，不存在则创建
/// 3. 返回是否需要创建 ATA 的指令列表（用于测试中的模拟）
///
/// 注意：实际创建 ATA 需要在测试中通过交易执行，这里只检查和返回指令
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL（用于空投）
/// * `payer` - 测试账户 Keypair
/// * `mints` - 需要创建的代币 Mint 地址列表
/// * `min_balance_sol` - 最小余额要求（SOL）
///
/// # 返回
/// * `Ok(instructions)` - 需要创建的 ATA 指令列表（如果都已存在则为空）
/// * `Err(String)` - 初始化失败
#[allow(dead_code)]
pub async fn init_test_account(
    rpc_client: &sol_trade_sdk::common::SolanaRpcClient,
    rpc_url: &str,
    payer: &Keypair,
    mints: &[Pubkey],
    min_balance_sol: u64,
) -> Result<Vec<Instruction>, String> {
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
        println!("   💰 余额不足: {} lamports (需要至少 {} lamports)", balance, min_balance_lamports);
        println!("   💰 正在空投 {} SOL...", min_balance_sol);

        // 使用空投函数
        airdrop_and_wait(rpc_url, &payer_pubkey, min_balance_sol).await?;
    } else {
        println!("   ✅ 余额充足: {} lamports ({:.2} SOL)", balance, balance as f64 / 1_000_000_000.0);
    }

    // ========================================
    // 步骤 2: 检查并记录需要创建的 ATA
    // ========================================
    let mut ata_instructions = Vec::new();
    let mut existing_count = 0;
    let mut create_count = 0;

    for mint in mints {
        let ata_address = spl_associated_token_account::get_associated_token_address_with_program_id(
            &payer_pubkey,
            mint,
            &spl_token::id(),
        );

        // 检查 ATA 是否存在
        match rpc_client.get_token_account_balance(&ata_address).await {
            Ok(_) => {
                existing_count += 1;
                println!("   ✅ ATA 已存在: {} (mint: {})", ata_address, mint);
            }
            Err(_) => {
                create_count += 1;
                println!("   📝 需要创建 ATA: {} (mint: {})", ata_address, mint);

                // 创建 ATA 指令
                let create_ix = spl_associated_token_account::instruction::create_associated_token_account(
                    &payer_pubkey,
                    &payer_pubkey,
                    mint,
                    &spl_token::id(),
                );
                ata_instructions.push(create_ix);
            }
        }
    }

    println!("   📊 ATA 检查完成: {} 个已存在, {} 个需要创建", existing_count, create_count);
    println!();

    Ok(ata_instructions)
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
        let ata_address = spl_associated_token_account::get_associated_token_address_with_program_id(
            owner,
            mint,
            &spl_token::id(),
        );

        // 检查 ATA 是否存在
        if rpc_client.get_token_account_balance(&ata_address).await.is_err() {
            let create_ix = spl_associated_token_account::instruction::create_associated_token_account(
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

// 重新导出常用的类型和函数
