use sol_trade_sdk::constants::{USDC_MINT, WSOL_TOKEN_ACCOUNT};
/// 测试 identify_quote_mint 函数是否可以被下游使用
use sol_trade_sdk::instruction::utils::pumpswap::identify_quote_mint;

#[test]
fn test_identify_quote_mint_is_exported() {
    // 验证函数可以从下游访问
    let result = identify_quote_mint(&USDC_MINT, &WSOL_TOKEN_ACCOUNT);
    assert_eq!(result, Some(USDC_MINT));
    println!("✅ identify_quote_mint 函数已成功导出，下游可以使用！");
}
