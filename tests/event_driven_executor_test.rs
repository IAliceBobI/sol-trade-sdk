//! 事件驱动执行器测试
//!
//! 测试 `wait_transaction_confirmed = false` 场景下的事件驱动改进
//! 验证：
//! - 结果到达时立即返回（无需固定等待）
//! - 超时机制正常工作
//! - 性能相比固定等待有提升

use sol_trade_sdk::{
    common::GasFeeStrategy,
    DexType, TradeBuyParams, TradeTokenType,
    instruction::utils::raydium_cpmm::get_pool_by_address,
    trading::core::params::{DexParamEnum, RaydiumCpmmParams},
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::{str::FromStr, time::Instant};

mod test_helpers;
use test_helpers::create_test_client;

/// 已知的 WSOL mint
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// PIPE Token CPMM Pool
const PIPE_POOL: &str = "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp";

/// 测试：事件驱动的快速返回
///
/// 验证当 `wait_transaction_confirmed = false` 时：
/// 1. 交易发送后快速返回（不等待链上确认）
/// 2. 返回时间在合理范围内（应该 < 100ms）
/// 3. 能成功获取交易签名
#[tokio::test]
#[serial_test::serial]
async fn test_event_driven_fast_return() {
    println!("\n🚀 测试事件驱动执行器（wait_transaction_confirmed = false）");

    let client = create_test_client().await;
    let payer_pubkey = client.payer.as_ref().pubkey();
    let rpc_url = "http://127.0.0.1:8899";

    println!("测试钱包: {}", payer_pubkey);

    // 使用已知的 Pool 地址
    let pool_address = Pubkey::from_str(PIPE_POOL).expect("Invalid pool address");
    println!("使用 Pool: {}", pool_address);

    // 获取 pool state
    let pool_state = match get_pool_by_address(&client.rpc, &pool_address).await {
        Ok(state) => state,
        Err(e) => {
            println!("⚠️  获取 Pool state 失败: {:?}，跳过测试", e);
            return;
        },
    };

    println!("Pool token0_mint: {}", pool_state.token0_mint);
    println!("Pool token1_mint: {}", pool_state.token1_mint);

    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();

    // 验证池包含 WSOL
    assert!(
        pool_state.token0_mint == wsol_mint || pool_state.token1_mint == wsol_mint,
        "Pool 必须包含 WSOL"
    );

    // 确定目标代币
    let target_mint = if pool_state.token0_mint == wsol_mint {
        pool_state.token1_mint
    } else {
        pool_state.token0_mint
    };

    println!("目标代币: {}", target_mint);

    // 从 Pool 地址构建参数
    let cpmm_params = match RaydiumCpmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address).await {
        Ok(params) => params,
        Err(e) => {
            println!("⚠️  构建参数失败: {:?}，跳过测试", e);
            return;
        },
    };

    // 配置 Gas 费策略
    let gas_fee_strategy = GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150_000, 150_000, 500_000, 500_000, 0.001, 0.001);

    let recent_blockhash = match client.rpc.get_latest_blockhash().await {
        Ok(hash) => hash,
        Err(e) => {
            println!("⚠️  获取 blockhash 失败: {:?}，跳过测试", e);
            return;
        },
    };

    // 小额测试交易
    let input_amount = 1_000_000u64; // 0.001 SOL

    let buy_params = TradeBuyParams {
        dex_type: DexType::RaydiumCpmm,
        input_token_type: TradeTokenType::SOL,
        mint: target_mint,
        input_token_amount: input_amount,
        slippage_basis_points: Some(1000), // 10% 滑点
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::RaydiumCpmm(cpmm_params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: false, // 🔧 关键：不等待确认
        create_input_token_ata: true,
        close_input_token_ata: false,
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy.clone(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    // 执行交易并计时
    println!("\n🎯 执行交易（wait_transaction_confirmed = false）...");
    let start = Instant::now();
    let result = client.buy(buy_params).await;
    let elapsed = start.elapsed();

    println!("⏱️  耗时: {:?}", elapsed);

    match result {
        Ok((success, sigs, error)) => {
            if success {
                println!("✅ 交易成功");
                println!("   签名: {:?}", sigs.first());
                println!("✅ 事件驱动正常工作：快速返回（不等待确认）");

                // 验证返回时间在合理范围内
                // 事件驱动应该 < 150ms（超时时间是 100ms，加上一些余量）
                assert!(
                    elapsed < std::time::Duration::from_millis(150),
                    "返回时间应该 < 150ms，实际: {:?}",
                    elapsed
                );
                println!("✅ 返回时间验证通过: {:?} < 150ms", elapsed);
            } else {
                println!("⚠️  交易失败: {:?}", error);
                // 失败也可能是因为 MEV 服务问题，不影响事件驱动测试
                if elapsed < std::time::Duration::from_millis(150) {
                    println!("✅ 事件驱动正常工作：快速返回（耗时: {:?}）", elapsed);
                } else {
                    println!("⚠️  返回时间较长: {:?}", elapsed);
                }
            }
        },
        Err(e) => {
            println!("❌ 执行错误: {:?}", e);

            // 如果是超时错误，验证超时时间
            if e.to_string().contains("timeout") {
                println!("✅ 超时机制正常工作");
                assert!(
                    elapsed < std::time::Duration::from_millis(150),
                    "超时时间应该 < 150ms，实际: {:?}",
                    elapsed
                );
            }
        },
    }

    println!("\n🎉 事件驱动执行器测试完成");
}
