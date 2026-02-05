//! 测试已知 Token 的 WSOL PumpSwap Pool
//!
//! 这个测试验证一些常见 Token 是否有 WSOL 配对的 PumpSwap Pool，
//! 并检查它们使用的是 Token 还是 Token2022 Program。
//!
//! 运行测试:
//!     cargo nextest run --package sol-trade-test-utils pumpswap_wsol_token_pools -- --nocapture

use sol_trade_sdk::{
    common::auto_mock_rpc::AutoMockRpcClient,
    constants::TOKEN_PROGRAM,
    instruction::utils::pumpswap::get_pool_by_mint,
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use serial_test::serial;

/// 已知的 Token 配置
struct TokenConfig {
    name: &'static str,
    mint: &'static str,
}

const COMMON_TOKENS: &[TokenConfig] = &[
    TokenConfig {
        name: "BONK",
        mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
    },
    TokenConfig {
        name: "RAY",
        mint: "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
    },
    TokenConfig {
        name: "ORCA",
        mint: "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE",
    },
    TokenConfig {
        name: "PUMP",
        mint: "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn",
    },
];

#[tokio::test]
#[serial]
async fn test_pumpswap_wsol_token_pools() {
    println!("\n=== 测试：常见 Token 的 WSOL PumpSwap Pool ===\n");

    let rpc_url = "http://127.0.0.1:8899";
    let rpc = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("pumpswap_wsol_token_pools".to_string()),
    );

    let wsol_mint = wsol_mint();
    let mut token_pools = Vec::new();
    let mut token2022_pools = Vec::new();

    for token_config in COMMON_TOKENS {
        let mint = Pubkey::from_str(token_config.mint).unwrap();

        println!("🔍 {} ({})", token_config.name, mint);

        match get_pool_by_mint(&rpc, &mint).await {
            Ok((pool_addr, pool)) => {
                // 检查是否是 WSOL 配对
                let is_wsol_pair = pool.base_mint == wsol_mint || pool.quote_mint == wsol_mint;

                if !is_wsol_pair {
                    println!("  ⚠️  找到 Pool，但不是 WSOL 配对");
                    println!("     Base: {}", pool.base_mint);
                    println!("     Quote: {}", pool.quote_mint);
                    println!();
                    continue;
                }

                // 确定配对 Token
                let other_mint = if pool.base_mint == wsol_mint {
                    pool.quote_mint
                } else {
                    pool.base_mint
                };

                // 查询 Token Program
                match rpc.get_account(&other_mint).await {
                    Ok(account) => {
                        let is_token = account.owner == TOKEN_PROGRAM;
                        let is_token2022 = account.owner == sol_trade_sdk::constants::TOKEN_2022_PROGRAM;

                        if is_token {
                            println!("  ✅ WSOL + Token (纯 Token)");
                            println!("     Pool: {}", pool_addr);
                            println!("     LP Supply: {}", pool.lp_supply);
                            println!();
                            token_pools.push((token_config.name, pool_addr, pool.lp_supply));
                        } else if is_token2022 {
                            println!("  🪙 WSOL + Token2022");
                            println!("     Pool: {}", pool_addr);
                            println!("     LP Supply: {}", pool.lp_supply);
                            println!();
                            token2022_pools.push((token_config.name, pool_addr, pool.lp_supply));
                        } else {
                            println!("  ❓ 未知 Token Program: {}", account.owner);
                            println!();
                        }
                    },
                    Err(e) => {
                        println!("  ❌ 无法查询 Token Program: {}", e);
                        println!();
                    }
                }
            },
            Err(e) => {
                println!("  ❌ 未找到 Pool: {}", e);
                println!();
            }
        }
    }

    // 打印总结
    println!("═══════════════════════════════════════");
    println!("📊 测试总结:");
    println!("  纯 Token 配对: {} 个", token_pools.len());
    println!("  Token2022 配对: {} 个", token2022_pools.len());
    println!();

    // 详细列出 Token Pool
    if !token_pools.is_empty() {
        println!("✅ Token Pool 列表:");
        for (name, addr, lp) in &token_pools {
            println!("  - {} ({})", name, addr);
            println!("    LP Supply: {}", lp);
        }
        println!();
    }

    // 详细列出 Token2022 Pool
    if !token2022_pools.is_empty() {
        println!("🪙 Token2022 Pool 列表:");
        for (name, addr, lp) in &token2022_pools {
            println!("  - {} ({})", name, addr);
            println!("    LP Supply: {}", lp);
        }
        println!();
    }

    // 断言：至少找到一个 WSOL Pool
    let total = token_pools.len() + token2022_pools.len();
    assert!(total > 0, "应该至少找到一个 WSOL PumpSwap Pool");

    println!("✅ 测试通过");
}

// ============ Helper Functions ============

/// WSOL Mint 地址
fn wsol_mint() -> Pubkey {
    Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap()
}
