# 从 test.sh 迁移到 Makefile

本文档说明如何从 `test.sh` 迁移到使用 Makefile 进行测试。

## 原来的 test.sh

```bash
#!/bin/bash

unset RUST_BACKTRACE
cargo test --no-fail-fast
```

## 使用 Makefile 替代

### 快速对比

| 原 test.sh | Makefile 命令 | 说明 |
|---|---|---|
| `./test.sh` | `make test-legacy` | 完全等效（包括 unset RUST_BACKTRACE） |
| `./test.sh` | `make test-cargo` | 等效 cargo test --no-fail-fast |
| `./test.sh` | `make test` | 推荐：使用 Nextest（更快） |

### 推荐使用方式

```bash
# 1. 最简单的方式（Nextest，更快）
make test

# 2. 完全兼容 test.sh 的方式
make test-legacy

# 3. 传统 cargo test 方式
make test-cargo
```

### 为什么推荐使用 Nextest？

1. **更快的执行速度**: 智能并行化，速度提升约 50%
2. **更好的输出**: 清晰的彩色输出，易于阅读
3. **超时检测**: 自动识别慢速测试
4. **失败不终止**: `--no-fail-fast` 是默认行为

### 各种测试场景

#### 运行所有测试

```bash
# 使用 Nextest（推荐）
make test

# 使用传统 cargo test
make test-cargo

# 完全兼容 test.sh
make test-legacy
```

#### 运行特定测试文件

```bash
# Nextest 方式（更快）
make test-wsol           # 运行 wsol_tests
make test-parser         # 运行解析器测试
make test-pool           # 运行 Pool 查询测试

# 传统 cargo test 方式
make test-cargo-file wsol_tests
cargo test --test wsol_tests --no-fail-fast
```

#### 运行快速测试

```bash
# 仅运行快速单元测试（10秒超时）
make test-fast
```

#### 列出所有测试

```bash
make list
```

### 迁移步骤

1. **直接替换**: 将 `./test.sh` 替换为 `make test`（使用 Nextest）
2. **保持兼容**: 如需完全兼容原行为，使用 `make test-legacy`
3. **逐步迁移**: 先确保 `make test-cargo` 正常工作，再切换到 `make test`

### 性能对比

在 sol-trade-sdk 项目中的实际测试时间：

```bash
# 原来的 test.sh（cargo test）
./test.sh              # 约 120-180 秒

# 使用 Nextest
make test              # 约 60-90 秒（提升 50%）

# 运行快速测试
make test-fast         # 约 10-20 秒
```

### 完整的 Makefile 命令列表

查看所有可用命令：

```bash
make help
```

输出：

```
sol-trade-sdk 测试命令（推荐使用 Nextest）

快速测试:
  make test-fast               运行快速单元测试（10秒超时）

完整测试:
  make test-all                运行所有测试（Nextest，推荐）
  make test                    运行所有测试（Nextest，推荐）
  make test-cargo              运行所有测试（传统 cargo test）
  make test-legacy             运行所有测试（test.sh 等效命令）

分类测试:
  make test-parser             运行交易解析器测试
  make test-pool               运行 Pool 查询测试
  make test-wsol               运行 WSOL 相关测试
  make test-seed               运行 Seed 优化测试
  make test-raydium            运行 Raydium 相关测试
  make test-pumpswap           运行 PumpSwap 相关测试

调试命令:
  make list                    列出所有测试
  make clean                   清理测试缓存
  make show-slow               显示慢速测试

CI 测试:
  make test-ci                 运行 CI 环境测试（更严格）
```

### 废弃 test.sh

迁移完成后，可以安全地删除 test.sh：

```bash
# 确认 make test 工作正常后
rm test.sh

# 或者保留作为备份
mv test.sh test.sh.bak
```

## 总结

- **推荐**: 使用 `make test`（Nextest，更快更好）
- **兼容**: 使用 `make test-legacy`（完全兼容 test.sh）
- **灵活**: 使用 `make test-xxx` 运行特定类型的测试

开始享受 Nextest 带来的速度提升吧！
