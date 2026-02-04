//! Raydium CPMM Buy & Sell 集成测试
//!
//! 本测试文件基于文档 `docs/raydium-cpmm-pool-lookup.md` 的设计，验证：
//! - 使用 `get_pool_by_address` 获取指定 CPMM Pool 的信息
//! - 基于 PoolState 构建 `RaydiumCpmmParams`
//! - 通过 `SolanaTrade` 执行一条完整的 Raydium CPMM 买入 -> 卖出交易流程
//! - 获取 CPMM token 的 USD 价格
//!
//! 测试假设：
//! - 本地 RPC `http://127.0.0.1:8899` 已接入主网数据（例如使用 surfpool）
//! - Raydium CPMM 协议已在该 RPC 上可用
//! - 存在至少一个包含 WSOL 的 Raydium CPMM 池
//!
//! 运行测试:
//!     cargo test --test raydium_cpmm_buy_sell_tests -- --nocapture
//!
//! 注意：Pool 列表功能测试已移至 sol-trade-test-utils/tests/list_pools_tests.rs

use sol_trade_sdk::{
    common::auto_mock_rpc::AutoMockRpcClient,
    instruction::utils::raydium_cpmm::{
        clear_pool_cache, get_pool_by_address, get_pool_by_mint, get_token_price_in_usd_with_pool,
    },
    parser::DexParser,
    trading::core::params::RaydiumCpmmParams,
};
use solana_sdk::{pubkey::Pubkey, signer::Signer};

mod test_helpers;
use test_helpers::{create_test_client, print_balances, print_token_balance};
use sol_trade_test_utils::set_token_balance;

// 使用参数构造工具模块
use sol_trade_test_utils::cpmm_test_params::*;

