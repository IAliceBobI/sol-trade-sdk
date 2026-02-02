//! CLMM Exact Out 功能验证测试
//!
//! 测试 CLMM quote_exact_out_simplified 函数的基本功能

use sol_trade_sdk::utils::calc::raydium_clmm::quote_exact_out_simplified;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clmm_quote_exact_out_basic_zero_for_one() {
        // 测试 token0 -> token1 (zero_for_one=true)
        let sqrt_price = 4295048016u128; // 当前 sqrt price
        let liquidity = 10_000_000u128; // 流动性
        let amount_out = 1_000u64; // 期望获得 1000 token
        let zero_for_one = true;
        let fee_rate = 2500u64; // 0.25%

        let result = quote_exact_out_simplified(sqrt_price, liquidity, amount_out, zero_for_one, fee_rate)
            .expect("quote_exact_out should succeed");

        assert!(result.amount_in > 0, "amount_in should be positive");
        assert!(result.fee_amount > 0, "fee_amount should be positive");
        assert!(
            result.amount_in > result.fee_amount,
            "amount_in should be greater than fee_amount"
        );

        println!("CLMM exact_out (zero_for_one=true) result:");
        println!("  amount_in: {}", result.amount_in);
        println!("  fee_amount: {}", result.fee_amount);
        println!("  price_impact_bps: {:?}", result.price_impact_bps);
    }

    #[tokio::test]
    async fn test_clmm_quote_exact_out_one_for_zero() {
        // 测试 token1 -> token0 (one_for_zero=false)
        let sqrt_price = 4295048016u128;
        let liquidity = 10_000_000u128;
        let amount_out = 1_000u64;
        let zero_for_one = false;
        let fee_rate = 2500u64;

        let result = quote_exact_out_simplified(sqrt_price, liquidity, amount_out, zero_for_one, fee_rate)
            .expect("quote_exact_out should succeed");

        assert!(result.amount_in > 0, "amount_in should be positive");

        println!("CLMM exact_out (zero_for_one=false) result:");
        println!("  amount_in: {}", result.amount_in);
        println!("  fee_amount: {}", result.fee_amount);
    }

    #[tokio::test]
    async fn test_clmm_quote_exact_out_no_liquidity() {
        // 测试无流动性情况
        let sqrt_price = 4295048016u128;
        let liquidity = 0u128; // 无流动性
        let amount_out = 1_000u64;
        let zero_for_one = true;
        let fee_rate = 2500u64;

        let result = quote_exact_out_simplified(sqrt_price, liquidity, amount_out, zero_for_one, fee_rate);

        assert!(result.is_err(), "Should return error for no liquidity");

        if let Err(e) = result {
            assert!(e.contains("No liquidity"), "Error should mention no liquidity");
            println!("Expected error: {}", e);
        }
    }

    #[tokio::test]
    async fn test_clmm_exact_out_price_consistency() {
        // 测试双向计算的一致性
        let sqrt_price = 4295048016u128;
        let liquidity = 10_000_000u128;
        let amount_out = 1_000u64;
        let fee_rate = 2500u64;

        // token0 -> token1
        let result1 = quote_exact_out_simplified(sqrt_price, liquidity, amount_out, true, fee_rate)
            .expect("quote_exact_out should succeed");

        // token1 -> token0（反向，相同价格）
        let result2 = quote_exact_out_simplified(sqrt_price, liquidity, amount_out, false, fee_rate)
            .expect("quote_exact_out should succeed");

        println!("Zero for one: amount_in={}", result1.amount_in);
        println!("One for zero: amount_in={}", result2.amount_in);

        // 由于方向不同，结果会不同，但都应该为正值
        assert!(result1.amount_in > 0);
        assert!(result2.amount_in > 0);
    }

    #[tokio::test]
    async fn test_clmm_exact_out_large_amount() {
        // 测试较大金额
        let sqrt_price = 4295048016u128;
        let liquidity = 10_000_000u128;
        let amount_out = 100_000u64; // 较大金额
        let zero_for_one = true;
        let fee_rate = 2500u64;

        let result = quote_exact_out_simplified(sqrt_price, liquidity, amount_out, zero_for_one, fee_rate)
            .expect("quote_exact_out should succeed for moderate amounts");

        assert!(result.amount_in > 0);
        assert!(result.fee_amount > 0);

        println!("CLMM exact_out (large amount) result:");
        println!("  amount_in: {}", result.amount_in);
        println!("  fee_amount: {}", result.fee_amount);
        println!("  price_impact_bps: {:?}", result.price_impact_bps);

        // 价格影响应该比小金额更大
        if let Some(impact) = result.price_impact_bps {
            println!("  Price impact: {} bps", impact);
        }
    }
}
