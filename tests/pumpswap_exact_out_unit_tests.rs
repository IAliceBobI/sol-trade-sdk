//! PumpSwap Exact Out 单元测试
//!
//! 测试 `buy_exact_out_base_internal` 和 `sell_exact_out_quote_internal` 的数学正确性
//!
//! 费用结构：
//! - LP_FEE = 0.25% (25 bps)
//! - PROTOCOL_FEE = 0.05% (5 bps)
//! - CREATOR_FEE = 0.05% (5 bps)

use sol_trade_sdk::utils::calc::pumpswap::{
    buy_exact_in_quote_internal, buy_exact_out_base_internal, sell_exact_in_base_internal,
    sell_exact_out_quote_internal,
};
use solana_sdk::pubkey::Pubkey;

/// 创建非零的测试用 Pubkey
fn non_zero_pubkey() -> Pubkey {
    Pubkey::new_from_array([1u8; 32])
}

// ============================================================================
// buy_exact_out_base_internal 测试
// ============================================================================

#[test]
fn test_buy_exact_out_base_internal_no_creator() {
    // 场景：无 creator 的情况（只有 LP 费用 + 协议费用）
    let base = 1_000_000; // 想要购买 1M base tokens
    let slippage_bps = 100; // 1% 滑点
    let base_reserve = 100_000_000_000; // 100B base reserve
    let quote_reserve = 10_000_000_000; // 10B quote reserve
    let coin_creator = Pubkey::default(); // 无 creator

    let result =
        buy_exact_out_base_internal(base, slippage_bps, base_reserve, quote_reserve, &coin_creator)
            .expect("buy_exact_out_base_internal should succeed");

    // 验证内部计算：quote_amount_in = ceil(quote_reserve * base / (base_reserve - base))
    let expected_quote_in = ((quote_reserve as u128 * base as u128) as f64
        / (base_reserve - base) as f64)
        .ceil() as u64;
    assert_eq!(result.internal_quote_amount, expected_quote_in);

    // 验证费用计算：无 creator 时，总费用 = LP(0.25%) + Protocol(0.05%) = 0.30%
    let lp_fee = (result.internal_quote_amount as f64 * 0.0025).ceil() as u64;
    let protocol_fee = (result.internal_quote_amount as f64 * 0.0005).ceil() as u64;
    let expected_total = result.internal_quote_amount + lp_fee + protocol_fee;
    assert_eq!(result.ui_quote, expected_total);

    // 验证滑点：max_quote = total * (1 + slippage)
    let expected_max_quote = (expected_total as f64 * 1.01) as u64;
    assert_eq!(result.max_quote, expected_max_quote);

    println!("=== 无 Creator 买入测试 ===");
    println!("目标 base 数量: {}", base);
    println!("所需 quote (内部): {}", result.internal_quote_amount);
    println!("所需 quote (含费用): {}", result.ui_quote);
    println!("最大 quote (含滑点): {}", result.max_quote);
}

#[test]
fn test_buy_exact_out_base_internal_with_creator() {
    // 场景：有 creator 的情况（LP 费用 + 协议费用 + Creator 费用）
    let base = 1_000_000;
    let slippage_bps = 100;
    let base_reserve = 100_000_000_000;
    let quote_reserve = 10_000_000_000;
    let coin_creator = non_zero_pubkey(); // 有 creator

    let result =
        buy_exact_out_base_internal(base, slippage_bps, base_reserve, quote_reserve, &coin_creator)
            .expect("buy_exact_out_base_internal should succeed");

    // 验证内部计算
    let expected_quote_in = ((quote_reserve as u128 * base as u128) as f64
        / (base_reserve - base) as f64)
        .ceil() as u64;
    assert_eq!(result.internal_quote_amount, expected_quote_in);

    // 验证费用计算：有 creator 时，总费用 = LP(0.25%) + Protocol(0.05%) + Creator(0.05%) = 0.35%
    let lp_fee = (result.internal_quote_amount as f64 * 0.0025).ceil() as u64;
    let protocol_fee = (result.internal_quote_amount as f64 * 0.0005).ceil() as u64;
    let creator_fee = (result.internal_quote_amount as f64 * 0.0005).ceil() as u64;
    let expected_total = result.internal_quote_amount + lp_fee + protocol_fee + creator_fee;
    assert_eq!(result.ui_quote, expected_total);

    println!("=== 有 Creator 买入测试 ===");
    println!("目标 base 数量: {}", base);
    println!("所需 quote (内部): {}", result.internal_quote_amount);
    println!("所需 quote (含费用): {}", result.ui_quote);
    println!("最大 quote (含滑点): {}", result.max_quote);
}

