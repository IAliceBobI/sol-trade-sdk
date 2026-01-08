//! Raydium AMM V4 (Raydium Liquidity Pool V4) Pool 查找集成测试
//!
//! Raydium AMM V4 是 Raydium 的传统自动做市商（AMM）协议，使用恒定乘积公式（x * y = k）进行流动性提供和交易。
//!
//! ## 程序信息
//! - **程序名称**: Raydium Liquidity Pool V4
//! - **程序地址**: `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8`
//! - **特性**: 集成 Serum 订单簿，支持限价单和市价单
//!
//! ## 费用结构
//! - **交易费**: 0.25% (25/10000)
//! - **Swap 费**: 0.25% (25/10000)
//! - **总费用**: 0.5%
//!
//! ## 已知 Pool
//! - **WSOL-USDC Pool**: `58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2`
//!   - Token0: WSOL (So11111111111111111111111111111111111111112)
//!   - Token1: USDC (EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
//!
//! ## 测试方法
//! - get_pool_by_address(rpc, amm) - 获取 AMM 信息（带缓存）
//!
//! 运行测试:
//!     cargo test --test raydium_amm_v4_pool_tests -- --nocapture
//!
//! 注意：使用 surfpool (localhost:8899) 进行测试

use sol_trade_sdk::instruction::utils::raydium_amm_v4::{
    get_pool_by_address,
    get_pool_by_address_force,
    get_pool_by_mint,
    get_pool_by_mint_force,
    list_pools_by_mint,
    clear_pool_cache,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// 用于串行测试
use serial_test::serial;

/// 已知的 Raydium AMM V4 pool 地址
/// WSOL-USDC pool on Raydium AMM V4
/// - Pool Address: 58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2
/// - Token0: WSOL (So11111111111111111111111111111111111111112)
/// - Token1: USDC (EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
const SOL_USDC_AMM: &str = "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2";

/// 测试：获取 AMM 信息并验证字段
#[tokio::test]
async fn test_fetch_amm_info() {
    println!("=== 测试：获取 AMM 信息并验证字段 ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    println!("获取 AMM 信息: {}", amm_address);
    let result = get_pool_by_address(&rpc, &amm_address).await;

    assert!(result.is_ok(), "Failed to fetch AMM info: {:?}", result.err());

    let amm_info = result.unwrap();

    // 打印所有字段用于调试
    println!("\n=== 提取的字段值 ===");
    println!("status: {}", amm_info.status);
    println!("nonce: {}", amm_info.nonce);
    println!("order_num: {}", amm_info.order_num);
    println!("depth: {}", amm_info.depth);
    println!("coin_decimals: {}", amm_info.coin_decimals);
    println!("pc_decimals: {}", amm_info.pc_decimals);
    println!("state: {}", amm_info.state);
    println!("reset_flag: {}", amm_info.reset_flag);
    println!("min_size: {}", amm_info.min_size);
    println!("vol_max_cut_ratio: {}", amm_info.vol_max_cut_ratio);
    println!("amount_wave: {}", amm_info.amount_wave);
    println!("coin_lot_size: {}", amm_info.coin_lot_size);
    println!("pc_lot_size: {}", amm_info.pc_lot_size);
    println!("min_price_multiplier: {}", amm_info.min_price_multiplier);
    println!("max_price_multiplier: {}", amm_info.max_price_multiplier);
    println!("sys_decimal_value: {}", amm_info.sys_decimal_value);
    println!("min_separate_numerator: {}", amm_info.fees.min_separate_numerator);
    println!("min_separate_denominator: {}", amm_info.fees.min_separate_denominator);
    println!("trade_fee_numerator: {}", amm_info.fees.trade_fee_numerator);
    println!("trade_fee_denominator: {}", amm_info.fees.trade_fee_denominator);
    println!("pnl_numerator: {}", amm_info.fees.pnl_numerator);
    println!("pnl_denominator: {}", amm_info.fees.pnl_denominator);
    println!("swap_fee_numerator: {}", amm_info.fees.swap_fee_numerator);
    println!("swap_fee_denominator: {}", amm_info.fees.swap_fee_denominator);
    println!("token_coin: {}", amm_info.token_coin);
    println!("token_pc: {}", amm_info.token_pc);
    println!("coin_mint: {}", amm_info.coin_mint);
    println!("pc_mint: {}", amm_info.pc_mint);
    println!("lp_mint: {}", amm_info.lp_mint);
    println!("open_orders: {}", amm_info.open_orders);
    println!("market: {}", amm_info.market);
    println!("serum_dex: {}", amm_info.serum_dex);
    println!("target_orders: {}", amm_info.target_orders);
    println!("withdraw_queue: {}", amm_info.withdraw_queue);
    println!("token_temp_lp: {}", amm_info.token_temp_lp);
    println!("amm_owner: {}", amm_info.amm_owner);
    println!("lp_amount: {}", amm_info.lp_amount);

    // 对比固定字段（根据 JSON 示例）
    assert_eq!(amm_info.status, 6, "status 不匹配");
    assert_eq!(amm_info.nonce, 254, "nonce 不匹配");
    assert_eq!(amm_info.order_num, 7, "order_num 不匹配");
    assert_eq!(amm_info.depth, 3, "depth 不匹配");
    assert_eq!(amm_info.coin_decimals, 9, "coin_decimals 不匹配");
    assert_eq!(amm_info.pc_decimals, 6, "pc_decimals 不匹配");
    assert_eq!(amm_info.state, 2, "state 不匹配");
    assert_eq!(amm_info.reset_flag, 0, "reset_flag 不匹配");
    assert_eq!(amm_info.min_size, 1000000, "min_size 不匹配");
    assert_eq!(amm_info.vol_max_cut_ratio, 500, "vol_max_cut_ratio 不匹配");
    assert_eq!(amm_info.amount_wave, 0, "amount_wave 不匹配");
    assert_eq!(amm_info.coin_lot_size, 1000000, "coin_lot_size 不匹配");
    assert_eq!(amm_info.pc_lot_size, 1000000, "pc_lot_size 不匹配");
    assert_eq!(amm_info.min_price_multiplier, 1, "min_price_multiplier 不匹配");
    assert_eq!(amm_info.max_price_multiplier, 1000000000, "max_price_multiplier 不匹配");
    assert_eq!(amm_info.sys_decimal_value, 1000000000, "sys_decimal_value 不匹配");
    assert_eq!(amm_info.fees.min_separate_numerator, 5, "min_separate_numerator 不匹配");
    assert_eq!(amm_info.fees.min_separate_denominator, 10000, "min_separate_denominator 不匹配");
    assert_eq!(amm_info.fees.trade_fee_numerator, 25, "trade_fee_numerator 不匹配");
    assert_eq!(amm_info.fees.trade_fee_denominator, 10000, "trade_fee_denominator 不匹配");
    assert_eq!(amm_info.fees.pnl_numerator, 12, "pnl_numerator 不匹配");
    assert_eq!(amm_info.fees.pnl_denominator, 100, "pnl_denominator 不匹配");
    assert_eq!(amm_info.fees.swap_fee_numerator, 25, "swap_fee_numerator 不匹配");
    assert_eq!(amm_info.fees.swap_fee_denominator, 10000, "swap_fee_denominator 不匹配");

    // 对比地址字段
    assert_eq!(
        amm_info.token_coin.to_string(),
        "DQyrAcCrDXQ7NeoqGgDCZwBvWDcYmFCjSb9JtteuvPpz",
        "token_coin 不匹配"
    );
    assert_eq!(
        amm_info.token_pc.to_string(),
        "HLmqeL62xR1QoZ1HKKbXRrdN1p3phKpxRMb2VVopvBBz",
        "token_pc 不匹配"
    );
    assert_eq!(
        amm_info.coin_mint.to_string(),
        "So11111111111111111111111111111111111111112",
        "coin_mint 不匹配"
    );
    assert_eq!(
        amm_info.pc_mint.to_string(),
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "pc_mint 不匹配"
    );
    assert_eq!(
        amm_info.lp_mint.to_string(),
        "8HoQnePLqPj4M7PUDzfw8e3Ymdwgc7NLGnaTUapubyvu",
        "lp_mint 不匹配"
    );
    assert_eq!(
        amm_info.open_orders.to_string(),
        "HmiHHzq4Fym9e1D4qzLS6LDDM3tNsCTBPDWHTLZ763jY",
        "open_orders 不匹配"
    );
    assert_eq!(
        amm_info.market.to_string(),
        "8BnEgHoWFysVcuFFX7QztDmzuH8r5ZFvyP3sYwn1XTh6",
        "market 不匹配"
    );
    assert_eq!(
        amm_info.serum_dex.to_string(),
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX",
        "serum_dex 不匹配"
    );
    assert_eq!(
        amm_info.target_orders.to_string(),
        "CZza3Ej4Mc58MnxWA385itCC9jCo3L1D7zc3LKy1bZMR",
        "target_orders 不匹配"
    );
    assert_eq!(
        amm_info.withdraw_queue.to_string(),
        "11111111111111111111111111111111",
        "withdraw_queue 不匹配"
    );
    assert_eq!(
        amm_info.token_temp_lp.to_string(),
        "11111111111111111111111111111111",
        "token_temp_lp 不匹配"
    );
    assert_eq!(
        amm_info.amm_owner.to_string(),
        "GThUX1Atko4tqhN2NaiTazWSeFWMuiUvfFnyJyUghFMJ",
        "amm_owner 不匹配"
    );

    // 忽略变动字段（这些字段会随着交易而变化）
    // - out_put 中的所有字段
    // - lp_amount
    // - padding

    println!("\n=== 所有固定字段验证通过 ===");
}

/// 测试：缓存功能
#[tokio::test]
async fn test_get_pool_by_address_cache() {
    println!("=== 测试：缓存功能 ===");

    let amm_address = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 清除缓存，确保测试从干净状态开始
    clear_pool_cache();
    println!("缓存已清除");

    // 第一次调用，应该从 RPC 查询
    println!("第一次调用 get_pool_by_address");
    let pool1 = get_pool_by_address(&rpc, &amm_address).await;
    assert!(pool1.is_ok(), "Failed to get pool by address: {:?}", pool1.err());
    let pool1 = pool1.unwrap();
    println!("第一次调用成功，status: {}", pool1.status);

    // 第二次调用，应该从缓存返回
    println!("第二次调用 get_pool_by_address（应该使用缓存）");
    let pool2 = get_pool_by_address(&rpc, &amm_address).await;
    assert!(pool2.is_ok(), "Failed to get pool by address: {:?}", pool2.err());
    let pool2 = pool2.unwrap();
    println!("第二次调用成功，status: {}", pool2.status);

    // 验证两次调用返回的数据相同
    assert_eq!(pool1.status, pool2.status, "缓存数据不一致");
    assert_eq!(pool1.nonce, pool2.nonce, "缓存数据不一致");
    assert_eq!(pool1.coin_mint, pool2.coin_mint, "缓存数据不一致");
    assert_eq!(pool1.pc_mint, pool2.pc_mint, "缓存数据不一致");
    println!("缓存验证通过，两次调用返回的数据相同");

    // 强制刷新，应该从 RPC 重新查询
    println!("调用 get_pool_by_address_force 强制刷新");
    let pool3 = get_pool_by_address_force(&rpc, &amm_address).await;
    assert!(pool3.is_ok(), "Failed to force refresh pool: {:?}", pool3.err());
    let pool3 = pool3.unwrap();
    println!("强制刷新成功，status: {}", pool3.status);

    // 验证强制刷新后的数据与之前的数据相同（因为数据没有变化）
    assert_eq!(pool1.status, pool3.status, "强制刷新后数据不一致");
    assert_eq!(pool1.nonce, pool3.nonce, "强制刷新后数据不一致");
    println!("强制刷新验证通过");

    // 清除缓存
    println!("调用 clear_pool_cache");
    clear_pool_cache();
    println!("缓存已清除");

    // 再次调用，应该从 RPC 查询
    println!("清除缓存后再次调用 get_pool_by_address");
    let pool4 = get_pool_by_address(&rpc, &amm_address).await;
    assert!(pool4.is_ok(), "Failed to get pool by address: {:?}", pool4.err());
    let pool4 = pool4.unwrap();
    println!("清除缓存后调用成功，status: {}", pool4.status);

    // 验证数据一致
    assert_eq!(pool1.status, pool4.status, "清除缓存后数据不一致");
    println!("清除缓存验证通过");

    println!("\n=== 所有缓存功能测试通过 ===");
}

/// 测试：基于指定 mint (WSOL) 获取 Pool
/// 注意：此测试会清除缓存，必须与其他测试串行运行
#[tokio::test]
#[serial]
async fn test_get_pool_by_mint_wsol() {
    println!("=== 测试：get_pool_by_mint (WSOL) ===");

    let wsol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")
        .expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 清理缓存，确保从干净状态开始
    clear_pool_cache();

    // 第一次查询：应从链上扫描并选择最优池
    let result1 = get_pool_by_mint(&rpc, &wsol_mint).await;
    assert!(result1.is_ok(), "get_pool_by_mint failed: {:?}", result1.err());
    let (pool_address_1, amm_info_1) = result1.unwrap();
    println!("第一次查询到的 Pool: {}", pool_address_1);
    println!("coin_mint: {}", amm_info_1.coin_mint);
    println!("pc_mint: {}", amm_info_1.pc_mint);

    // 验证返回的池确实包含 WSOL
    assert!(
        amm_info_1.coin_mint.to_string() == "So11111111111111111111111111111111111111112"
            || amm_info_1.pc_mint.to_string() == "So11111111111111111111111111111111111111112",
        "返回的 Pool 不包含 WSOL",
    );

    // 第二次查询：应命中缓存并返回相同结果
    let result2 = get_pool_by_mint(&rpc, &wsol_mint).await.unwrap();
    let (pool_address_2, amm_info_2) = result2;
    assert_eq!(pool_address_1, pool_address_2, "缓存中的 pool_address 不一致");
    assert_eq!(amm_info_1.lp_amount, amm_info_2.lp_amount, "缓存中的 AmmInfo 不一致");

    // 强制刷新：删除缓存后重新查询
    let result3 = get_pool_by_mint_force(&rpc, &wsol_mint).await.unwrap();
    let (pool_address_3, amm_info_3) = result3;
    println!("强制刷新后的 Pool: {}", pool_address_3);

    // 即便强制刷新，主池一般仍然相同（除非链上发生结构性变化）
    assert_eq!(pool_address_2, pool_address_3, "强制刷新后 pool_address 发生变化");
    assert_eq!(amm_info_2.coin_mint, amm_info_3.coin_mint, "强制刷新后 coin_mint 不一致");
    assert_eq!(amm_info_2.pc_mint, amm_info_3.pc_mint, "强制刷新后 pc_mint 不一致");
}

/// 测试：基于指定 mint (4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R) 获取 Pool
#[tokio::test]
#[ignore]
async fn test_get_pool_by_mint_custom() {
    // raydium 在 ammv4 上找不到。
    println!("=== 测试：get_pool_by_mint (4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R) ===");

    let target_mint = Pubkey::from_str("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R")
        .expect("Invalid target mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    // 清理缓存，确保从干净状态开始
    clear_pool_cache();

    // 第一次查询：应从链上扫描并选择最优池
    let result1 = get_pool_by_mint(&rpc, &target_mint).await;
    assert!(result1.is_ok(), "get_pool_by_mint failed: {:?}", result1.err());
    let (pool_address_1, amm_info_1) = result1.unwrap();
    println!("第一次查询到的 Pool: {}", pool_address_1);
    println!("coin_mint: {}", amm_info_1.coin_mint);
    println!("pc_mint: {}", amm_info_1.pc_mint);
    println!("lp_amount: {}", amm_info_1.lp_amount);

    // 验证返回的池确实包含目标 mint
    let target_mint_str = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R";
    assert!(
        amm_info_1.coin_mint.to_string() == target_mint_str
            || amm_info_1.pc_mint.to_string() == target_mint_str,
        "返回的 Pool 不包含目标 mint",
    );

    // 第二次查询：应命中缓存并返回相同结果
    let result2 = get_pool_by_mint(&rpc, &target_mint).await.unwrap();
    let (pool_address_2, amm_info_2) = result2;
    assert_eq!(pool_address_1, pool_address_2, "缓存中的 pool_address 不一致");
    assert_eq!(amm_info_1.lp_amount, amm_info_2.lp_amount, "缓存中的 AmmInfo 不一致");

    println!("\n缓存验证通过，pool_address 一致: {}", pool_address_2);
}

/// 测试：列出所有包含 WSOL 的 Raydium AMM V4 Pool
#[tokio::test]
async fn test_list_pools_by_mint_wsol() {
    println!("=== 测试：list_pools_by_mint (WSOL) ===");

    let wsol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")
        .expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let pools = list_pools_by_mint(&rpc, &wsol_mint, false).await;
    assert!(pools.is_ok(), "list_pools_by_mint failed: {:?}", pools.err());
    let pools = pools.unwrap();

    assert!(!pools.is_empty(), "WSOL 相关的 Pool 列表不应为空");

    // 所有池都应该包含 WSOL
    for (addr, amm) in pools.iter() {
        println!("WSOL Pool: {} (coin_mint={}, pc_mint={}, status={})", addr, amm.coin_mint, amm.pc_mint, amm.status);
        assert!(
            amm.coin_mint.to_string() == "So11111111111111111111111111111111111111112"
                || amm.pc_mint.to_string() == "So11111111111111111111111111111111111111112",
            "Pool {} 不包含 WSOL",
            addr,
        );
    }

    // 确认已知的 WSOL-USDC AMM 池在列表中（若本地 RPC 同步了主网数据）
    let target = Pubkey::from_str(SOL_USDC_AMM).expect("Invalid AMM address");
    let found = pools.iter().any(|(addr, _)| *addr == target);
    assert!(found, "WSOL-USDC 主池未出现在 list_pools_by_mint 结果中");
}

/// 测试：列出所有活跃的 WSOL Pool
#[tokio::test]
async fn test_list_active_pools_by_mint_wsol() {
    println!("=== 测试：list_pools_by_mint (WSOL, active only) ===");

    let wsol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112")
        .expect("Invalid WSOL mint");
    let rpc_url = "http://127.0.0.1:8899";
    let rpc = RpcClient::new(rpc_url.to_string());

    let pools = list_pools_by_mint(&rpc, &wsol_mint, true).await;
    assert!(pools.is_ok(), "list_pools_by_mint (active only) failed: {:?}", pools.err());
    let pools = pools.unwrap();

    assert!(!pools.is_empty(), "WSOL 相关的活跃 Pool 列表不应为空");

    // 所有池都应该包含 WSOL 且是活跃状态
    for (addr, amm) in pools.iter() {
        println!("Active WSOL Pool: {} (coin_mint={}, pc_mint={}, status={})", addr, amm.coin_mint, amm.pc_mint, amm.status);
        assert!(
            amm.coin_mint.to_string() == "So11111111111111111111111111111111111111112"
                || amm.pc_mint.to_string() == "So11111111111111111111111111111111111111112",
            "Pool {} 不包含 WSOL",
            addr,
        );
        assert_eq!(amm.status, 6, "Pool {} 不是活跃状态", addr); // 6 = ACTIVE
    }
}
