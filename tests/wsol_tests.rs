//! WSOL 集成测试
//!
//! 使用 surfpool (localhost:8899) 进行测试
//!
//! 运行测试:
//!     cargo test --test wsol_tests -- --nocapture
//!
//! 注意：需要确保 surfpool 正在运行

use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::str::FromStr;

mod test_helpers;
use test_helpers::{
    buy_pump_with_fixed_output, buy_pump_with_sol, create_test_client, get_token_balance,
    print_balances,
};

use crate::test_helpers::create_test_client_with_seed_optimize;

/// 测试：WSOL 包装完整流程
///
/// 这个测试验证：  
/// 1. SOL -> WSOL 包装
/// 2. WSOL -> SOL 部分解包装
/// 3. WSOL 账户关闭
#[tokio::test]
async fn test_wsol_wrap_complete_flow() {
    let client = create_test_client().await;
    let wrap_amount = 100_000_000; // 0.1 SOL in lamports
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let payer_pubkey =
        client.payer.try_pubkey().expect("Failed to get payer pubkey from TradingClient");

    println!("=== 测试 WSOL 完整流程 ===");
    println!("包装 {} lamports (0.1 SOL) 到 WSOL...", wrap_amount);

    // 打印初始余额
    let (sol_before, wsol_before) =
        print_balances(&rpc_url, &payer_pubkey).await.unwrap_or_else(|e| {
            panic!("获取初始余额失败: {}\n  钱包: {}\n  RPC: {}", e, payer_pubkey, rpc_url)
        });

    // Step 1: 包装 SOL 到 WSOL
    match client.wrap_sol_to_wsol(wrap_amount).await {
        Ok(signature) => {
            println!("✅ SOL -> WSOL 成功: {}", signature);
        }
        Err(e) => {
            println!("❌ SOL -> WSOL 失败: {}", e);
            panic!(
                "SOL -> WSOL 包装失败: {}\n  钱包: {}\n  包装金额: {} lamports ({:.4} SOL)\n  RPC: {}",
                e, payer_pubkey, wrap_amount, wrap_amount as f64 / 1e9, rpc_url
            );
        }
    }

    // 打印包装后余额
    let (sol_after_wrap, wsol_after_wrap) = print_balances(&rpc_url, &payer_pubkey)
        .await
        .unwrap_or_else(|e| panic!("获取包装后余额失败: {}\n  钱包: {}", e, payer_pubkey));
    assert!(wsol_after_wrap > wsol_before, "WSOL 余额应该增加");
    assert!(sol_after_wrap < sol_before, "SOL 余额应该减少");

    // Step 2: 解包装
    println!("\n解包装 {} lamports (0.1 SOL) 回 SOL...", wrap_amount);

    match client.wrap_wsol_to_sol(wrap_amount).await {
        Ok(signature) => {
            println!("✅ WSOL -> SOL 成功: {}", signature);
        }
        Err(e) => {
            println!("❌ WSOL -> SOL 失败: {}", e);
            panic!(
                "WSOL -> SOL 解包装失败: {}\n  钱包: {}\n  解包装金额: {} lamports ({:.4} SOL)",
                e,
                payer_pubkey,
                wrap_amount,
                wrap_amount as f64 / 1e9
            );
        }
    }

    // 打印解包装后余额
    let (sol_after_unwrap, wsol_after_unwrap) = print_balances(&rpc_url, &payer_pubkey)
        .await
        .unwrap_or_else(|e| panic!("获取解包装后余额失败: {}\n  钱包: {}", e, payer_pubkey));
    assert!(sol_after_unwrap > sol_after_wrap, "SOL 余额应该增加");
    assert!(wsol_after_unwrap < wsol_after_wrap, "WSOL 余额应该减少");

    // Step 3: 关闭 WSOL 账户
    println!("\n关闭 WSOL 账户...");
    match client.close_wsol().await {
        Ok(signature) => {
            println!("✅ 关闭 WSOL 账户成功: {}", signature);
        }
        Err(e) => {
            println!("❌ 关闭 WSOL 账户失败: {}", e);
            panic!(
                "关闭 WSOL 账户失败: {}\n  钱包: {}\n  WSOL ATA: {}",
                e,
                payer_pubkey,
                test_helpers::get_wsol_ata_address(&payer_pubkey)
            );
        }
    }

    // 打印关闭后余额
    let (_, wsol_final) = print_balances(&rpc_url, &payer_pubkey)
        .await
        .unwrap_or_else(|e| panic!("获取关闭后余额失败: {}\n  钱包: {}", e, payer_pubkey));
    assert_eq!(wsol_final, 0, "WSOL 账户关闭后余额应该为 0");

    println!("=== WSOL 完整流程测试通过 ===");
}

