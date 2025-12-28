# AGENTS.md

本文档为在此代码库中工作的智能体提供指导。

## 构建与测试命令

```bash
# 运行单个测试（debug 模式）
cargo test --debug test_name

# 构建并运行示例
cargo run --release -p example_name
```

## 关键项目结构

- 示例程序位于 `examples/` 目录下，每个示例都是独立的工作区成员
- **新增示例需要手动添加到 `Cargo.toml` 的 `workspace.members` 中**
- 性能追踪特性 `perf-trace` 在生产环境应禁用以获得最佳性能

## 核心编码规则

所有交易相关模块均使用 `Arc<T>` 进行共享所有权（见 `src/trading/`）。
性能优化代码位于 `src/perf/`，包含编译器优化、内核旁路、SIMD 等。
SWQoS 多节点并发发送实现位于 `src/swqos/`。
交易计算使用 `FastFn` 模式优化性能（见 `src/common/fast_fn.rs`）。
指令生成遵循各协议特定模式（见 `src/instruction/utils/`）。
