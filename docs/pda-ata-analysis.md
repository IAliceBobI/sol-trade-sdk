# PDA/ATA 计算与 Seed 优化

本文档分析 SDK 中 PDA (Program Derived Address) 和 ATA (Associated Token Account) 的计算逻辑，以及 Seed 优化的技术原理。

---

## 1. 基础概念

### 1.1 PDA (Program Derived Address)

**PDA 是看起来像公钥但没有对应私钥的地址**，由程序通过 `find_program_address` 派生得出。

```rust
// PumpFun 的 Bonding Curve PDA
let (bonding_curve, _bump) = Pubkey::find_program_address(
    &[b"bonding-curve", mint.as_ref()],
    &PUMP_FUN_PROGRAM_ID
);
```

**特性**：
- 地址落在 Ed25519 曲线外 → 无私钥，任何人无法控制
- 种子可以是：公钥、字符串、字节数组等
- 用途：存储程序状态、为 CPI 签名

### 1.2 ATA (Associated Token Account)

**ATA 是 PDA 的一种特殊形式**，专门用于 Token 账户，种子组合被标准化为：

```
[owner公钥, TOKEN_PROGRAM_ID, mint地址]
```

```rust
ATA = Pubkey::find_program_address(
    &[&wallet.to_bytes(), &token_program.to_bytes(), &mint.to_bytes()],
    &ASSOCIATED_TOKEN_PROGRAM_ID
)
```

### 1.3 PDA vs ATA

| 特性 | PDA | ATA |
|------|-----|-----|
| 是否有私钥 | 无 | 无 |
| 种子组合 | 自定义 | 固定：owner + token_program + mint |
| 用途 | 通用（存储状态、签名） | 专门存储 Token |

---

## 2. ATA 计算详解

SDK 提供两种 ATA 计算方式：**标准方式** 和 **Seed 优化方式**。

### 2.1 标准方式

使用 Solana 官方的 Associated Token Program：

```rust
// spl_associated_token_account.rs:47-71
Instruction {
    program_id: ASSOCIATED_TOKEN_PROGRAM_ID,
    accounts: vec![
        AccountMeta::new(*funding_address, true),  // 🔴 需要签名
        AccountMeta::new(associated_account_address, false),
        AccountMeta::new_readonly(*wallet_address, false),
        AccountMeta::new_readonly(*token_mint_address, false),
        AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
        AccountMeta::new_readonly(*token_program, false),
    ],
    data: vec![1],  // 1 = 幂等创建
}
```

**特点**：
- 1 条指令完成创建和初始化
- `funding_address` 必须签名（`signer = true`）
- 调用 Associated Token Program

### 2.2 Seed 优化方式

使用 System Program 的 `create_account_with_seed`：

```rust
// seed.rs:83-84
let create_acc = create_account_with_seed(
    payer, &ata_like, owner, seed, rent, len, token_program
);
```

**Seed 生成算法**（`seed.rs:64-76`）：

```rust
// 使用 FNV 哈希（比默认 hasher 快 2-3 倍）
let mut hasher = FnvHasher::default();
hasher.write(mint.as_ref());
let hash = hasher.finish();

// 截断为 32 位，转为 8 字符十六进制
let v = (hash & 0xFFFF_FFFF) as u32;
for i in 0..8 {
    let nibble = ((v >> (28 - i * 4)) & 0xF) as u8;
    buf[i] = match nibble {
        0..=9 => b'0' + nibble,           // 0-9 → '0'-'9'
        _ => b'a' + (nibble - 10),         // 10-15 → 'a'-'f'
    };
}
let seed = unsafe { std::str::from_utf8_unchecked(&buf) };

let ata_like = Pubkey::create_with_seed(payer, seed, token_program)?;
```

**返回两条指令**：
1. `create_account_with_seed` - 创建账户（系统程序）
2. `initialize_account3` - 初始化账户（Token 程序）

---

## 3. Seed 优化原理

### 3.1 为什么无需 Payer 签名？

