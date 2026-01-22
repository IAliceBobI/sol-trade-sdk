# Discriminator 系统

## 概述

Discriminator 系统是 Sol Trade SDK 解析器模块的核心组件，用于精确识别 DEX 协议的指令类型。该系统参考了 [solana-dex-parser](https://github.com/kjchoi0530/solana-dex-parser) 的设计理念，使用 8 字节（或 1 字节）的指令标识符来区分不同的操作类型。

## 设计目标

1. **精确识别**: 准确区分 Swap、添加流动性、移除流动性等操作
2. **统一接口**: 为不同协议提供一致的识别机制
3. **可扩展性**: 轻松添加新协议和新指令类型
4. **类型安全**: 使用枚举和编译时检查确保类型安全

## 架构设计

### 核心组件

```
src/parser/
├── discriminators.rs           # Discriminator 注册表
├── instruction_data_parser.rs  # 指令数据解析器
└── constants/
    └── discriminators.rs       # Discriminator 常量定义
```

### DiscriminatorRegistry

`DiscriminatorRegistry` 是系统的核心，负责注册和识别指令类型。

#### 数据结构

```rust
pub struct DiscriminatorRegistry {
    /// 存储 (protocol, discriminator_bytes) -> InstructionType
    discriminators: HashMap<(DexProtocol, [u8; 8]), InstructionType>,
}
```

#### 协议枚举

```rust
pub enum DexProtocol {
    PumpSwap,
    RaydiumClmm,
    RaydiumCpmm,
    RaydiumV4,
}
```

#### 指令类型枚举

```rust
pub enum InstructionType {
    Swap,           // 交换操作
    Buy,            // 买入操作
    Sell,           // 卖出操作
    CreatePool,     // 创建池
    AddLiquidity,   // 添加流动性
    RemoveLiquidity, // 移除流动性
    Unknown,        // 未知类型
}
```

## 各协议的 Discriminator

### PumpSwap (8 字节)

```rust
// BUY 操作
let buy = [102, 6, 61, 18, 1, 218, 235, 234];

// SELL 操作
let sell = [51, 230, 133, 164, 1, 127, 131, 173];

// 流动性操作
let create_pool = [233, 146, 209, 142, 207, 104, 64, 188];
let add_liquidity = [242, 35, 198, 137, 82, 225, 242, 182];
let remove_liquidity = [183, 18, 70, 156, 148, 109, 161, 34];
```

### Raydium CLMM (8 字节)

CLMM 使用多个变体来表示相同的操作类型：

```rust
// ADD_LIQUIDITY 操作
let add_liquidity = [
    [135, 128, 47, 77, 15, 152, 240, 49], // openPosition
    [77, 184, 74, 214, 112, 86, 241, 199], // openPositionV2
    [77, 255, 174, 82, 125, 29, 201, 46],  // openPositionWithToken22Nft
    [46, 156, 243, 118, 13, 205, 251, 178], // increaseLiquidity
    [133, 29, 89, 223, 69, 238, 176, 10],  // increaseLiquidityV2
];

// REMOVE_LIQUIDITY 操作
let remove_liquidity = [
    [160, 38, 208, 111, 104, 91, 44, 1], // decreaseLiquidity
    [58, 127, 188, 62, 79, 82, 196, 96], // decreaseLiquidityV2
];

// CREATE_POOL 操作
let create = [
    [233, 146, 209, 142, 207, 104, 64, 188], // createPool
];
```

### Raydium CPMM (8 字节)

```rust
// SWAP 操作
let swap = [0x8f, 0xbe, 0x5a, 0xda, 0xc4, 0x1e, 0x33, 0xde];

// 流动性操作
let create_pool = [175, 175, 109, 31, 13, 152, 155, 237];
let add_liquidity = [242, 35, 198, 137, 82, 225, 242, 182];
let remove_liquidity = [183, 18, 70, 156, 148, 109, 161, 34];
```

### Raydium V4 (1 字节)

V4 使用 1 字节 discriminator，系统会自动补零到 8 字节：

```rust
// SWAP 操作 (discriminator = 9)
let swap = [9, 0, 0, 0, 0, 0, 0, 0];

// 流动性操作
let add_liquidity = [1, 0, 0, 0, 0, 0, 0, 0];
let remove_liquidity = [2, 0, 0, 0, 0, 0, 0, 0];
let create_pool = [0, 0, 0, 0, 0, 0, 0, 0];
```

## 使用方法

### 基本用法

```rust
use sol_trade_sdk::parser::DiscriminatorRegistry;

// 创建注册表（自动注册所有协议）
let registry = DiscriminatorRegistry::default();

// 识别指令类型
let data = &[0x8f, 0xbe, 0x5a, 0xda, 0xc4, 0x1e, 0x33, 0xde];
let instr_type = registry.identify(DexProtocol::RaydiumCpmm, data);
assert_eq!(instr_type, InstructionType::Swap);
```

### 判断流动性操作

```rust
// 检查是否是流动性操作（应该被 Swap 解析器排除）
let is_liquidity = registry.is_liquidity_discriminator(
    DexProtocol::RaydiumClmm,
    &[135, 128, 47, 77, 15, 152, 240, 49]
);
assert!(is_liquidity); // openPosition
```

### 判断 Swap 操作

```rust
// 检查是否是 Swap 操作
let is_swap = registry.is_swap_discriminator(
    DexProtocol::PumpSwap,
    &[102, 6, 61, 18, 1, 218, 235, 234]
);
assert!(is_swap); // buy
```

## 在解析器中的集成

### 示例：CLMM 解析器

```rust
impl RaydiumClmmParser {
    fn is_swap_instruction(&self, data: &[u8]) -> bool {
        if data.len() < 8 {
            return false;
        }

        let registry = DiscriminatorRegistry::default();

        // 只有确认不是流动性操作才认为是 Swap
        !registry.is_liquidity_discriminator(
            ParserDexProtocol::RaydiumClmm,
            data
        )
    }
}
```

## Discriminator 来源

Discriminator 值来源于以下途径：

1. **官方文档**: 协议官方提供的指令定义
2. **solana-dex-parser**: 社区维护的解析器项目
3. **链上观察**: 通过观察实际交易总结出的模式
4. **IDL 文件**: Anchor 框架生成的接口定义文件

## 测试

### 单元测试

每个解析器都包含 discriminator 测试：

```rust
#[test]
fn test_is_swap_instruction() {
    let parser = RaydiumClmmParser::new();

    // Swap 指令
    let swap_data = [/* ... */];
    assert!(parser.is_swap_instruction(&swap_data));

    // 流动性操作
    let add_liq_data = [/* ... */];
    assert!(!parser.is_swap_instruction(&add_liq_data));
}
```

## 扩展性

### 添加新协议

1. 在 `DexProtocol` 枚举中添加新变体
2. 在 `DiscriminatorRegistry` 中添加注册方法
3. 在 `new()` 中调用注册方法
4. 在对应解析器中集成 discriminator 系统

```rust
// 1. 添加枚举
pub enum DexProtocol {
    // ... 现有协议
    NewProtocol,
}

// 2. 添加注册方法
impl DiscriminatorRegistry {
    fn register_new_protocol(&mut self) {
        let swap = [/* discriminator */];
        self.discriminators.insert(
            (DexProtocol::NewProtocol, swap),
            InstructionType::Swap
        );
    }
}
```

## 性能考虑

- **零成本抽象**: 使用枚举和编译时优化，运行时开销最小
- **HashMap 查找**: O(1) 时间复杂度
- **懒加载**: 使用 `LazyLock` 实现单例模式
- **内存高效**: 8 字节 key + 1 字节 value

## 相关文档

- [解析器实现总结](./parser_implementation_summary.md)
- [交易参数参考](./交易参数参考.md)
- [DEX 交易测试素材](./txs.md)

## 参考资料

- [solana-dex-parser](https://github.com/kjchoi0530/solana-dex-parser)
- [Anchor Discriminator](https://www.anchor-lang.com/docs/the-discriminator)
- [Solana Program Library](https://spl.solana.com/)
