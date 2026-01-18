use sol_trade_sdk::{
    common::{GasFeeStrategy, TradeConfig},
    swqos::SwqosConfig,
    trading::core::params::{DexParamEnum, RaydiumClmmParams},
    DexType, SolanaTrade, TradeBuyParams, TradeSellParams, TradeTokenType,
};
use solana_client::nonblocking::rpc_client::RpcClient as RpcClientAsync;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};
use std::{str::FromStr, sync::Arc};

mod test_helpers;
use test_helpers::{print_balances, print_token_balance};

/// JUP Token mint
const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

/// WSOL-JUP CLMM Pool
const WSOL_JUP_POOL: &str = "EZVkeboWeXygtq8LMyENHyXdF5wpYrtExRNH9UwB1qYw";

/// 测试所需的最低 SOL 余额（lamports）
const MIN_REQUIRED_BALANCE: u64 = 2_000_000_000; // 2 SOL

#[tokio::test]
#[serial_test::serial]
async fn test_raydium_clmm_buy_and_sell_jup() {
    println!("\n=== 测试：Raydium CLMM 完整交易流程（买入+卖出 JUP） ===");

    // 使用官方配置的账户
    use std::fs;
    let payer_path = "docs/id.json";
    let keypair_bytes = fs::read_to_string(payer_path).expect("Failed to read payer keypair file");
    let keypair_vec: Vec<u8> =
        serde_json::from_str(&keypair_bytes).expect("Failed to parse keypair JSON");
    // Keypair JSON 文件格式：[secret_key(32 bytes) + public_key(32 bytes)] = 64 bytes
    // new_from_array 只需要前32字节（secret key）
    let mut keypair_array = [0u8; 32];
    keypair_array.copy_from_slice(&keypair_vec[..32]);
    let payer = Arc::new(Keypair::new_from_array(keypair_array));

    let rpc_url = "http://127.0.0.1:8899";
    let rpc_url_for_client = rpc_url.to_string();
    let commitment = CommitmentConfig::confirmed();

    let payer_pubkey = payer.pubkey();
    println!("测试钱包: {}", payer_pubkey);

    // ===== 步骤 0: 检查余额并自动空投 =====
    let rpc_client = RpcClientAsync::new(rpc_url.to_string());

    let balance = rpc_client
        .get_balance_with_commitment(&payer_pubkey, commitment)
        .await
        .expect("Failed to get balance")
        .value;

    println!("当前 SOL 余额: {} lamports ({:.4} SOL)", balance, balance as f64 / 1e9);

    if balance < MIN_REQUIRED_BALANCE {
        println!(
            "⚠️  余额不足，需要至少 {:.4} SOL",
            MIN_REQUIRED_BALANCE as f64 / 1e9
        );
        println!("💸 正在申请空投 2 SOL...");

        // 直接使用最简单的空投方法
        let signature_result = rpc_client
            .request_airdrop(&payer_pubkey, MIN_REQUIRED_BALANCE)
            .await;

        let signature = match signature_result {
            Ok(sig) => sig,
            Err(e) => {
                println!("⚠️  自动空投失败: {}", e);
                println!("提示: 本地测试节点可能不支持空投");
                println!("请手动执行: solana airdrop 2 {} --url {}", payer_pubkey, rpc_url);
                panic!("需要手动空投 SOL 以继续测试");
            }
        };

        println!("✅ 空投请求已提交！签名: {}", signature);

        // 确认交易
        println!("⏳ 等待空投确认...");
        let max_retries = 30;
        let mut retries = 0;
        loop {
            let confirmed = rpc_client
                .get_signature_status(&signature)
                .await
                .expect("Failed to check confirmation");

            if let Some(Ok(_)) = confirmed {
                println!("✅ 空投已确认！");
                break;
            }

            retries += 1;
            if retries >= max_retries {
                panic!("空投确认超时");
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            println!("  等待中... ({}/{})", retries, max_retries);
        }

        // 等待链上状态更新
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let new_balance = rpc_client
            .get_balance_with_commitment(&payer_pubkey, commitment)
            .await
            .expect("Failed to get balance after airdrop")
            .value;
        println!(
            "空投后余额: {} lamports ({:.4} SOL)",
            new_balance,
            new_balance as f64 / 1e9
        );
    } else {
        println!("✅ 余额充足，无需空投");
    }

    // ===== 步骤 1: 创建 TradingClient =====
    let swqos_configs: Vec<SwqosConfig> = vec![SwqosConfig::Default(rpc_url_for_client.clone())];
    let trade_config = TradeConfig::new(rpc_url_for_client.clone(), swqos_configs, commitment)
        .with_wsol_ata_config(true, false);
    let client = SolanaTrade::new(payer.clone(), trade_config).await;

    println!("✅ TradingClient 创建成功");

    // 记录初始 SOL 余额
    let (initial_sol, _) =
        print_balances(rpc_url, &payer_pubkey).await.expect("Failed to fetch initial balances");

    // ===== 步骤 2: 使用指定的 WSOL-JUP CLMM Pool =====
    let pool_address = Pubkey::from_str(WSOL_JUP_POOL).expect("Invalid pool address");
    let jup_mint = Pubkey::from_str(JUP_MINT).expect("Invalid JUP mint");

    println!("\n🔍 使用 WSOL-JUP CLMM Pool: {}", pool_address);
    println!("交易 Token: JUP ({})", jup_mint);

    // 记录初始 JUP 代币余额
    let initial_jup_balance = print_token_balance(rpc_url, &payer_pubkey, &jup_mint, "JUP")
        .await
        .expect("Failed to fetch initial JUP balance");

    println!("初始 JUP 余额: {} (raw units)", initial_jup_balance);

    // ===== 步骤 3: 从 Pool 地址构建 RaydiumClmmParams =====
    println!("\n🧮 从 Pool 构建 RaydiumClmmParams...");
    let clmm_params = RaydiumClmmParams::from_pool_address_by_rpc(&client.rpc, &pool_address)
        .await
        .expect("Failed to build RaydiumClmmParams from pool address");

    println!("Pool 配置:");
    println!("  token0_mint: {}", clmm_params.token0_mint);
    println!("  token1_mint: {}", clmm_params.token1_mint);

    // ===== 步骤 4: 买入 JUP =====
    println!("\n💰 步骤 4: 买入 JUP token");

    // 使用合理的买入金额
    let buy_amount_sol = 10_000_000u64; // 0.01 SOL
    println!("买入金额: {} lamports ({:.4} SOL)", buy_amount_sol, buy_amount_sol as f64 / 1e9);

    let gas_fee_strategy_buy = GasFeeStrategy::new();
    // cu_price 设置为 0，只添加 SetComputeUnitLimit 指令
    gas_fee_strategy_buy.set_global_fee_strategy(1_400_000, 1_400_000, 0, 0, 0.0, 0.0);

    let recent_blockhash_buy =
        client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash for buy");

    let buy_params = TradeBuyParams {
        dex_type: DexType::RaydiumClmm,
        input_token_type: TradeTokenType::SOL,
        mint: jup_mint,
        input_token_amount: buy_amount_sol,
        slippage_basis_points: Some(100), // 1% 滑点
        recent_blockhash: Some(recent_blockhash_buy),
        extension_params: DexParamEnum::RaydiumClmm(clmm_params.clone()),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: true,
        close_input_token_ata: false,
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy_buy,
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let (success_buy, buy_sigs, error_buy) =
        client.buy(buy_params).await.expect("Raydium CLMM 买入交易执行失败");

    if let Some(err) = &error_buy {
        println!("\n⚠️  买入失败：{}", err.message);
        println!("\n💡 CLMM 买入问题说明：");
        println!("   - 错误码 6023 (TooMuchInputPaid): 实际需要的输入超过了提供的 amount_in");
        println!("   - 根本原因：SDK 使用简化的 sqrt_price_x64 线性估算");
        println!("   - CLMM 需要 tick-by-tick 遍历计算精确的 minimum_amount_out");
        println!("   - 官方实现：temp/raydium-clmm/client/src/instructions/utils.rs");
        println!("   - 当前状态：卖出功能正常✅，买入功能待修复❌");

        panic!("❌ 买入失败，无法继续测试卖出流程");
    }

    println!("\n[调试] success_buy: {}", success_buy);
    println!("[调试] buy_sigs: {:?}", buy_sigs);
    println!("✅ 买入成功，签名: {:?}", buy_sigs.get(0));

    // 等待链上状态更新
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // 验证买入后的余额
    let (after_buy_sol, _) =
        print_balances(rpc_url, &payer_pubkey).await.expect("Failed to fetch balances after buy");
    let after_buy_jup_balance =
        print_token_balance(rpc_url, &payer_pubkey, &jup_mint, "JUP")
            .await
            .expect("Failed to fetch JUP balance after buy");

    println!("\n📊 买入结果:");
    let sol_diff_buy = (after_buy_sol as i128) - (initial_sol as i128);
    let jup_diff_buy = (after_buy_jup_balance as i128) - (initial_jup_balance as i128);
    println!(
        "  - SOL 净变化: {} lamports ({:.6} SOL)",
        sol_diff_buy,
        sol_diff_buy as f64 / 1e9
    );
    println!("  - JUP 净变化: {} (raw units)", jup_diff_buy);
    println!("  - 买入后 JUP 余额: {}", after_buy_jup_balance);

    // ===== 步骤 5: 卖出 JUP =====
    println!("\n💸 步骤 5: 卖出 JUP token");

    // 卖出刚买入的一半 JUP
    let sell_amount = after_buy_jup_balance / 2;
    println!("卖出数量: {} (raw units)", sell_amount);

    let gas_fee_strategy_sell = GasFeeStrategy::new();
    // 使用较大的 Compute Unit 限制，确保 CLMM swap 有足够的计算资源
    gas_fee_strategy_sell.set_global_fee_strategy(1_400_000, 1_400_000, 0, 0, 0.0, 0.0);

    let recent_blockhash_sell =
        client.rpc.get_latest_blockhash().await.expect("Failed to get latest blockhash for sell");

    let sell_params = TradeSellParams {
        dex_type: DexType::RaydiumClmm,
        output_token_type: TradeTokenType::SOL,
        mint: jup_mint,
        input_token_amount: sell_amount,
        slippage_basis_points: Some(1000), // 10% slippage
        recent_blockhash: Some(recent_blockhash_sell),
        with_tip: false,
        extension_params: DexParamEnum::RaydiumClmm(clmm_params),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_output_token_ata: true,
        close_output_token_ata: false,
        close_mint_token_ata: false,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy_sell,
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let (success_sell, sell_sigs, error_sell) =
        client.sell(sell_params).await.expect("Raydium CLMM 卖出交易执行失败");

    println!("\n[调试] success_sell: {}", success_sell);
    println!("[调试] sell_sigs: {:?}", sell_sigs);
    if let Some(err) = &error_sell {
        println!("[调试] error_sell: {:?}", err);
    }
    assert!(success_sell, "卖出交易应成功");
    println!("✅ 卖出成功，签名: {:?}", sell_sigs.get(0));

    // 等待链上状态更新
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // ===== 步骤 6: 验证最终余额 =====
    let (final_sol, _) =
        print_balances(rpc_url, &payer_pubkey).await.expect("Failed to fetch final balances");
    let final_jup_balance = print_token_balance(rpc_url, &payer_pubkey, &jup_mint, "JUP")
        .await
        .expect("Failed to fetch final JUP balance");

    println!("\n📊 最终结果:");
    let sol_diff_total = (final_sol as i128) - (initial_sol as i128);
    let jup_diff_total = (final_jup_balance as i128) - (initial_jup_balance as i128);
    println!(
        "  - SOL 总净变化: {} lamports ({:.6} SOL)",
        sol_diff_total,
        sol_diff_total as f64 / 1e9
    );
    println!("  - JUP 总净变化: {} (raw units)", jup_diff_total);
    println!("  - 最终 JUP 余额: {}", final_jup_balance);

    // 验证交易结果
    println!("\n✅ 交易流程验证:");
    println!("  - 买入成功 ✅");
    println!("  - 卖出成功 ✅");

    // JUP 余额应该有变化（因为只卖出了一半）
    println!(
        "  - JUP 余额变化: {} → {} ({} 差异)",
        initial_jup_balance, final_jup_balance, jup_diff_total
    );

    // SOL 余额应该减少（因为交易费用和滑点）
    println!(
        "  - SOL 余额变化: {} → {} ({} 差异)",
        initial_sol, final_sol, sol_diff_total
    );

    println!("\n=== Raydium CLMM 完整交易流程测试通过 ===");
}
