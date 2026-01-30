//! 交易执行模式使用示例
//!
//! 展示如何使用 ExecutionMode 在不同场景下选择最适合的交易执行方式
//!
//! 运行示例:
//!     cargo run --example execution_mode_demo

use sol_trade_sdk::{
    DexType, common::SolanaRpcClient, instruction::utils::raydium_clmm::quote_exact_in,
    trading::ExecutionMode,
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;

/// WSOL-JUP CLMM Pool
const WSOL_JUP_POOL: &str = "EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw";

/// WSOL Mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// JUP Mint
const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📚 Sol Trade SDK - 交易执行模式示例");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let rpc_url = "http://127.0.0.1:8899";
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.to_string()));

    let pool_address = Pubkey::from_str(WSOL_JUP_POOL)?;
    let wsol_mint = Pubkey::from_str(WSOL_MINT)?;
    let jup_mint = Pubkey::from_str(JUP_MINT)?;

    let amount_in = 100_000_000u64; // 0.1 SOL

    // ========================================
    // 示例 1: 本地计算模式（快速估算）
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 示例 1: 本地计算模式");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let start = std::time::Instant::now();

    // 获取 Pool 状态
    let pool_state =
        sol_trade_sdk::instruction::utils::raydium_clmm::get_pool_by_address(&rpc, &pool_address)
            .await?;

    // 计算交易方向
    let zero_for_one = wsol_mint.to_string() == pool_state.token_mint0.to_string();

    // 本地计算
    let result = quote_exact_in(&rpc, &pool_address, amount_in, zero_for_one).await?;

    let duration = start.elapsed();

    println!("✅ 本地计算完成:");
    println!("   模式: {:?}", ExecutionMode::LocalCalculation);
    println!("   速度等级: {}/1 (最快)", ExecutionMode::LocalCalculation.speed_level());
    println!("   准确性等级: {}/3 (估算)", ExecutionMode::LocalCalculation.accuracy_level());
    println!("   输入: {} lamports (0.1 SOL)", amount_in);
    println!("   输出: {} JUP", result.amount_out);
    println!("   ⏱️  耗时: {:?}", duration);
    println!("   💡 适用场景: 价格估算、UI 预览\n");

    // ========================================
    // 示例 2: 模拟模式（准确验证）
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 示例 2: 链上模拟模式");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("⚠️  注意: 当前版本需要通过 TradingClient 使用模拟模式");
    println!("   client.buy(TradeBuyParams {{ simulate: true, .. }}).await?;\n");

    println!("✅ 模拟模式说明:");
    println!("   模式: {:?}", ExecutionMode::Simulation);
    println!("   速度等级: {}/3 (中等)", ExecutionMode::Simulation.speed_level());
    println!("   准确性等级: {}/3 (准确)", ExecutionMode::Simulation.accuracy_level());
    println!("   💡 适用场景: 交易验证、测试、确认滑点\n");

    // ========================================
    // 示例 3: 真实执行模式（实际交易）
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💎 示例 3: 真实执行模式");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("⚠️  注意: 当前版本通过 TradingClient 使用真实执行模式");
    println!("   client.buy(TradeBuyParams {{ simulate: false, .. }}).await?;\n");

    println!("✅ 真实执行说明:");
    println!("   模式: {:?}", ExecutionMode::RealExecution);
    println!("   速度等级: {}/3 (较慢)", ExecutionMode::RealExecution.speed_level());
    println!("   准确性等级: {}/3 (实际)", ExecutionMode::RealExecution.accuracy_level());
    println!("   💡 适用场景: 正式交易、改变链上状态\n");

    // ========================================
    // 使用场景对比
    // ========================================
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 使用场景对比");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("┌─────────────────┬──────────┬──────────┬──────────┬─────────────┐");
    println!("│ 场景            │ 推荐模式  │ 速度     │ 准确性   │  费用      │");
    println!("├─────────────────┼──────────┼──────────┼──────────┼─────────────┤");
    println!("│ UI 实时价格     │ 本地计算 │ < 10ms   │ ±1%     │  无         │");
    println!("│ 批量路径查询    │ 本地计算 │ ~100ms   │ ±1%     │  无         │");
    println!("│ 交易前验证      │ 模拟     │ 1-2s     │ 100%    │  无         │");
    println!("│ 测试交易        │ 模拟     │ 1-2s     │ 100%    │  无         │");
    println!("│ 正式交易        │ 真实执行 │ 2-5s     │ 100%    │  有         │");
    println!("└─────────────────┴──────────┴──────────┴──────────┴─────────────┘");

    println!("\n💡 推荐的三阶段交易流程:");
    println!("   1. UI 显示价格 → 本地计算 (快速)");
    println!("   2. 用户点击确认 → 模拟验证 (准确)");
    println!("   3. 最终确认交易 → 真实执行 (实际)");

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ 示例运行完成！");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}
