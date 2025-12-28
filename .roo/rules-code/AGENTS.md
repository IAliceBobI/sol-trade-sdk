# Project Coding Rules (Non-Obvious Only)

- 所有交易相关模块均使用 `Arc<T>` 进行共享所有权（见 `src/trading/`）
- 性能优化代码位于 `src/perf/`，包含编译器优化、内核旁路、SIMD 等
- SWQoS 多节点并发发送实现位于 `src/swqos/`
- 交易计算使用 `FastFn` 模式优化性能（见 `src/common/fast_fn.rs`）
- 指令生成遵循各协议特定模式（见 `src/instruction/utils/`）
- TradeFactory 使用 `LazyLock` 静态单例，修改后需重启程序
- FastFn 返回 `Arc<Vec<Instruction>>` 避免热路径中的克隆开销
- SWQoS 并发返回 `(bool, Vec<Signature>, Option<Error>)`，需检查部分成功场景
- 缓存键使用 Pubkey 部分字节哈希优化，避免完整克隆