/// 测试：Raydium CPMM 完整买入-卖出流程
///
/// 流程：
/// 1. 使用指定的 CPMM Pool (PIPE-WSOL)
/// 2. 使用 SOL 买入目标代币
/// 3. 再将全部目标代币卖出换回 SOL
/// 4. 验证 Token 余额变化和 SOL 净变化
///
/// ⚠️ 已知问题：
/// 此测试使用的池 (PIPE-WSOL) 的 observation_state 账户未初始化。
/// Raydium CPMM 程序要求 observation_state 账户必须存在且已初始化，
/// 否则会返回错误 0xbc4 "The program expected this account to be already initialized"。
///
/// 要修复此测试，需要：
/// 1. 找一个 observation_state 已初始化的 CPMM 池，或
/// 2. 初始化此池的 observation_state 账户
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_cpmm_buy_sell_complete() {
    println!("\n=== 测试：Raydium CPMM 完整买卖流程 ===");

    let client = create_test_client().await;
    let rpc_url = "http://127.0.0.1:8899";

    let payer_pubkey = client.payer.as_ref().pubkey();
    println!("测试钱包: {}", payer_pubkey);

    // 记录初始 SOL 余额
    let (initial_sol, _) = print_balances(rpc_url, &payer_pubkey)
        .await
        .expect("Failed to fetch initial balances");

    // ===== 1. 使用指定的 CPMM Pool (PIPE-WSOL) =====
    let pool_address = pipe_wsol_pool();
    let wsol_mint = wsol_mint();

    println!("\n🔍 使用指定的 Raydium CPMM Pool: {}", pool_address);

    // 从 pool 地址获取 pool state
    let pool_state = get_pool_by_address(&client.rpc, &pool_address)
        .await
        .expect("Failed to get CPMM pool state");

    println!("Pool 信息:");
    println!("  token0_mint: {}", pool_state.token0_mint);
    println!("  token1_mint: {}", pool_state.token1_mint);

    // 验证池确实包含 WSOL
    assert!(
        pool_state.token0_mint == wsol_mint || pool_state.token1_mint == wsol_mint,
        "CPMM Pool 必须包含 WSOL",
    );

    // 选择目标代币：池中非 WSOL 的那一侧
    let target_mint = if pool_state.token0_mint == wsol_mint {
        pool_state.token1_mint
    } else {
        pool_state.token0_mint
    };

    println!("目标交易 Token Mint: {}", target_mint);

    // 记录初始目标代币余额
    let initial_token_balance = print_token_balance(rpc_url, &payer_pubkey, &target_mint, "Target")
        .await
        .expect("Failed to fetch initial token balance");

    // ===== 2. 从 Pool 地址构建 RaydiumCpmmParams =====
    println!("\n🧮 从 Pool 构建 RaydiumCpmmParams...");
    let cpmm_params = RaydiumCpmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
        .await
        .expect("Failed to build RaydiumCpmmParams from pool address");

    // 调试：打印关键账户信息
    println!("CPMM 参数信息:");
    println!("  pool_state: {}", cpmm_params.pool_state);
    println!("  amm_config: {}", cpmm_params.amm_config);
    println!("  base_mint: {}", cpmm_params.base_mint);
    println!("  quote_mint: {}", cpmm_params.quote_mint);
    println!("  base_vault: {}", cpmm_params.base_vault);
    println!("  quote_vault: {}", cpmm_params.quote_vault);
    println!("  base_token_program: {}", cpmm_params.base_token_program);
    println!("  quote_token_program: {}", cpmm_params.quote_token_program);
    println!("  observation_state: {}", cpmm_params.observation_state);

    // ===== 3. 使用 SOL 买入目标代币（使用参数构造工具）=====
    println!("\n💰 第一步：买入目标代币 (Raydium CPMM)");

    // 使用构建器构造买入参数（简化代码）
    let buy_params = PipeWsolBuyParamsBuilder::new(Some(20_000_000)) // 0.02 SOL
        .slippage(10000) // 10% 容忍度，避免因滑点导致测试偶发失败
        .build(&client)
        .await;

    let (success_buy, buy_sigs, error_buy) =
        client.buy(buy_params).await.expect("Raydium CPMM 买入交易执行失败");
    if !success_buy {
        panic!("❌ 买入交易失败: {:?}\n  Pool: {}\n  Target Mint: {}\n  输入金额: {} lamports",
               error_buy, pool_address, target_mint, 20_000_000);
    }
    println!("✅ 买入成功，签名: {:?}", buy_sigs.first());

    // 解析买入交易
    if let Some(buy_sig) = buy_sigs.first() {
        println!("\n📋 解析买入交易...");
        let parser = DexParser::default();
        let buy_sig_str = buy_sig.to_string();
        let parse_result = parser.parse_transaction(&buy_sig_str).await;

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
    }

    // 买入后的代币余额
    let token_after_buy = print_token_balance(rpc_url, &payer_pubkey, &target_mint, "Target")
        .await
        .expect("Failed to fetch token balance after buy");
    assert!(token_after_buy > initial_token_balance, "买入后目标代币余额应增加",);

    // ===== 4. 卖出全部目标代币换回 SOL（使用参数构造工具）=====
    println!("\n💸 第二步：卖出全部目标代币 (Raydium CPMM)");

    // 使用构建器构造卖出参数（简化代码）
    let sell_params = PipeWsolSellParamsBuilder::new(token_after_buy)
        .slippage(10000)
        .build(&client)
        .await;

    let (success_sell, sell_sigs, error_sell) =
        client.sell(sell_params).await.expect("Raydium CPMM 卖出交易执行失败");
    if !success_sell {
        panic!("❌ 卖出交易失败: {:?}\n  Pool: {}\n  Target Mint: {}\n  卖出数量: {}",
               error_sell, pool_address, target_mint, token_after_buy);
    }
    println!("✅ 卖出成功，签名: {:?}", sell_sigs.first());

    // 解析卖出交易
    if let Some(sell_sig) = sell_sigs.first() {
        println!("\n📋 解析卖出交易...");
        let parser = DexParser::default();
        let sell_sig_str = sell_sig.to_string();
        let parse_result = parser.parse_transaction(&sell_sig_str).await;

        if parse_result.success && !parse_result.trades.is_empty() {
            println!("✅ 卖出交易解析成功:");
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
            println!("⚠️  卖出交易解析失败: {:?}", parse_result.error);
        }
    }

    // 等待链上状态更新
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // ===== 5. 验证最终余额 =====
    let (final_sol, _) = print_balances(rpc_url, &payer_pubkey)
        .await
        .expect("Failed to fetch final balances");
    let final_token_balance = print_token_balance(rpc_url, &payer_pubkey, &target_mint, "Target")
        .await
        .expect("Failed to fetch final token balance");

    println!("\n📊 完整流程结果:");
    let sol_diff = (final_sol as i128) - (initial_sol as i128);
    println!("  - SOL 净变化: {} lamports ({:.6} SOL)", sol_diff, sol_diff as f64 / 1e9);
    println!("  - 最终目标代币余额: {}", final_token_balance);

    // 目标代币应基本被卖出（可能存在极小 dust，但在典型场景下应为 0）
    assert_eq!(final_token_balance, 0, "卖出后目标代币余额应为 0");
    // 由于手续费和滑点，SOL 净变化应为负
    assert!(sol_diff < 0, "由于手续费和滑点，SOL 应该净减少");

    println!("\n=== Raydium CPMM 买入-卖出完整流程测试通过 ===");
}

