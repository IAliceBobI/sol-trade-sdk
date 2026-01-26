# Cargo Nextest 使用指南

本文档介绍如何在 sol-trade-sdk 项目中使用 `cargo nextest` - 一个更快的 Rust 测试运行器。

## 安装 Nextest

```bash
cargo install cargo-nextest
```

## 基本用法

### 运行所有测试

```bash
cargo nextest run
```

### 运行特定测试

```bash
# 运行特定测试
cargo nextest run test_seed_format

# 运行特定包的测试
cargo nextest run -p sol-trade-sdk

# 运行特定测试文件的所有测试
cargo nextest run wsol_tests
```

### 列出所有测试

```bash
cargo nextest list
```

### 仅编译不运行

```bash
cargo nextest run --no-run
```

## 配置文件

Nextest 配置位于 `.config/nextest.toml`，包含：

### 测试超时配置

不同类型的测试有不同的超时限制：

- **快速单元测试**: 10秒（解析器测试、discriminator 测试等）
- **Pool 查询测试**: 45秒（可能涉及网络请求）
- **交易解析器综合测试**: 60秒
- **实际交易测试**: 90秒（CLMM、CPMM、AMM V4 买卖测试）
- **兜底配置**: 60秒后标记为慢速，5次超时后终止

### Profile 配置

- **default**: 默认开发环境配置
- **ci**: CI 环境配置（更严格，30秒超时）

## 常用命令对比

| 传统 cargo test | cargo nextest | 说明 |
|---|---|---|
| `cargo test` | `cargo nextest run` | 运行所有测试 |
| `cargo test --test wsol_tests` | `cargo nextest run wsol_tests` | 运行特定测试文件 |
| `cargo test --package sol-trade-sdk` | `cargo nextest run -p sol-trade-sdk` | 运行特定包 |
| N/A | `cargo nextest list` | 列出所有测试 |

## Nextest 优势

1. **更快的测试执行**: 智能并行化，显著提升测试速度
2. **更好的输出**: 清晰的测试结果显示，支持彩色输出
3. **超时检测**: 自动识别慢速测试，帮助优化性能
4. **测试重试**: 可配置失败重试机制
5. **智能跳过**: 基于依赖变更智能跳过测试

## 测试分类

当前项目测试分为以下几类：

### 1. 快速单元测试（10秒）
- 解析器单元测试
- Discriminator 测试
- 配置测试

### 2. Pool 查询测试（45秒）
- 各 DEX 的 Pool 查询功能
- 可能涉及网络请求

### 3. 交易解析器测试（30-60秒）
- 交易解析器综合测试
- 真实交易解析测试

### 4. 实际交易测试（90秒）
- CLMM 买卖测试
- CPMM 买卖测试
- AMM V4 买卖测试

## 性能对比

使用 nextest 运行所有测试的性能对比（参考值）：

```bash
# 传统 cargo test
cargo test  # 约 120-180 秒

# 使用 nextest
cargo nextest run  # 约 60-90 秒（提升约 50%）
```

## 故障排查

### 测试超时

如果测试超时，可以：

1. 检查网络连接（Pool 查询测试）
2. 增加特定测试的超时时间（编辑 `.config/nextest.toml`）
3. 查看测试日志定位慢速操作

### 慢速测试识别

Nextest 会在输出中标记慢速测试（标记为 `[SLOW]`），帮助你优化性能。

## 与 CI 集成

在 CI 环境中使用更严格的配置：

```bash
cargo nextest run --profile ci
```

这会使用更短的超时和更严格的失败处理。

## 参考资源

- [Nextest 官方文档](https://nexte.st/)
- [Nextest GitHub](https://github.com/nextest-rs/nextest)
