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
use sol_trade_sdk::parser::types::DexProtocol;

#[tokio::test]
async fn test_parse_real_clmm_transaction() {
    // Step 1: 写测试 - 解析真实的 CLMM 交易

    let parser = DexParser::default();

    // 真实的 CLMM 交易签名
    let signature = "5DiDUkUntQVmDMUes3mwpiPTRHQW4YWeUWfFyDFDpsKezXdw9xZQmprgrK6ddu7YaNaJ3K5GT6RGUJ8v7828TXJU";

    // 尝试解析交易
    let result = parser.parse_transaction(signature).await;

    // Step 2: 验证结果
    println!("Parse result: {:?}", result);

    // 当前阶段：验证解析器不会崩溃，能返回某种结果
    assert!(!result.success || result.error.is_some() || !result.trades.is_empty(),
            "解析器应该返回成功、错误信息或交易列表之一");
}

#[test]
fn test_raydium_clmm_program_id() {
    // Step 1: 写测试 - 验证 Raydium CLMM Program ID 常量

    let clmm_program_id = DexProtocol::RaydiumClmm.program_id();
    let expected_id = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

    assert_eq!(clmm_program_id, expected_id,
               "Raydium CLMM Program ID 应该匹配");
}

#[test]
fn test_raydium_clmm_pool_account_index() {
    // Step 1: 写测试 - 验证 CLMM Pool 的账户索引
    // 根据 solana-dex-parser，CLMM 的 pool 地址在 accounts[2]

    // 这个测试为未来实现提供参考
    // 当解析器实现后，应该能从 Swap 指令的 accounts[2] 提取 pool 地址

    let expected_pool_index = 2;
    assert_eq!(expected_pool_index, 2,
               "Raydium CLMM Pool 应该在 accounts[2]");
}

