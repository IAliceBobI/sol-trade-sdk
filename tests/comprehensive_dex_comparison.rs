//! DEX Comprehensive Comparison Test
//!
//! Compare different DEX performance:
//! - exact_in vs exact_out
//! - buy vs sell
//! - local calculation
//!
//! Run: cargo nextest run comprehensive_dex_comparison -- --nocapture

use sol_trade_sdk::{
    common::SolanaRpcClient,
    instruction::utils::{
        raydium_amm_v4::{
            get_pool_by_address as amm_get_pool, quote_exact_in as amm_quote_exact_in,
        },
        raydium_clmm::{
            get_pool_by_address as clmm_get_pool, quote_exact_in as clmm_quote_exact_in,
            quote_exact_out as clmm_quote_exact_out,
        },
        raydium_cpmm::{
            get_pool_by_address as cpmm_get_pool, quote_exact_in as cpmm_quote_exact_in,
        },
    },
    utils::calc::{
        raydium_amm_v4::quote_exact_out as amm_quote_exact_out,
        raydium_cpmm::quote_exact_out as cpmm_quote_exact_out,
    },
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::str::FromStr;
use std::sync::Arc;

// Test Pool Addresses
const CLMM_WSOL_JUP_POOL: &str = "EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw";
const CPMM_PIPE_WSOL_POOL: &str = "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp";
const AMM_V4_SOL_USDC_POOL: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

#[derive(Debug)]
struct TestResult {
    dex_name: String,
    mode: String,
    direction: String,
    #[allow(dead_code)]
    calc_type: String,
    input_amount: u64,
    output_amount: u64,
    fee_amount: u64,
    success: bool,
    #[allow(dead_code)]
    error: Option<String>,
}

#[tokio::test]
#[serial_test::serial]
async fn test_comprehensive_dex_comparison() {
    println!("====================================================");
    println!("DEX Comprehensive Comparison Test");
    println!("====================================================\n");

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let rpc = Arc::new(SolanaRpcClient::new(rpc_url.clone()));
    let _payer = Arc::new(Keypair::new());

    let mut all_results = Vec::new();

    let test_amounts =
        vec![("Small", 1_000_000u64), ("Medium", 10_000_000u64), ("Large", 100_000_000u64)];

    // Test CLMM
    println!("\n[Testing Raydium CLMM]");
    println!("====================================================\n");
    match test_clmm_pools(&rpc, &test_amounts).await {
        Ok(results) => all_results.extend(results),
        Err(e) => println!("[ERROR] CLMM test failed: {}\n", e),
    }

    // Test CPMM
    println!("\n[Testing Raydium CPMM]");
    println!("====================================================\n");
    match test_cpmm_pools(&rpc, &test_amounts).await {
        Ok(results) => all_results.extend(results),
        Err(e) => println!("[ERROR] CPMM test failed: {}\n", e),
    }

    // Test AMM V4
    println!("\n[Testing Raydium AMM V4]");
    println!("====================================================\n");
    match test_amm_v4_pools(&rpc, &test_amounts).await {
        Ok(results) => all_results.extend(results),
        Err(e) => println!("[ERROR] AMM V4 test failed: {}\n", e),
    }

    print_summary_report(&all_results);
}

async fn test_clmm_pools(
    rpc: &Arc<SolanaRpcClient>,
    test_amounts: &Vec<(&str, u64)>,
) -> Result<Vec<TestResult>, String> {
    let pool_address = Pubkey::from_str(CLMM_WSOL_JUP_POOL).unwrap();
    println!("Pool: {}", pool_address);
    println!("Direction: WSOL <-> JUP\n");

    let pool_state = clmm_get_pool(rpc, &pool_address)
        .await
        .map_err(|e| format!("Failed to get pool: {}", e))?;

    println!("Pool State:");
    println!("  sqrt_price_x64: {}", pool_state.sqrt_price_x64);
    println!("  liquidity: {}", pool_state.liquidity);
    println!("  tick_current: {}\n", pool_state.tick_current);

    let mut results = Vec::new();

    for (label, amount) in test_amounts {
        println!("Test Amount: {} ({})\n", label, amount);
        println!("----------------------------------------------------");

        // Exact In Buy (SOL -> USDC)
        match clmm_quote_exact_in(rpc, &pool_address, *amount, true).await {
            Ok(quote) => {
                println!("[OK] Exact In Buy (SOL->USDC)");
                println!("  Input: {} lamports", amount);
                println!("  Output: {} units", quote.amount_out);
                println!("  Fee: {} lamports", quote.fee_amount);
                println!("  Price Impact: {} bps\n", quote.price_impact_bps.unwrap_or(0));

                results.push(TestResult {
                    dex_name: "CLMM".to_string(),
                    mode: "exact_in".to_string(),
                    direction: "buy".to_string(),
                    calc_type: "Local".to_string(),
                    input_amount: *amount,
                    output_amount: quote.amount_out,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact In Buy failed: {}\n", e);
            },
        }

        // Exact Out Buy
        match clmm_quote_exact_out(rpc, &pool_address, *amount, true).await {
            Ok(quote) => {
                println!("[OK] Exact Out Buy (Full)");
                println!("  Expected Output: {} lamports", amount);
                println!("  Required Input: {} units", quote.amount_in);
                println!("  Fee: {} lamports\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "CLMM".to_string(),
                    mode: "exact_out".to_string(),
                    direction: "buy".to_string(),
                    calc_type: "Local(Full)".to_string(),
                    input_amount: quote.amount_in,
                    output_amount: *amount,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact Out Buy failed: {}\n", e);
            },
        }

        // Exact In Sell (USDC -> SOL)
        match clmm_quote_exact_in(rpc, &pool_address, *amount, false).await {
            Ok(quote) => {
                println!("[OK] Exact In Sell (USDC->SOL)");
                println!("  Input: {} units", amount);
                println!("  Output: {} lamports", quote.amount_out);
                println!("  Fee: {} units\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "CLMM".to_string(),
                    mode: "exact_in".to_string(),
                    direction: "sell".to_string(),
                    calc_type: "Local".to_string(),
                    input_amount: *amount,
                    output_amount: quote.amount_out,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact In Sell failed: {}\n", e);
            },
        }

        // Exact Out Sell
        match clmm_quote_exact_out(rpc, &pool_address, *amount, false).await {
            Ok(quote) => {
                println!("[OK] Exact Out Sell (Full)");
                println!("  Expected Output: {} units", amount);
                println!("  Required Input: {} lamports", quote.amount_in);
                println!("  Fee: {} lamports\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "CLMM".to_string(),
                    mode: "exact_out".to_string(),
                    direction: "sell".to_string(),
                    calc_type: "Local(Full)".to_string(),
                    input_amount: quote.amount_in,
                    output_amount: *amount,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact Out Sell failed: {}\n", e);
            },
        }
    }

    Ok(results)
}

async fn test_cpmm_pools(
    rpc: &Arc<SolanaRpcClient>,
    test_amounts: &Vec<(&str, u64)>,
) -> Result<Vec<TestResult>, String> {
    let pool_address = Pubkey::from_str(CPMM_PIPE_WSOL_POOL).unwrap();
    println!("Pool: {}", pool_address);
    println!("Direction: WSOL <-> PIPE\n");

    let pool_state = cpmm_get_pool(rpc, &pool_address)
        .await
        .map_err(|e| format!("Failed to get pool: {}", e))?;

    println!("Pool State:");
    println!("  Token0 Mint: {}", pool_state.token0_mint);
    println!("  Token1 Mint: {}", pool_state.token1_mint);
    println!("  Token0 Vault: {}", pool_state.token0_vault);
    println!("  Token1 Vault: {}\n", pool_state.token1_vault);

    let token0_balance = rpc.get_token_account_balance(&pool_state.token0_vault).await;
    let token1_balance = rpc.get_token_account_balance(&pool_state.token1_vault).await;

    let (token0_reserve, token1_reserve) = match (token0_balance, token1_balance) {
        (Ok(t0), Ok(t1)) => {
            let t0_amt = t0.amount.parse::<u64>().unwrap_or(0);
            let t1_amt = t1.amount.parse::<u64>().unwrap_or(0);
            println!("  Token0 Reserve: {}", t0_amt);
            println!("  Token1 Reserve: {}\n", t1_amt);
            (t0_amt, t1_amt)
        },
        _ => {
            println!("[WARN] Cannot query Reserve\n");
            (0u64, 0u64)
        },
    };

    let mut results = Vec::new();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let is_token0_wsol = pool_state.token0_mint == wsol_mint;

    for (label, amount) in test_amounts {
        println!("Test Amount: {} ({})\n", label, amount);
        println!("----------------------------------------------------");

        // Exact In Buy
        match cpmm_quote_exact_in(rpc, &pool_address, *amount, is_token0_wsol).await {
            Ok(quote) => {
                println!("[OK] Exact In Buy");
                println!("  Input: {} lamports", amount);
                println!("  Output: {} units", quote.amount_out);
                println!("  Fee: {} lamports\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "CPMM".to_string(),
                    mode: "exact_in".to_string(),
                    direction: "buy".to_string(),
                    calc_type: "Local".to_string(),
                    input_amount: *amount,
                    output_amount: quote.amount_out,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact In Buy failed: {}\n", e);
            },
        }

        // Exact Out Buy
        match cpmm_quote_exact_out(token0_reserve, token1_reserve, *amount, is_token0_wsol) {
            Ok(quote) => {
                println!("[OK] Exact Out Buy");
                println!("  Expected Output: {} units", amount);
                println!("  Required Input: {} lamports", quote.amount_in);
                println!("  Fee: {} lamports\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "CPMM".to_string(),
                    mode: "exact_out".to_string(),
                    direction: "buy".to_string(),
                    calc_type: "Local".to_string(),
                    input_amount: quote.amount_in,
                    output_amount: *amount,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact Out Buy failed: {}\n", e);
            },
        }

        // Exact In Sell
        match cpmm_quote_exact_in(rpc, &pool_address, *amount, !is_token0_wsol).await {
            Ok(quote) => {
                println!("[OK] Exact In Sell");
                println!("  Input: {} units", amount);
                println!("  Output: {} lamports", quote.amount_out);
                println!("  Fee: {} units\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "CPMM".to_string(),
                    mode: "exact_in".to_string(),
                    direction: "sell".to_string(),
                    calc_type: "Local".to_string(),
                    input_amount: *amount,
                    output_amount: quote.amount_out,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact In Sell failed: {}\n", e);
            },
        }

        // Exact Out Sell
        match cpmm_quote_exact_out(token0_reserve, token1_reserve, *amount, !is_token0_wsol) {
            Ok(quote) => {
                println!("[OK] Exact Out Sell");
                println!("  Expected Output: {} lamports", amount);
                println!("  Required Input: {} units", quote.amount_in);
                println!("  Fee: {} lamports\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "CPMM".to_string(),
                    mode: "exact_out".to_string(),
                    direction: "sell".to_string(),
                    calc_type: "Local".to_string(),
                    input_amount: quote.amount_in,
                    output_amount: *amount,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact Out Sell failed: {}\n", e);
            },
        }
    }

    Ok(results)
}

async fn test_amm_v4_pools(
    rpc: &Arc<SolanaRpcClient>,
    test_amounts: &Vec<(&str, u64)>,
) -> Result<Vec<TestResult>, String> {
    let pool_address = Pubkey::from_str(AMM_V4_SOL_USDC_POOL).unwrap();
    println!("Pool: {}", pool_address);
    println!("Direction: SOL <-> USDC\n");

    let pool_state = amm_get_pool(rpc, &pool_address)
        .await
        .map_err(|e| format!("Failed to get pool: {}", e))?;

    println!("Pool State:");
    println!("  Coin Mint: {}", pool_state.token_coin);
    println!("  PC Mint: {}", pool_state.token_pc);
    println!("  Coin Vault: {}", pool_state.token_coin);
    println!("  PC Vault: {}\n", pool_state.token_pc);

    let coin_balance = rpc.get_token_account_balance(&pool_state.token_coin).await;
    let pc_balance = rpc.get_token_account_balance(&pool_state.token_pc).await;

    let (coin_reserve, pc_reserve) = match (coin_balance, pc_balance) {
        (Ok(c), Ok(p)) => {
            let c_amt = c.amount.parse::<u64>().unwrap_or(0);
            let p_amt = p.amount.parse::<u64>().unwrap_or(0);
            println!("  Coin Reserve: {}", c_amt);
            println!("  PC Reserve: {}\n", p_amt);
            (c_amt, p_amt)
        },
        _ => {
            println!("[WARN] Cannot query Reserve\n");
            (0u64, 0u64)
        },
    };

    let mut results = Vec::new();
    let wsol_mint = Pubkey::from_str(WSOL_MINT).unwrap();
    let is_coin_wsol = pool_state.token_coin == wsol_mint;

    for (label, amount) in test_amounts {
        println!("Test Amount: {} ({})\n", label, amount);
        println!("----------------------------------------------------");

        // Exact In Buy
        match amm_quote_exact_in(rpc, &pool_address, *amount, is_coin_wsol).await {
            Ok(quote) => {
                println!("[OK] Exact In Buy");
                println!("  Input: {} lamports", amount);
                println!("  Output: {} units", quote.amount_out);
                println!("  Fee: {} lamports\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "AMM V4".to_string(),
                    mode: "exact_in".to_string(),
                    direction: "buy".to_string(),
                    calc_type: "Local".to_string(),
                    input_amount: *amount,
                    output_amount: quote.amount_out,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact In Buy failed: {}\n", e);
            },
        }

        // Exact Out Buy
        match amm_quote_exact_out(coin_reserve, pc_reserve, *amount, is_coin_wsol) {
            Ok(quote) => {
                println!("[OK] Exact Out Buy");
                println!("  Expected Output: {} units", amount);
                println!("  Required Input: {} lamports", quote.amount_in);
                println!("  Fee: {} lamports\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "AMM V4".to_string(),
                    mode: "exact_out".to_string(),
                    direction: "buy".to_string(),
                    calc_type: "Local".to_string(),
                    input_amount: quote.amount_in,
                    output_amount: *amount,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact Out Buy failed: {}\n", e);
            },
        }

        // Exact In Sell
        match amm_quote_exact_in(rpc, &pool_address, *amount, !is_coin_wsol).await {
            Ok(quote) => {
                println!("[OK] Exact In Sell");
                println!("  Input: {} units", amount);
                println!("  Output: {} lamports", quote.amount_out);
                println!("  Fee: {} units\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "AMM V4".to_string(),
                    mode: "exact_in".to_string(),
                    direction: "sell".to_string(),
                    calc_type: "Local".to_string(),
                    input_amount: *amount,
                    output_amount: quote.amount_out,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact In Sell failed: {}\n", e);
            },
        }

        // Exact Out Sell
        match amm_quote_exact_out(coin_reserve, pc_reserve, *amount, !is_coin_wsol) {
            Ok(quote) => {
                println!("[OK] Exact Out Sell");
                println!("  Expected Output: {} lamports", amount);
                println!("  Required Input: {} units", quote.amount_in);
                println!("  Fee: {} lamports\n", quote.fee_amount);

                results.push(TestResult {
                    dex_name: "AMM V4".to_string(),
                    mode: "exact_out".to_string(),
                    direction: "sell".to_string(),
                    calc_type: "Local".to_string(),
                    input_amount: quote.amount_in,
                    output_amount: *amount,
                    fee_amount: quote.fee_amount,
                    success: true,
                    error: None,
                });
            },
            Err(e) => {
                println!("[ERROR] Exact Out Sell failed: {}\n", e);
            },
        }
    }

    Ok(results)
}

fn print_summary_report(all_results: &Vec<TestResult>) {
    println!("\n");
    println!("====================================================");
    println!("Comprehensive Test Report");
    println!("====================================================\n");

    let mut dex_groups: std::collections::HashMap<String, Vec<&TestResult>> =
        std::collections::HashMap::new();

    for result in all_results {
        dex_groups.entry(result.dex_name.clone()).or_insert_with(Vec::new).push(result);
    }

    for dex in &["CLMM".to_string(), "CPMM".to_string(), "AMM V4".to_string()] {
        if let Some(dex_results) = dex_groups.get(dex) {
            println!("+{}+", "-".repeat(52));
            println!("| {} |", dex);
            println!("+{}+", "-".repeat(52));

            for (mode, direction) in &[
                ("exact_in", "buy"),
                ("exact_out", "buy"),
                ("exact_in", "sell"),
                ("exact_out", "sell"),
            ] {
                let filtered: Vec<_> = dex_results
                    .iter()
                    .filter(|r| &r.mode == mode && &r.direction == direction)
                    .collect();

                if !filtered.is_empty() {
                    println!("[{}] {} (Local)", mode.to_uppercase(), direction.to_uppercase());

                    for result in filtered {
                        if result.success {
                            println!(
                                "  [OK] Input: {} -> Output: {} (Fee: {})",
                                format_amount(result.input_amount),
                                format_amount(result.output_amount),
                                format_amount(result.fee_amount)
                            );
                        }
                    }
                    println!();
                }
            }
        }
    }

    let total_tests = all_results.len();
    let successful_tests = all_results.iter().filter(|r| r.success).count();
    let failed_tests = total_tests - successful_tests;

    println!("====================================================");
    println!("Statistics:");
    println!("  Total: {}", total_tests);
    println!("  Success: {}", successful_tests);
    println!("  Failed: {}", failed_tests);
    println!("  Success Rate: {:.1}%", (successful_tests as f64 / total_tests as f64) * 100.0);
    println!("====================================================\n");
}

fn format_amount(amount: u64) -> String {
    if amount >= 1_000_000_000 {
        format!("{:.2}B", amount as f64 / 1_000_000_000.0)
    } else if amount >= 1_000_000 {
        format!("{:.2}M", amount as f64 / 1_000_000.0)
    } else if amount >= 1_000 {
        format!("{:.2}K", amount as f64 / 1_000.0)
    } else {
        format!("{}", amount)
    }
}
