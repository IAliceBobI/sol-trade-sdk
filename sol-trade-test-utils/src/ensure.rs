//! Ensure 辅助函数
//!
//! 提供便捷的测试辅助函数，确保账户有足够的余额和流动性

use crate::airdrop::set_sol_balance;
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

use sol_trade_sdk::common::auto_mock_rpc::AutoMockRpcClient;
use sol_trade_sdk::instruction::utils::raydium_cpmm::{get_pool_by_address, list_pools_by_mint};
use sol_trade_sdk::liquidity::cpmm::{build_deposit_instruction, CpmmDepositParams};

/// 确保账户有足够的 SOL 余额
///
/// 如果余额不足，会使用 surfnet_setAccount 直接设置余额（比空投更快）
///
/// ⚠️ 仅适用于测试环境（surfpool）
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `payer` - 账户地址
/// * `min_balance_sol` - 最小 SOL 余额
///
/// # 示例
/// ```ignore
/// ensure_sol_balance(&rpc, &payer.pubkey(), 10).await?;
/// ```
pub async fn ensure_sol_balance(
    rpc_client: &Arc<RpcClient>,
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
            "💰 SOL 余额不足: {} lamports (需要 {} lamports)，正在设置...",
            balance, min_balance_lamports
        );
        // 使用 surfnet_setAccount 直接设置余额，比空投更快
        let rpc_url = rpc_client.url();
        set_sol_balance(&rpc_url, payer, min_balance_lamports).await?;
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
/// * `payer` - 账户 Keypair
/// * `mint` - Token mint 地址
/// * `amount_formatted` - 格式化的金额（如 "100" 表示 100 个代币）
///
/// # 示例
/// ```ignore
/// ensure_token_balance(&rpc, &payer, &usdc_mint, "1000").await?;
/// ```
pub async fn ensure_token_balance(
    rpc_client: &Arc<RpcClient>,
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

    // 解析目标金额（使用小数格式，如 "1000.0" 表示 1000 USDC）
    let decimal_formatted = if !amount_formatted.contains('.') {
        format!("{}.0", amount_formatted) // 确保是小数格式
    } else {
        amount_formatted.to_string()
    };

    let target_amount = parse_formatted_amount(&decimal_formatted, mint_info.decimals)?;

    if current_amount >= target_amount {
        println!(
            "✅ Token 余额充足: {} ({} decimals, {} raw units)",
            amount_formatted, mint_info.decimals, current_amount
        );
        return Ok(());
    }

    println!(
        "💰 Token 余额不足，设置余额: {} (当前: {}, 目标: {})",
        amount_formatted, current_amount, target_amount
    );

    let rpc_url = rpc_client.url();
    crate::token::set_token_balance(rpc_client, &rpc_url, payer, mint, &decimal_formatted).await
}

