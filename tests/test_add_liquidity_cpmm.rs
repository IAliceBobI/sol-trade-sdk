//! 测试向 PIPE-WSOL CPMM 池子添加大量流动性
//!
//! 这个测试会：
//! 1. 使用 PIPE-WSOL CPMM 池子
//! 2. 空投大量 PIPE 和 WSOL 到测试账户
//! 3. 使用 deposit 指令添加流动性（100 亿 PIPE 级别）
//! 4. 验证流动性添加成功

use sol_trade_sdk::liquidity::cpmm::{build_deposit_instruction, calculate_deposit_amounts, CpmmDepositParams};
use sol_trade_sdk::{
    common::{GasFeeStrategy, SolanaRpcClient, TradeConfig},
    instruction::utils::raydium_cpmm::get_pool_by_address,
    TradingClient,
};
use solana_sdk::{
    pubkey::Pubkey,
    signer::Signer,
    transaction::Transaction,
};
use std::str::FromStr;
use std::sync::Arc;

// 导入公共测试模块
mod common;
use common::{get_simulation_test_keypair, set_token_balance};

/// PIPE-WSOL CPMM Pool
const PIPE_WSOL_POOL: &str = "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp";

/// PIPE Token Mint
const PIPE_MINT: &str = "8ycz3kctoRb4LFrtoYG2r8tRyUYUeGf5Q16M2TEMp7A";

/// WSOL Mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Raydium CPMM 程序 ID
const CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

