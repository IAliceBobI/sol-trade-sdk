# identify_quote_mint 使用示例

`identify_quote_mint` 函数用于从两个 mint 地址中识别出 quote mint（计价货币）。

## 功能说明

**优先级**：
1. USD 稳定币（USDC、USDT、USD1）- 最高优先级
2. SOL/WSOL - 次优先级
3. 其他 token - 返回 None

## 导入路径

```rust
use sol_trade_sdk::instruction::utils::pumpswap::identify_quote_mint;
use sol_trade_sdk::constants::{USDC_MINT, WSOL_TOKEN_ACCOUNT};
```

## 使用示例

### 示例 1：USDC vs WSOL

```rust
use sol_trade_sdk::instruction::utils::pumpswap::identify_quote_mint;
use sol_trade_sdk::constants::{USDC_MINT, WSOL_TOKEN_ACCOUNT};

// USDC vs WSOL - 返回 USDC（USD 优先级更高）
let quote = identify_quote_mint(&USDC_MINT, &WSOL_TOKEN_ACCOUNT);
assert_eq!(quote, Some(USDC_MINT));
```

### 示例 2：SOL vs 任意 Token

```rust
use sol_trade_sdk::instruction::utils::pumpswap::identify_quote_mint;
use sol_trade_sdk::constants::WSOL_TOKEN_ACCOUNT;
use solana_sdk::pubkey::Pubkey;

let random_token = Pubkey::new_unique();

// SOL vs 任意 token - 返回 SOL
let quote = identify_quote_mint(&WSOL_TOKEN_ACCOUNT, &random_token);
assert_eq!(quote, Some(WSOL_TOKEN_ACCOUNT));
```

### 示例 3：两个非主流 Token

```rust
use sol_trade_sdk::instruction::utils::pumpswap::identify_quote_mint;
use solana_sdk::pubkey::Pubkey;

let token_a = Pubkey::new_unique();
let token_b = Pubkey::new_unique();

// 两个非主流 token - 返回 None
let quote = identify_quote_mint(&token_a, &token_b);
assert_eq!(quote, None);
```

### 示例 4：USD1 vs SOL

```rust
use sol_trade_sdk::instruction::utils::pumpswap::identify_quote_mint;
use sol_trade_sdk::constants::{USD1_TOKEN_ACCOUNT, SOL_MINT};

// USD1 vs SOL - 返回 USD1（USD 优先级更高）
let quote = identify_quote_mint(&USD1_TOKEN_ACCOUNT, &SOL_MINT);
assert_eq!(quote, Some(USD1_TOKEN_ACCOUNT));
```

## 实际应用场景

### 场景 1：自动确定交易对方向

```rust
use sol_trade_sdk::instruction::utils::pumpswap::identify_quote_mint;

// 在构建交易时，自动识别哪个是 quote asset
let base_mint = get_base_mint();
let quote_mint = get_quote_mint();

// 如果方向反了，自动调整
if let Some(identified_quote) = identify_quote_mint(&base_mint, &quote_mint) {
    if identified_quote == base_mint {
        // 需要交换方向
        std::mem::swap(&mut base_mint, &mut quote_mint);
    }
}
```

### 场景 2：价格计算时的计价货币识别

```rust
use sol_trade_sdk::instruction::utils::pumpswap::identify_quote_mint;

// 在计算价格时，确定用哪个货币作为计价单位
let token_a_mint = pool.token_a_mint;
let token_b_mint = pool.token_b_mint;

match identify_quote_mint(&token_a_mint, &token_b_mint) {
    Some(quote_mint) => {
        // 使用 quote mint 计算价格
        let price = calculate_price_in_quote(pool, quote_mint);
    }
    None => {
        // 没有主流 quote asset，使用其他策略
        let price = calculate_price_raw(pool);
    }
}
```

## 支持的常量

| 常量 | Mint 地址 | 类型 |
|------|----------|------|
| `USDC_MINT` | EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v | USD 稳定币 |
| `USDT_MINT` | Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB | USD 稳定币 |
| `USD1_TOKEN_ACCOUNT` | USD1ttGY1N17NEEHLmELoaybftRBUSErhqYiQzvEmuB | USD 稳定币 |
| `SOL_MINT` | So11111111111111111111111111111111111111111 | SOL |
| `WSOL_TOKEN_ACCOUNT` | So11111111111111111111111111111111111111112 | Wrapped SOL |

## 注意事项

1. **优先级顺序**：USD > SOL/WSOL > 其他
2. **相同优先级**：如果两个 mint 优先级相同（如 USDC vs USDT），返回第一个参数
3. **返回 None**：当两个 mint 都不是主流 quote asset 时返回 None
4. **顺序无关**：函数会自动比较，无论传入顺序如何

## 相关函数

- `is_hot_mint` - 判断单个 mint 是否为主流资产
