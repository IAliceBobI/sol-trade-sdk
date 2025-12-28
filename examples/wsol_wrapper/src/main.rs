use sol_trade_sdk::{common::TradeConfig, swqos::SwqosConfig, SolanaTrade};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;
use std::{fs, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 WSOL Wrapper Example");
    println!("This example demonstrates:");
    println!("1. Wrapping SOL to WSOL");
    println!("2. Partial unwrapping WSOL back to SOL using seed account");
    println!("3. Closing WSOL account and unwrapping remaining balance");

    // Initialize SolanaTrade client
    let solana_trade = create_solana_trade_client().await?;

    // Example 1: Wrap SOL to WSOL
    println!("\n📦 Example 1: Wrapping SOL to WSOL");
    let wrap_amount = 1_000_000; // 0.001 SOL in lamports
    println!("Wrapping {} lamports (0.001 SOL) to WSOL...", wrap_amount);

    match solana_trade.wrap_sol_to_wsol(wrap_amount).await {
        Ok(signature) => {
            println!("✅ Successfully wrapped SOL to WSOL!");
            println!("Transaction signature: {}", signature);
            println!("Explorer: https://solscan.io/tx/{}", signature);
        }
        Err(e) => {
            println!("❌ Failed to wrap SOL to WSOL: {}", e);
            return Ok(());
        }
    }

    // Wait a moment before partial unwrapping
    println!("\n⏳ Waiting 3 seconds before partial unwrapping...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Example 2: Unwrap half of the WSOL back to SOL using seed account
    println!("\n🔄 Example 2: Unwrapping half of WSOL back to SOL using seed account");
    let unwrap_amount = wrap_amount / 2; // Half of the wrapped amount
    println!("Unwrapping {} lamports (0.0005 SOL) back to SOL using seed account...", unwrap_amount);

    match solana_trade.wrap_wsol_to_sol(unwrap_amount).await {
        Ok(signature) => {
            println!("✅ Successfully unwrapped half of WSOL back to SOL using seed account!");
            println!("Transaction signature: {}", signature);
            println!("Explorer: https://solscan.io/tx/{}", signature);
        }
        Err(e) => {
            println!("❌ Failed to unwrap WSOL to SOL: {}", e);
        }
    }

    // Wait a moment before final unwrapping
    println!("\n⏳ Waiting 3 seconds before final unwrapping...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Example 3: Close WSOL account and unwrap all remaining balance
    println!("\n🔒 Example 3: Closing WSOL account and unwrapping remaining balance");
    println!("Closing WSOL account and unwrapping all remaining balance to SOL...");

    match solana_trade.close_wsol().await {
        Ok(signature) => {
            println!("✅ Successfully closed WSOL account and unwrapped remaining balance!");
            println!("Transaction signature: {}", signature);
            println!("Explorer: https://solscan.io/tx/{}", signature);
        }
        Err(e) => {
            println!("❌ Failed to close WSOL account: {}", e);
        }
    }

    println!("\n🎉 WSOL Wrapper example completed!");
    Ok(())
}

/// Create and initialize SolanaTrade client
async fn create_solana_trade_client() -> Result<SolanaTrade, Box<dyn std::error::Error>> {
    println!("🚀 Initializing SolanaTrade client...");
    
    // Read payer keypair from ~/.config/solana/id.json
    let home_dir = std::env::var("HOME").expect("HOME environment variable not set");
    let keypair_path = format!("{}/.config/solana/id.json", home_dir);
    println!("Loading keypair from: {}", keypair_path);
    
    let keypair_data = fs::read_to_string(&keypair_path)
        .expect(&format!("Failed to read keypair file: {}", keypair_path));
    
    // Parse JSON and extract private key array
    let private_key: Vec<u8> = serde_json::from_str(&keypair_data)
        .expect("Failed to parse keypair JSON");
    
    // Use the first 32 bytes as the secret key
    let secret_key: [u8; 32] = private_key[0..32].try_into().expect("Invalid key length");
    let payer = Keypair::new_from_array(secret_key);
    
    let rpc_url = "http://127.0.0.1:8899".to_string();
    let commitment = CommitmentConfig::confirmed();
    let swqos_configs: Vec<SwqosConfig> = vec![SwqosConfig::Default(rpc_url.clone())];
    let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment);
    let solana_trade = SolanaTrade::new(Arc::new(payer), trade_config).await;
    println!("✅ SolanaTrade client initialized successfully!");
    Ok(solana_trade)
}
