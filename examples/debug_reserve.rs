use sol_trade_sdk::SolanaRpcClient;
use sol_trade_sdk::instruction::utils::raydium_amm_v4::get_pool_by_address;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let rpc = SolanaRpcClient::new("http://127.0.0.1:8899".to_string());
    let pool_address = Pubkey::from_str("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2")?;
    
    let amm_info = get_pool_by_address(&rpc, &pool_address).await?;
    
    println!("Pool vaults:");
    println!("  token_coin: {}", amm_info.token_coin);
    println!("  token_pc: {}", amm_info.token_pc);
    
    let coin_balance = rpc.get_token_account_balance(&amm_info.token_coin).await?;
    let pc_balance = rpc.get_token_account_balance(&amm_info.token_pc).await?;
    
    println!("\nReserves:");
    println!("  coin_reserve (WSOL): {}", coin_balance.amount);
    println!("  pc_reserve (USDC): {}", pc_balance.amount);
    
    let coin_amt = coin_balance.amount.parse::<u64>().unwrap_or(0);
    let pc_amt = pc_balance.amount.parse::<u64>().unwrap_or(0);
    
    println!("\nFormatted:");
    println!("  coin_reserve: {} WSOL (lamports)", coin_amt);
    println!("  pc_reserve: {} USDC (smallest unit)", pc_amt);
    println!("  coin_reserve: {:.9} WSOL", coin_amt as f64 / 1e9);
    println!("  pc_reserve: {:.6} USDC", pc_amt as f64 / 1e6);
    
    Ok(())
}