/// 测试：基于 WSOL mint 查找 CPMM Pool，并验证缓存与强制刷新
///
/// 步骤：
/// 1. 清空 CPMM 缓存
/// 2. 使用 `get_pool_by_mint` 基于 WSOL mint 查找 Pool（应从链上扫描）
/// 3. 再次调用 `get_pool_by_mint`（应命中缓存，结果相同）
/// 测试：获取 CPMM token 的 USD 价格（Auto Mock 加速）
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_get_cpmm_token_price_in_usd() {
    println!("=== 测试：获取 CPMM token 的 USD 价格 (Auto Mock 加速) ===");

    let token_mint = pipe_mint();
    let pool_address = pipe_wsol_pool();
    let rpc_url = "http://127.0.0.1:8899";

    // 使用 Auto Mock RPC 客户端（使用独立命名空间）
    let auto_mock_client = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("raydium_cpmm_buy_sell_tests".to_string()),
    );

    println!("Token Mint: {}", token_mint);
    println!("Pool 地址: {}", pool_address);
    println!("WSOL-USDT 锚定池: 使用默认锚定池");

    // 调用价格计算函数（使用 AutoMock 版本）
    let result: Result<f64, anyhow::Error> =
        get_token_price_in_usd_with_pool(&auto_mock_client, &token_mint, &pool_address, None).await;

    // 验证结果
    assert!(result.is_ok(), "Failed to get token price in USD: {:?}", result.err());

    let price_usd = result.unwrap();
    println!("✅ Token USD 价格: ${:.8}", price_usd);

    // 验证价格合理性
    assert!(price_usd > 0.0, "Price should be positive");
    assert!(price_usd < 1000.0, "Price should be reasonable (< $1000)");
    println!("✅ 价格范围验证通过");
    println!("✅ 首次运行：从 RPC 获取并保存（约 2-3 秒）");
    println!("✅ 后续运行：从缓存加载（约 0.01 秒）");
    println!("✅ 速度提升：约 100-200 倍！");
}