/// 测试：连续多次包装 WSOL
///
/// 验证 ATA 复用机制是否正常工作
#[tokio::test]
async fn test_wsol_multiple_wraps() {
    let client = create_test_client().await;
    let wrap_amount = 50_000_000; // 0.05 SOL

    println!("=== 测试连续多次 WSOL 包装 ===");

    for i in 1..=3 {
        println!("\n第 {} 次包装...", i);

        match client.wrap_sol_to_wsol(wrap_amount).await {
            Ok(signature) => {
                println!("  ✅ 第 {} 次包装成功: {}", i, signature);
            }
            Err(e) => {
                println!("  ❌ 第 {} 次包装失败: {}", i, e);
                panic!(
                    "连续 WSOL 包装失败（第 {} 次）: {}\n  钱包: {}\n  包装金额: {} lamports",
                    i,
                    e,
                    client.payer.pubkey(),
                    wrap_amount
                );
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    println!("=== 连续包装测试通过 ===");

    // 清理：关闭 WSOL 账户
    if let Err(e) = client.close_wsol().await {
        println!("⚠️  清理 WSOL 账户失败: {}", e);
    }
}

/// 测试：创建 WSOL ATA（幂等性）
///
/// 验证重复创建 ATA 不会失败
#[tokio::test]
async fn test_wsol_ata_creation_idempotent() {
    let client = create_test_client().await;

    println!("=== 测试 WSOL ATA 幂等创建 ===");

    // 第一次创建
    match client.wrap_sol_to_wsol(10_000_000).await {
        Ok(sig) => println!("✅ 第一次创建成功: {}", sig),
        Err(e) => panic!(
            "第一次 WSOL ATA 创建失败: {}\n  钱包: {}\n  包装金额: 10_000_000 lamports",
            e,
            client.payer.pubkey()
        ),
    }

    // 第二次创建（应该幂等成功）
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    match client.wrap_sol_to_wsol(10_000_000).await {
        Ok(sig) => println!("✅ 第二次创建成功: {}", sig),
        Err(e) => panic!(
            "第二次 WSOL ATA 创建失败（幂等性测试失败）: {}\n  钱包: {}",
            e,
            client.payer.pubkey()
        ),
    }

    println!("=== ATA 幂等创建测试通过 ===");

    // 清理
    if let Err(e) = client.close_wsol().await {
        println!("⚠️  清理 WSOL 账户失败: {}", e);
    }
}

#[tokio::test]
async fn test_trade_with_wsol() {
    // cargo test --package sol-trade-sdk --test wsol_tests -- test_trade_with_wsol --exact --nocapture
    let client = create_test_client_with_seed_optimize(false).await;

    println!("=== 测试交易中使用 WSOL (工具函数版) ===");

    // 使用一个已知的 PumpSwap 池进行测试
    let pool = Pubkey::from_str("539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR").expect(
        "Failed to parse PumpSwap pool address: 539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR",
    );
    let mint = Pubkey::from_str("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn")
        .expect("Failed to parse Pump mint address: pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn");

    // 使用工具函数购买 0.01 SOL 的 Pump 代币
    let (success, signatures, error) = buy_pump_with_sol(&client, pool, mint, 10_000_000, Some(500))
        .await
        .unwrap_or_else(|e| panic!(
            "Pump 交易执行失败: {}\n  Pool: {}\n  Mint: {}\n  购买金额: 10_000_000 lamports (0.01 SOL)",
            e, pool, mint
        ));

    assert!(success, "Pump 交易应该成功");
    assert!(!signatures.is_empty(), "应该获得交易签名");

    println!("✅ 交易成功！签名数量: {}", signatures.len());
    for (i, sig) in signatures.iter().enumerate() {
        println!("    [{}] {}", i + 1, sig);
    }

    if let Some(err) = error {
        println!("⚠️  交易有警告: {}", err);
    }

    println!("=== 交易 WSOL 测试完成 ===");
}

/// 测试：使用 fixed_output_token_amount 购买指定数量代币
///
/// 验证：
/// 1. 使用 fixed_output_token_amount 参数指定精确的代币购买数量
/// 2. 交易前后验证 Token 余额变化
/// 3. 验证实际买入数量与预期一致
#[tokio::test]
async fn test_trade_with_fixed_output_token_amount() {
    // cargo test --package sol-trade-sdk --test wsol_tests -- test_trade_with_fixed_output_token_amount --exact --nocapture
    let client = create_test_client_with_seed_optimize(false).await;

    println!("=== 测试 fixed_output_token_amount 参数 ===");

    // 使用一个已知的 PumpSwap 池进行测试
    let pool = Pubkey::from_str("539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR").expect(
        "Failed to parse PumpSwap pool address: 539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR",
    );
    let mint = Pubkey::from_str("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn")
        .expect("Failed to parse Pump mint address: pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn");
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let payer_pubkey =
        client.payer.try_pubkey().expect("Failed to get payer pubkey from TradingClient");

    // 要购买的代币数量
    let target_token_amount = 10_000u64;

    println!("  目标购买数量: {} 个代币", target_token_amount);
    println!("  Pool: {}", pool);
    println!("  Token Mint: {}", mint);

    // 查询交易前的 Token 余额
    let balance_before = get_token_balance(&rpc_url, &payer_pubkey, &mint)
        .await
        .expect("Failed to get token balance before trade");
    println!("  交易前余额: {}", balance_before);

    // 使用工具函数购买指定数量的代币
    let result =
        buy_pump_with_fixed_output(&client, pool, mint, target_token_amount, Some(500)).await;

    // 验证交易结果
    match result {
        Ok((success, signatures, _error)) => {
            assert!(success, "交易应该成功");
            assert!(!signatures.is_empty(), "应该获得交易签名");

            println!("  ✅ 交易成功！签名数量: {}", signatures.len());
            for (i, sig) in signatures.iter().enumerate() {
                println!("    [{}] {}", i + 1, sig);
            }

            // 等待一下让链上状态更新
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            // 查询交易后的 Token 余额
            let balance_after = get_token_balance(&rpc_url, &payer_pubkey, &mint)
                .await
                .expect("Failed to get token balance after trade");
            println!("  交易后余额: {}", balance_after);

            // 计算实际买入数量
            let actual_amount = balance_after.saturating_sub(balance_before);
            println!("  实际买入数量: {}", actual_amount);

            // 验证买入数量大于 0
            assert!(actual_amount > 0, "实际买入数量应该大于 0，实际: {}", actual_amount);

            // 验证买入数量接近目标（考虑滑点，允许 10% 误差）
            let min_expected = target_token_amount * 90 / 100; // 90% 下限
            let max_expected = target_token_amount * 110 / 100; // 110% 上限
            assert!(
                actual_amount >= min_expected && actual_amount <= max_expected,
                "实际买入数量应该在 {} ~ {} 范围内，实际: {}",
                min_expected,
                max_expected,
                actual_amount
            );

            println!(
                "  ✅ 验证通过：实际买入 {} 个代币 (目标: {})",
                actual_amount, target_token_amount
            );
        }
        Err(e) => {
            panic!(
                "固定输出数量交易执行失败: {}\n  Pool: {}\n  Mint: {}\n  目标数量: {} 个代币",
                e, pool, mint, target_token_amount
            );
        }
    }

    println!("=== fixed_output_token_amount 测试通过 ===");
}
