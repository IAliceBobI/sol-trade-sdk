# PumpSwap 非标准池支持 - 开发计划

## 1. 任务背景

### 任务描述
当前项目（Sol Trade SDK）只支持新的 pool 数据格式的解析（标准池/Canonical Pool），希望旧的 pool 也支持（非标准池/Non-canonical Pool）。

### 当前问题
1. SDK 的 `PumpSwapParams::from_pool_address_by_rpc` 方法假设所有池都是标准池
2. PDA 派生逻辑硬编码，使用固定的种子 `[0, 0]`
3. 缺少池类型识别机制，无法区分标准池和非标准池

### 目标和期望
1. 支持非标准池（Non-canonical Pool）的交易
2. 保持向后兼容性，不影响标准池功能
3. 确保 `pumpswap_direct_trading` 和 `pumpswap_direct_trading_new_pool` 两个示例都能正常工作

---

## 2. 团队分析

### 架构师视角

#### 架构影响分析
- **核心问题**：PDA 派生逻辑单一化，无法适配非标准池
- **影响范围**：主要集中在 `src/instruction/pumpswap.rs` 和 `src/instruction/utils/pumpswap_types.rs`
- **架构风险**：低 - 修改集中在单一模块，不影响其他 DEX 协议

#### 设计模式建议
- **策略模式**：根据池类型（index）动态选择 PDA 派生策略
- **工厂模式增强**：保持现有的 `DexParamEnum::PumpSwap` 枚举，在 `PumpSwapParams` 中添加池类型字段
- **向后兼容性设计**：默认行为保持不变，通过 RPC 获取池数据后自动识别池类型

### 性能专家视角

#### 性能影响评估
- **RPC 调用开销**：无变化（仍为 1 次 `get_account`）
- **计算开销**：无变化（PDA 派生复杂度相同）
- **内存开销**：可忽略（增加 < 100 字节）

#### 优化建议
1. **可选缓存**：使用 LRU 缓存池数据，避免重复 RPC 调用
2. **并发优化**：池数据获取可与账户余额检查并发执行

### 安全专家视角

#### 安全风险评估
1. **池类型误判**：RPC 返回的池数据可能不准确 → 添加 PDA 验证
2. **PDA 派生错误**：非标准池的派生逻辑可能有误 → 添加验证步骤

#### 防护措施
1. **输入验证**：验证所有池数据字段
2. **PDA 重新计算**：使用派生的地址验证池地址
3. **错误处理**：明确的错误消息，便于调试

### 系统分析师视角

#### 调用链路分析
```
client.buy(buy_params)
    ↓
trading/core/executor.rs: execute_buy()
    ↓
trading/factory.rs: create_executor()
    ↓
instruction/pumpswap.rs: PumpSwapParams::from_pool_address_by_rpc()
    ↓
构建交易指令
    ↓
swqos/*.rs: 发送交易到 MEV 服务
```

#### 数据流分析
```
用户输入
    ↓
TradeBuyParams (包含 pool mint)
    ↓
PumpSwapParams::from_pool_address_by_rpc(pool_address)
    ↓
RPC 调用 → get_account(pool_address)
    ↓
解析 Pool 数据 (236 字节)
    ↓
提取 pool.index 判断池类型
    ↓
派生 PDA（根据池类型）
    ↓
构建交易指令
    ↓
签名并发送
```

#### 需要修改的关键函数
| 函数 | 文件 | 修改内容 |
|---|---|---|
| `from_pool_address_by_rpc` | `src/instruction/pumpswap.rs` | 核心修改：添加池类型识别和动态 PDA 派生 |

---

## 3. 技术方案

### 整体方案设计
1. 通过 RPC 获取池账户数据
2. 解析 `pool.index` 字段判断池类型
3. 根据池类型使用不同的 PDA 派生逻辑
4. 构建正确的交易指令

### 关键技术点

#### 技术点 1：池类型判断
```rust
match pool.index {
    0 => PoolType::Canonical,
    _ => PoolType::NonCanonical { index: pool.index, pool_authority: pool.pool_authority },
}
```