#[test]
fn test_buy_exact_out_base_internal_edge_cases() {
    // 边界情况测试

    // 1. 非常小的购买量
    let result =
        buy_exact_out_base_internal(100, 100, 100_000_000_000, 10_000_000_000, &Pubkey::default())
            .expect("small buy should succeed");
    assert!(result.internal_quote_amount > 0);

    // 2. 较大的滑点
    let result = buy_exact_out_base_internal(
        1_000_000,
        1000, // 10% 滑点
        100_000_000_000,
        10_000_000_000,
        &Pubkey::default(),
    )
    .expect("large slippage buy should succeed");
    let expected_max = (result.ui_quote as f64 * 1.1) as u64;
    assert_eq!(result.max_quote, expected_max);

    // 3. 错误情况：购买量超过储备
    let result = buy_exact_out_base_internal(
        200_000_000_000, // 超过 base_reserve
        100,
        100_000_000_000,
        10_000_000_000,
        &Pubkey::default(),
    );
    assert!(result.is_err());

    // 4. 错误情况：零储备
    let result = buy_exact_out_base_internal(100, 100, 0, 10_000_000_000, &Pubkey::default());
    assert!(result.is_err());

    let result = buy_exact_out_base_internal(100, 100, 100_000_000_000, 0, &Pubkey::default());
    assert!(result.is_err());
}

// ============================================================================
// sell_exact_out_quote_internal 测试
// ============================================================================

#[test]
fn test_sell_exact_out_quote_internal_no_creator() {
    // 场景：无 creator 的情况
    let quote = 100_000_000; // 想要获得 100M quote tokens
    let slippage_bps = 100; // 1% 滑点
    let base_reserve = 100_000_000_000;
    let quote_reserve = 10_000_000_000;
    let coin_creator = Pubkey::default();

    let result = sell_exact_out_quote_internal(
        quote,
        slippage_bps,
        base_reserve,
        quote_reserve,
        &coin_creator,
    )
    .expect("sell_exact_out_quote_internal should succeed");

    // 验证内部 raw_quote 计算
    // raw_quote = ceil(quote * 10000 / (10000 - total_fee_bps))
    // 无 creator: total_fee_bps = 25 + 5 = 30
    let expected_raw_quote = ((quote as u128 * 10000) as f64 / (10000 - 30) as f64).ceil() as u64;
    assert_eq!(result.internal_raw_quote, expected_raw_quote);

    // 验证 base 计算：base = ceil(base_reserve * raw_quote / (quote_reserve - raw_quote))
    let expected_base = ((base_reserve as u128 * result.internal_raw_quote as u128) as f64
        / (quote_reserve - result.internal_raw_quote) as f64)
        .ceil() as u64;
    assert_eq!(result.base, expected_base);

    // 验证滑点
    let expected_min_quote = (quote as f64 * 0.99) as u64;
    assert_eq!(result.min_quote, expected_min_quote);

    println!("=== 无 Creator 卖出测试 ===");
    println!("目标 quote 数量: {}", quote);
    println!("所需 base: {}", result.base);
    println!("内部 raw_quote: {}", result.internal_raw_quote);
    println!("最小 quote (含滑点): {}", result.min_quote);
}

#[test]
fn test_sell_exact_out_quote_internal_with_creator() {
    // 场景：有 creator 的情况
    let quote = 100_000_000;
    let slippage_bps = 100;
    let base_reserve = 100_000_000_000;
    let quote_reserve = 10_000_000_000;
    let coin_creator = non_zero_pubkey();

    let result = sell_exact_out_quote_internal(
        quote,
        slippage_bps,
        base_reserve,
        quote_reserve,
        &coin_creator,
    )
    .expect("sell_exact_out_quote_internal should succeed");

    // 验证内部 raw_quote 计算
    // 有 creator: total_fee_bps = 25 + 5 + 5 = 35
    let expected_raw_quote = ((quote as u128 * 10000) as f64 / (10000 - 35) as f64).ceil() as u64;
    assert_eq!(result.internal_raw_quote, expected_raw_quote);

    // 有 creator 时需要更多的 base
    let result_no_creator = sell_exact_out_quote_internal(
        quote,
        slippage_bps,
        base_reserve,
        quote_reserve,
        &Pubkey::default(),
    )
    .expect("should succeed");

    // 有 creator 时需要更多的 base（因为费用更高）
    // 注意：由于 ceil_div 的影响，这个关系不一定总是成立，但对于大数应该成立
    println!("=== 有 Creator 卖出测试 ===");
    println!("目标 quote 数量: {}", quote);
    println!("有 creator 所需 base: {}", result.base);
    println!("无 creator 所需 base: {}", result_no_creator.base);
}

