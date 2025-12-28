# Project Coding Rules (Non-Obvious Only)

- 所有交易相关模块均使用 `Arc<T>` 进行共享所有权（见 `src/trading/`）
- 性能优化代码位于 `src/perf/`，包含编译器优化、内核旁路、SIMD 等
- SWQoS 多节点并发发送实现位于 `src/swqos/`
- 交易计算使用 `FastFn` 模式优化性能（见 `src/common/fast_fn.rs`）
- 指令生成遵循各协议特定模式（见 `src/instruction/utils/`）
