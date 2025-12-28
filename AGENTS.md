# AGENTS.md

本文档为在此代码库中工作的智能体提供指导。

## 项目概述

本项目是 Solana 区块链交易 SDK，使用 Rust 语言编写，支持 PumpFun、PumpSwap、Raydium、BONK、Meteora 等 DEX 协议的交易功能。

## 构建与测试命令

```bash
# 运行单个测试（debug 模式）
cargo test --debug test_name

# 构建并运行示例
cargo run --release -p example_name
```

## 开发注意事项

- 示例程序位于 `examples/` 目录下，每个示例都是独立的工作区成员
- **新增示例需要手动添加到 `Cargo.toml` 的 `workspace.members` 中**
- 性能追踪特性 `perf-trace` 在生产环境应禁用以获得最佳性能
- 所有交易相关模块均使用 `Arc<T>` 进行共享所有权