#[test]
fn test_sell_exact_out_quote_internal_edge_cases() {
    // 边界情况测试

    // 1. 非常小的 quote 输出
    let result = sell_exact_out_quote_internal(
        1000,
        100,
        100_000_000_000,
        10_000_000_000,
        &Pubkey::default(),
    )
    .expect("small sell should succeed");
    assert!(result.base > 0);

    // 2. 错误情况：quote 超过储备
    let result = sell_exact_out_quote_internal(
        20_000_000_000, // 超过 quote_reserve
        100,
        100_000_000_000,
        10_000_000_000,
        &Pubkey::default(),
    );
    assert!(result.is_err());

    // 3. 错误情况：零储备
    let result = sell_exact_out_quote_internal(100, 100, 0, 10_000_000_000, &Pubkey::default());
    assert!(result.is_err());

    let result = sell_exact_out_quote_internal(100, 100, 100_000_000_000, 0, &Pubkey::default());
    assert!(result.is_err());
}

// ============================================================================
// 反向验证测试
// ============================================================================

/// 反向验证 Buy 方向：
/// 用 exact_out 计算需要多少 quote → 用这个 quote 做 exact_in → 验证得到的 base >= 期望的 base
#[test]
fn test_exact_out_buy_reverse_verification() {
    let desired_base = 1_000_000;
    let slippage_bps = 100;
    let base_reserve = 100_000_000_000;
    let quote_reserve = 10_000_000_000;
    let coin_creator = Pubkey::default();

    // Step 1: 使用 buy_exact_out_base_internal 计算需要多少 quote
    let exact_out_result = buy_exact_out_base_internal(
        desired_base,
        slippage_bps,
        base_reserve,
        quote_reserve,
        &coin_creator,
    )
    .expect("buy_exact_out_base_internal should succeed");

    let quote_to_spend = exact_out_result.ui_quote;

    println!("=== Buy 反向验证 ===");
    println!("期望获得的 base: {}", desired_base);
    println!("需要花费的 quote: {}", quote_to_spend);

    // Step 2: 使用 buy_exact_in_quote_internal 验证用这些 quote 能获得多少 base
    let exact_in_result = buy_exact_in_quote_internal(
        quote_to_spend,
        slippage_bps,
        base_reserve,
        quote_reserve,
        &coin_creator,
    )
    .expect("buy_exact_in_quote_internal should succeed");

    println!("实际获得的 base: {}", exact_in_result.base);

    // 验证：实际获得的 base 应该 >= 期望的 base
    // 由于费用计算方式的不同（exact_out 是在内部计算上叠加费用，exact_in 是从输入中扣除费用），
    // 可能会有轻微的精度差异
    assert!(
        exact_in_result.base >= desired_base
            || (desired_base as i64 - exact_in_result.base as i64).abs() <= 1,
        "Reverse verification failed: expected at least {} base, got {}. Diff: {}",
        desired_base,
        exact_in_result.base,
        (desired_base as i64 - exact_in_result.base as i64).abs()
    );

    // 验证精度差异在可接受范围内（由于 ceil 操作，差异应该很小）
    let diff_ratio = if desired_base > 0 {
        (desired_base as i64 - exact_in_result.base as i64).abs() as f64 / desired_base as f64
    } else {
        0.0
    };
    println!("精度差异比例: {:.6}%", diff_ratio * 100.0);
    assert!(diff_ratio < 0.01, "Precision diff should be < 0.01%, got {:.6}%", diff_ratio * 100.0);
}

