//! Ensure 辅助函数
//!
//! 提供便捷的测试辅助函数，确保账户有足够的余额和流动性

use crate::airdrop::airdrop_and_wait;
use crate::token::{get_mint_info, parse_formatted_amount};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::sync::Arc;

use sol_trade_sdk::instruction::utils::raydium_cpmm::get_pool_by_address;
use sol_trade_sdk::liquidity::cpmm::{build_deposit_instruction, CpmmDepositParams};

/// 确保账户有足够的 SOL 余额
///
/// 如果余额不足，会自动空投
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL
/// * `payer` - 账户地址
/// * `min_balance_sol` - 最小 SOL 余额
///
/// # 示例
/// ```ignore
/// ensure_sol_balance(&rpc, "http://127.0.0.1:8899", &payer.pubkey(), 10).await?;
/// ```
pub async fn ensure_sol_balance(
    rpc_client: &Arc<RpcClient>,
    rpc_url: &str,
    payer: &Pubkey,
    min_balance_sol: u64,
) -> Result<(), String> {
    let min_balance_lamports = min_balance_sol * LAMPORTS_PER_SOL;

    let balance = rpc_client
        .get_balance(payer)
        .await
        .map_err(|e| format!("查询余额失败: {}", e))?;

    if balance < min_balance_lamports {
        println!(
            "💰 SOL 余额不足: {} lamports (需要 {} lamports)，正在空投...",
            balance, min_balance_lamports
        );
        airdrop_and_wait(rpc_url, payer, min_balance_sol).await?;
    } else {
        println!(
            "✅ SOL 余额充足: {} lamports ({:.2} SOL)",
            balance,
            balance as f64 / 1_000_000_000.0
        );
    }

    Ok(())
}

/// 确保账户有足够的 Token 余额（使用 surfnet_setTokenAccount）
///
/// ⚠️ 仅适用于测试环境（surfpool）
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL
/// * `payer` - 账户 Keypair
/// * `mint` - Token mint 地址
/// * `amount_formatted` - 格式化的金额（如 "100" 表示 100 个代币）
///
/// # 示例
/// ```ignore
/// ensure_token_balance(&rpc, "http://127.0.0.1:8899", &payer, &usdc_mint, "1000").await?;
/// ```
pub async fn ensure_token_balance(
    rpc_client: &Arc<RpcClient>,
    rpc_url: &str,
    payer: &Keypair,
    mint: &Pubkey,
    amount_formatted: &str,
) -> Result<(), String> {
    let payer_pubkey = payer.pubkey();

    // 查询 mint 信息
    let mint_info = get_mint_info(rpc_client, mint).await?;

    // 计算 ATA 地址
    let ata_address =
        get_associated_token_address_with_program_id(&payer_pubkey, mint, &mint_info.token_program);

    // 检查当前余额
    let current_amount = match rpc_client.get_token_account_balance(&ata_address).await {
        Ok(balance_info) => balance_info.amount.parse::<u64>().unwrap_or(0),
        Err(_) => 0,
    };

    // 解析目标金额
    let target_amount = parse_formatted_amount(amount_formatted, mint_info.decimals)?;

    if current_amount >= target_amount {
        println!(
            "✅ Token 余额充足: {} ({} decimals)",
            amount_formatted, mint_info.decimals
        );
        return Ok(());
    }

    println!(
        "💰 Token 余额不足，设置余额: {} (当前: {}, 目标: {})",
        amount_formatted, current_amount, target_amount
    );

    crate::token::set_token_balance(rpc_client, rpc_url, payer, mint, amount_formatted).await
}

