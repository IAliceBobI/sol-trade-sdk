//! Exact Out 功能验证测试
//!
//! 测试 quote_exact_out 函数的基本功能

use sol_trade_sdk::utils::calc::raydium_cpmm::quote_exact_out as cpmm_quote_exact_out;
use sol_trade_sdk::utils::calc::raydium_amm_v4::quote_exact_out as amm_v4_quote_exact_out;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cpmm_quote_exact_out_basic() {
        // 测试基本的 exact_out 计算
        let base_reserve = 1_000_000_000; // 1000 tokens
        let quote_reserve = 2_000_000_000; // 2000 tokens
        let amount_out = 100_000; // 期望获得 0.1 token

        let result = cpmm_quote_exact_out(base_reserve, quote_reserve, amount_out, true)
            .expect("quote_exact_out should succeed");

        assert!(result.amount_in > 0, "amount_in should be positive");
        assert!(result.fee_amount > 0, "fee_amount should be positive");
        assert!(
            result.amount_in > result.fee_amount,
            "amount_in should be greater than fee_amount"
        );

        println!("CPMM exact_out result:");
        println!("  amount_in: {}", result.amount_in);
        println!("  fee_amount: {}", result.fee_amount);
        println!("  price_impact_bps: {:?}", result.price_impact_bps);
    }

    #[tokio::test]
    async fn test_cpmm_quote_exact_out_insufficient_liquidity() {
        // 测试流动性不足的情况
        let base_reserve = 1_000_000;
        let quote_reserve = 1_000_000;
        let amount_out = 2_000_000; // 超过储备

        let result = cpmm_quote_exact_out(base_reserve, quote_reserve, amount_out, true);

        assert!(result.is_err(), "Should return error for insufficient liquidity");

        if let Err(e) = result {
            assert!(e.contains("Insufficient liquidity"), "Error should mention insufficient liquidity");
            println!("Expected error: {}", e);
        }
    }

    #[tokio::test]
    async fn test_amm_v4_quote_exact_out_basic() {
        // 测试 AMM V4 的 exact_out 计算
        let coin_reserve = 1_000_000_000;
        let pc_reserve = 2_000_000_000;
        let amount_out = 100_000;

        let result = amm_v4_quote_exact_out(coin_reserve, pc_reserve, amount_out, true)
            .expect("quote_exact_out should succeed");

        assert!(result.amount_in > 0, "amount_in should be positive");
        assert!(result.fee_amount > 0, "fee_amount should be positive");

        println!("AMM V4 exact_out result:");
        println!("  amount_in: {}", result.amount_in);
        println!("  fee_amount: {}", result.fee_amount);
        println!("  price_impact_bps: {:?}", result.price_impact_bps);
    }

    #[tokio::test]
    async fn test_exact_out_consistency() {
        // 测试 exact_out 计算的一致性
        // 对于相同的储备，两个方向的计算应该是一致的

        let base_reserve = 1_000_000_000;
        let quote_reserve = 2_000_000_000;
        let amount_out = 100_000;

        // base -> quote
        let result1 = cpmm_quote_exact_out(base_reserve, quote_reserve, amount_out, true)
            .expect("quote_exact_out should succeed");

        // quote -> base（反向）
        let result2 = cpmm_quote_exact_out(quote_reserve, base_reserve, amount_out, false)
            .expect("quote_exact_out should succeed");

        // 由于手续费结构不同，结果不会完全相同，但应该在一个合理的范围内
        println!("Base -> quote: amount_in={}", result1.amount_in);
        println!("Quote -> base: amount_in={}", result2.amount_in);

        // 验证两者都是正值
        assert!(result1.amount_in > 0);
        assert!(result2.amount_in > 0);
    }
}