这是 **System Program** 和 **Associated Token Program** 两个不同程序的设计差异：

| | 标准方式 | Seed 方式 |
|---|---|---|
| **指令来源** | Associated Token Program | System Program |
| **签名要求** | 强制 `funding_address` 签名 | **不要求签名** |
| **授权方式** | 显式签名授权 | PDA 派生关系隐含授权 |

**System Program 内部逻辑**：

```
┌─────────────────────────────────────────────────────────┐
│  System Program: create_account_with_seed               │
│                                                         │
│  1. 计算 PDA = Base + Seed + OwnerProgram               │
│     （确定性计算，任何人都可以算）                        │
│                                                         │
│  2. 从 Base 账户原子扣除 lamports                        │
│     （运行时自动处理，无需签名）                          │
│                                                         │
│  3. 创建目标账户，owner = token_program                  │
└─────────────────────────────────────────────────────────┘
```

**关键点**：PDA 派生的确定性 + System Program 的内置授权 = 无需额外签名

### 3.2 性能对比

| 方式 | 原理 | bump 试错 | 签名数 | 性能 |
|------|------|----------|--------|------|
| `find_program_address` | 从 bump=255 向下遍历 | 有 | 1 | 基准 |
| `create_with_seed` | 直接构造地址 | 无 | 0 | 快 ~10x |

### 3.3 使用条件

Seed 优化仅在以下条件启用：

```rust
if use_seed
    && !mint.eq(&WSOL_TOKEN_ACCOUNT)      // 排除 WSOL
    && !mint.eq(&SOL_TOKEN_ACCOUNT)       // 排除 SOL
    && (token_program.eq(&TOKEN_PROGRAM)  // Token 程序
        || token_program.eq(&TOKEN_PROGRAM_2022))
```

---

## 4. Token vs Token-2022

| | Token Program | Token-2022 Program |
|---|---|---|
| **Program ID** | `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` | `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb` |
| **ATA 种子** | `owner + TokenProgram + mint` | `owner + Token2022Program + mint` |
| **同一 owner+mint 的 ATA** | 地址 A | 地址 **不同** |
| **账户大小** | 165 bytes | 165 + extensions |

**注意**：由于 `token_program_id` 是 PDA 推导的种子之一，相同 owner+mint 组合会为两个 program 生成**不同**的 ATA 地址。

---

## 5. 各协议 PDA 汇总

### 5.1 PumpFun Protocol

| PDA 类型 | 种子 | 用途 |
|----------|------|------|
| Bonding Curve | `["bonding-curve", mint]` | 存储代币储备和价格 |
| Creator Vault | `["creator-vault", creator]` | 存储创建者 SOL 收益 |
| User Volume Accumulator | `["user_volume_accumulator", user]` | 跟踪用户交易量 |
| Metaplex Metadata | `["metadata", MPL_TOKEN_METADATA_PROGRAM_ID, mint]` | 代币元数据 |

### 5.2 PumpSwap Protocol

| PDA 类型 | 种子 | 用途 |
|----------|------|------|
| Pool Authority | `["creator_vault", coin_creator]` | 验证金库操作授权 |
| Canonical Pool | `["pool", [0, 0], pool_authority, mint, wsol_mint]` | PumpFun 迁移池 |
| User Volume | `["user_volume_accumulator", user]` | 用户交易量追踪 |

### 5.3 Raydium CPMM Protocol

| PDA 类型 | 种子 |
|----------|------|
| Pool PDA | `["pool", amm_config, mint1, mint2]` |
| Vault PDA | `["pool_vault", pool_state, mint]` |
| Observation State | `["observation", pool_state]` |

---

## 6. 缓存机制

### 6.1 ATA 缓存

```rust
// fast_fn.rs
static ATA_CACHE: Lazy<DashMap<AtaCacheKey, Pubkey>> =
    Lazy::new(|| DashMap::with_capacity(100_000));
```

**缓存策略**：