#### 技术点 2：动态 PDA 派生
```rust
// 标准池（index = 0）
let (pool_authority, _) = Pubkey::find_program_address(
    &[b"pool-authority", mint.as_ref()],
    &PUMPFUN_PROGRAM_ID,
);
let (pool_pda, _) = Pubkey::find_program_address(
    &[b"pool", &[0u8, 0u8], pool_authority.as_ref(), mint.as_ref(), wsol_mint.as_ref()],
    &PUMPSWAP_PROGRAM_ID,
);

// 非标准池（index ≠ 0）
let (pool_pda, _) = Pubkey::find_program_address(
    &[b"pool", &pool.index.to_le_bytes(), pool.pool_authority.as_ref(), pool.base_mint.as_ref(), pool.quote_mint.as_ref()],
    &PUMPSWAP_PROGRAM_ID,
);
```

#### 技术点 3：账户验证
```rust
assert_eq!(pool_address, pool_pda, "Pool address mismatch");
```

### 实现策略
- **阶段 1**：类型定义和结构体修改
- **阶段 2**：核心逻辑实现
- **阶段 3**：测试和验证
- **阶段 4**：文档和优化

---

## 4. 任务分解

### 阶段 1：类型定义和结构体修改

#### 任务 1.1：添加 PoolType 枚举
- **文件路径**：`src/instruction/utils/pumpswap_types.rs`
- **具体描述**：添加 `PoolType` 枚举，区分标准池和非标准池
- **优先级**：High

#### 任务 1.2：更新 Pool 结构体
- **文件路径**：`src/instruction/utils/pumpswap_types.rs`
- **具体描述**：确保 Pool 结构体包含所有必要字段（pool_bump, index, creator, base_mint, quote_mint, lp_mint, pool_base_token_account, pool_quote_token_account, lp_supply, coin_creator, is_mayhem_mode）
- **优先级**：High

### 阶段 2：核心逻辑实现

#### 任务 2.1：修改 from_pool_address_by_rpc 方法
- **文件路径**：`src/instruction/pumpswap.rs`
- **具体描述**：添加池类型识别逻辑，根据 pool.index 判断池类型
- **优先级**：High

#### 任务 2.2：实现标准池 PDA 派生
- **文件路径**：`src/instruction/pumpswap.rs`
- **具体描述**：实现标准池的 PDA 派生逻辑（保持现有逻辑）
- **优先级**：High

#### 任务 2.3：实现非标准池 PDA 派生
- **文件路径**：`src/instruction/pumpswap.rs`
- **具体描述**：实现非标准池的 PDA 派生逻辑，使用动态种子
- **优先级**：High

#### 任务 2.4：添加 PDA 验证逻辑
- **文件路径**：`src/instruction/pumpswap.rs`
- **具体描述**：添加 PDA 验证步骤，确保派生地址与实际地址一致
- **优先级**：Medium

### 阶段 3：测试和验证

#### 任务 3.1：测试非标准池
- **文件路径**：`examples/pumpswap_direct_trading/src/main.rs`
- **具体描述**：运行示例程序，验证非标准池交易功能
- **优先级**：High

#### 任务 3.2：验证标准池
- **文件路径**：`examples/pumpswap_direct_trading_new_pool/src/main.rs`
- **具体描述**：运行示例程序，验证标准池功能不受影响
- **优先级**：High

#### 任务 3.3：添加单元测试
- **文件路径**：`tests/pumpswap_test.rs`
- **具体描述**：添加单元测试，覆盖标准池和非标准池场景
- **优先级**：Medium

### 阶段 4：文档和优化

#### 任务 4.1：更新文档
- **文件路径**：`docs/PumpSwap池类型.md`
- **具体描述**：更新文档，说明非标准池支持
- **优先级**：Low

#### 任务 4.2：性能优化
- **文件路径**：`src/instruction/pumpswap.rs`
- **具体描述**：可选：添加 LRU 缓存避免重复 RPC 调用
- **优先级**：Low

---

