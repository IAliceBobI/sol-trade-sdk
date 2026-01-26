# Makefile for sol-trade-sdk with cargo nextest support
# 使用方法: make <target>

.PHONY: help test test-all test-fast test-slow list clean test-cargo test-legacy

# 默认目标
.DEFAULT_GOAL := help

# 帮助信息
help:
	@echo "sol-trade-sdk 测试命令（推荐使用 Nextest）"
	@echo ""
	@echo "快速测试:"
	@echo "  make test-fast               运行快速单元测试（10秒超时）"
	@echo ""
	@echo "完整测试:"
	@echo "  make test-all                运行所有测试（Nextest，推荐）"
	@echo "  make test                    运行所有测试（Nextest，推荐）"
	@echo "  make test-cargo              运行所有测试（传统 cargo test）"
	@echo "  make test-legacy             运行所有测试（test.sh 等效命令）"
	@echo ""
	@echo "分类测试:"
	@echo "  make test-parser             运行交易解析器测试"
	@echo "  make test-pool               运行 Pool 查询测试"
	@echo "  make test-wsol               运行 WSOL 相关测试"
	@echo "  make test-seed               运行 Seed 优化测试"
	@echo "  make test-raydium            运行 Raydium 相关测试"
	@echo "  make test-pumpswap           运行 PumpSwap 相关测试"
	@echo ""
	@echo "调试命令:"
	@echo "  make list                    列出所有测试"
	@echo "  make clean                   清理测试缓存"
	@echo "  make show-slow               显示慢速测试"
	@echo ""
	@echo "CI 测试:"
	@echo "  make test-ci                 运行 CI 环境测试（更严格）"

# 运行所有测试
test-all: test
test:
	@echo "运行所有测试..."
	cargo nextest run

# 运行快速单元测试
test-fast:
	@echo "运行快速单元测试..."
	cargo nextest run \
		'test(test_parse|test_error|test_config|test_discriminator|test_adapter|test_seed)'

# 运行交易解析器测试
test-parser:
	@echo "运行交易解析器测试..."
	cargo nextest run \
		'dex_parser_comprehensive|dex_parser_real_tx|dex_parser_unit|tx_adapter_tests|parser_discriminator'

# 运行 Pool 查询测试
test-pool:
	@echo "运行 Pool 查询测试..."
	cargo nextest run \
		'test(test_pool_|test_find_pool|test_get_pool|pumpswap_pool_tests|raydium_clmm_pool_tests|raydium_amm_v4_pool_tests)'

# 运行 WSOL 相关测试
test-wsol:
	@echo "运行 WSOL 相关测试..."
	cargo nextest run 'wsol_tests'

# 运行 Seed 优化测试
test-seed:
	@echo "运行 Seed 优化测试..."
	cargo nextest run 'seed_optimize_tests'

# 运行 Raydium 相关测试
test-raydium:
	@echo "运行 Raydium 相关测试..."
	cargo nextest run \
		'raydium_clmm|raydium_cpmm|raydium_amm_v4|raydium_api_pool_tests'

# 运行 PumpSwap 相关测试
test-pumpswap:
	@echo "运行 PumpSwap 相关测试..."
	cargo nextest run 'pumpswap'

# CI 环境测试
test-ci:
	@echo "运行 CI 环境测试（严格配置）..."
	cargo nextest run --profile ci

# 列出所有测试
list:
	@echo "列出所有测试..."
	cargo nextest list

# 清理测试缓存
clean:
	@echo "清理测试缓存..."
	cargo clean
	rm -rf target/nextest

# 显示慢速测试
show-slow:
	@echo "识别慢速测试..."
	cargo nextest run --status-level=slow 2>&1 | grep -A 5 "SLOW"

# ============================================================================
# 传统 cargo test 命令（兼容性）
# ============================================================================

# 使用传统 cargo test 运行所有测试
test-cargo:
	@echo "使用传统 cargo test 运行所有测试..."
	cargo test --no-fail-fast

# test.sh 的等效命令（完全兼容原 test.sh）
test-legacy:
	@echo "运行 test.sh 等效命令..."
	@unset RUST_BACKTRACE && cargo test --no-fail-fast

# 带输出信息的传统测试
test-cargo-verbose:
	@echo "使用传统 cargo test 运行所有测试（详细输出）..."
	cargo test --no-fail-fast -- --nocapture --test-threads=1

# 运行特定测试文件的传统方式
test-cargo-file%:
	@echo "运行特定测试文件: $*"
	cargo test --test $* --no-fail-fast