```
┌─────────────────────────────────────────────────────┐
│  Fast Path: 从 ATA_CACHE 获取（DashMap，锁自由）     │
│  如果命中 → 直接返回（O(1)）                         │
└─────────────────────────────────────────────────────┘
                    │ 未命中
                    ▼
┌─────────────────────────────────────────────────────┐
│  Slow Path: 计算新 ATA                              │
│  - 计算 ATA（标准或 Seed 优化）                      │
│  - 存入缓存                                         │
│  - 返回                                             │
└─────────────────────────────────────────────────────┘
```

### 6.2 PDA 缓存

```rust
// fast_fn.rs
static PDA_CACHE: Lazy<DashMap<PdaCacheKey, Pubkey>> =
    Lazy::new(|| DashMap::with_capacity(100_000));
```

**缓存键类型**：
- `PumpFunBondingCurve(mint)`
- `PumpFunCreatorVault(creator)`
- `BonkPool(base_mint, quote_mint)`

### 6.3 性能优化总结

| 优化项 | 实现方式 | 收益 |
|--------|----------|------|
| **ATA 地址缓存** | `ATA_CACHE` (DashMap) | 避免重复计算 PDA |
| **指令缓存** | `INSTRUCTION_CACHE` | 避免重复构建创建指令 |
| **Arc 共享** | 缓存返回 `Arc<Vec<Instruction>>` | 减少克隆开销 |
| **Seed 优化** | `create_with_seed` | 跳过 bump 试错 |
| **原子租金读取** | `AtomicU64` + `Relaxed` | 无锁并发访问 |

---

## 7. 核心 API 汇总

### 7.1 ATA 计算

| 函数 | 文件 | use_seed | 缓存 |
|------|------|----------|------|
| `get_associated_token_address_with_program_id_fast` | fast_fn.rs | false | ✅ |
| `get_associated_token_address_with_program_id_fast_use_seed` | fast_fn.rs | 参数控制 | ✅ |
| `get_associated_token_address_with_program_id_use_seed` | seed.rs | true | ❌ |

### 7.2 ATA 创建

| 函数 | 文件 | use_seed | 缓存 |
|------|------|----------|------|
| `create_associated_token_account_idempotent_fast` | fast_fn.rs | false | ✅ |
| `create_associated_token_account_idempotent_fast_use_seed` | fast_fn.rs | 参数控制 | ✅ |
| `create_associated_token_account_use_seed` | seed.rs | true | ❌ |

### 7.3 调用关系

```
TradingClient::buy/sell
    │
    ▼
_create_associated_token_account_idempotent_fast (fast_fn)
    │
    ├── use_seed=true? ──是──► create_associated_token_account_use_seed (seed)
    │
    └── 否 ──► 标准 ATA 创建指令
         [缓存: INSTRUCTION_CACHE]

get_associated_token_address_with_program_id_fast (fast_fn)
    │
    ├── use_seed=true? ──是──► get_associated_token_address_use_seed (seed)
    │
    └── 否 ─� 标准 get_associated_token_address
         [缓存: ATA_CACHE]
```

---

## 8. 协议程序 ID 汇总

| 协议 | 程序ID |
|------|--------|
| PumpFun | `6EF8rrecthR5DkC8qq98t33Dtk8KZA1Ad8` |
| PumpSwap AMM | `PMrmM5WwYfPrKJV8Mm7W45g76xHWP4Skg7` |
| Bonk | `BonkS3qs8i713KcFwcJ7fJbLG2VqJ9j38v` |
| Raydium CPMM | `CPMMoo8L3F4NbT8bKV2c7G7Kb9e4Nx` |
| Raydium CLMM | `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK` |
| Meteora Damm V2 | `cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG` |
| Metaplex Metadata | `metaqbxxUerdq28cj1RbAWkYQm3ybzjb26a8t` |
| Associated Token Program | `ATokenGPvbdGVxr1b2hvZbsiqL5W34GdCh` |
| Token Program | `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` |
| Token2022 | `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb` |