/// 确保 PIPE-WSOL Pool 有足够的流动性
///
/// 便捷函数，专门用于确保 PIPE-WSOL pool 有指定数量的 WSOL 流动性。
/// 如果当前 WSOL vault 余额不足，会自动添加流动性以达到目标值。
///
/// ⚠️ 仅适用于测试环境
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL
/// * `payer` - 账户 Keypair
/// * `min_wsol_sol` - 最小 WSOL 流动性（SOL 单位，如 1000 表示 1000 SOL）
///
/// # 示例
/// ```ignore
/// // 确保 PIPE-WSOL pool 至少有 1000 SOL 的流动性
/// ensure_pipe_wsol_pool_liquidity(
///     &rpc,
///     "http://127.0.0.1:8899",
///     &payer,
///     1000,
/// ).await?;
/// ```
pub async fn ensure_pipe_wsol_pool_liquidity(
    rpc_client: &Arc<RpcClient>,
    rpc_url: &str,
    payer: &Keypair,
    min_wsol_sol: u64,
) -> Result<(), String> {
    use crate::cpmm_test_params::pipe_wsol_pool;

    let pool_address = pipe_wsol_pool();
    let min_wsol_lamports = min_wsol_sol * 1_000_000_000; // 转换为 lamports

    println!("🪙 检查 PIPE-WSOS Pool 流动性...");
    println!("   Pool: {}", pool_address);
    println!("   目标 WSOL 流动性: {} SOL ({} lamports)", min_wsol_sol, min_wsol_lamports);

    // 1. 获取池子状态
    let pool_state = get_pool_by_address(rpc_client, &pool_address)
        .await
        .map_err(|e| format!("获取池子状态失败: {}", e))?;

    // 2. 检查当前 WSOL vault 余额
    let current_wsol_balance = match rpc_client
        .get_token_account_balance(&pool_state.token1_vault)
        .await
    {
        Ok(balance_info) => balance_info.amount.parse::<u64>().unwrap_or(0),
        Err(_) => 0,
    };

    let current_wsol_sol = current_wsol_balance / 1_000_000_000;

    println!(
        "   当前 WSOL 流动性: {} SOL ({} lamports)",
        current_wsol_sol, current_wsol_balance
    );

    // 3. 如果流动性充足，直接返回
    if current_wsol_balance >= min_wsol_lamports {
        println!("✅ 流动性充足\n");
        return Ok(());
    }

    // 4. 计算需要添加的流动性
    let needed_wsol_lamports = min_wsol_lamports - current_wsol_balance;
    let needed_wsol_sol = needed_wsol_lamports / 1_000_000_000;

    println!(
        "💰 流动性不足，需要添加 {} SOL 的流动性...\n",
        needed_wsol_sol
    );

    // 5. 获取当前 PIPE vault 余额
    let current_pipe_balance = match rpc_client
        .get_token_account_balance(&pool_state.token0_vault)
        .await
    {
        Ok(balance_info) => balance_info.amount.parse::<u64>().unwrap_or(0),
        Err(_) => 0,
    };

    // 6. 根据 CPMM 公式计算需要添加的 PIPE 和 LP 数量
    // 公式: (added_wsol / current_wsol) = (added_lp / current_lp) = (added_pipe / current_pipe)
    let multiplier = (needed_wsol_lamports as u128) * 1000 / (current_wsol_balance as u128);
    let needed_lp =
        (pool_state.lp_supply as u128 * multiplier / 1000) as u64;
    let needed_pipe = ((current_pipe_balance as u128 * multiplier) / 1000) as u64;

    println!("📐 计算需要添加的流动性:");
    println!("   LP Token: {} (约 {:.2} 亿)", needed_lp, needed_lp as f64 / 100_000_000.0);
    println!(
        "   PIPE: {} (约 {:.2} 亿)",
        needed_pipe,
        needed_pipe as f64 / 100_000_000.0
    );
    println!(
        "   WSOL: {} ({} SOL)\n",
        needed_wsol_lamports, needed_wsol_sol
    );

    // 7. 转换为格式化字符串（用于 ensure_token_balance）
    // PIPE decimals = 6
    let needed_pipe_formatted = format!("{}", needed_pipe);
    // WSOL decimals = 9
    let needed_wsol_formatted = format!("{}", needed_wsol_lamports);

    // 8. 使用通用的 ensure_cpmm_liquidity 函数添加流动性
    ensure_cpmm_liquidity(
        rpc_client,
        rpc_url,
        payer,
        &pool_address,
        needed_lp,
        &needed_pipe_formatted,
        &needed_wsol_formatted,
    )
    .await
}

