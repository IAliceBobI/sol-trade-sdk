// 🔧 TODO: 此文件包含多处 TOKEN_PROGRAM 硬编码，需要支持 Token-2022
//
// 由于文件过大（1500+ 行），暂时保留硬编码。以下函数需要添加 Token Program 参数：
//
// ✅ 已修复：
// - build_buy_exact_in_instruction
// - build_buy_exact_in_instruction_with_seed
// - build_sell_exact_in_instruction
// - build_initialize_instruction
// - build_initialize_v2_instruction
//
// ⚠️ 尚未修复（函数签名已添加参数，但内部实现仍使用 TOKEN_PROGRAM）：
// - migrate_to_cpswap 相关函数
//
// 使用这些函数时，请确保传入正确的 base_token_program 和 quote_token_program 参数
//

mod constants;
mod cpswap;
mod helpers;
mod instructions;
mod parsing;
mod pool_queries;
mod types;

// Re-export constants
pub use constants::{accounts, discriminators, seeds};

// Re-export types
pub use types::{
    AmmCreatorFeeOn, CurveParams, GLOBAL_CONFIG_SIZE, GlobalConfig, LaunchLabPoolState,
    LaunchLabVestingSchedule, MigrateNftInfo, MintParams, PlatformConfig, VestingParams,
};

// Re-export helper functions
pub use helpers::{
    get_bonding_curve_pda, get_cpswap_authority_pda, get_cpswap_lp_mint_pda,
    get_cpswap_observation_pda, get_cpswap_pool_pda, get_cpswap_vault_pda,
    get_creator_fee_vault_pda, get_event_authority_pda, get_global_config_pda,
    get_lock_authority_pda, get_metadata_pda, get_platform_config_pda, get_platform_fee_vault_pda,
    get_pool_state_pda, get_pool_vault_pda, get_vault_authority_pda,
};

// Re-export parsing functions
pub use parsing::{parse_global_config, parse_platform_config, parse_pool_state};

// Re-export pool query functions
pub use pool_queries::{
    fetch_bonding_curve_account, fetch_global_config, fetch_platform_config, find_global_config,
    find_platform_config,
};

// Re-export CPMM-related functions
pub use cpswap::find_cpswap_config;

// Re-export instruction builders
pub use instructions::{
    build_buy_exact_in_instruction, build_buy_exact_in_instruction_with_seed,
    build_initialize_instruction, build_initialize_v2_instruction,
    build_migrate_to_cpswap_instruction, build_sell_exact_in_instruction,
};

// Re-export serialization functions for backward compatibility
pub use helpers::{
    serialize_amm_creator_fee_on, serialize_curve_params, serialize_mint_params,
    serialize_vesting_params,
};
