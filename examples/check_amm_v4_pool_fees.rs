use sol_trade_sdk::SolanaRpcClient;
use sol_trade_sdk::instruction::utils::raydium_amm_v4::get_pool_by_address;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let rpc = SolanaRpcClient::new("http://127.0.0.1:8899".to_string());
    let pool_address = Pubkey::from_str("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2")?;
    
    let amm_info = get_pool_by_address(&rpc, &pool_address).await?;
    
    println!("Pool fees:");
    println!("  trade_fee_numerator: {}", amm_info.fees.trade_fee_numerator);
    println!("  trade_fee_denominator: {}", amm_info.fees.trade_fee_denominator);
    println!("  swap_fee_numerator: {}", amm_info.fees.swap_fee_numerator);
    println!("  swap_fee_denominator: {}", amm_info.fees.swap_fee_denominator);
    
    let trade_fee_rate = amm_info.fees.trade_fee_numerator as f64 / amm_info.fees.trade_fee_denominator as f64 * 100.0;
    let swap_fee_rate = amm_info.fees.swap_fee_numerator as f64 / amm_info.fees.swap_fee_denominator as f64 * 100.0;
    
    println!("\nFee rates:");
    println!("  trade_fee: {:.4}%", trade_fee_rate);
    println!("  swap_fee: {:.4}%", swap_fee_rate);
    println!("  total: {:.4}%", trade_fee_rate + swap_fee_rate);
    
    println!("\nHardcoded constants:");
    println!("  TRADE_FEE_NUMERATOR: 25");
    println!("  TRADE_FEE_DENOMINATOR: 10000");
    println!("  SWAP_FEE_NUMERATOR: 25");
    println!("  SWAP_FEE_DENOMINATOR: 10000");
    
    Ok(())
}