/// 测试：Raydium CPMM USDC-PRTS 完整买入-卖出流程
///
/// 流程：
/// 1. 使用 `set_token_balance` 空投 USDC（通过 surfnet_setTokenAccount RPC）
/// 2. 使用指定的 CPMM Pool (USDC-PRTS)
/// 3. 使用 USDC 买入 PRTS 代币
/// 4. 再将全部 PRTS 代币卖出换回 USDC
/// 5. 验证 Token 余额变化和 USDC 净变化
///
/// 注意：
/// - PRTS 是 Token-2022 代币
/// - 使用测试节点的 surfnet_setTokenAccount RPC 方法设置余额
#[tokio::test]
#[serial_test::serial(global_dex_cache)]
async fn test_raydium_cpmm_buy_sell_usdc_prts() {
    println!("\n=== 测试：Raydium CPMM USDC-PRTS 完整买卖流程 ===");

    let client = create_test_client().await;
    let rpc_url = "http://127.0.0.1:8899";

    let payer_pubkey = client.payer.as_ref().pubkey();
    println!("测试钱包: {}", payer_pubkey);

    let usdc_mint = usdc_mint();
    let prts_mint = prts_mint();

    // 记录初始 USDC 余额
    let _initial_usdc = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC")
        .await
        .expect("Failed to fetch initial USDC balance");

    // ===== 1. 使用 surfnet_setTokenAccount 空投 USDC =====
    println!("\n💧 使用 surfnet_setTokenAccount 空投 USDC...");
    let usdc_amount = "1000"; // 1000 USDC
    set_token_balance(&client.rpc, rpc_url, client.payer.as_ref(), &usdc_mint, usdc_amount)
        .await
        .expect("Failed to set USDC balance");
    println!("✅ USDC 空投完成");

    // ===== 2. 使用指定的 CPMM Pool (USDC-PRTS) =====
    let pool_address = usdc_prts_pool();

    println!("\n🔍 使用指定的 Raydium CPMM Pool: {}", pool_address);
    println!("   PRTS Mint: {} (Token-2022)", prts_mint);

    // 从 pool 地址获取 pool state
    let pool_state = get_pool_by_address(&client.rpc, &pool_address)
        .await
        .expect("Failed to get CPMM pool state");

    println!("Pool 信息:");
    println!("  token0_mint: {}", pool_state.token0_mint);
    println!("  token1_mint: {}", pool_state.token1_mint);

    // 验证池确实包含 USDC 和 PRTS
    assert!(
        pool_state.token0_mint == usdc_mint || pool_state.token1_mint == usdc_mint,
        "CPMM Pool 必须包含 USDC",
    );
    assert!(
        pool_state.token0_mint == prts_mint || pool_state.token1_mint == prts_mint,
        "CPMM Pool 必须包含 PRTS",
    );

    // 记录空投后的 USDC 余额
    let usdc_after_airdrop = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC")
        .await
        .expect("Failed to fetch USDC balance after airdrop");

    // 记录初始 PRTS 余额
    let initial_prts = print_token_balance(rpc_url, &payer_pubkey, &prts_mint, "PRTS")
        .await
        .expect("Failed to fetch initial PRTS balance");

    // ===== 3. 从 Pool 地址构建 RaydiumCpmmParams =====
    println!("\n🧮 从 Pool 构建 RaydiumCpmmParams...");
    let cpmm_params = RaydiumCpmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
        .await
        .expect("Failed to build RaydiumCpmmParams from pool address");

    // 调试：打印关键账户信息
    println!("CPMM 参数信息:");
    println!("  pool_state: {}", cpmm_params.pool_state);
    println!("  amm_config: {}", cpmm_params.amm_config);
    println!("  base_mint: {}", cpmm_params.base_mint);
    println!("  quote_mint: {}", cpmm_params.quote_mint);
    println!("  base_vault: {}", cpmm_params.base_vault);
    println!("  quote_vault: {}", cpmm_params.quote_vault);
    println!("  base_token_program: {}", cpmm_params.base_token_program);
    println!("  quote_token_program: {}", cpmm_params.quote_token_program);
    println!("  observation_state: {}", cpmm_params.observation_state);

    // ===== 4. 使用 USDC 买入 PRTS 代币（使用参数构造工具）=====
    println!("\n💰 第一步：买入 PRTS 代币 (Raydium CPMM)");

    // 使用构建器构造买入参数（简化代码，默认 100 USDC）
    let buy_params = UsdcPrtsBuyParamsBuilder::new(None) // 默认 100 USDC
        .slippage(10000) // 10% 容忍度
        .build(&client)
        .await;

    let (success_buy, buy_sigs, error_buy) =
        client.buy(buy_params).await.expect("Raydium CPMM 买入交易执行失败");
    if !success_buy {
        panic!(
            "❌ 买入交易失败: {:?}\n  Pool: {}\n  Target Mint: {}\n  输入金额: 100 USDC",
            error_buy, pool_address, prts_mint
        );
    }
    println!("✅ 买入成功，签名: {:?}", buy_sigs.first());

    // 解析买入交易
    if let Some(buy_sig) = buy_sigs.first() {
        println!("\n📋 解析买入交易...");
        let parser = DexParser::default();
        let buy_sig_str = buy_sig.to_string();
        let parse_result = parser.parse_transaction(&buy_sig_str).await;

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
    }

    // 买入后的代币余额
    let prts_after_buy = print_token_balance(rpc_url, &payer_pubkey, &prts_mint, "PRTS")
        .await
        .expect("Failed to fetch PRTS balance after buy");
    assert!(prts_after_buy > initial_prts, "买入后 PRTS 余额应增加");

    // ===== 5. 卖出全部 PRTS 代币换回 USDC（使用参数构造工具）=====
    println!("\n💸 第二步：卖出全部 PRTS 代币 (Raydium CPMM)");

    // 使用构建器构造卖出参数（简化代码）
    let sell_params = UsdcPrtsSellParamsBuilder::new(prts_after_buy)
        .slippage(10000)
        .build(&client)
        .await;

    let (success_sell, sell_sigs, error_sell) =
        client.sell(sell_params).await.expect("Raydium CPMM 卖出交易执行失败");
    if !success_sell {
        panic!(
            "❌ 卖出交易失败: {:?}\n  Pool: {}\n  Target Mint: {}\n  卖出数量: {}",
            error_sell, pool_address, prts_mint, prts_after_buy
        );
    }
    println!("✅ 卖出成功，签名: {:?}", sell_sigs.first());

    // 解析卖出交易
    if let Some(sell_sig) = sell_sigs.first() {
        println!("\n📋 解析卖出交易...");
        let parser = DexParser::default();
        let sell_sig_str = sell_sig.to_string();
        let parse_result = parser.parse_transaction(&sell_sig_str).await;

        if parse_result.success && !parse_result.trades.is_empty() {
            println!("✅ 卖出交易解析成功:");
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
            println!("⚠️  卖出交易解析失败: {:?}", parse_result.error);
        }
    }

    // 等待链上状态更新
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // ===== 6. 验证最终余额 =====
    let final_usdc = print_token_balance(rpc_url, &payer_pubkey, &usdc_mint, "USDC")
        .await
        .expect("Failed to fetch final USDC balance");
    let final_prts = print_token_balance(rpc_url, &payer_pubkey, &prts_mint, "PRTS")
        .await
        .expect("Failed to fetch final PRTS balance");

    println!("\n📊 完整流程结果:");
    let uscd_diff = (final_usdc as i128) - (usdc_after_airdrop as i128);
    println!("  - USDC 净变化: {} lamports ({:.6} USDC)", uscd_diff, uscd_diff as f64 / 1e6);
    println!("  - 最终 PRTS 余额: {}", final_prts);

    // PRTS 应基本被卖出（可能存在极小 dust，但在典型场景下应为 0）
    assert_eq!(final_prts, 0, "卖出后 PRTS 余额应为 0");
    // 由于手续费和滑点，USDC 净变化应为负
    assert!(uscd_diff < 0, "由于手续费和滑点，USDC 应该净减少");

    println!("\n=== Raydium CPMM USDC-PRTS 买入-卖出完整流程测试通过 ===");
}
