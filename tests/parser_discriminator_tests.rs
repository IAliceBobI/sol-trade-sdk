use sol_trade_sdk::parser::discriminators::{
    DiscriminatorRegistry, InstructionType, DexProtocol
};

#[test]
fn test_raydium_clmm_unknown_discriminator() {
    let registry = DiscriminatorRegistry::default();

    // 未注册的指令 discriminator 应该返回 Unknown
    // 例如：普通的 Swap 操作（目前未在注册表中）
    let unknown_instruction = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];

    let instr_type = registry.identify(
        DexProtocol::RaydiumClmm,
        &unknown_instruction
    );

    assert_eq!(instr_type, InstructionType::Unknown);
}

#[test]
fn test_raydium_clmm_liquidity_operations() {
    let registry = DiscriminatorRegistry::default();

    // openPosition 应该被识别为 AddLiquidity
    let open_position = [135, 128, 47, 77, 15, 152, 240, 49];
    assert_eq!(
        registry.identify(DexProtocol::RaydiumClmm, &open_position),
        InstructionType::AddLiquidity
    );

    // decreaseLiquidity 应该被识别为 RemoveLiquidity
    let decrease_liquidity = [160, 38, 208, 111, 104, 91, 44, 1];
    assert_eq!(
        registry.identify(DexProtocol::RaydiumClmm, &decrease_liquidity),
        InstructionType::RemoveLiquidity
    );

    // createPool 应该被识别为 CreatePool
    let create_pool = [233, 146, 209, 142, 207, 104, 64, 188];
    assert_eq!(
        registry.identify(DexProtocol::RaydiumClmm, &create_pool),
        InstructionType::CreatePool
    );
}

#[test]
fn test_raydium_clmm_liquidity_exclusion() {
    let registry = DiscriminatorRegistry::default();

    // openPosition discriminator（应该被识别为流动性操作）
    let open_position = [135, 128, 47, 77, 15, 152, 240, 49];

    let instr_type = registry.identify(
        DexProtocol::RaydiumClmm,
        &open_position
    );

    // 应该返回 Liquidity 操作，不是 Swap
    assert_eq!(instr_type, InstructionType::AddLiquidity);

    // 验证 is_liquidity_discriminator 正确识别
    assert!(registry.is_liquidity_discriminator(
        DexProtocol::RaydiumClmm,
        &open_position
    ));

    // 验证未注册的指令不是流动性操作
    let unknown_instruction = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11];
    assert!(!registry.is_liquidity_discriminator(
        DexProtocol::RaydiumClmm,
        &unknown_instruction
    ));
}

#[test]
fn test_pumpswap_buy_discriminator() {
    let registry = DiscriminatorRegistry::default();

    let buy_instruction = [102, 6, 61, 18, 1, 218, 235, 234];

    let instr_type = registry.identify(
        DexProtocol::PumpSwap,
        &buy_instruction
    );

    assert_eq!(instr_type, InstructionType::Buy);
}

#[test]
fn test_pumpswap_sell_discriminator() {
    let registry = DiscriminatorRegistry::default();

    let sell_instruction = [51, 230, 133, 164, 1, 127, 131, 173];

    let instr_type = registry.identify(
        DexProtocol::PumpSwap,
        &sell_instruction
    );

    assert_eq!(instr_type, InstructionType::Sell);
}

#[test]
fn test_pumpswap_liquidity_operations() {
    let registry = DiscriminatorRegistry::default();

    // CreatePool
    let create_pool = [233, 146, 209, 142, 207, 104, 64, 188];
    assert_eq!(
        registry.identify(DexProtocol::PumpSwap, &create_pool),
        InstructionType::CreatePool
    );

    // AddLiquidity
    let add_liquidity = [242, 35, 198, 137, 82, 225, 242, 182];
    assert_eq!(
        registry.identify(DexProtocol::PumpSwap, &add_liquidity),
        InstructionType::AddLiquidity
    );

    // RemoveLiquidity
    let remove_liquidity = [183, 18, 70, 156, 148, 109, 161, 34];
    assert_eq!(
        registry.identify(DexProtocol::PumpSwap, &remove_liquidity),
        InstructionType::RemoveLiquidity
    );
}

#[test]
fn test_is_swap_discriminator() {
    let registry = DiscriminatorRegistry::default();

    // PumpSwap Buy 和 Sell 应该被识别为 Swap 操作
    let buy_instruction = [102, 6, 61, 18, 1, 218, 235, 234];
    assert!(registry.is_swap_discriminator(
        DexProtocol::PumpSwap,
        &buy_instruction
    ));

    let sell_instruction = [51, 230, 133, 164, 1, 127, 131, 173];
    assert!(registry.is_swap_discriminator(
        DexProtocol::PumpSwap,
        &sell_instruction
    ));

    // 流动性操作不应该被识别为 Swap
    let add_liquidity = [242, 35, 198, 137, 82, 225, 242, 182];
    assert!(!registry.is_swap_discriminator(
        DexProtocol::PumpSwap,
        &add_liquidity
    ));
}

#[test]
fn test_short_instruction_data() {
    let registry = DiscriminatorRegistry::default();

    // 少于 8 字节的指令数据应该返回 Unknown
    let short_data = [0x01, 0x02, 0x03];

    let instr_type = registry.identify(
        DexProtocol::PumpSwap,
        &short_data
    );

    assert_eq!(instr_type, InstructionType::Unknown);
}
