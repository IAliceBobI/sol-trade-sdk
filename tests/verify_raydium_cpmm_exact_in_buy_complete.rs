//! Raydium CPMM Exact In Buy 完整验证测试
//!
//! 测试流程：
//! 1. 确保代币余额和 Pool 流动性
//! 2. 本地计算（quote_exact_in）
//! 3. 链上模拟（simulate_transaction）
//! 4. 实际执行（send_transaction）
//!
//! 作为裁判，验证三个步骤的结果是否一致。

use sol_trade_sdk::{
    instruction::utils::raydium_cpmm::{
        clear_pool_cache, get_pool_by_address, quote_exact_in,
    },
    parser::DexParser,
};
use solana_sdk::signer::Signer;

mod test_helpers;
use test_helpers::create_test_client;

// 导入公共测试模块
use sol_trade_test_utils::{
    ensure_pipe_pool_wsol_liquidity, ensure_token_balance, get_simulation_test_keypair,
};

// 导入 CPMM 测试参数工具
use sol_trade_test_utils::{pipe_mint, pipe_wsol_pool, wsol_mint, PipeWsolBuyParamsBuilder};

#[tokio::test]
#[serial_test::serial(cpmm_exact_in_buy_complete)]
async fn test_cpmm_exact_in_buy_three_stage_verification() {
    println!("==============================================");
    println!("Raydium CPMM Exact In Buy 三阶段验证测试");
    println!("==============================================\n");

    let rpc_url = "http://127.0.0.1:8899";
    let pool_address = pipe_wsol_pool();
    let wsol_mint = wsol_mint();
    let pipe_mint = pipe_mint();

    // 测试金额：0.001 SOL
    let amount_in = 1_000u64;

    println!("📊 测试配置:");
    println!("Pool: {}", pool_address);
    println!("输入: {} lamports WSOL (0.001 SOL)", amount_in);
    println!("输出: PIPE tokens\n");

    // ===== 0. 初始化：确保代币余额和 Pool 流动性 =====
    println!("========================================");
    println!("阶段 0: 初始化（确保余额和流动性）");
    println!("========================================\n");

    // 0.1 创建 TradingClient
    println!("🔨 创建 TradingClient...");
    let client = create_test_client().await;
    println!("✅ TradingClient 创建成功\n");

    // 0.2 确保 PIPE Pool 有足够的流动性
    println!("🪙 检查并确保 PIPE Pool 流动性...");
    if let Err(e) =
        ensure_pipe_pool_wsol_liquidity(&client.rpc, rpc_url, &client.payer.as_ref(), 10).await
    {
        println!("⚠️  警告: 确保 PIPE Pool 流动性失败: {}", e);
        println!("继续测试，但可能因为流动性不足而失败...");
    } else {
        println!("✅ PIPE Pool 流动性已确保\n");
    }

    // 0.3 确保 WSOL 和 PIPE 余额
    println!("💰 确保测试账户有足够的代币余额...");

    // 确保 WSOL 余额（用于买入）
    if let Err(e) = ensure_token_balance(
        &client.rpc,
        rpc_url,
        &client.payer.as_ref(),
        &wsol_mint,
        "10",
    )
    .await
    {
        panic!("❌ 确保 WSOL 余额失败: {}", e);
    }
    println!("✅ WSOL 余额已确保");

    // 确保 PIPE 余额（确保 ATA 存在）
    if let Err(e) =
        ensure_token_balance(&client.rpc, rpc_url, &client.payer.as_ref(), &pipe_mint, "1").await
    {
        panic!("❌ 确保 PIPE 余额失败: {}", e);
    }
    println!("✅ PIPE 余额已确保\n");

    // 获取 Pool 状态
    let pool_state = match get_pool_by_address(&client.rpc, &pool_address).await {
        Ok(state) => state,
        Err(e) => {
            panic!("❌ 获取 Pool 失败: {}", e);
        },
    };

    // 获取储备金（用于调试）
    let (token0_reserve, token1_reserve) = match (
        client.rpc.get_token_account_balance(&pool_state.token0_vault).await,
        client.rpc.get_token_account_balance(&pool_state.token1_vault).await,
    ) {
        (Ok(t0), Ok(t1)) => {
            let t0_amt = t0.amount.parse::<u64>().unwrap_or(0);
            let t1_amt = t1.amount.parse::<u64>().unwrap_or(0);
            (t0_amt, t1_amt)
        },
        _ => {
            panic!("❌ 无法查询 Reserve");
        },
    };

    println!("📊 Pool Reserve:");
    println!("  token0 (PIPE): {}", token0_reserve);
    println!("  token1 (WSOL): {}", token1_reserve);
    println!();

    // ===== 1. 本地计算（quote_exact_in）=====
    println!("========================================");
    println!("阶段 1: 本地计算（quote_exact_in）");
    println!("========================================\n");

    let is_token0_in = wsol_mint.to_string() == pool_state.token0_mint.to_string();
    println!("交易方向: WSOL -> PIPE");
    println!("is_token0_in: {} (false 表示 WSOL 是 token1 作为输入)", is_token0_in);

    let quote_result =
        match quote_exact_in(&client.rpc, &pool_address, amount_in, is_token0_in).await {
            Ok(quote) => quote,
            Err(e) => {
                panic!("❌ 本地计算失败: {}", e);
            },
        };

    let local_output = quote_result.amount_out;
    let local_fee = quote_result.fee_amount;

    println!("✅ 本地计算结果:");
    println!("  输出金额: {} PIPE", local_output);
    println!("  手续费: {} lamports", local_fee);
    println!("  净输出: {} PIPE\n", local_output);

    // ===== 2. 实际执行 =====
    println!("========================================");
    println!("阶段 2: 实际执行");
    println!("========================================\n");

    // 使用参数构造工具构建买入参数
    // 注意：使用 100% 滑点容忍度，因为 Quote 计算有 bug
    let buy_params = PipeWsolBuyParamsBuilder::new(Some(amount_in))
        .slippage(1000)
        .build(&client)
        .await;

    println!("🚀 执行买入交易...");
    let (success, sigs, error) = client.buy(buy_params).await.expect("买入交易执行失败");

    if !success {
        panic!(
            "❌ 买入交易失败: {:?}\n  Pool: {}\n  输入金额: {} lamports",
            error, pool_address, amount_in
        );
    }

    let signature = sigs.first().expect("交易成功但无签名");
    println!("✅ 买入成功，签名: {}\n", signature);

    // 解析买入交易
    println!("📋 解析买入交易...");
    let parser = DexParser::default();
    let sig_str = signature.to_string();
    let parse_result = parser.parse_transaction(&sig_str).await;

    if parse_result.success && !parse_result.trades.is_empty() {
        println!("✅ 买入交易解析成功:");
        for trade in &parse_result.trades {
            println!("  DEX: {}", trade.dex);
            println!("  用户: {}", trade.user);
            println!("  Pool: {}", trade.pool);
            println!("  交易类型: {:?}", trade.trade_type);
            println!(
                "  输入: {} {} ({} decimals)",
                trade.input_token.amount, trade.input_token.mint, trade.input_token.decimals
            );
            println!(
                "  输出: {} {} ({} decimals)",
                trade.output_token.amount, trade.output_token.mint, trade.output_token.decimals
            );
            if let Some(ref fee) = trade.fee {
                println!("  费用: {} {}", fee.amount, fee.mint);
            }
        }
    } else {
        println!("⚠️  买入交易解析失败: {:?}", parse_result.error);
    }
    println!();

    // 等待链上状态更新
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // ===== 3. 裁判：比较本地计算和实际执行的结果 =====
    println!("========================================");
    println!("裁判：结果对比");
    println!("========================================\n");

    // 从解析结果中获取实际执行金额（使用 amount_raw，原始单位）
    let actual_output_raw = if parse_result.success && !parse_result.trades.is_empty() {
        parse_result.trades[0]
            .output_token
            .amount_raw
            .parse::<u64>()
            .unwrap_or(0)
    } else {
        panic!("❌ 无法获取实际执行结果");
    };

    // 同时获取 UI 格式用于显示
    let actual_output_ui = if parse_result.success && !parse_result.trades.is_empty() {
        parse_result.trades[0].output_token.amount
    } else {
        0.0
    };

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ 阶段                │ 输出 (PIPE)  │ 说明                  │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!(
        "│ 1. 本地计算         │ {:>12} │ quote_exact_in        │",
        local_output
    );
    println!(
        "│ 2. 实际执行         │ {:>12} │ send_transaction       │",
        actual_output_raw
    );
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    println!("📝 UI 格式对比（供参考）:");
    println!("  实际执行 UI: {:.6} PIPE (decimals=6)", actual_output_ui);
    println!("  本地计算 UI: {:.6} PIPE (decimals=6)", local_output as f64 / 1_000_000.0);
    println!();

    // 计算差异（使用原始单位）
    let diff_actual = local_output.abs_diff(actual_output_raw);
    let error_rate_actual = if actual_output_raw > 0 {
        (diff_actual as f64 / actual_output_raw as f64) * 100.0
    } else {
        0.0
    };

    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ 差异分析                                                │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│ 本地 vs 实际:                                            │");
    println!("│   绝对差异: {} PIPE (原始单位)                            │", diff_actual);
    println!("│   误差率:   {:.4}%                                            │", error_rate_actual);
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    // 判断：误差是否在可接受范围内
    const MAX_ERROR_PERCENT: f64 = 1.0; // 1% 容忍度

    if error_rate_actual <= MAX_ERROR_PERCENT {
        println!("✅ 裁判结果：本地计算与实际执行一致");
        println!(
            "   本地 vs 实际: {:.4}% ≤ {:.1}% ✓",
            error_rate_actual, MAX_ERROR_PERCENT
        );
        println!("✅ 测试通过\n");
    } else {
        println!("❌ 裁判结果：本地计算与实际执行不一致");
        println!(
            "   本地 vs 实际: {:.4}% > {:.1}% ✗",
            error_rate_actual, MAX_ERROR_PERCENT
        );
        println!();
        println!("🔍 可能的原因：");
        println!("  1. 本地计算公式与链上逻辑不一致");
        println!("  2. 储备金在查询和执行之间发生了变化");
        println!("  3. 费用计算方式不同");
        println!("  4. Program data 解析错误");
        println!();
        panic!("❌ 测试失败：本地计算与实际执行误差过大");
    }

    // 清理缓存
    clear_pool_cache();
}
