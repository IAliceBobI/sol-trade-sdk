# Pool 结构体元数据扩充设计

## 概述

为所有 DEX Pool 结构体添加元数据字段（`program_id`、`dex_name`、`dex_display_name`），统一返回格式，避免单独查询 DEX 信息。

**目标**：
- 在 Pool 结构体中包含 DEX program_id 和名称信息
- 统一 `get_pool_by_address` 等函数的返回格式
- 利用现有的 `DexProtocol` 枚举获取名称

**范围**：
- 先在 PumpSwap 上试点验证
- 确认设计方案后推广到所有 7 个 DEX

## 设计决策

### 1. Pool 结构体字段

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pool {
    // ===== 新增元数据字段 =====
    /// DEX Program ID（从账户 owner 自动填充）
    pub program_id: Pubkey,

    /// DEX 协议名称（如 "pumpswap"）
    pub dex_name: String,

    /// DEX 显示名称（如 "PumpSwap"）
    pub dex_display_name: String,

    // ===== 原有链上数据字段 =====
    pub pool_bump: u8,
    pub index: u16,
    pub creator: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub lp_supply: u64,
    pub coin_creator: Pubkey,
    pub is_mayhem_mode: bool,
}
```

**关键点**：
- 移除 `BorshDeserialize` derive（新字段不在链上数据中）
- 创建辅助结构体 `PoolDataOnly` 用于 Borsh 反序列化

### 2. 反序列化函数

```rust
// 辅助结构体：只包含链上数据字段
#[derive(BorshDeserialize)]
struct PoolDataOnly {
    // 只有链上数据的字段
}

/// 修改后的 pool_decode：需要 program_id 参数
pub fn pool_decode(data: &[u8], program_id: Pubkey) -> Option<Pool> {
    // 1. 反序列化链上数据
    let pool_data: PoolDataOnly = borsh::from_slice(&data[..POOL_SIZE]).ok()?;

    // 2. 从 DexProtocol 获取名称
    let (dex_name, dex_display_name) = match DexProtocol::from_program_id(&program_id) {
        Some(protocol) => (
            protocol.name().to_string(),
            protocol.display_name().to_string(),
        ),
        None => ("unknown".to_string(), "Unknown DEX".to_string()),
    };

    // 3. 构建完整 Pool
    Some(Pool {
        program_id,
        dex_name,
        dex_display_name,
        ..pool_data
    })
}
```

### 3. 缓存策略

**核心原则**：分离 **find** 和 **get** 操作

| 函数类型 | 缓存策略 | 原因 |
|---------|---------|------|
| `get_pool_by_address()` | ❌ 不缓存 | 每次获取最新 reserve 数据 |
| `get_pool_by_mint()` | ✅ 缓存 | mint → 地址映射不变 |
| `list_pools_by_mint()` | ✅ 缓存 | 查找操作较慢，值得缓存 |
| `get_pool_by_mint_force()` | ✅ 强制刷新 | 更新缓存 |
| `clear_pool_cache()` | ✅ 清除所有 | 手动管理缓存 |

**实现**：

```rust
// get_pool_by_address：不缓存，每次从链上获取
pub async fn get_pool_by_address<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    pool_address: &Pubkey,
) -> Result<Pool, anyhow::Error> {
    // ❌ 不检查缓存
    let account = rpc.get_account(pool_address).await?;
    let pool = pool_decode(&account.data[8..], account.owner)?;
    // ❌ 不写入缓存
    Ok(pool)
}

// get_pool_by_mint：缓存完整结果
pub async fn get_pool_by_mint<T: PoolRpcClient + ?Sized>(
    rpc: &T,
    mint: &Pubkey,
) -> Result<(Pubkey, Pool), anyhow::Error> {
    // ✅ 检查缓存
    if let Some(pool) = pump_swap_cache::get_cached_pool_by_mint(mint) {
        return Ok(pool);
    }

    // 缓存未命中，从链上获取
    let result = find_pool_by_mint_impl(rpc, mint).await?;

    // ✅ 写入缓存
    pump_swap_cache::cache_pool_by_mint(mint, &result);
    Ok(result)
}
```

### 4. 错误处理

| 场景 | 处理方式 |
|------|----------|
| Pool 数据长度不足 | 返回 `None`（Option） |
| 未知 DEX program_id | 使用 "unknown" fallback |
| RPC 调用失败 | 返回 `Err`，详细错误信息 |
| Owner 验证失败 | 返回错误，防止数据污染 |
| 缓存读取失败 | 降级到 RPC 查询 |

### 5. 测试策略

#### 单元测试

- `test_pool_decode_with_known_dex`：验证已知 DEX 的解析
- `test_pool_decode_with_unknown_dex`：验证未知 DEX 的 fallback
- `test_pool_decode_invalid_data`：验证数据长度检查

#### 集成测试

- `test_get_pool_by_address_no_cache`：验证无缓存行为
- `test_get_pool_by_mint_with_cache`：验证缓存命中
- `test_get_pool_by_mint_force`：验证强制刷新
- `test_clear_pool_cache`：验证缓存清理

## 实施步骤

### Step 1: PumpSwap 试点

**文件修改清单**：

1. `src/instruction/utils/pumpswap_types.rs`
   - 添加 `program_id`, `dex_name`, `dex_display_name` 字段
   - 移除 `BorshDeserialize` derive
   - 创建 `PoolDataOnly` 辅助结构体
   - 修改 `pool_decode` 函数签名

2. `src/instruction/utils/pumpswap.rs`
   - 更新 `get_pool_by_address`（移除缓存）
   - 更新 `get_pool_by_mint`（保留缓存）
   - 更新 `list_pools_by_mint`（保留缓存）

3. `tests/pumpswap_pool_tests.rs`
   - 添加元数据验证测试
   - 添加缓存行为测试

**验证命令**：

```bash
# 运行 PumpSwap 测试
cargo nextest run pumpswap_pool -- --nocapture

# 检查编译
cargo check --package sol-trade-sdk
```

### Step 2: 推广到其他 DEX

按以下顺序实施（每个 DEX 修改模式相同）：

1. Raydium CPMM
2. Raydium AMM V4
3. Raydium CLMM
4. PumpFun
5. Bonk
6. Meteora DAMM V2
7. Raydium LaunchLab

**每个 DEX 的修改**：

- `src/instruction/utils/<dex>_types.rs`：修改 Pool 结构体和 `*_decode` 函数
- `src/instruction/utils/<dex>.rs`：更新查询函数
- `tests/<dex>_tests.rs`：添加测试

### Step 3: 更新文档

更新 `docs/Pool查询方法.md`，反映新的 API 和缓存策略。

## 影响范围

### 破坏性变更

- `Pool` 结构体新增 3 个字段（向后兼容，使用 `Default`）
- `pool_decode` 函数签名改变（需要传入 `program_id`）
- 缓存行为改变（`get_pool_by_address` 不再缓存）

### 兼容性

- 现有的 `get_pool_by_mint` API 保持不变
- `clear_pool_cache()` 功能保持不变
- `*_force` 函数保持不变

## 后续工作

完成后可以考虑：

1. **性能监控**：验证缓存策略的有效性
2. **TTL 缓存**：为 `get_pool_by_mint` 添加过期时间
3. **元数据索引**：建立 program_id → Pool 的反向索引
4. **批查询优化**：利用 `getMultipleAccounts` 优化批查询

## 参考资料

- `src/constants/dex_protocols.rs`：DexProtocol 枚举定义
- `docs/pumpswap dex.json`：PumpSwap IDL metadata
- `temp/raydium-idl/`：Raydium DEX IDL 文件
