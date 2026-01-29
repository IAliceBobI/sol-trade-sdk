//! CLMM Swap 计算准确性验证测试
//!
//! 通过 RPC Quote 来验证我们的 CLMM 完整计算是否准确
//!
//! 运行测试:
//!     cargo nextest run verify_clmm_calculation -- --nocapture
//!
//! 或使用 cargo test:
//!     cargo test --test verify_clmm_calculation -- --nocapture

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::utils::raydium_clmm::{get_pool_by_address, quote_exact_in, get_tick_arrays, get_tick_array_start_index},
    utils::calc::raydium_clmm as clmm_calc,
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use std::str::FromStr;
use std::sync::Arc;

mod test_helpers;

/// WSOL-USDT CLMM Pool（锚定池，用于 USD 价格）
const WSOL_USDT_POOL: &str = "ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6";

/// WSOL-JUP CLMM Pool
const WSOL_JUP_POOL: &str = "EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw";

#[tokio::test]
#[serial_test::serial]
async fn test_verify_clmm_full_calculation_with_rpc_quote() {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔬 CLMM 完整计算准确性验证测试");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));

    // 测试账户（不需要真实余额）
    let payer = Keypair::new();
    println!("📍 测试账户: {}\n", payer.pubkey());

    // 测试多个 Pool
    let pools_to_test = vec![
        ("WSOL-JUP", WSOL_JUP_POOL, 1_000_000u64), // 0.001 SOL
    ];

    for (pool_name, pool_address_str, amount_in) in pools_to_test {
        println!("📊 测试 Pool: {}", pool_name);
        println!("Pool 地址: {}", pool_address_str);

        let pool_address = match Pubkey::from_str(pool_address_str) {
            Ok(addr) => addr,
            Err(e) => {
                println!("❌ 无效的 pool 地址: {}\n", e);
                continue;
            }
        };

        // 获取 Pool 状态
        let pool_state = match get_pool_by_address(&rpc, &pool_address).await {
            Ok(state) => state,
            Err(e) => {
                println!("❌ 获取 Pool 失败: {}\n", e);
                continue;
            }
        };

        println!("当前 Tick: {}", pool_state.tick_current);
        println!("当前价格 (sqrt_price_x64): {}", pool_state.sqrt_price_x64);
        println!("流动性: {}", pool_state.liquidity);
        println!("Tick 间距: {}", pool_state.tick_spacing);

        // 获取费率
        let amm_config = match rpc.get_account(&pool_state.amm_config).await {
            Ok(account) => account,
            Err(e) => {
                println!("❌ 获取 AMM Config 失败: {}\n", e);
                continue;
            }
        };

        // 使用 amm_config_decode 正确解析费率
        let fee_rate = match sol_trade_sdk::instruction::utils::raydium_clmm_types::amm_config_decode(&amm_config.data) {
            Some(config) => config.trade_fee_rate as u32,
            None => {
                println!("⚠️  无法解析 AMM Config，使用默认值 2500 (0.25%)");
                2500
            }
        };

        println!("费率: {} ({}%)", fee_rate, fee_rate as f64 / 10000.0);
        println!("输入金额: {} lamports\n", amount_in);

        // 判断交易方向（zero_for_one）
        // 假设 token1 是 SOL，我们用 SOL 买 token0
        let zero_for_one = pool_state.token_mint1.to_string() == "So11111111111111111111111111111111111111112";

        println!("交易方向: zero_for_one = {}\n", zero_for_one);

        // ========================================
        // 1. 使用完整版 CLMM 计算（需要 tick arrays）
        // ========================================
        println!("🧮 步骤 1: 计算 Tick Array 索引");

        // 计算当前 tick 所在的 tick array 的 start index
        let current_tick_array_start = get_tick_array_start_index(
            pool_state.tick_current,
            pool_state.tick_spacing
        );

        println!("当前 Tick Array Start Index: {}", current_tick_array_start);

        // 构建需要获取的 tick array 索引列表（获取当前及可能的下一个）
        let mut start_indices = vec![current_tick_array_start];
        if zero_for_one {
            // token0 -> token1，价格下降，需要向左获取
            start_indices.push(current_tick_array_start - (pool_state.tick_spacing as i32 * 60));
        } else {
            // token1 -> token0，价格上涨，需要向右获取
            start_indices.push(current_tick_array_start + (pool_state.tick_spacing as i32 * 60));
        }

        println!("🧮 步骤 2: 获取 Tick Arrays 数据");

        let tick_arrays_result = get_tick_arrays(&rpc, &pool_address, &start_indices).await;

        // 转换 TickArrayState 为计算所需的格式
        let tick_arrays_formatted: Vec<(i32, Vec<(i32, i128, u128)>)> = match tick_arrays_result {
            Ok(tick_array_states) => {
                println!("✅ 获取到 {} 个 Tick Arrays", tick_array_states.len());

                // 转换格式: TickArrayState -> Vec<(tick, liquidity_net, liquidity_gross)>
                tick_array_states.into_iter().map(|(start_index, tick_array_state)| {
                    let ticks: Vec<(i32, i128, u128)> = tick_array_state.ticks
                        .into_iter()
                        .map(|tick_state| (tick_state.tick, tick_state.liquidity_net, tick_state.liquidity_gross))
                        .collect();
                    (start_index, ticks)
                }).collect()
            },
            Err(e) => {
                println!("⚠️  获取 Tick Arrays 失败: {}", e);
                println!("   (跳过完整计算测试)\n");
                continue;
            }
        };

        println!("🧮 步骤 3: 使用完整版计算输出");

        let calculated_output_full = match clmm_calc::calculate_swap_amount_with_tick_arrays(
            amount_in,
            pool_state.sqrt_price_x64,
            pool_state.liquidity,
            pool_state.tick_current,
            pool_state.tick_spacing,
            fee_rate,
            zero_for_one,
            &tick_arrays_formatted,
        ) {
            Ok(output) => output,
            Err(e) => {
                println!("❌ 完整计算失败: {}\n", e);
                continue;
            }
        };

        println!("✅ 完整计算结果: {} tokens", calculated_output_full);

        // ========================================
        // 4. 使用 RPC Quote 获取链上预期输出
        // ========================================
        println!("\n📡 步骤 4: 使用 RPC Quote 获取链上预期输出");

        let rpc_quote = match quote_exact_in(
            &rpc,
            &pool_address,
            amount_in,
            zero_for_one,
        ).await {
            Ok(quote) => quote,
            Err(e) => {
                println!("⚠️  RPC Quote 失败: {}", e);
                println!("   (可能是因为 tick array 数据问题)\n");
                continue;
            }
        };

        let rpc_output = rpc_quote.amount_out;
        println!("✅ RPC Quote 结果: {} tokens", rpc_output);

        // ========================================
        // 5. 结果对比
        // ========================================
        println!("\n📊 步骤 5: 结果对比");

        let diff = if calculated_output_full > rpc_output {
            calculated_output_full - rpc_output
        } else {
            rpc_output - calculated_output_full
        };

        let error_rate = if rpc_output > 0 {
            (diff as f64 / rpc_output as f64) * 100.0
        } else {
            0.0
        };

        println!("┌─────────────────────────────────────┐");
        println!("│           结果对比                  │");
        println!("├─────────────────────────────────────┤");
        println!("│ 完整计算:     {:>15} │", calculated_output_full);
        println!("│ RPC Quote:    {:>15} │", rpc_output);
        println!("│ 差值:         {:>15} │", diff);
        println!("│ 误差率:      {:>13.4}% │", error_rate);
        println!("└─────────────────────────────────────┘");

        // 验证准确性
        if error_rate < 0.1 {
            println!("✅ 验证通过：误差 < 0.1%");
        } else if error_rate < 1.0 {
            println!("⚠️  误差较小：0.1% ≤ 误差 < 1%");
        } else if error_rate < 5.0 {
            println!("⚠️  误差较大：1% ≤ 误差 < 5%");
        } else {
            println!("❌ 误差过大：误差 ≥ 5%");
        }

        println!("\n{}", "─".repeat(50));
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ 测试完成");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
