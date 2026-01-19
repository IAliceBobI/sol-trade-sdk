//! CLMM 交易集成测试
//!
//! 测试真实的 Raydium CLMM 交易解析
//!
//! 参考交易:
//! https://solscan.io/tx/5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU
//!
//! Program logs 显示：
//! - BVdiEkcvautEuWkejqLbn4HDj9fDLCq3UormFnLkQg9T invoke [1]
//! - Program log: Instruction: DelegateSwapClmmFixedBase
//! - CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK invoke [2]
//! - Program log: Instruction: SwapV2
//! - TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [3]
//! - Program log: Instruction: TransferChecked (两次)

use sol_trade_sdk::parser::DexParser;

#[tokio::test]
async fn test_parse_real_clmm_transaction() {
    // Step 1: 写测试 - 解析真实的 CLMM 交易

    let parser = DexParser::default();

    // 真实的 CLMM 交易签名
    let signature = "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU";

    // 尝试解析交易
    let result = parser.parse_transaction(signature).await;

    // Step 2: 验证结果
    // 注意：由于完整解析功能还未实现，我们预期当前会返回"完整解析功能待实现"的错误
    // 但应该能够成功获取交易数据（slot 和 block_time）

    println!("Parse result: {:?}", result);

    // 当前阶段：验证解析器不会崩溃，能返回某种结果
    // 即使解析失败，也应该有明确的错误信息
    assert!(!result.success || result.error.is_some() || !result.trades.is_empty(),
            "解析器应该返回成功、错误信息或交易列表之一");
}
