//! 测试 PumpSwap Pool 列出和分类功能（使用已知 Token）
//!
//! 由于本地测试节点不支持 get_program_accounts，
//! 此测试使用已知的 Token 列表来验证分类功能。
//!
//! 运行测试:
//!     cargo nextest run --package sol-trade-test-utils test_list_pumpswap_wsol_pools -- --nocapture

use serial_test::serial;
use sol_trade_sdk::{
    common::auto_mock_rpc::AutoMockRpcClient, constants::TOKEN_PROGRAM,
    instruction::utils::pumpswap::get_pool_by_mint,
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::test]
#[serial]
async fn test_list_pumpswap_wsol_pools() {
    println!("\n=== 测试：列出 PumpSwap WSOL Pool 并分类 ===\n");

    let rpc_url = "http://127.0.0.1:8899";
    let rpc = AutoMockRpcClient::new_with_namespace(
        rpc_url.to_string(),
        Some("list_pumpswap_wsol_pools".to_string()),
    );

    let wsol_mint = wsol_mint();

    // 使用已知的 Token 来测试分类功能
    let known_tokens = vec![
        ("BONK", "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"),
        ("RAY", "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"),
    ];

    let mut found_token_pools = 0;
    let mut found_token2022_pools = 0;

    for (name, mint_str) in known_tokens {
        let mint = Pubkey::from_str(mint_str).unwrap();

        match get_pool_by_mint(&rpc, &mint).await {
            Ok((pool_addr, pool)) => {
                let is_wsol_pair = pool.base_mint == wsol_mint || pool.quote_mint == wsol_mint;

                if is_wsol_pair {
                    let other_mint =
                        if pool.base_mint == wsol_mint { pool.quote_mint } else { pool.base_mint };

                    if let Ok(account) = rpc.get_account(&other_mint).await {
                        if account.owner == TOKEN_PROGRAM {
                            println!("✅ {} WSOL Pool (Token): {}", name, pool_addr);
                            found_token_pools += 1;
                        } else if account.owner == sol_trade_sdk::constants::TOKEN_2022_PROGRAM {
                            println!("🪙 {} WSOL Pool (Token2022): {}", name, pool_addr);
                            found_token2022_pools += 1;
                        }
                    }
                }
            },
            Err(_) => {},
        }
    }

    println!("\n📊 分类结果:");
    println!("  Token Pool: {} 个", found_token_pools);
    println!("  Token2022 Pool: {} 个", found_token2022_pools);

    let total = found_token_pools + found_token2022_pools;
    assert!(total > 0, "应该至少找到一个 WSOL PumpSwap Pool");

    println!("\n✅ 测试通过");
}

// ============ Helper Functions ============

/// WSOL Mint 地址
fn wsol_mint() -> Pubkey {
    Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap()
}
