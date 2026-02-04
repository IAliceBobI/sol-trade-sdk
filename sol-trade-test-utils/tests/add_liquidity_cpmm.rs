//! 测试向 PIPE-WSOL CPMM 池子添加流动性
//!
//! 这个测试会：
//! 1. 使用 PIPE-WSOL CPMM 池子
//! 2. 设置测试账户的 PIPE 和 WSOL 余额
//! 3. 使用 deposit 指令添加流动性
//! 4. 验证流动性添加成功

use sol_trade_sdk::common::SolanaRpcClient;
use sol_trade_test_utils::{
    get_simulation_test_keypair,
    pipe_mint,
    pipe_wsol_pool,
    wsol_mint,
    PipeWsolLiquidityBuilder,
};
use std::sync::Arc;

/// 格式化代币数量为可读格式
fn format_token_amount(amount: u64, decimals: u8) -> String {
    if decimals == 0 {
        return amount.to_string();
    }

    let divisor = 10_u64.pow(decimals as u32);
    let whole = amount / divisor;
    let fraction = amount % divisor;

    if fraction == 0 {
        whole.to_string()
    } else {
        // 去掉尾部的零
        let fraction_str = fraction.to_string();
        let trimmed = fraction_str.trim_end_matches('0');
        format!("{}.{:0<width$}", whole, trimmed, width = decimals as usize)
            .trim_end_matches('.')
            .to_string()
    }
}

