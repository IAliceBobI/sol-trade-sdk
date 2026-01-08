use anyhow::Result;
use sol_trade_sdk::instruction::utils::raydium_amm_v4::{
    get_pool_by_mint,
    get_pool_by_mint_force,
    list_pools_by_mint,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    // 示例：使用本地 surfpool / RPC 代理
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 以 WSOL mint 为例
    let wsol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")?;

    println!("=== Raydium AMM V4: get_pool_by_mint (WSOL) ===");
    let (pool_address, amm_info) = get_pool_by_mint(&rpc, &wsol_mint).await?;
    println!("最优 Pool 地址: {}", pool_address);
    println!("coin_mint: {}", amm_info.coin_mint);
    println!("pc_mint: {}", amm_info.pc_mint);
    println!("lp_amount: {}", amm_info.lp_amount);

    println!("\n=== Raydium AMM V4: list_pools_by_mint (WSOL) ===");
    let all_pools = list_pools_by_mint(&rpc, &wsol_mint).await?;
    println!("共找到 {} 个包含 WSOL 的池", all_pools.len());
    for (addr, amm) in all_pools.iter().take(5) {
        println!(
            "Pool: {} (coin_mint={}, pc_mint={}, lp_amount={})",
            addr, amm.coin_mint, amm.pc_mint, amm.lp_amount
        );
    }

    println!("\n=== Raydium AMM V4: get_pool_by_mint_force (WSOL) ===");
    let (pool_address2, _) = get_pool_by_mint_force(&rpc, &wsol_mint).await?;
    println!("强制刷新后最优 Pool 地址: {}", pool_address2);

    Ok(())
}
