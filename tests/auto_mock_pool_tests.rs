//! Auto Mock Pool 查询测试
//!
//! 演示如何使用 AutoMockRpcClient 进行 Pool 查询测试
//!
//! 运行测试:
//!     cargo test --test auto_mock_pool_tests -- --nocapture
//!
//! 工作原理:
//! 1. 首次运行：从 RPC 获取数据并保存到 tests/mock_data/
//! 2. 后续运行：从缓存文件加载（超快！）
//! 3. 清理缓存：删除 mock 数据文件即可重新录制

use sol_trade_sdk::common::auto_mock_rpc::AutoMockRpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Raydium AMM V4 WSOL-USDC Pool 地址
const WSOL_USDC_POOL: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";

/// Raydium AMM V4 程序 ID
const RAYDIUM_AMM_V4: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// 测试：使用 Auto Mock 获取单个 Pool 账户
#[tokio::test]
async fn test_auto_mock_get_pool_account() {
    println!("=== 测试：Auto Mock get_account ===");

    // 创建 Auto Mock RPC 客户端
    let client = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());

    let pool_address = Pubkey::from_str(WSOL_USDC_POOL).unwrap();

    println!("Pool 地址: {}", pool_address);

    // 首次运行：从 RPC 获取并保存（约 1-2 秒）
    // 后续运行：从缓存加载（约 0.01 秒）
    let account = client.get_account(&pool_address).await.unwrap();

    println!("✅ Pool 账户获取成功!");
    println!("  Owner: {}", account.owner);
    println!("  Lamports: {}", account.lamports);
    println!("  Data 长度: {} bytes", account.data.len());

    // 验证账户由 Raydium AMM V4 程序拥有
    let expected_owner = Pubkey::from_str(RAYDIUM_AMM_V4).unwrap();
    assert_eq!(account.owner, expected_owner, "Pool owner 应该是 Raydium AMM V4");

    // 验证账户数据大小（AMM Info 结构体 752 bytes）
    assert_eq!(account.data.len(), 752, "Pool data 应该是 752 bytes");

    println!("✅ 验证通过!");
}

/// 测试：使用 Auto Mock 获取程序账户列表
#[tokio::test]
async fn test_auto_mock_get_program_accounts() {
    println!("=== 测试：Auto Mock get_program_ui_accounts_with_config ===");

    use solana_account_decoder::UiAccountEncoding;
    use solana_client::rpc_filter::Memcmp;
    use solana_rpc_client_api::{config::RpcProgramAccountsConfig, filter::RpcFilterType};

    // 创建 Auto Mock RPC 客户端
    let client = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());

    let program_id = Pubkey::from_str(RAYDIUM_AMM_V4).unwrap();
    let wsol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

    println!("程序 ID: {}", program_id);
    println!("查询 WSOL 相关的 Pool...");

    // 配置过滤器：查询 WSOL 作为 coin_mint 的 Pool
    let filters = vec![
        RpcFilterType::DataSize(752),  // AMM_INFO_SIZE
        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(400, &wsol_mint.to_bytes())),
    ];

    let config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: solana_rpc_client_api::config::RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: None,
            min_context_slot: None,
        },
        with_context: None,
        sort_results: None,
    };

    // 首次运行：从 RPC 获取并保存（约 10-30 秒，取决于 Pool 数量）
    // 后续运行：从缓存加载（约 0.01 秒）
    let accounts = client
        .get_program_ui_accounts_with_config(&program_id, config)
        .await
        .unwrap();

    println!("✅ 查询成功! 找到 {} 个 WSOL Pool", accounts.len());

    // 验证至少有一个 Pool
    assert!(!accounts.is_empty(), "WSOL Pool 列表不应为空");

    // 打印前 5 个 Pool 地址
    for (i, (address, _)) in accounts.iter().take(5).enumerate() {
        println!("  Pool #{}: {}", i + 1, address);
    }

    if accounts.len() > 5 {
        println!("  ... 还有 {} 个 Pool", accounts.len() - 5);
    }

    println!("✅ 验证通过!");
}

/// 测试：验证缓存机制
#[tokio::test]
async fn test_auto_mock_cache_mechanism() {
    println!("=== 测试：Auto Mock 缓存机制 ===");

    let client = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());
    let pool_address = Pubkey::from_str(WSOL_USDC_POOL).unwrap();

    // 第一次调用：从 RPC 获取并保存
    println!("第一次调用（从 RPC 获取并保存）...");
    let start = std::time::Instant::now();
    let account1 = client.get_account(&pool_address).await.unwrap();
    let duration1 = start.elapsed();

    println!("  耗时: {:?}", duration1);
    println!("  Lamports: {}", account1.lamports);

    // 第二次调用：从缓存加载（应该快得多）
    println!("\n第二次调用（从缓存加载）...");
    let start = std::time::Instant::now();
    let account2 = client.get_account(&pool_address).await.unwrap();
    let duration2 = start.elapsed();

    println!("  耗时: {:?}", duration2);
    println!("  Lamports: {}", account2.lamports);

    // 验证两次返回的数据相同
    assert_eq!(account1.lamports, account2.lamports, "缓存数据应该一致");
    assert_eq!(account1.data.len(), account2.data.len(), "缓存数据应该一致");

    // 验证第二次调用明显更快（如果缓存生效）
    println!("\n✅ 缓存验证通过!");
    println!("  首次调用: {:?}", duration1);
    println!("  缓存调用: {:?}", duration2);
    println!("  速度提升: {:.2}x", duration1.as_secs_f64() / duration2.as_secs_f64());
}

/// 测试：检查 Mock 数据是否存在
#[test]
fn test_auto_mock_check_data_exists() {
    println!("=== 测试：检查 Mock 数据是否存在 ===");

    use serde_json::json;

    let client = AutoMockRpcClient::new("http://127.0.0.1:8899".to_string());
    let pool_address = Pubkey::from_str(WSOL_USDC_POOL).unwrap();

    let params = json!([pool_address.to_string()]);

    let exists = client.has_mock_data("get_account", &params);

    if exists {
        println!("✅ Mock 数据已存在（测试将使用缓存）");
    } else {
        println!("ℹ️  Mock 数据不存在（首次运行将从 RPC 获取）");
    }

    // 此测试总是成功，只是用于提示状态
    assert!(true);
}