#[tokio::test]
#[serial_test::serial]
#[ignore = "需要本地测试节点"]
async fn test_add_liquidity_to_cpmm_pool() {
    println!("\n========================================");
    println!("测试: 向 CPMM 池子添加流动性");
    println!("========================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    let pool_address = pipe_wsol_pool();
    let pipe_mint = pipe_mint();
    let wsol_mint = wsol_mint();
    let payer = Arc::new(get_simulation_test_keypair());

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("PIPE Mint: {}", pipe_mint);
    println!("WSOL Mint: {}", wsol_mint);
    println!("测试账户: {}\n", payer.pubkey());

    // 1. 获取 Pool 状态
    use sol_trade_sdk::instruction::utils::raydium_cpmm::get_pool_by_address;

    let pool_state = match get_pool_by_address(&rpc, &pool_address).await {
        Ok(state) => state,
        Err(e) => {
            println!("❌ 获取 Pool 失败: {}\n", e);
            panic!("获取 Pool 失败");
        },
    };

    println!("✅ Pool 状态获取成功:");
    println!("  LP Supply: {}", pool_state.lp_supply);
    println!();

    // 记录初始 LP Supply（用于后续验证）
    let initial_lp_supply = pool_state.lp_supply;

    // 2. 获取当前金库余额
    let token0_balance = rpc.get_token_account_balance(&pool_state.token0_vault).await;
    let token1_balance = rpc.get_token_account_balance(&pool_state.token1_vault).await;

    let (token0_reserve, token1_reserve) = match (token0_balance, token1_balance) {
        (Ok(t0), Ok(t1)) => {
            let t0_amt = t0.amount.parse::<u64>().unwrap_or(0);
            let t1_amt = t1.amount.parse::<u64>().unwrap_or(0);
            println!("📊 当前金库余额:");
            println!(
                "  Token0 (PIPE): {} (raw: {})",
                format_token_amount(t0_amt, pool_state.mint0_decimals),
                t0_amt
            );
            println!(
                "  Token1 (WSOL): {} (raw: {})",
                format_token_amount(t1_amt, pool_state.mint1_decimals),
                t1_amt
            );
            println!();
            (t0_amt, t1_amt)
        },
        _ => {
            println!("❌ 无法查询金库余额\n");
            panic!("无法查询金库余额");
        },
    };

    // 3. 设置测试账户余额
    println!("💰 设置测试账户代币余额...\n");

    use sol_trade_test_utils::set_token_balance;

    // 设置 PIPE 余额（150 亿 PIPE）
    let pipe_amount_str = "15000000000";
    if let Err(e) = set_token_balance(&rpc, &rpc_url, &payer, &pipe_mint, pipe_amount_str).await {
        println!("❌ 设置 PIPE 余额失败: {}\n", e);
        panic!("设置 PIPE 余额失败");
    }

    // 设置 WSOL 余额（50,000 WSOL）
    let wsol_amount_str = "50000";
    if let Err(e) = set_token_balance(&rpc, &rpc_url, &payer, &wsol_mint, wsol_amount_str).await {
        println!("❌ 设置 WSOL 余额失败: {}\n", e);
        panic!("设置 WSOL 余额失败");
    }

    println!("✅ 代币余额设置成功\n");

    // 4. 计算要铸造的 LP 代币数量（680 亿）
    let lp_token_amount = 68_000_000_000_u64;

    println!("🪙 要铸造的 LP 代币: {}", lp_token_amount);
    println!();

    // 5. 使用构建器构建 Deposit 指令
    let (deposit_instruction, calculated_amounts, owner_lp_token) =
        PipeWsolLiquidityBuilder::new(lp_token_amount)
            .max_pipe(12_000_000_000_000_000) // 120 亿 PIPE
            .max_wsol(100_000_000_000)        // 100,000 WSOL
            .build_instruction(payer.pubkey(), &pool_state, token0_reserve, token1_reserve);

    // 6. 显示计算结果
    if let Some((calc_token0, calc_token1)) = calculated_amounts {
        println!("📐 计算结果（基于 CPMM 公式）:");
        println!("  Token0 (PIPE): {}", calc_token0);
        println!("  Token1 (WSOL): {} lamports", calc_token1);
        println!();
    }

    println!("📝 Deposit 指令已构建");
    println!();

    // 7. 检查并创建 LP token ATA
    println!("🔧 检查并创建 LP token ATA...");
    let create_lp_ata_instruction =
        spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &payer.pubkey(),
            &pool_state.lp_mint,
            &spl_token::id(),
        );

    // 先查询 LP ATA 是否存在
    let lp_ata_exists = rpc.get_account(&owner_lp_token).await.is_ok();

    // 8. 获取最新 blockhash
    let blockhash = rpc.get_latest_blockhash().await.expect("Failed to get blockhash");

    // 9. 构建并发送交易
    let instructions = if lp_ata_exists {
        println!("  ✅ LP ATA 已存在\n");
        vec![deposit_instruction]
    } else {
        println!("  🆕 创建 LP ATA\n");
        vec![create_lp_ata_instruction, deposit_instruction]
    };

    use solana_sdk::{signer::Signer, transaction::Transaction};

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

            // 10. 验证 Pool LP Supply 变化
            std::thread::sleep(std::time::Duration::from_secs(2));

            println!("📊 验证流动性添加结果...\n");

            match get_pool_by_address(&rpc, &pool_address).await {
                Ok(new_pool_state) => {
                    let lp_supply_increase =
                        new_pool_state.lp_supply.saturating_sub(initial_lp_supply);

                    println!("🪙 LP Supply 变化:");
                    println!("  添加前: {}", initial_lp_supply);
                    println!("  添加后: {}", new_pool_state.lp_supply);
                    println!("  增加: {}", lp_supply_increase);
                    println!();

                    // 验证 LP supply 是否增加了预期的数量（允许小幅误差）
                    assert!(
                        lp_supply_increase >= lp_token_amount,
                        "LP Supply 增加不足: 预期 {}, 实际 {}",
                        lp_token_amount,
                        lp_supply_increase
                    );

                    println!("✅ LP Supply 验证通过");
                    println!();
                },
                Err(e) => {
                    println!("❌ 无法获取更新后的 Pool 状态: {}\n", e);
                    panic!("无法获取更新后的 Pool 状态");
                },
            }

            // 11. 验证金库余额变化
            let new_token0_balance =
                rpc.get_token_account_balance(&pool_state.token0_vault).await;
            let new_token1_balance =
                rpc.get_token_account_balance(&pool_state.token1_vault).await;

            if let (Ok(new_t0), Ok(new_t1)) = (new_token0_balance, new_token1_balance) {
                let new_t0_amt = new_t0.amount.parse::<u64>().unwrap_or(0);
                let new_t1_amt = new_t1.amount.parse::<u64>().unwrap_or(0);

                let t0_increase = new_t0_amt.saturating_sub(token0_reserve);
                let t1_increase = new_t1_amt.saturating_sub(token1_reserve);

                println!("📊 更新后的金库余额:");
                println!(
                    "  Token0 (PIPE): {} (增加: {})",
                    format_token_amount(new_t0_amt, pool_state.mint0_decimals),
                    format_token_amount(t0_increase, pool_state.mint0_decimals)
                );
                println!(
                    "  Token1 (WSOL): {} (增加: {})",
                    format_token_amount(new_t1_amt, pool_state.mint1_decimals),
                    format_token_amount(t1_increase, pool_state.mint1_decimals)
                );
                println!();
            }

            // 12. 验证用户 LP 代币余额
            match rpc.get_token_account_balance(&owner_lp_token).await {
                Ok(lp_balance) => {
                    let lp_amt = lp_balance.amount.parse::<u64>().unwrap_or(0);
                    println!(
                        "🪙 用户 LP 代币余额: {} (raw: {})",
                        format_token_amount(lp_amt, pool_state.lp_mint_decimals),
                        lp_amt
                    );
                    println!();
                },
                Err(e) => {
                    println!("⚠️  无法查询 LP 余额: {}\n", e);
                },
            }

            println!("✅ 流动性添加测试通过！\n");
        },
        Err(e) => {
            println!("❌ 交易失败: {}\n", e);
            panic!("交易失败: {}", e);
        },
    }
}
