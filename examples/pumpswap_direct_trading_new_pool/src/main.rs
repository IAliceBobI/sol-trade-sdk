use sol_trade_sdk::{
    common::{fast_fn::get_associated_token_address_with_program_id_fast_use_seed, TradeConfig},
    swqos::SwqosConfig,
    trading::{
        core::params::{DexParamEnum, PumpSwapParams},
        factory::DexType,
    },
    SolanaTrade, TradeTokenType,
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;
use solana_sdk::{pubkey::Pubkey, signer::Signer};
use std::fs;
use std::{str::FromStr, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("Testing PumpSwap trading...");

    let client = create_solana_trade_client().await?;
    let slippage_basis_points = Some(100);
    let recent_blockhash = client.rpc.get_latest_blockhash().await?;
    let pool = Pubkey::from_str("2xHRmdXSKURh8CkMERbNhYCiQGHGsjLWMgckEP9bmLKK").unwrap();
    let mint_pubkey = Pubkey::from_str("Ew8KqgSitYucieR5KnSAL2SUFspcwA8AgSuZ5xWspump").unwrap();

    let gas_fee_strategy = sol_trade_sdk::common::GasFeeStrategy::new();
    gas_fee_strategy.set_global_fee_strategy(150000, 150000, 500000, 500000, 0.001, 0.001);

    // Buy tokens
    println!("Buying tokens from PumpSwap...");
    let buy_sol_amount = 100_000;
    let buy_params = sol_trade_sdk::TradeBuyParams {
        dex_type: DexType::PumpSwap,
        input_token_type: TradeTokenType::SOL,
        mint: mint_pubkey,
        input_token_amount: buy_sol_amount,
        slippage_basis_points: slippage_basis_points,
        recent_blockhash: Some(recent_blockhash),
        extension_params: DexParamEnum::PumpSwap(
            PumpSwapParams::from_pool_address_by_rpc(&client.rpc, &pool).await?,
        ),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_input_token_ata: true,
        close_input_token_ata: false, // 修改：不自动关闭 WSOL ATA，避免连续交易时状态不一致
        create_mint_ata: true,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy.clone(),
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let (success, signatures, trade_error) = client.buy(buy_params).await?;
    if !success {
        if let Some(error) = trade_error {
            return Err(anyhow::anyhow!("Buy failed: {:?}", error));
        }
        return Err(anyhow::anyhow!("Buy failed: Unknown error"));
    }
    println!("Buy successful! Signatures: {:?}", signatures);

    // Sell tokens
    println!("Selling tokens from PumpSwap...");

    let rpc = client.rpc.clone();
    let payer = client.payer.pubkey();
    let program_id = sol_trade_sdk::constants::TOKEN_PROGRAM_2022;
    let account = get_associated_token_address_with_program_id_fast_use_seed(
        &payer,
        &mint_pubkey,
        &program_id,
        client.use_seed_optimize,
    );
    let balance = rpc.get_token_account_balance(&account).await?;
    let amount_token = balance.amount.parse::<u64>().unwrap();
    let sell_params = sol_trade_sdk::TradeSellParams {
        dex_type: DexType::PumpSwap,
        output_token_type: TradeTokenType::SOL,
        mint: mint_pubkey,
        input_token_amount: amount_token,
        slippage_basis_points: slippage_basis_points,
        recent_blockhash: Some(recent_blockhash),
        with_tip: false,
        extension_params: DexParamEnum::PumpSwap(
            PumpSwapParams::from_pool_address_by_rpc(&client.rpc, &pool).await?,
        ),
        address_lookup_table_account: None,
        wait_transaction_confirmed: true,
        create_output_token_ata: true,
        close_output_token_ata: false, // 修改：不自动关闭 WSOL ATA，保持账户复用
        close_mint_token_ata: false,
        durable_nonce: None,
        fixed_output_token_amount: None,
        gas_fee_strategy: gas_fee_strategy,
        simulate: false,
        on_transaction_signed: None,
        callback_execution_mode: None,
    };

    let (success, signatures, trade_error) = client.sell(sell_params).await?;
    if !success {
        if let Some(error) = trade_error {
            return Err(anyhow::anyhow!("Sell failed: {:?}", error));
        }
        return Err(anyhow::anyhow!("Sell failed: Unknown error"));
    }
    println!("Sell successful! Signatures: {:?}", signatures);

    Ok(())
}

/// Create and initialize SolanaTrade client
async fn create_solana_trade_client() -> Result<SolanaTrade, anyhow::Error> {
    println!("🚀 Initializing SolanaTrade client...");

    // Read payer keypair from ~/.config/solana/id.json
    let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
    let keypair_path = format!("{}/.config/solana/id.json", home_dir);
    println!("Loading keypair from: {}", keypair_path);

    let keypair_data = fs::read_to_string(&keypair_path)
        .expect(&format!("Failed to read keypair file: {}", keypair_path));

    // Parse JSON and extract private key array
    let private_key: Vec<u8> =
        serde_json::from_str(&keypair_data).expect("Failed to parse keypair JSON");

    // Use the first 32 bytes as the secret key
    let secret_key: [u8; 32] = private_key[0..32].try_into().expect("Invalid key length");
    let payer = Keypair::new_from_array(secret_key);

    let rpc_url = "http://127.0.0.1:8899".to_string();
    let commitment = CommitmentConfig::confirmed();

    // Create RPC client for airdrop (before SolanaTrade initialization)
    let rpc_client = sol_trade_sdk::common::SolanaRpcClient::new_with_commitment(
        rpc_url.clone(),
        commitment.clone(),
    );

    let sol_balance = rpc_client.get_balance(&payer.pubkey()).await?;
    let sol_balance_sol = sol_balance as f64 / 1e9;
    println!("💰 Payer SOL balance: {:.9} SOL", sol_balance_sol);

    if sol_balance_sol < 10.0 {
        // Airdrop 10 SOL to the account
        println!("💸 Airdropping 10 SOL to account...");
        let airdrop_amount = 10_000_000_000; // 10 SOL in lamports
        let _ = rpc_client.request_airdrop(&payer.pubkey(), airdrop_amount).await?;
        println!("✅ Airdrop successful!");
    }

    // Now create SolanaTrade client (after airdrop, before ATA creation)
    let swqos_configs: Vec<SwqosConfig> = vec![SwqosConfig::Default(rpc_url.clone())];
    let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment).with_wsol_ata_config(
        true,  // create_wsol_ata_on_startup: 禁用启动时自动创建
        false, // use_seed_optimize: 启用 Seed 优化
    );
    let solana_trade = SolanaTrade::new(Arc::new(payer), trade_config).await;

    println!("✅ SolanaTrade client initialized successfully!");
    Ok(solana_trade)
}

// async fn create_solana_trade_client() -> AnyResult<SolanaTrade> {
//     println!("🚀 Initializing SolanaTrade client...");
//     let payer = Keypair::from_base58_string("use_your_payer_keypair_here");
//     let rpc_url = "https://api.mainnet-beta.solana.com".to_string();
//     let commitment = CommitmentConfig::confirmed();
//     let swqos_configs: Vec<SwqosConfig> = vec![SwqosConfig::Default(rpc_url.clone())];
//     let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment);
//     let solana_trade = SolanaTrade::new(Arc::new(payer), trade_config).await;
//     println!("✅ SolanaTrade client initialized successfully!");
//     Ok(solana_trade)
// }
