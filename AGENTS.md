# AGENTS.md

Guidelines for AI agents working on **Sol Trade SDK** - a Rust SDK for Solana DEX trading.

## Project Overview

- **Version**: 4.0.0
- **Language**: Rust Edition 2024
- **Dependencies**: solana-sdk 3.0.x (critical - must not change)
- **Test Node**: Local `127.0.0.1:8899` (surfpool, forked from mainnet)
- **Important**: Never use `--release` for testing (too slow)

## Build & Test Commands

```bash
# Build
cargo build
cargo check          # Fast type check

# Testing (use nextest, NOT cargo test)
cargo nextest run                          # Run all tests
cargo nextest run <test_name>              # Run single test
cargo nextest run raydium_clmm_pool_tests  # Example: specific test

# Using Makefile
make test          # All tests
make test-fast     # Quick unit tests only
make list          # List all tests

# Code quality
cargo fmt
cargo clippy
```

## Code Style Guidelines

### Language & Comments
- **Comments**: Use Chinese for all comments and documentation
- **Doc comments**: Use `///` for public API, `//!` for module-level docs

### Imports (Grouped & Ordered)
```rust
// 1. Standard library
use std::sync::Arc;

// 2. External crates (alphabetical)
use anyhow::{Result, anyhow};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

// 3. Internal crate imports
use crate::trading::core::params::SwapParams;
use crate::constants::DexProtocol;
```

### Formatting
- **Max width**: 100 characters (`rustfmt.toml`)
- **Indent**: 4 spaces (no tabs)
- **Edition**: 2024
- Run `cargo fmt` before committing

### Naming Conventions
- **Types (structs/enums/traits)**: `PascalCase` - `TradeExecutor`, `DexType`
- **Functions/variables**: `snake_case` - `build_buy_instructions()`, `pool_address`
- **Constants**: `SCREAMING_SNAKE_CASE` - `SOL_MINT`, `DEFAULT_SLIPPAGE`
- **Modules**: `snake_case` - `raydium_cpmm`, `gas_fee_strategy`
- **Type aliases**: `CamelCase` - `GasStrategyKey`, `GasStrategyMap`

### Types & Math
- **Token calculations**: Use `u128` for precision, never `f64` or `u64`
- **Results**: Use `anyhow::Result<T>` or custom error types
- **Options**: Prefer explicit error handling over `unwrap()` in production code
- **Overflow**: Use `checked_add`, `checked_mul` etc. with explicit error handling

### Error Handling
```rust
// Preferred: Use anyhow for simple errors
use anyhow::{Result, anyhow};

// For complex errors, define custom error types
#[derive(Debug, thiserror::Error)]
pub enum TradeError {
    #[error("Insufficient balance: {0}")]
    InsufficientBalance(String),
    #[error("Pool not found: {0}")]
    PoolNotFound(Pubkey),
}
```

### File Organization
- **Max lines**: Split modules when files exceed 800 lines
- **Module structure**: `mod.rs` or module files with `pub use` re-exports
- **Tests**: Inline `#[cfg(test)]` modules for unit tests, separate `tests/` for integration

## Architecture Overview

```
src/
├── client/           # TradingClient - main user-facing API
├── trading/          # Trading engine (factory, core traits, executor)
├── instruction/      # DEX-specific instruction builders (7 DEXs)
├── swqos/            # MEV service clients (Jito, Bloxroute, etc.)
├── common/           # Shared utilities (gas, cache, detectors)
├── utils/calc/       # Math calculations per DEX
├── parser/           # Transaction parser
└── liquidity/        # Liquidity management
```

### Key Design Patterns

1. **Factory + LazyLock Singleton** (`src/trading/factory.rs`):
```rust
pub enum DexType { PumpFun, PumpSwap, RaydiumCpmm, RaydiumAmmV4, RaydiumClmm, Bonk, MeteoraDammV2 }
let executor = TradeFactory::create_executor(DexType::PumpFun);
```

2. **Type-Safe Parameters** (`src/trading/core/params.rs`):
```rust
pub enum DexParamEnum {
    PumpFun(PumpFunParams),
    PumpSwap(PumpSwapParams),
    // ...
}
```

3. **Shared Infrastructure** (`src/infrastructure.rs`):
```rust
let infra = Arc::new(TradingInfrastructure::new(config).await);
let client = TradingClient::from_infrastructure(payer, infra.clone(), true);
```

## Testing

- **Framework**: cargo nextest (configured in `.config/nextest.toml`)
- **Timeout**: 60s default, 90s for transaction tests
- **Retries**: 2 for flaky network tests
- **Serial tests**: Some tests marked `#[serial_test::serial]` - must run alone

### Test Utilities
```rust
use sol_trade_test_utils::{ensure_sol_balance, ensure_token_balance};
ensure_sol_balance(&rpc, "http://127.0.0.1:8899", &payer.pubkey(), 10).await?;
```

## Debugging

View Program Logs via Solscan:
```
https://solscan.io/tx/<TX_SIGNATURE>?cluster=custom&customUrl=http://127.0.0.1:8899
```

Common failures:
- **CU exceeded**: Check `GasFeeStrategy.compute_unit_limit`
- **Slippage**: Transaction reverts without fee consumption
- **Account not found**: Wrong pool address or mint

## Critical File Locations

| Component | Location |
|-----------|----------|
| `DexType` enum | `src/trading/factory.rs:14` |
| `DexParamEnum` | `src/trading/core/params.rs` |
| `GasFeeStrategy` | `src/common/gas_fee_strategy.rs` |
| `InstructionBuilder` trait | `src/trading/core/traits.rs` |
| DEX Program IDs | `src/constants/dex_protocols.rs` |
| Token addresses | `src/constants/tokens.rs` |

## External References

DEX source code in `./temp/dex/` for reference:
- Raydium CPMM: `./temp/dex/raydium-cp-swap`
- Raydium AMM V4: `./temp/dex/raydium-amm`
- Raydium CLMM: `./temp/dex/raydium-clmm`

## MCP Services

- **context7**: Dependency library docs
- **solana-mcp-server**: Solana expertise
- **surfpool**: Local test node queries