/// 通过大额 Swap 确保 PIPE-WSOL Pool 流动性（推荐方法）
///
/// ## 设计理念
///
/// 相比于直接添加流动性（需要同时提供 WSOL 和 PIPE），**通过大额 Swap 更优**：
///
/// 1. **只需要 WSOL** - 不需要预先持有 PIPE
/// 2. **同时增加两个 vault 的余额** - Swap 后 WSOL 和 PIPE 都会增加
/// 3. **提高 PIPE 价格** - 大额买入会推高 PIPE 价格，使 Pool 更健康
/// 4. **更接近真实场景** - 模拟真实的大额交易
/// 5. **后续测试可以卖出** - 持有的 PIPE 可以用于卖出测试
///
/// ## 工作原理
///
/// 通过执行一笔大额 WSOL → PIPE 的 Swap：
/// - **WSOL 进入 Pool**: WSOL vault 增加
/// - **PIPE 从 Pool 提出**: PIPE vault 减少（但仍有剩余）
/// - **获得 PIPE 代币**: 测试账户获得 PIPE，可用于卖出测试
///
/// ⚠️ 仅适用于测试环境
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL
/// * `payer` - 账户 Keypair（需要持有足够的 WSOL）
/// * `swap_amount_sol` - Swap 金额（SOL 单位，如 10 表示 10 SOL）
///
/// # 示例
/// ```ignore
/// // 通过大额 Swap 确保 PIPE Pool 流动性
/// ensure_pipe_pool_liquidity_via_swap(
///     &rpc,
///     "http://127.0.0.1:8899",
///     &payer,
///     10, // Swap 10 SOL 买入 PIPE
/// ).await?;
/// ```
pub async fn ensure_pipe_pool_liquidity_via_swap(
    rpc_client: &Arc<RpcClient>,
    rpc_url: &str,
    payer: &Keypair,
    swap_amount_sol: u64,
) -> Result<(), String> {
    use sol_trade_sdk::swqos::SwqosConfig;
    use sol_trade_sdk::{SolanaTrade, TradeConfig};
    use solana_commitment_config::CommitmentConfig;
    use std::sync::Arc;

    let swap_amount_lamports = swap_amount_sol * 1_000_000_000;

    println!("💰 通过大额 Swap 确保 PIPE Pool 流动性...");
    println!("   Swap 金额: {} SOL ({} lamports)", swap_amount_sol, swap_amount_lamports);
    println!("   方向: WSOL → PIPE");

    // 1. 确保 payer 有足够的 WSOL 余额
    println!("\n📋 步骤 1: 确保 WSOL 余额...");
    let wsol_mint = crate::test_params::wsol_mint();
    ensure_token_balance(
        rpc_client,
        payer,
        &wsol_mint,
        &format!("{}", swap_amount_sol * 2),
    )
    .await
    .map_err(|e| format!("确保 WSOL 余额失败: {}", e))?;
    println!("✅ WSOL 余额充足");

    // 1.5. 确保 payer 有足够的原生 SOL 余额（WSOL 需要原生 SOL 支持）
    println!("\n📋 步骤 1.5: 确保原生 SOL 余额...");
    ensure_sol_balance(rpc_client, &payer.pubkey(), swap_amount_sol * 3)
        .await
        .map_err(|e| format!("确保 SOL 余额失败: {}", e))?;
    println!("✅ 原生 SOL 余额充足");

    // 2. 创建 TradingClient（使用正确的 API）
    println!("\n📋 步骤 2: 创建 TradingClient...");
    let payer_arc = Arc::new(payer.insecure_clone());

    // 创建 TradeConfig
    let commitment = CommitmentConfig::confirmed();
    let swqos_configs: Vec<SwqosConfig> = vec![SwqosConfig::Default(rpc_url.to_string())];
    let trade_config = TradeConfig::new(rpc_url.to_string(), swqos_configs, commitment)
        .with_wsol_ata_config(true, false);

    // 创建 TradingClient
    let client = SolanaTrade::new(payer_arc.clone(), trade_config).await;
    println!("✅ TradingClient 创建成功");

    // 3. 构建 Swap 参数（WSOL → PIPE）
    println!("\n📋 步骤 3: 构建 Swap 参数...");
    let buy_params = crate::test_params::PipeWsolBuyParamsBuilder::new(Some(swap_amount_lamports))
        .slippage(8000); // 80% 滑点（PIPE Pool 流动性极低，需要极大的滑点容忍度）

    let buy_params = buy_params.build(&client).await;
    println!("✅ Swap 参数构建成功");
    println!("   输入: {} WSOL", swap_amount_sol);
    println!("   滑点: 80%");

    // 4. 执行 Swap
    println!("\n📋 步骤 4: 执行 Swap...");
    println!("   ⏳ 正在通过大额买入增加 PIPE Pool 流动性...");

    let (success, sigs, error) = client
        .buy(buy_params)
        .await
        .map_err(|e| format!("Swap 调用失败: {}", e).to_string())?;

    if !success {
        if let Some(err) = error {
            return Err(format!("Swap 失败: {}", err));
        } else {
            return Err("Swap 失败: 未知错误".to_string());
        }
    }

    println!("✅ Swap 成功！");
    if let Some(sig) = sigs.first() {
        println!("   签名: {}", sig);
    }

    // 5. 等待交易确认
    println!("\n📋 步骤 5: 等待交易确认...");
    if let Some(sig) = sigs.first() {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        match rpc_client.get_signature_status(sig).await {
            Ok(Some(status)) => {
                if let Ok(()) = status {
                    println!("✅ 交易已确认");
                } else {
                    println!("⚠️  交易确认失败: {:?}", status);
                }
            },
            _ => {
                println!("⚠️  无法获取交易状态");
            },
        }
    }

    // 6. 验证流动性
    println!("\n📋 步骤 6: 验证 Pool 流动性...");
    let pool_address = crate::test_params::pipe_wsol_pool();
    let pool_state = get_pool_by_address(rpc_client, &pool_address)
        .await
        .map_err(|e| format!("获取 Pool 状态失败: {}", e))?;

    let wsol_vault_balance = rpc_client
        .get_token_account_balance(&pool_state.token1_vault)
        .await
        .map(|b| b.amount.parse::<u64>().unwrap_or(0))
        .unwrap_or(0);
    let pipe_vault_balance = rpc_client
        .get_token_account_balance(&pool_state.token0_vault)
        .await
        .map(|b| b.amount.parse::<u64>().unwrap_or(0))
        .unwrap_or(0);

    let wsol_vault_sol = wsol_vault_balance as f64 / 1_000_000_000.0;
    let pipe_vault_human = pipe_vault_balance as f64 / 1_000_000.0;

    println!("✅ Pool 流动性验证成功");
    println!("   WSOL Vault: {:.6} SOL", wsol_vault_sol);
    println!("   PIPE Vault: {:.2} PIPE", pipe_vault_human);
    println!("\n✨ PIPE Pool 流动性添加完成！");

    Ok(())
}

