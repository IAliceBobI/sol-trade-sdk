//! 统一缓存管理模块
//!
//! 提供一键清除所有 DEX 协议 Pool 缓存的功能。

/// 清除所有协议的 Pool 缓存
/// ```
pub fn clear_all_pool_caches() {
    // PumpSwap
    crate::instruction::utils::pumpswap::clear_pool_cache();

    // Raydium CPMM
    crate::instruction::utils::raydium_cpmm::clear_pool_cache();

    // Raydium CLMM
    crate::instruction::utils::raydium_clmm::clear_pool_cache();

    // Bonk
    crate::instruction::utils::bonk::clear_pool_cache();

    // Meteora DAMM V2
    crate::instruction::utils::meteora_damm_v2::clear_pool_cache();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_all_pool_caches_no_panic() {
        // 确保不 panic
        clear_all_pool_caches();
    }

    #[test]
    fn test_clear_all_pool_caches_repeatedly() {
        // 多次调用应该也没问题
        clear_all_pool_caches();
        clear_all_pool_caches();
        clear_all_pool_caches();
    }
}
