//! 测试：检查已知的 Raydium CPMM 池子是否能通过 mint 查询找到

use sol_trade_sdk::instruction::utils::raydium_cpmm::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";
const METATRON_MINT: &str = "5C8M6tArtMYnYxz9TH4vE1S1NzohrX21qXG2Kir5LQUt";
const KNOWN_POOL_ADDRESS: &str = "DZ1Yr5V1hyU2ryJWchAhCQcKRHL5RxS7G9DHmbx1rcEz";

#[tokio::test]
async fn test_check_known_cpmm_pool() {
    println!("=== 测试：检查已知 CPMM 池子是否可通过 mint 查询找到 ===");

    let metatron_mint = Pubkey::from_str(METATRON_MINT).expect("Invalid Metatron mint");
    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let known_pool_pubkey = Pubkey::from_str(KNOWN_POOL_ADDRESS).expect("Invalid pool address");

    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());
    
    // 方法1：直接通过 RPC 获取池状态（最可靠）
    println!("\n--- 方法1：直接通过 RPC 获取池状态 ---");
    match get_pool_by_address(&rpc, &known_pool_pubkey).await {
        Ok(pool_state) => {
            println!("✅ 成功获取池状态:");
            println!("  Pool Address: {}", known_pool_pubkey);
            println!("  Token0 Mint: {}", pool_state.token0_mint);
            println!("  Token1 Mint: {}", pool_state.token1_mint);
            println!("  Amm Config: {}", pool_state.amm_config);
            
            // 验证是否匹配已知信息
            let has_metatron = pool_state.token0_mint == metatron_mint || pool_state.token1_mint == metatron_mint;
            let has_wsol = pool_state.token0_mint == wsol_mint || pool_state.token1_mint == wsol_mint;
            
            if has_metatron && has_wsol {
                println!("✅ 池子信息完全匹配！");
            } else {
                println!("⚠️  池子信息不匹配:");
                println!("  包含 Metatron Mint: {}", has_metatron);
                println!("  包含 WSOL Mint: {}", has_wsol);
            }
        }
        Err(e) => {
            println!("❌ 无法通过 RPC 获取池状态: {}", e);
        }
    }
    
    // 方法4：手动构建 PDA 并验证
    println!("\n--- 方法4：手动构建 PDA 并验证 ---");
    
    // 需要知道 amm_config 来构建 PDA
    // 这里我们尝试一些常见的 amm_config
    let common_configs = vec![
        "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 常见的 amm_config
        "Gs6hMWE6gZHq8t6sWqnZZom1MHnFPtbHCY9oXtf9wE2c",
        "6amNY8ssAFozbUfgovULdqfymC2z8XrLeH6WzM6awtRh",
    ];
    
    for config_str in common_configs {
        match Pubkey::from_str(config_str) {
            Ok(amm_config) => {
                if let Some(pool_pda) = get_pool_pda(&amm_config, &metatron_mint, &wsol_mint) {
                    println!("  AmmConfig {}: 推导出 PDA = {}", config_str, pool_pda);
                    
                    if pool_pda == known_pool_pubkey {
                        println!("  ✅ PDA 匹配！使用的 AmmConfig: {}", config_str);
                        
                        // 验证该 PDA 是否真的存在
                        match rpc.get_account(&pool_pda).await {
                            Ok(account) => {
                                if !account.data.is_empty() {
                                    println!("  ✅ 账户存在且有数据");
                                } else {
                                    println!("  ⚠️  账户存在但无数据");
                                }
                            }
                            Err(e) => {
                                println!("  ❌ 无法获取账户数据: {}", e);
                            }
                        }
                    }
                } else {
                    println!("  AmmConfig {}: 无法推导出有效的 PDA", config_str);
                }
            }
            Err(_) => {
                println!("  无效的 AmmConfig: {}", config_str);
            }
        }
    }
    
    println!("\n=== 测试完成 ===");
}

#[tokio::test]
async fn test_list_pools_by_wsol_mint() {
    println!("=== 测试：通过 WSOL Mint 查询池子 ===");

    let wsol_mint = Pubkey::from_str(WSOL_MINT).expect("Invalid WSOL mint");
    let known_pool_pubkey = Pubkey::from_str(KNOWN_POOL_ADDRESS).expect("Invalid pool address");

    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());
    
    println!("\n--- 通过 WSOL Mint 查询池子 ---");
    match list_pools_by_mint(&rpc, &wsol_mint).await {
        Ok(pools) => {
            println!("WSOL Mint 相关池子数量: {}", pools.len());
            
            let mut found = false;
            for (pool_addr, pool_state) in pools.iter() {
                println!("  池子: {} (token0={}, token1={})", 
                    pool_addr, pool_state.token0_mint, pool_state.token1_mint);
                
                if *pool_addr == known_pool_pubkey {
                    println!("  ✅ 找到目标池子!");
                    found = true;
                }
            }
            
            if !found {
                println!("  ❌ 未通过 WSOL Mint 找到目标池子");
            }
        }
        Err(e) => {
            println!("❌ 通过 WSOL Mint 查询失败: {}", e);
        }
    }
    
    println!("\n=== 测试完成 ===");
}

#[tokio::test]
async fn test_list_pools_by_metatron_mint() {
    println!("=== 测试：通过 Metatron Mint 查询池子 ===");

    let metatron_mint = Pubkey::from_str(METATRON_MINT).expect("Invalid Metatron mint");
    let known_pool_pubkey = Pubkey::from_str(KNOWN_POOL_ADDRESS).expect("Invalid pool address");

    let rpc_url = "https://api.mainnet-beta.solana.com";
    let rpc = RpcClient::new(rpc_url.to_string());
    
    println!("\n--- 通过 Metatron Mint 查询池子 ---");
    match list_pools_by_mint(&rpc, &metatron_mint).await {
        Ok(pools) => {
            println!("Metatron Mint 相关池子数量: {}", pools.len());
            
            let mut found = false;
            for (pool_addr, pool_state) in pools.iter() {
                println!("  池子: {} (token0={}, token1={})", 
                    pool_addr, pool_state.token0_mint, pool_state.token1_mint);
                
                if *pool_addr == known_pool_pubkey {
                    println!("  ✅ 找到目标池子!");
                    found = true;
                }
            }
            
            if !found {
                println!("  ❌ 未通过 Metatron Mint 找到目标池子");
            }
        }
        Err(e) => {
            println!("❌ 通过 Metatron Mint 查询失败: {}", e);
        }
    }
    
    println!("\n=== 测试完成 ===");
}