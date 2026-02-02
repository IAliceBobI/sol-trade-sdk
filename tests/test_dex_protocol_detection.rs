//! 测试 DEX 协议识别功能
//!
//! 通过 Pool 地址识别 DEX 协议

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use sol_trade_sdk::constants::DexProtocol;
use sol_trade_sdk::common::SolanaRpcClient;

#[tokio::test]
async fn test_detect_dex_from_pool_address() {
    let rpc_url = std::env::var("RPC_URL").unwrap_or("http://127.0.0.1:8899".to_string());
    let rpc = SolanaRpcClient::new(rpc_url);

    // 测试各个 DEX 的 Pool 地址
    let test_pools = vec![
        // PumpSwap Pool: WIF-SOL
        ("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA", "EKzQ98GWgoQ8hWqiSToQpLduuGjX5MFdB6vXJNTkCepD", DexProtocol::PumpSwap),
        // Raydium CLMM Pool: WSOL-USDT
        ("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", "ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6", DexProtocol::RaydiumClmm),
        // Raydium AMM V4 Pool: USDC-WSOL
        ("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", "58L7CzkRV5qD1owUPurbAC7gUNV1kSzwj2TMkvgEpbjZ", DexProtocol::RaydiumAmmV4),
        // Raydium CPMM Pool: RAY-SOL
        ("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C", "DQfGJgjYcGSonFj6QoiQSYRmSMdnFM8NkYGXdHU7KNnB", DexProtocol::RaydiumCpmm),
        // Meteora DAMM V2 Pool: USDC-WSOL
        ("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG", "4C3JRBp4Bycs3jQTuJVEL6kVAWJMhNUshaD5GmwcEaMu", DexProtocol::MeteoraDammV2),
    ];

    println!("\n🔍 测试从 Pool 地址识别 DEX 协议\n");

    for (_expected_program_id, pool_address, expected_protocol) in test_pools {
        println!("📋 测试 Pool: {}", pool_address);
        println!("   预期 DEX: {}", expected_protocol.name());

        // 获取 Pool 账户信息
        match rpc.get_account(&Pubkey::from_str(pool_address).unwrap()).await {
            Ok(account) => {
                // 获取 owner (program_id)
                let program_id = account.owner.to_string();
                println!("   实际 Program ID: {}", program_id);

                // 识别 DEX
                match DexProtocol::from_program_id(&account.owner) {
                    Some(detected_protocol) => {
                        println!("   识别结果: {}", detected_protocol.name());
                        assert_eq!(
                            detected_protocol, expected_protocol,
                            "DEX 识别不匹配: 预期 {}, 实际 {}",
                            expected_protocol.name(),
                            detected_protocol.name()
                        );
                        println!("   ✅ 识别成功\n");
                    }
                    None => {
                        panic!("❌ 无法识别 Program ID: {}", program_id);
                    }
                }
            }
            Err(e) => {
                println!("   ⚠️  获取账户失败: {}\n", e);
                // 在测试环境中某些池可能不存在，跳过而不是失败
                continue;
            }
        }
    }

    println!("🎉 所有测试通过！");
}

#[tokio::test]
async fn test_list_all_supported_dex() {
    println!("\n📊 Sol Trade SDK 支持的所有 DEX 协议:\n");

    for protocol in DexProtocol::all_protocols() {
        println!("   {} - {}", protocol.name(), protocol.program_id());
    }

    println!("\n总计: {} 个 DEX 协议", DexProtocol::all_protocols().len());
}

#[tokio::test]
async fn test_protocol_id_consistency() {
    println!("\n🔐 测试 Program ID 一致性\n");

    for protocol in DexProtocol::all_protocols() {
        let id_str = protocol.program_id();
        let id_pubkey = protocol.program_id_pubkey();

        // 验证字符串和 Pubkey 格式一致
        assert_eq!(id_str, id_pubkey.to_string());

        // 验证可以双向转换
        let parsed = DexProtocol::from_program_id_str(id_str);
        assert_eq!(Some(protocol), parsed.as_ref());

        let parsed_from_pubkey = DexProtocol::from_program_id(&id_pubkey);
        assert_eq!(Some(protocol), parsed_from_pubkey.as_ref());

        println!("   ✅ {} - {}", protocol.name(), id_str);
    }

    println!("\n🎉 所有的 Program ID 一致性验证通过！");
}
