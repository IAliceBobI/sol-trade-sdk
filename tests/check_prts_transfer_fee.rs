//! 检查 PRTS Token 的 Transfer Fee 配置

use sol_trade_sdk::common::SolanaRpcClient;
use solana_sdk::pubkey::Pubkey;

#[tokio::test]
async fn test_check_prts_transfer_fee() {
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = SolanaRpcClient::new(rpc_url.to_string());

    // PRTS Token Mint
    let prts_mint = Pubkey::from_str_const("3PQkX8yfuxoe9kuBoLCEZoxzi9LG4w8Ci2JWWGNfPRTS");

    println!("=== 检查 PRTS Token Mint 信息 ===\n");

    // 获取 Mint 账户
    let mint_account = rpc.get_account(&prts_mint).await.expect("获取 Mint 账户失败");

    println!("Mint 数据长度: {} bytes", mint_account.data.len());
    println!("Mint Owner: {}", mint_account.owner);

    // 检查是否是 Token-2022
    let token_2022_program = Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
    let is_token2022 = mint_account.owner == token_2022_program;
    println!("是否 Token-2022: {}\n", is_token2022);

    if is_token2022 {
        println!("=== 解析 Token-2022 扩展 ===\n");

        // 尝试解析 Transfer Fee 扩展
        use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
        use spl_token_2022::state::Mint as Mint2022;

        if let Ok(mint_with_extensions) = StateWithExtensions::<Mint2022>::unpack(&mint_account.data) {
            println!("✅ 成功解析 Token-2022 Mint");
            println!("Decimals: {}", mint_with_extensions.base.decimals);
            println!("Mint Authority: {:?}", mint_with_extensions.base.mint_authority);
            println!("Supply: {}\n", mint_with_extensions.base.supply);

            // 获取所有扩展类型
            let extensions = mint_with_extensions.get_extension_types();
            println!("启用的扩展类型:");
            if let Ok(ext) = extensions {
                println!("  - {:?}", ext);
            }
            println!();

            // 尝试获取 Transfer Fee 扩展
            use spl_token_2022::extension::transfer_fee::TransferFeeConfig;

            if let Ok(transfer_fee) = mint_with_extensions.get_extension::<TransferFeeConfig>() {
                println!("✅ 发现 Transfer Fee 扩展!");
                println!("  Transfer Fee 配置:");
                println!("    Transfer Fee Authority: {:?}", transfer_fee.transfer_fee_config_authority);
                println!("    Withdraw Withheld Authority: {:?}", transfer_fee.withdraw_withheld_authority);
                println!("    Withheld Amount: {:?}", transfer_fee.withheld_amount);

                // 检查旧的 Transfer Fee 配置
                let old_fee = &transfer_fee.older_transfer_fee;
                println!("    旧 Transfer Fee:");
                println!("      Epoch: {:?}", old_fee.epoch);
                println!("      最大 Fee: {:?}", old_fee.maximum_fee);
                println!("      Transfer Fee 基点: {:?}", old_fee.transfer_fee_basis_points);

                // 检查新的 Transfer Fee 配置
                let new_fee = &transfer_fee.newer_transfer_fee;
                println!("    新 Transfer Fee:");
                println!("      Epoch: {:?}", new_fee.epoch);
                println!("      最大 Fee: {:?}", new_fee.maximum_fee);
                println!("      Transfer Fee 基点: {:?}", new_fee.transfer_fee_basis_points);
            } else {
                println!("❌ 未找到 Transfer Fee 扩展");
            }
        } else {
            println!("❌ 解析 Token-2022 Mint 失败");
        }
    }

    // 检查 Pool Vault 的余额
    println!("\n=== 检查 Pool Vault 余额 ===");

    let _pool_address = Pubkey::from_str_const("7Cvz28TyKnGuL8GAtbsVFu1FJ3Po7A37Zc8JSJqkSPDp");

    // Token0 Vault (PRTS)
    let token0_vault = Pubkey::from_str_const("HYuXxtUhtHQbwr7dSHDry6wUV4oqcTvSQgWxYp8Jakpx");
    // Token1 Vault (USDC)
    let token1_vault = Pubkey::from_str_const("nky38vubmfKNQebNKmynAKsnT15zpfYzoJRVdERhv2F");

    let (token0_balance, token1_balance) = tokio::join!(
        rpc.get_token_account_balance(&token0_vault),
        rpc.get_token_account_balance(&token1_vault),
    );

    match token0_balance {
        Ok(balance) => println!("Token0 Vault (PRTS): {} (原始单位)", balance.amount),
        Err(e) => println!("获取 Token0 Vault 失败: {}", e),
    }

    match token1_balance {
        Ok(balance) => println!("Token1 Vault (USDC): {} (原始单位)", balance.amount),
        Err(e) => println!("获取 Token1 Vault 失败: {}", e),
    }
}
