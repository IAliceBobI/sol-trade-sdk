# AGENTS.md

本文档为在此代码库中工作的智能体提供指导。

## 项目概述

本项目是 Solana 区块链交易 SDK，使用 Rust 语言编写，支持 PumpFun、PumpSwap、Raydium、BONK、Meteora 等 DEX 协议的交易功能。

## 构建与测试命令

```bash
# 构建项目
cargo build --release

# 运行所有测试
cargo test

# 运行单个测试（debug 模式）
cargo test --debug test_name

# 构建并运行示例
cargo run --release -p example_name
```

## 代码风格规范

- 缩进：使用 4 个空格
- 最大行宽：100 字符
- Rust 版本：2021
- 遵循 `rustfmt.toml` 配置

## 项目结构说明

```
src/
├── common/          # 通用工具模块（地址查找、Gas 策略、Nonce 缓存等）
├── constants/       # 常量定义（账户地址、精度、交易参数等）
├── instruction/     # DEX 协议指令生成（各协议的交易指令）
├── perf/            # 性能优化模块（编译器优化、内核旁路等）
├── swqos/           # Solana RPC 优化与多节点并发发送
├── trading/         # 交易核心模块（工厂模式、执行器、中间件系统）
└── utils/           # 工具模块（价格计算、代币信息等）
```

## 开发注意事项

- 示例程序位于 `examples/` 目录下，每个示例都是独立的工作区成员
- 新增示例需要手动添加到 `Cargo.toml` 的 `workspace.members` 中
- 性能追踪特性 `perf-trace` 在生产环境应禁用
- 所有交易相关模块均使用 `Arc<T>` 进行共享所有权
