//! DEX 检测功能集成测试

use sol_trade_sdk::common::dex_detector::{detect_dex_from_pool, detect_dex_from_pools_batch, DexInfo};
use sol_trade_sdk::constants::DexProtocol;
use sol_trade_sdk::common::SolanaRpcClient;

#[tokio::test]
async fn test_detect_dex_from_pumpswap_pool() {
    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let rpc = SolanaRpcClient::new(rpc_url);

    // PumpSwap PUMP-WSOL Pool（来自文档的测试 Pool）
    let pool_address = "539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR";
    let result = detect_dex_from_pool(&rpc, pool_address).await;

    if let Err(ref e) = result {
        eprintln!("PumpSwap 检测失败: {:?}", e);
    }
    assert!(result.is_ok(), "应该成功识别 PumpSwap Pool");

    let dex_info = result.unwrap();
    assert_eq!(dex_info.protocol, DexProtocol::PumpSwap);
    assert_eq!(dex_info.dex_name(), "pumpswap");
    assert_eq!(dex_info.display_name(), "PumpSwap");
    assert_eq!(
        dex_info.program_id,
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA"
    );
}

#[tokio::test]
async fn test_detect_dex_from_raydium_clmm_pool() {
    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let rpc = SolanaRpcClient::new(rpc_url);

    // Raydium CLMM WSOL-USDT Pool
    let pool_address = "ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6";
    let result = detect_dex_from_pool(&rpc, pool_address).await;

    assert!(result.is_ok(), "应该成功识别 Raydium CLMM Pool");

    let dex_info = result.unwrap();
    assert_eq!(dex_info.protocol, DexProtocol::RaydiumClmm);
    assert_eq!(dex_info.dex_name(), "raydium_clmm");
    assert_eq!(dex_info.display_name(), "Raydium CLMM");
}

#[tokio::test]
async fn test_detect_dex_from_raydium_amm_v4_pool() {
    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let rpc = SolanaRpcClient::new(rpc_url);

    // Raydium AMM V4 SOL-USDC Pool（来自文档的测试 Pool）
    let pool_address = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";
    let result = detect_dex_from_pool(&rpc, pool_address).await;

    if let Err(ref e) = result {
        eprintln!("Raydium AMM V4 检测失败: {:?}", e);
    }
    assert!(result.is_ok(), "应该成功识别 Raydium AMM V4 Pool");

    let dex_info = result.unwrap();
    assert_eq!(dex_info.protocol, DexProtocol::RaydiumAmmV4);
    assert_eq!(dex_info.dex_name(), "raydium_amm_v4");
    assert_eq!(dex_info.display_name(), "Raydium AMM V4");
}

#[tokio::test]
async fn test_detect_dex_from_raydium_cpmm_pool() {
    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let rpc = SolanaRpcClient::new(rpc_url);

    // Raydium CPMM PIPE-WSOL Pool（来自文档的测试 Pool）
    let pool_address = "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp";
    let result = detect_dex_from_pool(&rpc, pool_address).await;

    if let Err(ref e) = result {
        eprintln!("Raydium CPMM 检测失败: {:?}", e);
    }
    assert!(result.is_ok(), "应该成功识别 Raydium CPMM Pool");

    let dex_info = result.unwrap();
    assert_eq!(dex_info.protocol, DexProtocol::RaydiumCpmm);
    assert_eq!(dex_info.dex_name(), "raydium_cpmm");
    assert_eq!(dex_info.display_name(), "Raydium CPMM");
}

#[tokio::test]
#[ignore = "需要 Meteora DAMM V2 Pool 在测试节点上"]
async fn test_detect_dex_from_meteora_damm_v2_pool() {
    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let rpc = SolanaRpcClient::new(rpc_url);

    // Meteora DAMM V2 USDC-WSOL Pool（来自其他测试文件）
    let pool_address = "4C3JRBp4Bycs3jQTuJVEL6kVAWJMhNUshaD5GmwcEaMu";
    let result = detect_dex_from_pool(&rpc, pool_address).await;

    if let Err(ref e) = result {
        eprintln!("Meteora DAMM V2 检测失败: {:?}", e);
    }
    assert!(result.is_ok(), "应该成功识别 Meteora DAMM V2 Pool");

    let dex_info = result.unwrap();
    assert_eq!(dex_info.protocol, DexProtocol::MeteoraDammV2);
    assert_eq!(dex_info.dex_name(), "meteora_damm_v2");
    assert_eq!(dex_info.display_name(), "Meteora DAMM V2");
}

#[tokio::test]
async fn test_detect_dex_invalid_pool_address() {
    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let rpc = SolanaRpcClient::new(rpc_url);

    // 无效的 Pool 地址
    let pool_address = "Invalid1111111111111111111111111111111";
    let result = detect_dex_from_pool(&rpc, pool_address).await;

    assert!(result.is_err(), "无效地址应该返回错误");

    let err = result.unwrap_err();
    assert!(err.to_string().contains("无效的 Pool 地址"));
}

#[tokio::test]
async fn test_detect_dex_unknown_program() {
    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let rpc = SolanaRpcClient::new(rpc_url);

    // 使用 System Program 作为 owner（不是 DEX）
    let pool_address = "11111111111111111111111111111111"; // System Program
    let result = detect_dex_from_pool(&rpc, pool_address).await;

    // 可能失败（账户不存在）或识别为未知协议
    if let Ok(_dex_info) = result {
        // 如果成功，应该是未知协议
        panic!("System Program 不应该被识别为 DEX");
    }
    // 如果失败，这是预期的
}

#[tokio::test]
async fn test_detect_dex_batch() {
    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let rpc = SolanaRpcClient::new(rpc_url);

    let pools = vec![
        "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2", // Raydium AMM V4
        "BnYsRpYvJpz6biY3hV6U9smChVePCJ6YyupVDfcnXpTp", // Raydium CPMM
        "539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR", // PumpSwap
    ];

    let results = detect_dex_from_pools_batch(&rpc, &pools).await;

    assert_eq!(results.len(), 3, "应该成功识别所有 Pool");

    let protocols: Vec<_> = results.iter().map(|info| info.protocol).collect();
    assert!(protocols.contains(&DexProtocol::RaydiumAmmV4));
    assert!(protocols.contains(&DexProtocol::RaydiumCpmm));
    assert!(protocols.contains(&DexProtocol::PumpSwap));
}

#[tokio::test]
async fn test_dex_info_methods() {
    let info = DexInfo::new(
        "58L7CzkRV5qD1owUPurbAC7gUNV1kSzwj2TMkvgEpbjZ".to_string(),
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(),
    )
    .expect("应该成功创建 DexInfo");

    // 测试各个方法
    assert_eq!(info.dex_name(), "raydium_amm_v4");
    assert_eq!(info.display_name(), "Raydium AMM V4");
    assert_eq!(info.pool_address, "58L7CzkRV5qD1owUPurbAC7gUNV1kSzwj2TMkvgEpbjZ");
    assert_eq!(
        info.program_id,
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
    );
    assert_eq!(info.protocol, DexProtocol::RaydiumAmmV4);
}
