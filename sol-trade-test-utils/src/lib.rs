//! Solana 交易测试工具库
//!
//! 提供便捷的测试辅助函数，包括：
//! - 空投 SOL
//! - 设置 Token 余额
//! - 确保账户余额
//! - 添加流动性
//! - 列出和查询 Pool
//!
//! # 示例
//!
//! ```ignore
//! use sol_trade_test_utils::{ensure_sol_balance, ensure_token_balance};
//!
//! #[tokio::test]
//! async fn my_test() {
//!     let rpc_url = "http://127.0.0.1:8899";
//!     let rpc = Arc::new(SolanaRpcClient::new(rpc_url.to_string()));
//!     let payer = Arc::new(Keypair::new());
//!
//!     // 确保 SOL 余额
//!     ensure_sol_balance(&rpc, rpc_url, &payer.pubkey(), 10).await?;
//!
//!     // 确保 Token 余额
//!     ensure_token_balance(&rpc, rpc_url, &payer, &mint, "1000").await?;
//! }
//! ```

pub mod airdrop;
pub mod dex_verification;
pub mod ensure;
pub mod pool_list;
pub mod proxy_http;
pub mod test_params;
pub mod token;

// 重新导出常用类型
pub use airdrop::airdrop_and_wait;
pub use ensure::{
    ensure_cpmm_liquidity, ensure_pipe_pool_liquidity_via_swap, ensure_sol_balance,
    ensure_token_balance, ensure_usdc_prts_pool_usdc_liquidity,
};
pub use token::{get_mint_info, mint_token_to, set_token_balance, transfer_token_to, MintInfo};

// 重新导出 DEX 测试参数构建器
pub use test_params::{
    bonk_mint, bonk_wsol_pool, create_test_gas_fee_strategy, pipe_mint, pipe_wsol_pool, prts_mint,
    pump_mint, pump_wsol_pool, solett_mint, solett_wsol_pool, usdc_mint, usdc_prts_pool, usdt_mint,
    usdt_wsol_pool, wsol_mint, BonkWsolBuyParamsBuilder, BonkWsolSellParamsBuilder,
    CpmmLiquidityBuilder, PipeWsolBuyParamsBuilder, PipeWsolLiquidityBuilder,
    PipeWsolSellParamsBuilder, PumpWsolBuyParamsBuilder, PumpWsolSellParamsBuilder,
    SolettWsolClmmBuyParamsBuilder, SolettWsolClmmSellParamsBuilder, UsdcPrtsBuyParamsBuilder,
    UsdcPrtsSellParamsBuilder, UsdtWsolClmmBuyParamsBuilder, UsdtWsolClmmSellParamsBuilder,
    BONK_MINT, BONK_WSOL_POOL, PIPE_MINT, PIPE_WSOL_POOL, PRTS_MINT, PUMP_MINT, PUMP_WSOL_POOL,
    SOLETT_MINT, SOLETT_WSOL_POOL, USDC_MINT, USDC_PRTS_POOL, USDT_MINT, USDT_WSOL_POOL, WSOL_MINT,
};

// 重新导出 Pool 列出工具
pub use pool_list::{
    // AMM V4
    list_and_classify_amm_v4_pools,
    // CLMM
    list_and_classify_clmm_pools,
    list_and_classify_pools,
    // PumpSwap
    list_and_classify_pumpswap_pools,
    list_usdc_amm_v4_pools,
    list_usdc_pools,
    list_wsol_amm_v4_pools,
    list_wsol_clmm_pools,
    list_wsol_pumpswap_pools,
    print_amm_v4_pool_classification,
    print_clmm_pool_classification,
    print_pool_classification,
    print_pumpswap_pool_classification,
    AmmV4PoolClassification,
    AmmV4PoolInfo,
    ClmmPoolClassification,
    ClmmPoolInfo,
    PoolClassification,
    PoolInfo,
    PumpSwapPoolClassification,
    PumpSwapPoolInfo,
};

// 导入常用类型
use solana_sdk::signature::Keypair;

/// 固定的测试模拟账户（已有 10 SOL 余额）
/// 注意：这个账户是预先创建并空投过的，不需要在测试中重复空投
///
/// 地址: 8be6dbPmZH1URHXyFTbY876QuVunrD8wTZhHGXjEdrvj
pub const SIMULATION_TEST_KEYPAIR: &str =
    "2cUyNj1YLguzrU89Xu2AcnGZD9qcNjEJo5QTg4tBs9foVXzLF3fBdBXiUdMmb867T9EK8FfKUQCH8FR5oD3bYVew";

/// 获取固定的模拟测试 Keypair
pub fn get_simulation_test_keypair() -> Keypair {
    Keypair::from_base58_string(SIMULATION_TEST_KEYPAIR)
}
