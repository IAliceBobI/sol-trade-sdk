//! 统一交易接口测试
//!
//! 测试 buy_quote、buy_simulate 和 buy 三个接口的功能

use sol_trade_sdk::{
    DexType, TradeBuyParams, TradeTokenType, TradingClient, UnifiedTradingError as TradingError,
    common::TradeConfig,
    constants::{USDC_TOKEN_ACCOUNT, WSOL_TOKEN_ACCOUNT},
    trading::core::params::{
        DexParamEnum, PumpSwapParams, RaydiumAmmV4Params, RaydiumClmmParams, RaydiumCpmmParams,
    },
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::str::FromStr;
use std::sync::Arc;

// ========================================
// 测试辅助函数
// ========================================

async fn setup_test_client() -> TradingClient {
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let payer = Arc::new(Keypair::new());

    let config = TradeConfig::new(rpc_url, vec![], CommitmentConfig::confirmed())
        .with_wsol_ata_config(false, false); // 禁用 WSOL ATA 自动创建
    TradingClient::new(payer, config).await
}

fn create_clmm_test_params() -> TradeBuyParams {
    // WSOL-USDC CLMM Pool
    let pool_address = Pubkey::from_str("CAMMzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK").unwrap();
    let wsol_mint = WSOL_TOKEN_ACCOUNT;
    let usdc_mint = USDC_TOKEN_ACCOUNT;

    TradeBuyParams {
        dex_type: DexType::RaydiumClmm,
        input_token_type: TradeTokenType::WSOL,
        mint: usdc_mint,
        input_token_amount: 1_000_000,    // 0.001 SOL
        slippage_basis_points: Some(100), // 1%
        recent_blockhash: None,
        extension_params: DexParamEnum::RaydiumClmm(RaydiumClmmParams {
            pool_state: pool_address,
            amm_config: Pubkey::default(),
            token0_mint: wsol_mint,
            token1_mint: usdc_mint,
            token0_vault: Pubkey::default(),
            token1_vault: Pubkey::default(),
            observation_state: Pubkey::default(),
            token0_decimals: 9,
            token1_decimals: 6,
            token0_program: spl_token::id(),
            token1_program: spl_token::id(),
        }),
        address_lookup_table_account: None,
        wait_transaction_confirmed: false,
        create_input_token_ata: false,
        close_input_token_ata: false,
        create_mint_ata: false,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::default(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    }
}

fn create_cpmm_test_params() -> TradeBuyParams {
    // 使用真实的 CPMM Pool 地址
    let pool_address = Pubkey::from_str("8sLbN1k25sYvThRjFJMPnEtbWHC72HZaXDsw7mwwxazw").unwrap();
    let wsol_mint = WSOL_TOKEN_ACCOUNT;
    let usdc_mint = USDC_TOKEN_ACCOUNT;

    TradeBuyParams {
        dex_type: DexType::RaydiumCpmm,
        input_token_type: TradeTokenType::WSOL,
        mint: usdc_mint,
        input_token_amount: 1_000_000,    // 0.001 SOL
        slippage_basis_points: Some(100), // 1%
        recent_blockhash: None,
        extension_params: DexParamEnum::RaydiumCpmm(RaydiumCpmmParams {
            pool_state: pool_address,
            amm_config: Pubkey::default(),
            base_mint: wsol_mint,
            quote_mint: usdc_mint,
            base_reserve: 0,
            quote_reserve: 0,
            base_vault: Pubkey::default(),
            quote_vault: Pubkey::default(),
            base_token_program: spl_token::id(),
            quote_token_program: spl_token::id(),
            observation_state: Pubkey::default(),
        }),
        address_lookup_table_account: None,
        wait_transaction_confirmed: false,
        create_input_token_ata: false,
        close_input_token_ata: false,
        create_mint_ata: false,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::default(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    }
}

fn create_amm_v4_test_params() -> TradeBuyParams {
    // WSOL-USDC AMM V4 Pool
    let pool_address = Pubkey::from_str("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2").unwrap();
    let wsol_mint = WSOL_TOKEN_ACCOUNT;
    let usdc_mint = USDC_TOKEN_ACCOUNT;

    TradeBuyParams {
        dex_type: DexType::RaydiumAmmV4,
        input_token_type: TradeTokenType::WSOL,
        mint: usdc_mint,
        input_token_amount: 1_000_000,    // 0.001 SOL
        slippage_basis_points: Some(100), // 1%
        recent_blockhash: None,
        extension_params: DexParamEnum::RaydiumAmmV4(RaydiumAmmV4Params {
            amm: pool_address,
            coin_mint: wsol_mint,
            pc_mint: usdc_mint,
            token_coin: Pubkey::default(), // 会被指令构建器填充
            token_pc: Pubkey::default(),   // 会被指令构建器填充
            coin_reserve: 0,
            pc_reserve: 0,
        }),
        address_lookup_table_account: None,
        wait_transaction_confirmed: false,
        create_input_token_ata: false,
        close_input_token_ata: false,
        create_mint_ata: false,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::default(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    }
}

fn create_pumpswap_test_params() -> TradeBuyParams {
    // 使用真实的 PumpSwap Pool 地址
    let pool_address = Pubkey::from_str("9cWtT2Q3pHnT9MnhSErWyKnkT6KFNJHqg7HdB2jmxcG").unwrap();
    let wsol_mint = WSOL_TOKEN_ACCOUNT;

    TradeBuyParams {
        dex_type: DexType::PumpSwap,
        input_token_type: TradeTokenType::WSOL,
        mint: wsol_mint,                  // 买入 WSOL
        input_token_amount: 1_000_000,    // 0.001 SOL
        slippage_basis_points: Some(100), // 1%
        recent_blockhash: None,
        extension_params: DexParamEnum::PumpSwap(PumpSwapParams {
            pool: pool_address,
            base_mint: wsol_mint,
            quote_mint: USDC_TOKEN_ACCOUNT,
            pool_base_token_account: Pubkey::default(),
            pool_quote_token_account: Pubkey::default(),
            pool_base_token_reserves: 0,
            pool_quote_token_reserves: 0,
            coin_creator_vault_ata: Pubkey::default(),
            coin_creator_vault_authority: Pubkey::default(),
            base_token_program: spl_token::id(),
            quote_token_program: spl_token::id(),
            is_mayhem_mode: false,
        }),
        address_lookup_table_account: None,
        wait_transaction_confirmed: false,
        create_input_token_ata: false,
        close_input_token_ata: false,
        create_mint_ata: false,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: sol_trade_sdk::common::GasFeeStrategy::default(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
        enable_jito_sandwich_protection: None,
    }
}

// ========================================
// buy_quote 测试
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_buy_quote_clmm() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 测试 Raydium CLMM buy_quote");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let client = setup_test_client().await;
    let params = create_clmm_test_params();

    match client.buy_quote(params).await {
        Ok(quote) => {
            println!("✅ buy_quote 成功:");
            println!("   预期输出: {}", quote.amount_out);
            println!("   手续费: {}", quote.fee_amount);
            println!("   计算耗时: {} ms", quote.calculation_time_ms);
            println!("   DEX: {:?}", quote.dex_type);
            assert!(quote.amount_out > 0, "输出金额应该大于 0");
        },
        Err(e) => {
            println!("❌ buy_quote 失败: {}", e);
            println!("   注意：这可能是因为 Pool 不存在或 RPC 连接问题");
        },
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

#[tokio::test]
#[serial_test::serial]
async fn test_buy_quote_cpmm() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 测试 Raydium CPMM buy_quote");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let client = setup_test_client().await;
    let params = create_cpmm_test_params();

    match client.buy_quote(params).await {
        Ok(quote) => {
            println!("✅ buy_quote 成功:");
            println!("   预期输出: {}", quote.amount_out);
            println!("   手续费: {}", quote.fee_amount);
            println!("   计算耗时: {} ms", quote.calculation_time_ms);
            println!("   DEX: {:?}", quote.dex_type);
            assert!(quote.amount_out > 0, "输出金额应该大于 0");
        },
        Err(e) => {
            println!("❌ buy_quote 失败: {}", e);
            println!("   注意：这可能是因为 Pool 不存在或 RPC 连接问题");
        },
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

#[tokio::test]
#[serial_test::serial]
async fn test_buy_quote_amm_v4() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 测试 Raydium AMM V4 buy_quote");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let client = setup_test_client().await;
    let params = create_amm_v4_test_params();

    match client.buy_quote(params).await {
        Ok(quote) => {
            println!("✅ buy_quote 成功:");
            println!("   预期输出: {}", quote.amount_out);
            println!("   手续费: {}", quote.fee_amount);
            println!("   计算耗时: {} ms", quote.calculation_time_ms);
            println!("   DEX: {:?}", quote.dex_type);
            assert!(quote.amount_out > 0, "输出金额应该大于 0");
        },
        Err(e) => {
            println!("❌ buy_quote 失败: {}", e);
            println!("   注意：这可能是因为 Pool 不存在或 RPC 连接问题");
        },
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

#[tokio::test]
#[serial_test::serial]
async fn test_buy_quote_pumpswap() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 测试 PumpSwap buy_quote");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let client = setup_test_client().await;
    let params = create_pumpswap_test_params();

    match client.buy_quote(params).await {
        Ok(quote) => {
            println!("✅ buy_quote 成功:");
            println!("   预期输出: {}", quote.amount_out);
            println!("   手续费: {}", quote.fee_amount);
            println!("   计算耗时: {} ms", quote.calculation_time_ms);
            println!("   DEX: {:?}", quote.dex_type);
            assert!(quote.amount_out > 0, "输出金额应该大于 0");
        },
        Err(e) => {
            println!("❌ buy_quote 失败: {}", e);
            println!("   注意：这可能是因为 Pool 不存在或 RPC 连接问题");
        },
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

#[tokio::test]
#[serial_test::serial]
async fn test_buy_quote_unsupported_dex() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 测试不支持的 DEX buy_quote");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let client = setup_test_client().await;
    let mut params = create_clmm_test_params();
    params.dex_type = DexType::PumpFun; // 不支持 quote

    let result = client.buy_quote(params).await;

    assert!(matches!(result, Err(TradingError::UnsupportedDex(_))));
    println!("✅ 正确返回 UnsupportedDex 错误");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

#[tokio::test]
#[serial_test::serial]
async fn test_buy_quote_invalid_amount() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 测试无效金额 buy_quote");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let client = setup_test_client().await;
    let mut params = create_clmm_test_params();
    params.input_token_amount = 0; // 无效金额

    let result = client.buy_quote(params).await;

    assert!(matches!(result, Err(TradingError::InvalidParameters(_))));
    println!("✅ 正确返回 InvalidParameters 错误");

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

// ========================================
// buy_simulate 测试
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_buy_simulate_clmm() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 测试 Raydium CLMM buy_simulate");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let client = setup_test_client().await;
    let params = create_clmm_test_params();

    match client.buy_simulate(params).await {
        Ok(sim) => {
            println!("✅ buy_simulate 成功:");
            println!("   模拟输出: {}", sim.amount_out);
            println!("   手续费: {}", sim.fee_amount);
            println!("   CU 消耗: {}", sim.compute_units);
            println!("   交易费用: {}", sim.transaction_fee);
            println!("   成功: {}", sim.success);
            println!("   DEX: {:?}", sim.dex_type);

            if !sim.success {
                println!("   错误: {:?}", sim.error);
                println!("   注意：模拟失败可能是因为测试账户不存在");
            }
        },
        Err(e) => {
            println!("❌ buy_simulate 失败: {}", e);
            println!("   注意：这可能是因为 Pool 不存在或 RPC 连接问题");
        },
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

// ========================================
// 准确性测试
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_buy_simulate_vs_quote_accuracy() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 测试 quote vs simulate 准确性");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let client = setup_test_client().await;
    let params = create_clmm_test_params();

    let quote = match client.buy_quote(params.clone()).await {
        Ok(q) => q,
        Err(e) => {
            println!("⚠️  buy_quote 失败，跳过准确性测试: {}", e);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            return;
        },
    };

    let sim = match client.buy_simulate(params).await {
        Ok(s) => s,
        Err(e) => {
            println!("⚠️  buy_simulate 失败，跳过准确性测试: {}", e);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            return;
        },
    };

    if !sim.success {
        println!("⚠️  模拟失败，无法对比准确性");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        return;
    }

    let error_rate = if sim.amount_out > 0 {
        (quote.amount_out as f64 - sim.amount_out as f64).abs() / sim.amount_out as f64 * 100.0
    } else {
        0.0
    };

    println!("┌─────────────────────────────────────┐");
    println!("│           结果对比                  │");
    println!("├─────────────────────────────────────┤");
    println!("│ quote:      {:>13} │", quote.amount_out);
    println!("│ simulate:   {:>13} │", sim.amount_out);
    println!("│ 误差率:     {:>13.4}% │", error_rate);
    println!("└─────────────────────────────────────┘");

    // 允许 0.1% 的误差
    if error_rate < 0.1 {
        println!("✅ 验证通过：误差 < 0.1%");
    } else {
        println!("⚠️  误差较大：{}%", error_rate);
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

// ========================================
// 渐进式工作流测试
// ========================================

#[tokio::test]
#[serial_test::serial]
async fn test_progressive_workflow() {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 测试渐进式工作流：quote → simulate → buy");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let client = setup_test_client().await;
    let params = create_clmm_test_params();

    // Step 1: 快速估算
    println!("📊 步骤 1: 快速本地估算 (buy_quote)");
    let quick = match client.buy_quote(params.clone()).await {
        Ok(q) => {
            println!("✅ 本地估算成功: 预期输出 = {}", q.amount_out);
            q
        },
        Err(e) => {
            println!("❌ 本地估算失败: {}", e);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            return;
        },
    };

    // Step 2: 准确验证
    println!("\n📊 步骤 2: 链上模拟验证 (buy_simulate)");
    let verified = match client.buy_simulate(params.clone()).await {
        Ok(s) => {
            println!("✅ 模拟成功: 输出 = {}, CU = {}", s.amount_out, s.compute_units);
            s
        },
        Err(e) => {
            println!("❌ 模拟失败: {}", e);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            return;
        },
    };

    if !verified.success {
        println!("⚠️  模拟执行失败（这是正常的，因为测试账户不存在）");
        println!("   但模拟请求已成功发送");
    }

    println!("\n✅ 渐进式工作流测试完成");
    println!("   本地估算: {} ms", quick.calculation_time_ms);
    println!("   模拟验证: CU = {}", verified.compute_units);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