/// 确保池子有足够的流动性（添加流动性）
///
/// ⚠️ 仅适用于测试环境
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL
/// * `payer` - 账户 Keypair
/// * `pool_address` - CPMM 池子地址
/// * `lp_token_amount` - 要添加的 LP 代币数量
/// * `token0_amount_formatted` - Token0 数量（格式化，如 "1000"）
/// * `token1_amount_formatted` - Token1 数量（格式化，如 "1.5"）
///
/// # 示例
/// ```ignore
/// ensure_cpmm_liquidity(
///     &rpc,
///     "http://127.0.0.1:8899",
///     &payer,
///     &pool_address,
///     1_000_000_000, // 10 亿 LP
///     "10000",        // 10000 PIPE
///     "10",           // 10 WSOL
/// ).await?;
/// ```
pub async fn ensure_cpmm_liquidity(
    rpc_client: &Arc<RpcClient>,
    rpc_url: &str,
    payer: &Keypair,
    pool_address: &Pubkey,
    lp_token_amount: u64,
    token0_amount_formatted: &str,
    token1_amount_formatted: &str,
) -> Result<(), String> {
    let payer_pubkey = payer.pubkey();

    println!("🪙 检查池子流动性...");
    println!("   Pool: {}", pool_address);
    println!("   目标 LP 数量: {}", lp_token_amount);

    // 1. 获取池子状态
    let pool_state = get_pool_by_address(rpc_client, pool_address)
        .await
        .map_err(|e| format!("获取池子状态失败: {}", e))?;

    // 2. 检查当前 LP supply
    let current_lp_supply = pool_state.lp_supply;
    if current_lp_supply >= lp_token_amount {
        println!(
            "✅ 流动性充足: {} LP (目标: {} LP)",
            current_lp_supply, lp_token_amount
        );
        return Ok(());
    }

    println!(
        "💰 流动性不足，添加流动性... (当前: {} LP, 目标: {} LP)",
        current_lp_supply, lp_token_amount
    );

    // 3. 确保用户有足够的代币余额
    println!("\n📝 检查用户代币余额...");

    // 确保 Token0 余额
    ensure_token_balance(
        rpc_client,
        rpc_url,
        payer,
        &pool_state.token0_mint,
        token0_amount_formatted,
    )
    .await?;

    // 确保 Token1 余额
    ensure_token_balance(
        rpc_client,
        rpc_url,
        payer,
        &pool_state.token1_mint,
        token1_amount_formatted,
    )
    .await?;

    // 4. 获取当前金库余额
    let token0_balance = rpc_client
        .get_token_account_balance(&pool_state.token0_vault)
        .await;
    let token1_balance = rpc_client
        .get_token_account_balance(&pool_state.token1_vault)
        .await;

    let (_token0_reserve, _token1_reserve) = match (token0_balance, token1_balance) {
        (Ok(t0), Ok(t1)) => {
            let t0_amt = t0.amount.parse::<u64>().unwrap_or(0);
            let t1_amt = t1.amount.parse::<u64>().unwrap_or(0);
            (t0_amt, t1_amt)
        },
        _ => return Err("无法查询金库余额".to_string()),
    };

    // 5. 构建添加流动性指令
    println!("\n🔨 构建添加流动性指令...");

    // 派生 ATA 地址
    let owner_lp_token = get_associated_token_address_with_program_id(
        &payer_pubkey,
        &pool_state.lp_mint,
        &spl_token::id(),
    );

    let token_0_account = get_associated_token_address_with_program_id(
        &payer_pubkey,
        &pool_state.token0_mint,
        &spl_token::id(),
    );

    let token_1_account = get_associated_token_address_with_program_id(
        &payer_pubkey,
        &pool_state.token1_mint,
        &spl_token::id(),
    );

    // 解析目标金额作为最大值
    let mint_info0 = get_mint_info(rpc_client, &pool_state.token0_mint).await?;
    let max_token0 = parse_formatted_amount(token0_amount_formatted, mint_info0.decimals)?;

    let mint_info1 = get_mint_info(rpc_client, &pool_state.token1_mint).await?;
    let max_token1 = parse_formatted_amount(token1_amount_formatted, mint_info1.decimals)?;

    // 构建 deposit 指令
    let deposit_params = CpmmDepositParams {
        pool_state: *pool_address,
        owner_lp_token,
        token_0_account,
        token_1_account,
        token_0_vault: pool_state.token0_vault,
        token_1_vault: pool_state.token1_vault,
        token_0_mint: pool_state.token0_mint,
        token_1_mint: pool_state.token1_mint,
        lp_mint: pool_state.lp_mint,
        lp_token_amount: lp_token_amount - current_lp_supply, // 只添加差值
        maximum_token_0_amount: max_token0,
        maximum_token_1_amount: max_token1,
        token_program: spl_token::id(),
    };

    let deposit_instruction = build_deposit_instruction(deposit_params, payer_pubkey);

    // 6. 检查并创建 LP ATA
    let lp_ata_exists = rpc_client.get_account(&owner_lp_token).await.is_ok();

    let mut instructions = Vec::new();

    if !lp_ata_exists {
        println!("   📝 创建 LP ATA...");
        let create_lp_ata_instruction =
            spl_associated_token_account::instruction::create_associated_token_account(
                &payer_pubkey,
                &payer_pubkey,
                &pool_state.lp_mint,
                &spl_token::id(),
            );
        instructions.push(create_lp_ata_instruction);
    }

    instructions.push(deposit_instruction);

    // 7. 发送交易
    println!("\n🚀 发送添加流动性交易...");

    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| format!("获取 blockhash 失败: {}", e))?;

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer_pubkey),
        &[payer],
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .await
        .map_err(|e| format!("发送交易失败: {}", e))?;

    println!("✅ 添加流动性成功: {}", signature);

    // 8. 等待 2 秒
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // 9. 验证
    match get_pool_by_address(rpc_client, pool_address).await {
        Ok(new_pool_state) => {
            let new_lp_supply = new_pool_state.lp_supply;
            println!("\n✅ 流动性添加成功！");
            println!("   之前 LP Supply: {}", current_lp_supply);
            println!("   之后 LP Supply: {}", new_lp_supply);
            println!(
                "   增加: {}",
                new_lp_supply.saturating_sub(current_lp_supply)
            );
        },
        Err(e) => {
            println!("⚠️  无法验证: {}", e);
        },
    }

    Ok(())
}