## 5. Todo 列表

以下任务已添加到 todo 列表，使用 `$todo_read` 查看：

- [ ] 任务 1.1：添加 PoolType 枚举 - `src/instruction/utils/pumpswap_types.rs` (high)
- [ ] 任务 1.2：更新 Pool 结构体 - `src/instruction/utils/pumpswap_types.rs` (high)
- [ ] 任务 2.1：修改 from_pool_address_by_rpc 方法 - `src/instruction/pumpswap.rs` (high)
- [ ] 任务 2.2：实现标准池 PDA 派生 - `src/instruction/pumpswap.rs` (high)
- [ ] 任务 2.3：实现非标准池 PDA 派生 - `src/instruction/pumpswap.rs` (high)
- [ ] 任务 2.4：添加 PDA 验证逻辑 - `src/instruction/pumpswap.rs` (medium)
- [ ] 任务 3.1：测试非标准池 - `examples/pumpswap_direct_trading/src/main.rs` (high)
- [ ] 任务 3.2：验证标准池 - `examples/pumpswap_direct_trading_new_pool/src/main.rs` (high)
- [ ] 任务 3.3：添加单元测试 - `tests/pumpswap_test.rs` (medium)
- [ ] 任务 4.1：更新文档 - `docs/PumpSwap池类型.md` (low)
- [ ] 任务 4.2：性能优化 - `src/instruction/pumpswap.rs` (low)

---

## 6. 实施步骤

1. **步骤 1**：读取并理解现有的 PumpSwap 类型定义和实现
2. **步骤 2**：添加 PoolType 枚举和更新 Pool 结构体
3. **步骤 3**：修改 from_pool_address_by_rpc 方法，添加池类型识别
4. **步骤 4**：实现标准池和非标准池的 PDA 派生逻辑
5. **步骤 5**：添加 PDA 验证逻辑
6. **步骤 6**：使用 pumpswap_direct_trading 示例测试非标准池
7. **步骤 7**：使用 pumpswap_direct_trading_new_pool 示例验证标准池
8. **步骤 8**：添加单元测试
9. **步骤 9**：更新文档
10. **步骤 10**：可选性能优化

---

## 7. 风险评估

| 风险 | 描述 | 应对措施 |
|---|---|---|
| 标准池功能受影响 | 修改可能破坏现有功能 | 充分测试标准池示例 |
| PDA 派生错误 | 非标准池的派生逻辑可能有误 | 添加 PDA 验证步骤 |
| RPC 数据不一致 | 池数据可能不完整或错误 | 添加数据验证和错误处理 |
| 编译错误 | 类型定义变更可能导致编译错误 | 逐步修改，及时编译验证 |

---

## 8. 验证标准

1. ✅ `pumpswap_direct_trading` 示例可以成功执行非标准池交易
2. ✅ `pumpswap_direct_trading_new_pool` 示例可以成功执行标准池交易
3. ✅ PDA 派生逻辑正确，所有地址验证通过
4. ✅ 无编译错误和警告
5. ✅ 单元测试全部通过

---

## 9. 注意事项

1. **向后兼容性**：确保修改不影响现有标准池的功能
2. **错误处理**：提供清晰的错误消息，便于调试
3. **性能**：避免引入不必要的 RPC 调用
4. **测试**：在测试网或本地验证后再部署到主网
5. **代码风格**：遵循 Rust 标准代码风格
6. **注释**：添加必要的代码注释，说明池类型判断逻辑

---

## 10. 相关资源

- **文档**：`docs/PumpSwap池类型.md`
- **示例代码**：
  - `examples/pumpswap_direct_trading/src/main.rs`
  - `examples/pumpswap_direct_trading_new_pool/src/main.rs`
- **核心实现**：
  - `src/instruction/pumpswap.rs`
  - `src/instruction/utils/pumpswap_types.rs`
- **常量定义**：`src/constants/accounts.rs`

---

## 11. 版本信息

- **创建日期**：2025-12-30
- **状态**：待实施
- **负责人**：待分配
- **预计工作量**：2-3 天