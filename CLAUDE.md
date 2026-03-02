# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Sol Trade SDK** is a Rust SDK for Solana DEX trading, supporting 7 DEX protocols and multiple MEV protection services.

- **Version**: 4.0.0
- **Language**: Rust Edition 2024
- **Solana Dependencies**: 3.0.x (critical: uses solana-sdk 3.0.0)
- **Test Node**: Local `127.0.0.1:8899` (surfpool, forked from Solana mainnet)
- **Important**: Never use `--release` for testing (too slow); use `cargo nextest` instead of `cargo test`

## Common Commands

```bash
# Build (development)
cargo build

# Type check (fast)
cargo check

# Run all tests (use nextest, not cargo test)
cargo nextest run

# Run specific test
cargo nextest run <test_name>

# Run test with output
cargo test --test <test_name> -- --nocapture

# Using Makefile
make test              # Run all tests
make test-fast         # Quick unit tests
make list              # List all tests

# Format code
cargo fmt

# Lint
cargo clippy
```

## High-Level Architecture

### Module Structure

```
src/
├── client/           # TradingClient - main user-facing API
│   ├── trading.rs    # buy/sell implementation
│   ├── quote.rs      # Unified quote interface
│   ├── simulation.rs # On-chain simulation
│   └── constructor.rs # Builder pattern (SolanaTrade)
├── trading/          # Trading engine
│   ├── factory.rs    # TradeFactory - creates DexType executors
│   ├── core/         # Core traits and executor
│   │   ├── traits.rs # TradeExecutor, InstructionBuilder
│   │   ├── executor.rs # GenericTradeExecutor
│   │   └── params.rs # DexParamEnum (type-safe params)
│   └── middleware/   # Middleware system
├── instruction/      # DEX-specific instruction builders
│   ├── pumpfun.rs, pumpswap.rs, bonk.rs
│   ├── raydium_amm_v4.rs, raydium_cpmm.rs, raydium_clmm.rs
│   └── meteora_damm_v2.rs
├── swqos/            # MEV service clients (Jito, Bloxroute, etc.)
├── common/           # Shared utilities
│   ├── dex_detector.rs    # Pool -> DEX detection
│   ├── dex_pool_cache.rs  # Pool data caching
│   └── gas_fee_strategy.rs # Fee configuration
├── utils/calc/       # Math calculations
│   ├── clmm_math/    # Raydium CLMM math
│   ├── raydium_*.rs  # Quote calculations per DEX
│   └── pumpfun.rs, pumpswap.rs, bonk.rs
├── parser/           # Transaction parser
└── liquidity/        # Liquidity management (CPMM deposit)
```

### Key Design Patterns

#### 1. Factory + LazyLock Singleton (`src/trading/factory.rs`)

Zero-cost singleton pattern for executor instances:

```rust
pub enum DexType { PumpFun, PumpSwap, RaydiumCpmm, RaydiumAmmV4, RaydiumClmm, Bonk, MeteoraDammV2 }

// Usage: creates executor once, clones Arc thereafter
let executor = TradeFactory::create_executor(DexType::PumpFun);
```

#### 2. Type-Safe Parameters (`src/trading/core/params.rs`)

Zero-overhead abstraction using enums:

```rust
pub enum DexParamEnum {
    PumpFun(PumpFunParams),
    PumpSwap(PumpSwapParams),
    RaydiumCpmm(RaydiumCpmmParams),
    // ...
}
```

#### 3. Shared Infrastructure (`src/infrastructure.rs`)

Multiple wallets share expensive resources (RPC, SWQOS clients):

```rust
let infra = Arc::new(TradingInfrastructure::new(config).await);
let client1 = TradingClient::from_infrastructure(payer1, infra.clone(), true);
let client2 = TradingClient::from_infrastructure(payer2, infra.clone(), true);
```

#### 4. InstructionBuilder Trait (`src/trading/core/traits.rs`)

Each DEX implements instruction building; GenericTradeExecutor handles execution:

```rust
pub trait InstructionBuilder: Send + Sync {
    async fn build_buy_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>>;
    async fn build_sell_instructions(&self, params: &SwapParams) -> Result<Vec<Instruction>>;
}
```

## Critical File Locations

| Component | Location |
|-----------|----------|
| **DexType enum** | `src/trading/factory.rs:14` |
| **DexParamEnum** | `src/trading/core/params.rs` |
| **GasFeeStrategy** | `src/common/gas_fee_strategy.rs` |
| **TradeFactory** | `src/trading/factory.rs` |
| **InstructionBuilder** | `src/trading/core/traits.rs` |
| **DEX Program IDs** | `src/constants/dex_protocols.rs` |
| **Token addresses** | `src/constants/tokens.rs` |

## Testing

### Test Configuration

- Uses `cargo nextest` with config in `.config/nextest.toml`
- Test timeout: 60s default (varies by test type)
- Retries: 2 for flaky network tests
- Some tests marked `#[serial_test::serial]` - must run alone

### Key Test Patterns

```bash
# DEX-specific tests
cargo nextest run raydium_clmm_pool_tests
cargo nextest run raydium_cpmm_buy_sell_tests
cargo nextest run pumpswap_pool_tests

# Quote verification tests
cargo nextest run verify_*

# Parser tests
cargo nextest run dex_parser_comprehensive
```

### Test Utilities (`sol-trade-test-utils/`)

```rust
use sol_trade_test_utils::{airdrop_and_wait, ensure_sol_balance, ensure_token_balance};
use sol_trade_test_utils::get_simulation_test_keypair;

// Ensure balance for tests
ensure_sol_balance(&rpc, "http://127.0.0.1:8899", &payer.pubkey(), 10).await?;
ensure_token_balance(&rpc, "http://127.0.0.1:8899", &payer, &mint, "1000").await?;
```

## Debugging Failed Transactions

Use Solscan with local cluster to view Program Logs:

```
https://solscan.io/tx/<TX_SIGNATURE>?cluster=custom&customUrl=http://127.0.0.1:8899
```

Common failures:
- **CU exceeded**: Check `GasFeeStrategy.compute_unit_limit`
- **Slippage**: Transaction reverts without fee consumption
- **Account not found**: Wrong pool address or mint

## External Code References

Official DEX source code cloned to `./temp/` for reference:

| DEX | Path |
|-----|------|
| Raydium CPMM | `./temp/dex/raydium-cp-swap` |
| Raydium AMM V4 | `./temp/dex/raydium-amm` |
| Raydium CLMM | `./temp/dex/raydium-clmm` |

When referencing official code in comments, use absolute paths like `./temp/dex/raydium-clmm/programs/amm/src/instructions/swap.rs`.

## Code Style Guidelines

- **Language**: Use Chinese for comments and documentation
- **File size**: Split modules when files exceed 800 lines
- **Math**: Use precise integer arithmetic (u128), never f64/u64 for token calculations
- **Error handling**: Use `anyhow::Result` or custom error types
- **Formatting**: `max_width = 100`, Rust Edition 2024

## MCP Services Available

- **context7**: Dependency library docs (solana-sdk, spl-token, etc.)
- **solana-mcp-server**: Solana expertise
- **surfpool**: Local test node queries
- **browser-mcp**: Solscan transaction inspection

## Documentation Index

- `docs/Gas费策略.md` - Gas fee configuration
- `docs/Nonce使用指南.md` - Durable nonce usage
- `docs/地址查找表.md` - Address lookup tables
- `docs/DEX_AND_POOL_REFERENCE.md` - Pool addresses for testing
- `docs/txs.md` - Real transaction signatures for debugging