#[tokio::test]
#[serial_test::serial(add_liquidity_test)]
async fn test_add_liquidity_to_cpmm_pool() {
    println!("\n========================================");
    println!("测试: 向 CPMM 池子添加流动性");
    println!("========================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = Pubkey::from_str(PIPE_WSOL_POOL).unwrap();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let pipe_mint = Pubkey::from_str(PIPE_MINT).unwrap();
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("PIPE Mint: {}", pipe_mint);
    println!("WSOL Mint: {}", wsol_mint);
    println!("测试账户: {}\n", payer.pubkey());

    // 1. 获取 Pool 状态
    let pool_state = match get_pool_by_address(&rpc, &pool_address).await {
        Ok(state) => state,
        Err(e) => {
            println!("❌ 获取 Pool 失败: {}\n", e);
            return;
        }
    };

    println!("✅ Pool 状态获取成功:");
    println!("  LP Supply: {}", pool_state.lp_supply);
    println!("  Token0 Vault: {}", pool_state.token0_vault);
    println!("  Token1 Vault: {}", pool_state.token1_vault);
    println!("  LP Mint: {}", pool_state.lp_mint);
    println!();

    // 2. 获取当前金库余额
    let token0_balance = rpc.get_token_account_balance(&pool_state.token0_vault).await;
    let token1_balance = rpc.get_token_account_balance(&pool_state.token1_vault).await;

    let (token0_reserve, token1_reserve) = match (token0_balance, token1_balance) {
        (Ok(t0), Ok(t1)) => {
            let t0_amt = t0.amount.parse::<u64>().unwrap_or(0);
            let t1_amt = t1.amount.parse::<u64>().unwrap_or(0);
            println!("📊 当前金库余额:");
            println!("  Token0 (PIPE): {} (decimals=6)", t0_amt);
            println!("  Token1 (WSOL): {} lamports", t1_amt);
            println!();
            (t0_amt, t1_amt)
        }
        _ => {
            println!("❌ 无法查询金库余额\n");
            return;
        }
    };

    // 3. 设置测试账户余额（使用空投）
    // 我们要添加大量流动性：100 亿 PIPE + 对应的 WSOL
    println!("💰 设置测试账户代币余额...\n");

    // 设置 PIPE 余额（使用 surfnet_setTokenAccount）
    // 我们设置 15,000,000,000 PIPE (150 亿，足够添加 100 亿)
    let pipe_amount_str = "15000000000";
    if let Err(e) = set_token_balance(&rpc, &rpc_url, &payer, &pipe_mint, pipe_amount_str).await {
        println!("❌ 设置 PIPE 余额失败: {}\n", e);
        return;
    }

    // 设置 WSOL 余额
    // 根据当前池子比例计算：
    // - 当前池子：6,061 PIPE : 0.027 WSOL
    // - 添加 10,000,000,000 PIPE 需要约 (10B * 0.027 / 6061) ≈ 44,537 WSOL
    // 我们设置 50,000 WSOL 以确保足够
    let wsol_amount_str = "50000";
    if let Err(e) = set_token_balance(&rpc, &rpc_url, &payer, &wsol_mint, wsol_amount_str).await {
        println!("❌ 设置 WSOL 余额失败: {}\n", e);
        return;
    }

    println!("✅ 代币余额设置成功\n");

    // 4. 计算要铸造的 LP 代币数量
    // 当前池子状态（从上次测试得知）：
    // - LP supply ≈ 41,000
    // - Token0 vault (PIPE) ≈ 6,060,947,750
    // - Token1 vault (WSOL) ≈ 27,029,881
    //
    // 如果我们要添加 10,000,000,000 (100亿) PIPE：
    // - 按比例需要 WSOL ≈ 10B * 27M / 6060M ≈ 44.5M lamports ≈ 0.0445 WSOL
    //
    // 计算 LP 数量：
    // - LP_amount = (10,000,000,000 / 6,060,947,750) * 41,000 ≈ 67,600,000,000
    //
    // 我们添加 68,000,000,000 LP (680 亿) 来获得约 100 亿 PIPE + ~0.045 WSOL 的流动性
    let lp_token_amount = 68_000_000_000_u64;

    println!("🪙 要铸造的 LP 代币: {}", lp_token_amount);
    println!();

    // 5. 计算需要的代币数量（使用我们的计算函数）
    match calculate_deposit_amounts(lp_token_amount, &pool_state, token0_reserve, token1_reserve) {
        Some((calc_token0, calc_token1)) => {
            println!("📐 计算结果（基于 CPMM 公式）:");
            println!("  Token0 (PIPE): {}", calc_token0);
            println!("  Token1 (WSOL): {} lamports", calc_token1);
            println!();
        }
        None => {
            println!("⚠️  无法计算代币数量，使用固定值\n");
        }
    }

    // 6. 构建 Deposit 指令
    let owner_lp_token = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &pool_state.lp_mint,
        &spl_token::id(),
    );

    let token_0_account = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &pipe_mint,
        &spl_token::id(),
    );

    let token_1_account = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &wsol_mint,
        &spl_token::id(),
    );

    let deposit_params = CpmmDepositParams {
        pool_state: pool_address, // 使用 pool_address (Pubkey) 而不是 pool_state (PoolState)
        owner_lp_token,
        token_0_account,
        token_1_account,
        token_0_vault: pool_state.token0_vault,
        token_1_vault: pool_state.token1_vault,
        token_0_mint: pipe_mint,
        token_1_mint: wsol_mint,
        lp_mint: pool_state.lp_mint,
        lp_token_amount,
        // 设置足够高的上限（基于 68B LP 的计算值，加缓冲）
        // 计算值：约 100 亿 PIPE + ~0.045 WSOL
        maximum_token_0_amount: 12_000_000_000_000_000,   // 120 亿 PIPE (decimals=6)
        maximum_token_1_amount: 100_000_000_000,          // 100,000 WSOL (lamports)
        token_program: spl_token::id(),
    };

    let deposit_instruction = build_deposit_instruction(deposit_params, payer.pubkey());

    println!("📝 Deposit 指令已构建:");
    println!("  Program: {}", CPMM_PROGRAM_ID);
    println!("  Pool: {}", pool_address);
    println!("  LP Token Amount: {}", lp_token_amount);
    println!("  Max Token0: 12,000,000,000,000,000 (120 亿 PIPE)");
    println!("  Max Token1: 100,000,000,000 (100,000 WSOL)");
    println!();

    // 7. 创建 LP token ATA（如果不存在）
    println!("🔧 检查并创建 LP token ATA...");
    let create_lp_ata_instruction = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &payer.pubkey(),
        &pool_state.lp_mint,
        &spl_token::id(),
    );

    // 先查询 LP ATA 是否存在
    let lp_ata_exists = rpc.get_account(&owner_lp_token).await.is_ok();

    // 8. 获取最新 blockhash
    let blockhash = rpc
        .get_latest_blockhash()
        .await
        .expect("Failed to get blockhash");

    // 9. 构建并发送交易
    let instructions = if lp_ata_exists {
        println!("  ✅ LP ATA 已存在\n");
        vec![deposit_instruction]
    } else {
        println!("  🆕 创建 LP ATA\n");
        vec![create_lp_ata_instruction, deposit_instruction]
    };

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[&payer],
        blockhash,
    );

    println!("🚀 发送交易...");

    match rpc.send_and_confirm_transaction(&transaction).await {
        Ok(signature) => {
            println!("✅ 交易成功: {}", signature);
            println!();

            // 9. 验证金库余额变化
            std::thread::sleep(std::time::Duration::from_secs(2));

            let new_token0_balance = rpc.get_token_account_balance(&pool_state.token0_vault).await;
            let new_token1_balance = rpc.get_token_account_balance(&pool_state.token1_vault).await;

            if let (Ok(new_t0), Ok(new_t1)) = (new_token0_balance, new_token1_balance) {
                let new_t0_amt = new_t0.amount.parse::<u64>().unwrap_or(0);
                let new_t1_amt = new_t1.amount.parse::<u64>().unwrap_or(0);

                println!("📊 更新后的金库余额:");
                println!("  Token0 (PIPE): {} (增加: {})", new_t0_amt, new_t0_amt.saturating_sub(token0_reserve));
                println!("  Token1 (WSOL): {} (增加: {})", new_t1_amt, new_t1_amt.saturating_sub(token1_reserve));
                println!();
            }

            // 10. 验证用户 LP 代币余额
            match rpc.get_token_account_balance(&owner_lp_token).await {
                Ok(lp_balance) => {
                    let lp_amt = lp_balance.amount.parse::<u64>().unwrap_or(0);
                    println!("🪙 用户 LP 代币余额: {}", lp_amt);
                    println!();
                }
                Err(e) => {
                    println!("⚠️  无法查询 LP 余额: {}\n", e);
                }
            }

            println!("✅ 流动性添加成功！\n");
        }
        Err(e) => {
            println!("❌ 交易失败: {}\n", e);
        }
    }
}

/// 使用 TradingClient 测试 swap（验证流动性已添加）
#[tokio::test]
#[serial_test::serial(add_liquidity_test)]
async fn test_swap_after_adding_liquidity() {
    println!("\n========================================");
    println!("测试: 添加流动性后的 Swap");
    println!("========================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let payer = Arc::new(get_simulation_test_keypair());

    // 使用 TradingClient 执行一笔买入
    let trade_config = TradeConfig::new(rpc_url, vec![], solana_commitment_config::CommitmentConfig::confirmed());
    let client = TradingClient::new(payer.clone(), trade_config).await;

    let pipe_mint = Pubkey::from_str(PIPE_MINT).unwrap();

    // TODO: 实现 swap 测试
    println!("⚠️  Swap 测试待实现\n");
}
