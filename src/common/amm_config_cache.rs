//! AmmConfig 费率缓存管理
//!
//! 用于缓存 Raydium CPMM Pool 的 AmmConfig 费率信息，避免重复 RPC 调用。

use crate::instruction::utils::raydium_cpmm_types::AmmConfig;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use solana_sdk::pubkey::Pubkey;
use std::time::{Duration, Instant};

/// 缓存条目：费率数据 + 过期时间
#[derive(Clone, Debug)]
struct CacheEntry {
    /// 费率配置
    config: AmmConfig,
    /// 缓存创建时间
    created_at: Instant,
}

/// AmmConfig 费率缓存
///
/// 缓存结构: {amm_config_pubkey -> (config, created_at)}
///
/// # TTL 策略
///
/// - 缓存有效期: 30 分钟
/// - 过期后会在下次访问时自动清理
///
/// # 线程安全
///
/// 使用 `DashMap` 保证多线程安全，无需加锁。
pub struct AmmConfigCache;

/// TTL: 30 分钟
const CACHE_TTL: Duration = Duration::from_secs(30 * 60);

/// 最大缓存数量
const MAX_CACHE_SIZE: usize = 1000;

/// 内部缓存存储
static CACHE: Lazy<DashMap<Pubkey, CacheEntry>> =
    Lazy::new(|| DashMap::with_capacity(MAX_CACHE_SIZE));

impl AmmConfigCache {
    /// 获取缓存的 AmmConfig
    ///
    /// 如果缓存存在且未过期，返回 `Some(config)`，否则返回 `None`。
    ///
    /// # Arguments
    ///
    /// * `amm_config` - AmmConfig 账户地址
    ///
    /// # Returns
    ///
    /// * `Some(AmmConfig)` - 缓存命中且未过期
    /// * `None` - 缓存未命中或已过期
    #[inline]
    pub fn get(amm_config: &Pubkey) -> Option<AmmConfig> {
        CACHE.get(amm_config).and_then(|entry| {
            // 检查是否过期
            if entry.created_at.elapsed() < CACHE_TTL {
                Some(entry.config.clone())
            } else {
                // 过期，移除缓存
                CACHE.remove(amm_config);
                None
            }
        })
    }

    /// 插入或更新缓存
    ///
    /// # Arguments
    ///
    /// * `amm_config` - AmmConfig 账户地址
    /// * `config` - 费率配置数据
    #[inline]
    pub fn insert(amm_config: Pubkey, config: AmmConfig) {
        CACHE.insert(amm_config, CacheEntry { config, created_at: Instant::now() });
    }

    /// 清除所有缓存
    ///
    /// 通常用于：
    /// - 测试后清理
    /// - 强制刷新所有配置
    #[inline]
    pub fn clear() {
        CACHE.clear();
    }

    /// 清理过期缓存条目
    ///
    /// 主动清理所有已过期的缓存条目，释放内存。
    /// 通常不需要手动调用，因为过期的条目会在下次访问时自动清理。
    #[inline]
    pub fn cleanup_expired() {
        CACHE.retain(|_key, entry| entry.created_at.elapsed() < CACHE_TTL);
    }

    /// 获取缓存统计信息
    ///
    /// # Returns
    ///
    /// (总条目数, 有效条目数, 过期条目数)
    pub fn stats() -> (usize, usize, usize) {
        let total = CACHE.len();
        let mut valid = 0;
        let mut expired = 0;

        CACHE.iter().for_each(|entry| {
            if entry.created_at.elapsed() < CACHE_TTL {
                valid += 1;
            } else {
                expired += 1;
            }
        });

        (total, valid, expired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        AmmConfigCache::clear();

        let pubkey = Pubkey::new_unique();
        let config = AmmConfig {
            trade_fee_rate: 2500,
            protocol_fee_rate: 120000,
            fund_fee_rate: 40000,
            ..Default::default()
        };

        // 插入缓存
        AmmConfigCache::insert(pubkey, config.clone());

        // 获取缓存
        let cached = AmmConfigCache::get(&pubkey);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().trade_fee_rate, 2500);
    }

    #[test]
    fn test_cache_miss() {
        AmmConfigCache::clear();

        let pubkey = Pubkey::new_unique();
        assert!(AmmConfigCache::get(&pubkey).is_none());
    }

    #[test]
    fn test_cache_clear() {
        AmmConfigCache::clear();

        let pubkey = Pubkey::new_unique();
        let config = AmmConfig::default();
        AmmConfigCache::insert(pubkey, config);

        assert!(AmmConfigCache::get(&pubkey).is_some());

        AmmConfigCache::clear();
        assert!(AmmConfigCache::get(&pubkey).is_none());
    }

    #[test]
    fn test_cache_stats() {
        AmmConfigCache::clear();

        let config = AmmConfig::default();

        for _ in 0..5 {
            let pubkey = Pubkey::new_unique();
            AmmConfigCache::insert(pubkey, config.clone());
        }

        let (total, valid, expired) = AmmConfigCache::stats();
        assert_eq!(total, 5);
        assert_eq!(valid, 5);
        assert_eq!(expired, 0);
    }
}
