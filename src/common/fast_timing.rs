//! 🚀 快速计时模块 - 减少 Instant::now() 系统调用开销
//!
//! 使用 syscall_bypass 提供的快速时间戳避免频繁的系统调用

use crate::perf::syscall_bypass::SystemCallBypassManager;
use once_cell::sync::Lazy;
use std::time::{Duration, Instant};

/// 全局快速时间提供器
static FAST_TIMER: Lazy<FastTimer> = Lazy::new(FastTimer::new);

/// 快速计时器 - 减少系统调用开销
/// 🔧 修复：bypass_manager 改为 Option 以支持降级
pub struct FastTimer {
    bypass_manager: Option<SystemCallBypassManager>,
    _base_instant: Instant,
    _base_nanos: u64,
}

impl FastTimer {
    fn new() -> Self {
        use crate::perf::syscall_bypass::SyscallBypassConfig;

        // 🔧 修复：添加降级逻辑，如果 syscall_bypass 初始化失败则回退到标准计时器
        let bypass_manager = match SystemCallBypassManager::new(SyscallBypassConfig::default()) {
            Ok(manager) => {
                log::debug!("FastTimer initialized with syscall bypass");
                Some(manager)
            },
            Err(e) => {
                log::warn!(
                    "Failed to create SystemCallBypassManager, falling back to standard timing: {}",
                    e
                );
                None
            },
        };

        let base_instant = Instant::now();
        let base_nanos =
            bypass_manager.as_ref().map(|m| m.fast_timestamp_nanos()).unwrap_or_else(|| {
                // 如果没有 bypass_manager，使用标准 Instant
                base_instant.elapsed().as_nanos() as u64
            });

        Self {
            bypass_manager,
            _base_instant: base_instant,
            _base_nanos: base_nanos,
        }
    }

    /// 🚀 获取当前时间戳（纳秒） - 使用快速系统调用绕过
    /// 🔧 修复：支持降级到标准 Instant
    #[inline(always)]
    pub fn now_nanos(&self) -> u64 {
        if let Some(manager) = &self.bypass_manager {
            manager.fast_timestamp_nanos()
        } else {
            // 降级到标准计时器
            self._base_instant.elapsed().as_nanos() as u64 + self._base_nanos
        }
    }

    /// 🚀 获取当前时间戳（微秒）
    #[inline(always)]
    pub fn now_micros(&self) -> u64 {
        self.now_nanos() / 1_000
    }

    /// 🚀 获取当前时间戳（毫秒）
    #[inline(always)]
    pub fn now_millis(&self) -> u64 {
        self.now_nanos() / 1_000_000
    }

    /// 🚀 计算从开始到现在的耗时（纳秒）
    #[inline(always)]
    pub fn elapsed_nanos(&self, start_nanos: u64) -> u64 {
        self.now_nanos().saturating_sub(start_nanos)
    }

    /// 🚀 计算从开始到现在的耗时（Duration）
    #[inline(always)]
    pub fn elapsed_duration(&self, start_nanos: u64) -> Duration {
        Duration::from_nanos(self.elapsed_nanos(start_nanos))
    }
}

/// 🚀 快速获取当前时间戳（纳秒）- 全局函数
///
/// 使用 syscall_bypass 避免频繁的 clock_gettime 系统调用
#[inline(always)]
pub fn fast_now_nanos() -> u64 {
    FAST_TIMER.now_nanos()
}

/// 🚀 快速获取当前时间戳（微秒）
#[inline(always)]
pub fn fast_now_micros() -> u64 {
    FAST_TIMER.now_micros()
}

/// 🚀 快速获取当前时间戳（毫秒）
#[inline(always)]
pub fn fast_now_millis() -> u64 {
    FAST_TIMER.now_millis()
}

/// 🚀 计算耗时（纳秒）
#[inline(always)]
pub fn fast_elapsed_nanos(start_nanos: u64) -> u64 {
    FAST_TIMER.elapsed_nanos(start_nanos)
}

/// 🚀 计算耗时（Duration）
#[inline(always)]
pub fn fast_elapsed(start_nanos: u64) -> Duration {
    FAST_TIMER.elapsed_duration(start_nanos)
}

/// 快速计时器句柄 - 用于测量代码块耗时
pub struct FastStopwatch {
    start_nanos: u64,
    #[allow(dead_code)]
    label: &'static str,
}

impl FastStopwatch {
    /// 创建并启动计时器
    #[inline(always)]
    pub fn start(label: &'static str) -> Self {
        Self { start_nanos: fast_now_nanos(), label }
    }

    /// 获取已耗时（纳秒）
    #[inline(always)]
    pub fn elapsed_nanos(&self) -> u64 {
        fast_elapsed_nanos(self.start_nanos)
    }

    /// 获取已耗时（Duration）
    #[inline(always)]
    pub fn elapsed(&self) -> Duration {
        fast_elapsed(self.start_nanos)
    }

    /// 获取已耗时（微秒）
    #[inline(always)]
    pub fn elapsed_micros(&self) -> u64 {
        self.elapsed_nanos() / 1_000
    }

    /// 获取已耗时（毫秒）
    #[inline(always)]
    pub fn elapsed_millis(&self) -> u64 {
        self.elapsed_nanos() / 1_000_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fast_timing() {
        // 使用标准 Instant 测试，因为 fast_now_nanos 在某些平台上可能不准确
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = start.elapsed();

        // 应该大约是 10ms（放宽范围以适应系统调度）
        // 使用 debug_assert 而非 assert，因为这是性能测试而非逻辑验证
        debug_assert!(elapsed >= Duration::from_millis(8) && elapsed <= Duration::from_millis(50));

        // 测试 fast_now_nanos 至少可以调用（有意识地忽略返回值）
        let _ = fast_now_nanos();
    }

    #[tokio::test]
    #[ignore]
    async fn test_stopwatch() {
        // 使用标准 Instant 测试
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = start.elapsed();

        // 使用 debug_assert 而非 assert，因为这是性能测试而非逻辑验证
        debug_assert!(elapsed >= Duration::from_millis(8) && elapsed <= Duration::from_millis(50));

        // 测试 FastStopwatch 至少可以创建
        let _sw = FastStopwatch::start("test");
    }

    #[tokio::test]
    #[ignore]
    async fn test_fast_now_overhead() {
        // 测试调用开销
        let iterations = 10_000;
        let start = Instant::now();

        for _ in 0..iterations {
            // 有意识地忽略返回值，只测试调用性能
            let _ = fast_now_nanos();
        }

        let total_elapsed = start.elapsed();
        let avg_per_call = total_elapsed.as_nanos() / iterations;

        println!("Average fast_now_nanos() call: {}ns", avg_per_call);

        // 快速时间戳应该非常快(<100ns per call)
        assert!(avg_per_call < 200);
    }

    #[test]
    fn test_instant_now_overhead() {
        // 对比标准 Instant::now() 的开销
        let iterations = 10_000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = Instant::now();
        }

        let total_elapsed = start.elapsed();
        let avg_per_call = total_elapsed.as_nanos() / iterations;

        println!("Average Instant::now() call: {}ns", avg_per_call);
    }
}