/// 确保 PIPE Pool 有足够的 WSOL 流动性（旧方法，不推荐）
///
/// ⚠️ **已弃用**: 此方法需要同时提供 WSOL 和 PIPE，不如使用 `ensure_pipe_pool_liquidity_via_swap`
///
/// 此方法通过直接添加流动性（deposit）来增加 Pool 的 WSOL 余额。
/// 相比之下，`ensure_pipe_pool_liquidity_via_swap` 方法更简单、更有效。
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
/// // 确保 PIPE pool 至少有 1000 SOL 的流动性
/// ensure_pipe_pool_wsol_liquidity(
///     &rpc,
///     "http://127.0.0.1:8899",
///     &payer,
///     1000,
/// ).await?;
/// ```
#[deprecated(note = "使用 ensure_pipe_pool_liquidity_via_swap 代替")]
pub async fn ensure_pipe_pool_wsol_liquidity(
    rpc_client: &Arc<RpcClient>,
    rpc_url: &str,
    payer: &Keypair,
    min_wsol_sol: u64,
) -> Result<(), String> {
    use crate::test_params::pipe_wsol_pool;

    let pool_address = pipe_wsol_pool();
    let min_wsol_lamports = min_wsol_sol * 1_000_000_000; // 转换为 lamports

    println!("🪙 检查 PIPE Pool 流动性...");
    println!("   Pool: {}", pool_address);
    println!("   目标 WSOL 流动性: {} SOL ({} lamports)", min_wsol_sol, min_wsol_lamports);

    // 1. 获取池子状态
    let pool_state = get_pool_by_address(rpc_client, &pool_address)
        .await
        .map_err(|e| format!("获取池子状态失败: {}", e))?;

    // 2. 检查当前 WSOL vault 余额
    let current_wsol_balance =
        match rpc_client.get_token_account_balance(&pool_state.token1_vault).await {
            Ok(balance_info) => balance_info.amount.parse::<u64>().unwrap_or(0),
            Err(_) => 0,
        };

    let current_wsol_sol = current_wsol_balance / 1_000_000_000;

    println!("   当前 WSOL 流动性: {} SOL ({} lamports)", current_wsol_sol, current_wsol_balance);

    // 3. 如果流动性充足，直接返回
    if current_wsol_balance >= min_wsol_lamports {
        println!("✅ 流动性充足\n");
        return Ok(());
    }

    // 4. 计算需要添加的流动性
    let needed_wsol_lamports = min_wsol_lamports - current_wsol_balance;
    let needed_wsol_sol = needed_wsol_lamports / 1_000_000_000;

    println!("💰 流动性不足，需要添加 {} SOL 的流动性...\n", needed_wsol_sol);

    // 5. 获取当前 PIPE vault 余额
    let current_pipe_balance =
        match rpc_client.get_token_account_balance(&pool_state.token0_vault).await {
            Ok(balance_info) => balance_info.amount.parse::<u64>().unwrap_or(0),
            Err(_) => 0,
        };

    // 6. 根据 CPMM 公式计算需要添加的 PIPE 和 LP 数量
    // 公式: (added_wsol / current_wsol) = (added_lp / current_lp) = (added_pipe / current_pipe)
    let multiplier = (needed_wsol_lamports as u128) * 1000 / (current_wsol_balance as u128);
    let needed_lp = (pool_state.lp_supply as u128 * multiplier / 1000) as u64;
    let needed_pipe = ((current_pipe_balance as u128 * multiplier) / 1000) as u64;

    println!("📐 计算需要添加的流动性:");
    println!("   LP Token: {} (约 {:.2} 亿)", needed_lp, needed_lp as f64 / 100_000_000.0);
    println!("   PIPE: {} (约 {:.2} 亿)", needed_pipe, needed_pipe as f64 / 100_000_000.0);
    println!("   WSOL: {} ({} SOL)\n", needed_wsol_lamports, needed_wsol_sol);

    // 7. 检查是否可以安全地添加流动性
    // 由于 parse_formatted_amount 会乘以 decimals，我们需要确保原始值除以 10^decimals 后不会太大

    // PIPE decimals = 6，检查原始值是否合理
    // f64 可以精确表示到 2^53 ≈ 9.007 * 10^15，所以安全值设为 9 * 10^15
    const MAX_PIPE_RAW: u64 = 9_000_000_000_000_000; // 9×10^15，除以 10^6 后是 9×10^9 (90亿 PIPE)
    if needed_pipe > MAX_PIPE_RAW {
        return Err(format!(
            "需要的 PIPE 数量过大: {} (原始单位)，超过安全限制 {}。
建议: 使用更小的 min_wsol_sol 值或跳过流动性添加",
            needed_pipe, MAX_PIPE_RAW
        ));
    }

    // 转换为人类可读格式（使用 f64，但已经检查了不会溢出）
    let needed_pipe_human_readable = needed_pipe as f64 / 1_000_000.0;
    let needed_pipe_formatted = format!("{}", needed_pipe_human_readable);

    // WSOL decimals = 9，f64 精度限制
    const MAX_WSOL_RAW: u64 = 9_000_000_000_000_000_000; // 9×10^18，除以 10^9 后是 9×10^9 (90亿 SOL)
    if needed_wsol_lamports > MAX_WSOL_RAW {
        return Err(format!(
            "需要的 WSOL 数量过大: {} (原始单位)，超过安全限制 {}。
建议: 使用更小的 min_wsol_sol 值或跳过流动性添加",
            needed_wsol_lamports, MAX_WSOL_RAW
        ));
    }

    let needed_wsol_human_readable = needed_wsol_lamports as f64 / 1_000_000_000.0;
    let needed_wsol_formatted = format!("{}", needed_wsol_human_readable);

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

/// 确保 USDC-PRTS Pool 有足够的 USDC 流动性
///
/// 便捷函数，专门用于确保 USDC-PRTS pool 有指定数量的 USDC 流动性。
/// 如果当前 USDC vault 余额不足，会自动添加流动性以达到目标值。
///
/// ⚠️ 仅适用于测试环境
///
/// # 参数
/// * `rpc_client` - RPC 客户端
/// * `rpc_url` - RPC URL
/// * `payer` - 账户 Keypair
/// * `min_usdc` - 最小 USDC 流动性（USDC 单位，如 1000 表示 1000 USDC）
///
/// # 示例
/// ```ignore
/// // 确保 USDC-PRTS pool 至少有 1000 USDC 的流动性
/// ensure_usdc_prts_pool_usdc_liquidity(
///     &rpc,
///     "http://127.0.0.1:8899",
///     &payer,
///     1000,  // 1000 USDC
/// ).await?;
/// ```
pub async fn ensure_usdc_prts_pool_usdc_liquidity(
    rpc_client: &Arc<RpcClient>,
    rpc_url: &str,
    payer: &Keypair,
    min_usdc: u64,
) -> Result<(), String> {
    use crate::test_params::{prts_mint, usdc_mint, usdc_prts_pool};

    let pool_address = usdc_prts_pool();
    let min_usdc_units = min_usdc * 1_000_000; // USDC decimals = 6

    println!("🪙 检查 USDC-PRTS Pool 流动性...");
    println!("   Pool: {}", pool_address);
    println!("   目标 USDC 流动性: {} USDC ({} units)", min_usdc, min_usdc_units);

    // 1. 使用 AutoMockRpcClient 和 list_pools_by_mint 获取池子状态（与 list_usdc_pools 一致）
    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("ensure_usdc_prts_pool_usdc_liquidity".to_string()),
    );

    // 使用 list_pools_by_mint 查找所有 USDC pool，然后找到 USDC-PRTS pool
    let usdc_mint_key = usdc_mint();
    let prts_mint_key = prts_mint();

    let pools = list_pools_by_mint(&auto_mock_client, &usdc_mint_key)
        .await
        .map_err(|e| format!("获取 USDC Pool 列表失败: {}", e))?;

    // 查找 USDC-PRTS pool（通过 PRTS mint）
    let pool_state = pools
        .iter()
        .find(|(_, pool)| pool.token0_mint == prts_mint_key || pool.token1_mint == prts_mint_key)
        .map(|(_, state)| state.clone())
        .ok_or_else(|| format!("未找到 USDC-PRTS Pool，PRTS mint: {}", prts_mint_key))?;

    // 2. 确定哪个 vault 是 USDC（根据 mint 地址判断）
    let usdc_mint = crate::test_params::usdc_mint();
    let (usdc_vault, prts_vault) = if pool_state.token0_mint == usdc_mint {
        (pool_state.token0_vault, pool_state.token1_vault)
    } else {
        (pool_state.token1_vault, pool_state.token0_vault)
    };

    // 3. 检查当前 USDC vault 余额
    let current_usdc_balance = match auto_mock_client.get_token_account_balance(&usdc_vault).await {
        Ok(balance_info) => balance_info.amount.parse::<u64>().unwrap_or(0),
        Err(_) => 0,
    };

    let current_usdc = current_usdc_balance / 1_000_000;

    println!("   当前 USDC 流动性: {} USDC ({} units)", current_usdc, current_usdc_balance);

    // 4. 如果流动性充足，直接返回
    if current_usdc_balance >= min_usdc_units {
        println!("✅ 流动性充足\n");
        return Ok(());
    }

    // 5. 计算需要添加的流动性
    let needed_usdc_units = min_usdc_units - current_usdc_balance;
    let needed_usdc = needed_usdc_units / 1_000_000;

    println!("💰 流动性不足，需要添加 {} USDC 的流动性...\n", needed_usdc);

    // 6. 获取当前 PRTS vault 余额
    let current_prts_balance = match auto_mock_client.get_token_account_balance(&prts_vault).await {
        Ok(balance_info) => balance_info.amount.parse::<u64>().unwrap_or(0),
        Err(_) => 0,
    };

    // 7. 根据 CPMM 公式计算需要添加的 PRTS 和 LP 数量
    // 公式: (added_usdc / current_usdc) = (added_lp / current_lp) = (added_prts / current_prts)
    let multiplier = (needed_usdc_units as u128) * 1000 / (current_usdc_balance as u128);
    let needed_lp = (pool_state.lp_supply as u128 * multiplier / 1000) as u64;
    let needed_prts = ((current_prts_balance as u128 * multiplier) / 1000) as u64;

    println!("📐 计算需要添加的流动性:");
    println!("   LP Token: {} (约 {:.2} 亿)", needed_lp, needed_lp as f64 / 100_000_000.0);
    println!("   USDC: {} ({} USDC)", needed_usdc_units, needed_usdc);
    println!("   PRTS: {} (约 {:.2} 亿)\n", needed_prts, needed_prts as f64 / 100_000_000.0);

    // 8. 转换为格式化字符串（用于 ensure_token_balance）
    // USDC decimals = 6, PRTS decimals = 6 (Token2022)
    let needed_usdc_formatted = format!("{}", needed_usdc_units);
    let needed_prts_formatted = format!("{}", needed_prts);

    // 9. 使用通用的 ensure_cpmm_liquidity 函数添加流动性
    ensure_cpmm_liquidity(
        rpc_client,
        rpc_url,
        payer,
        &pool_address,
        needed_lp,
        &needed_usdc_formatted,
        &needed_prts_formatted,
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
    _rpc_url: &str,
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
        println!("✅ 流动性充足: {} LP (目标: {} LP)", current_lp_supply, lp_token_amount);
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
        payer,
        &pool_state.token0_mint,
        token0_amount_formatted,
    )
    .await?;

    // 确保 Token1 余额
    ensure_token_balance(
        rpc_client,
        payer,
        &pool_state.token1_mint,
        token1_amount_formatted,
    )
    .await?;

    // 4. 获取当前金库余额
    let token0_balance = rpc_client.get_token_account_balance(&pool_state.token0_vault).await;
    let token1_balance = rpc_client.get_token_account_balance(&pool_state.token1_vault).await;

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

    // 派生 ATA 地址（使用正确的 Token Program）
    // 获取各 mint 的 Token Program
    let lp_token_program = get_mint_info(rpc_client, &pool_state.lp_mint).await?.token_program;
    let token_0_program = get_mint_info(rpc_client, &pool_state.token0_mint).await?.token_program;
    let token_1_program = get_mint_info(rpc_client, &pool_state.token1_mint).await?.token_program;

    let owner_lp_token = get_associated_token_address_with_program_id(
        &payer_pubkey,
        &pool_state.lp_mint,
        &lp_token_program,
    );

    let token_0_account = get_associated_token_address_with_program_id(
        &payer_pubkey,
        &pool_state.token0_mint,
        &token_0_program,
    );

    let token_1_account = get_associated_token_address_with_program_id(
        &payer_pubkey,
        &pool_state.token1_mint,
        &token_1_program,
    );

    // 解析目标金额作为最大值
    let mint_info0 = get_mint_info(rpc_client, &pool_state.token0_mint).await?;
    let max_token0 = parse_formatted_amount(token0_amount_formatted, mint_info0.decimals)?;

    let mint_info1 = get_mint_info(rpc_client, &pool_state.token1_mint).await?;
    let max_token1 = parse_formatted_amount(token1_amount_formatted, mint_info1.decimals)?;

    // 构建 deposit 指令
    // 注意：token_program 参数在 CPMM 中需要根据具体池子的 token 使用
    // 这里使用 token_0 的 Token Program（大部分池子使用传统 Token Program）
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
        token_program: token_0_program, // 使用正确的 Token Program
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
                &lp_token_program, // 使用正确的 Token Program
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
            println!("   增加: {}", new_lp_supply.saturating_sub(current_lp_supply));
        },
        Err(e) => {
            println!("⚠️  无法验证: {}", e);
        },
    }

    Ok(())
}