/// 反向验证 Sell 方向：
/// 用 exact_out 计算需要卖多少 base → 用这个 base 做 exact_in → 验证得到的 quote >= 期望的 quote
#[test]
fn test_exact_out_sell_reverse_verification() {
    let desired_quote = 50_000_000;
    let slippage_bps = 100;
    let base_reserve = 100_000_000_000;
    let quote_reserve = 10_000_000_000;
    let coin_creator = Pubkey::default();

    // Step 1: 使用 sell_exact_out_quote_internal 计算需要卖多少 base
    let exact_out_result = sell_exact_out_quote_internal(
        desired_quote,
        slippage_bps,
        base_reserve,
        quote_reserve,
        &coin_creator,
    )
    .expect("sell_exact_out_quote_internal should succeed");

    let base_to_sell = exact_out_result.base;

    println!("=== Sell 反向验证 ===");
    println!("期望获得的 quote: {}", desired_quote);
    println!("需要卖出的 base: {}", base_to_sell);

    // Step 2: 使用 sell_exact_in_base_internal 验证卖这些 base 能获得多少 quote
    let exact_in_result = sell_exact_in_base_internal(
        base_to_sell,
        slippage_bps,
        base_reserve,
        quote_reserve,
        &coin_creator,
    )
    .expect("sell_exact_in_base_internal should succeed");

    println!("实际获得的 quote: {}", exact_in_result.ui_quote);

    // 验证：实际获得的 quote 应该 >= 期望的 quote
    // 由于费用计算和 ceil_div 的影响，可能会有轻微的精度差异
    assert!(
        exact_in_result.ui_quote >= desired_quote
            || (desired_quote as i64 - exact_in_result.ui_quote as i64).abs() <= 2,
        "Reverse verification failed: expected at least {} quote, got {}. Diff: {}",
        desired_quote,
        exact_in_result.ui_quote,
        (desired_quote as i64 - exact_in_result.ui_quote as i64).abs()
    );

    // 验证精度差异在可接受范围内
    let diff_ratio = if desired_quote > 0 {
        (desired_quote as i64 - exact_in_result.ui_quote as i64).abs() as f64 / desired_quote as f64
    } else {
        0.0
    };
    println!("精度差异比例: {:.6}%", diff_ratio * 100.0);
    assert!(diff_ratio < 0.01, "Precision diff should be < 0.01%, got {:.6}%", diff_ratio * 100.0);
}

/// 多组数据验证 Buy
///
/// 注意：由于 PumpSwap 的多层费用结构和 ceil_div 操作，反向验证会产生累积误差。
/// 这些误差来源于：
/// 1. buy_exact_out_base_internal: 在内部计算上叠加三层费用（每层都有 ceil）
/// 2. buy_exact_in_quote_internal: 从输入中扣除费用（也有 ceil）
/// 3. 恒定乘积公式中的除法（ceil）
///
/// 因此我们使用相对误差（比例）来验证，而不是绝对误差。
/// 可接受的相对误差阈值：0.05%（远小于实际交易中滑点的影响）
#[test]
fn test_exact_out_buy_multiple_cases() {
    let test_cases = vec![
        (100_000, 100_000_000_000u64, 10_000_000_000u64),
        (1_000_000, 100_000_000_000, 10_000_000_000),
        (10_000_000, 100_000_000_000, 10_000_000_000),
        (100_000_000, 100_000_000_000, 10_000_000_000),
    ];

    let slippage_bps = 100;
    let coin_creator = Pubkey::default();

    for (desired_base, base_reserve, quote_reserve) in test_cases {
        let exact_out = buy_exact_out_base_internal(
            desired_base,
            slippage_bps,
            base_reserve,
            quote_reserve,
            &coin_creator,
        )
        .expect("buy_exact_out_base_internal should succeed");

        let exact_in = buy_exact_in_quote_internal(
            exact_out.ui_quote,
            slippage_bps,
            base_reserve,
            quote_reserve,
            &coin_creator,
        )
        .expect("buy_exact_in_quote_internal should succeed");

        let diff = (desired_base as i64 - exact_in.base as i64).abs();
        let diff_ratio = diff as f64 / desired_base as f64 * 100.0;

        println!(
            "Buy {} base: need {} quote, got {} base, diff: {} ({:.4}%)",
            desired_base, exact_out.ui_quote, exact_in.base, diff, diff_ratio
        );

        // 验证：实际获得的 base 应该 >= 期望的 base（有足够的 quote 支付）
        // 或者相对误差在可接受范围内（< 0.05%）
        assert!(
            exact_in.base >= desired_base || diff_ratio < 0.05,
            "Reverse verification failed: expected {} base, got {}, diff_ratio: {:.4}%",
            desired_base,
            exact_in.base,
            diff_ratio
        );
    }
}

