# Project Architecture Rules (Non-Obvious Only)

- 交易执行器使用 `Arc<dyn TradeExecutor>` 单例模式，通过 `LazyLock` 在编译时静态初始化，实现零运行时开销
- 指令构建遵循策略模式，每个 DEX 协议有独立的 `InstructionBuilder` 实现
- FastFn 模块使用 `DashMap` 实现无锁缓存，返回 `Arc<Vec<Instruction>>` 避免克隆开销
- SWQoS 支持多节点并发发送，交易结果返回 `(bool, Vec<Signature>, Option<Error>)` 以支持部分成功场景
- 性能优化层 (`src/perf/`) 与业务逻辑分离，包含 SIMD/AVX2 内存操作、分支预测优化
- 中间件系统 (`src/trading/middleware/`) 支持指令处理链式扩展
