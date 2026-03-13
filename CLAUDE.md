# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) for working with code in this repository.

## Project Overview

**Sol Trade SDK** - Rust SDK for Solana DEX trading, supporting 7 DEX protocols.

- **Language**: Rust Edition 2024
- **Solana**: 3.0.x (uses solana-sdk 3.0.0)
- **Test Node**: Local `127.0.0.1:8899` (surfpool, forked from mainnet)
- **Important**: Use `cargo nextest run` (NOT `cargo test`), never `--release` for testing

## Supported DEX

| DEX | DexType |
|-----|---------|
| PumpFun | `DexType::PumpFun` |
| PumpSwap | `DexType::PumpSwap` |
| Raydium CPMM | `DexType::RaydiumCpmm` |
| Raydium AMM V4 | `DexType::RaydiumAmmV4` |
| Raydium CLMM | `DexType::RaydiumClmm` |
| Meteora DAMM V2 | `DexType::MeteoraDammV2` |
| Bonk | `DexType::Bonk` |

## Architecture

```
src/
├── client/           # TradingClient - 主用户 API
├── trading/          # 交易引擎 (factory.rs, core/, middleware/)
├── instruction/      # DEX 指令构建器
├── swqos/            # MEV 服务客户端 (Jito, Bloxroute)
├── common/           # 共享工具 (gas_fee_strategy.rs, dex_detector.rs)
├── utils/calc/       # 数学计算 (u128 精确运算)
└── parser/           # 交易解析器
```

### Key Files

| 组件 | 位置 |
|------|------|
| DexType enum | `src/trading/factory.rs` |
| InstructionBuilder | `src/trading/core/traits.rs` |
| DexParamEnum | `src/trading/core/params.rs` |
| GasFeeStrategy | `src/common/gas_fee_strategy.rs` |
| DEX Program IDs | `src/constants/dex_protocols.rs` |

### Design Patterns

```rust
// 1. Factory 单例
let executor = TradeFactory::create_executor(DexType::PumpFun);

// 2. 共享基础设施
let infra = Arc::new(TradingInfrastructure::new(config).await);
let client = TradingClient::from_infrastructure(payer, infra, true);

// 3. InstructionBuilder trait
pub trait InstructionBuilder: Send + Sync {
    async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>>;
    async fn build_sell_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>>;
}
```

## Commands

```bash
cargo build          # 构建
cargo nextest run    # 运行测试（必须用 nextest）
cargo fmt            # 格式化
make show-all        # 查看 DEX 错误统计
```

## Testing

```rust
use sol_trade_test_utils::{ensure_sol_balance, ensure_token_balance, get_simulation_test_keypair};

// 确保 SOL/Token 余额（必要时空投/铸造）
ensure_sol_balance(&rpc, "http://127.0.0.1:8899", &payer.pubkey(), 10).await?;
ensure_token_balance(&rpc, "http://127.0.0.1:8899", &payer, &mint, "1000").await?;

// 预充值测试密钥对（已有 10 SOL）
let payer = get_simulation_test_keypair();
// 地址: 8be6dbPmZH1URHXyFTbY876QuVunrD8wTZhHGXjEdrvj
```

## Debugging

本地交易失败查看: `https://solscan.io/tx/<SIGNATURE>?cluster=custom&customUrl=http://127.0.0.1:8899`

常见失败:
- **CU exceeded**: 增加 `GasFeeStrategy.compute_unit_limit`
- **Slippage**: 增加 `slippage_basis_points`

## Code Style

- 注释用中文
- 数学计算用 u128（不用 f64）
- 文件超过 800 行拆分模块
- 引用官方代码用绝对路径: `./temp/dex/...`
