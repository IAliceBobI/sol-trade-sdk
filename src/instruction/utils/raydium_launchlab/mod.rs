// ✅ Token-2022 支持已完成
//
// 所有函数都已添加 base_token_program 和 quote_token_program 参数：
// - build_buy_exact_in_instruction
// - build_buy_exact_in_instruction_with_seed
// - build_sell_exact_in_instruction
// - build_initialize_instruction
// - build_initialize_v2_instruction
// - build_migrate_to_cpswap_instruction
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

// Re-export helper functions (内部实现)
// 移除未使用的导出，避免 clippy 警告

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

// Re-export serialization functions for backward compatibility (内部实现)
// 移除未使用的导出，避免 clippy 警告