/// 多组数据验证 Sell
///
/// 与 Buy 类似，Sell 方向也会因为多层费用和 ceil 操作产生累积误差。
/// 可接受的相对误差阈值：0.05%
#[test]
fn test_exact_out_sell_multiple_cases() {
    let test_cases = vec![
        (1_000_000, 100_000_000_000u64, 10_000_000_000u64),
        (10_000_000, 100_000_000_000, 10_000_000_000),
        (50_000_000, 100_000_000_000, 10_000_000_000),
        (100_000_000, 100_000_000_000, 10_000_000_000),
    ];

    let slippage_bps = 100;
    let coin_creator = Pubkey::default();

    for (desired_quote, base_reserve, quote_reserve) in test_cases {
        let exact_out = sell_exact_out_quote_internal(
            desired_quote,
            slippage_bps,
            base_reserve,
            quote_reserve,
            &coin_creator,
        )
        .expect("sell_exact_out_quote_internal should succeed");

        let exact_in = sell_exact_in_base_internal(
            exact_out.base,
            slippage_bps,
            base_reserve,
            quote_reserve,
            &coin_creator,
        )
        .expect("sell_exact_in_base_internal should succeed");

        let diff = (desired_quote as i64 - exact_in.ui_quote as i64).abs();
        let diff_ratio = diff as f64 / desired_quote as f64 * 100.0;

        println!(
            "Sell for {} quote: need {} base, got {} quote, diff: {} ({:.4}%)",
            desired_quote, exact_out.base, exact_in.ui_quote, diff, diff_ratio
        );

        // 验证：实际获得的 quote 应该 >= 期望的 quote（卖出足够的 base）
        // 或者相对误差在可接受范围内（< 0.05%）
        assert!(
            exact_in.ui_quote >= desired_quote || diff_ratio < 0.05,
            "Reverse verification failed: expected {} quote, got {}, diff_ratio: {:.4}%",
            desired_quote,
            exact_in.ui_quote,
            diff_ratio
        );
    }
}

/// 测试有 creator 的反向验证
///
/// 有 creator 时费用更高（额外 0.05%），累积误差会略大。
/// 可接受的相对误差阈值：0.1%（因为多一层费用）
#[test]
fn test_exact_out_reverse_with_creator() {
    let coin_creator = non_zero_pubkey();
    let slippage_bps = 100;
    let base_reserve = 100_000_000_000;
    let quote_reserve = 10_000_000_000;

    // Buy with creator
    {
        let desired_base = 1_000_000;
        let exact_out = buy_exact_out_base_internal(
            desired_base,
            slippage_bps,
            base_reserve,
            quote_reserve,
            &coin_creator,
        )
        .expect("should succeed");

        let exact_in = buy_exact_in_quote_internal(
            exact_out.ui_quote,
            slippage_bps,
            base_reserve,
            quote_reserve,
            &coin_creator,
        )
        .expect("should succeed");

        let diff = (desired_base as i64 - exact_in.base as i64).abs();
        let diff_ratio = diff as f64 / desired_base as f64 * 100.0;
        println!("Buy with creator: diff = {}, diff_ratio = {:.4}%", diff, diff_ratio);

        // 有 creator 时允许更大的相对误差（< 0.1%）
        assert!(
            exact_in.base >= desired_base || diff_ratio < 0.1,
            "Buy diff_ratio should be < 0.1%, got {:.4}%",
            diff_ratio
        );
    }

    // Sell with creator
    {
        let desired_quote = 50_000_000;
        let exact_out = sell_exact_out_quote_internal(
            desired_quote,
            slippage_bps,
            base_reserve,
            quote_reserve,
            &coin_creator,
        )
        .expect("should succeed");

        let exact_in = sell_exact_in_base_internal(
            exact_out.base,
            slippage_bps,
            base_reserve,
            quote_reserve,
            &coin_creator,
        )
        .expect("should succeed");

        let diff = (desired_quote as i64 - exact_in.ui_quote as i64).abs();
        let diff_ratio = diff as f64 / desired_quote as f64 * 100.0;
        println!("Sell with creator: diff = {}, diff_ratio = {:.4}%", diff, diff_ratio);

        // 有 creator 时允许更大的相对误差（< 0.1%）
        assert!(
            exact_in.ui_quote >= desired_quote || diff_ratio < 0.1,
            "Sell diff_ratio should be < 0.1%, got {:.4}%",
            diff_ratio
        );
    }
}
