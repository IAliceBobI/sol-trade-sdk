# Raydium AMM V4 通过 Mint 查找 Pool 功能实现计划

## 1. 任务背景

### 任务描述
为 Raydium AMM V4 添加通过 mint 查找 pool 的功能，使其具有与 PumpSwap 类似的能力。

### 当前问题
- Raydium AMM V4 目前只支持通过地址查询 pool
- 缺少通过 mint 查询的功能，导致用户需要先知道 pool 地址才能查询
- 与 PumpSwap 的功能不一致，增加了使用复杂度

### 目标和期望
- 实现 `get_pool_by_mint` - 通过 mint 查询最优池
- 实现 `get_pool_by_mint_force` - 通过 mint 强制刷新
- 实现 `list_pools_by_mint` - 列出所有包含该 mint 的 pool
- 添加缓存机制，提高查询性能
- 提供与 PumpSwap 一致的 API 接口

## 2. 团队分析

### 架构师视角
**架构影响分析**：
- 需要在 `src/instruction/utils/raydium_amm_v4.rs` 中添加新的查询函数
- 需要扩展缓存模块，添加 `MINT_TO_POOL_CACHE` 缓存
- 需要遵循现有的代码结构和命名规范

**设计模式建议**：
- 使用缓存模式（Cache Pattern）减少 RPC 调用
- 使用策略模式（Strategy Pattern）处理不同的查询场景（最优池 vs 所有池）
- 使用工厂模式（Factory Pattern）统一查询接口

### 性能专家视角
**性能影响评估**：
- `getProgramAccounts` 查询可能返回大量数据，需要优化数据切片
- 缓存可以显著减少重复查询的延迟
- 并行查询 baseMint 和 quoteMint 可以提高响应速度

**优化建议**：
- 使用 `dataSlice` 只获取必要的数据（减少网络传输）
- 使用 `DashMap` 实现并发安全的缓存
- 设置合理的缓存大小限制（50,000）
- 优先查询 WSOL 交易对（最常见场景）

### 安全专家视角
**安全风险评估**：
- 需要验证返回的 pool 确实属于 Raydium AMM V4 程序
- 需要处理 RPC 查询失败的情况
- 需要防止缓存污染

**防护措施**：
- 验证账户 owner 为 `RAYDIUM_AMM_V4` 程序
- 使用 `anyhow::Error` 进行统一的错误处理
- 提供强制刷新功能以解决缓存不一致问题

### 系统分析师视角
**调用链路分析**：
```
用户调用 get_pool_by_mint(mint)
  ↓
检查 MINT_TO_POOL_CACHE 缓存
  ↓ (缓存未命中)
并行查询 baseMint 和 quoteMint
  ↓
合并结果并去重
  ↓
选择最优池（优先 WSOL 交易对，按 LP 供应量排序）
  ↓
更新缓存
  ↓
返回结果
```

**数据流分析**：
- 输入：`mint: Pubkey`
- 输出：`(Pubkey, AmmInfo)` 或 `Vec<(Pubkey, AmmInfo)>`
- 中间数据：RPC 返回的账户数据、解码后的 `AmmInfo` 结构

## 3. 技术方案

### 整体方案设计
1. **缓存层**：添加 `MINT_TO_POOL_CACHE` 缓存，存储 mint → pool_address 的映射
2. **查询层**：使用 `getProgramAccounts` + `memcmp` 过滤器查询链上数据
3. **选择层**：根据策略选择最优池（优先 WSOL 交易对、LP 供应量）
4. **API 层**：提供三个公共函数：`get_pool_by_mint`、`get_pool_by_mint_force`、`list_pools_by_mint`

### 关键技术点
1. **偏移量计算**：
   - `coin_mint` offset: 376
   - `pc_mint` offset: 408
   - 需要从 `AmmInfo` 结构中验证这些偏移量

2. **RPC 查询优化**：
   - 使用 `getProgramUiAccountsWithConfig` 查询
   - 使用 `dataSlice` 只获取必要数据（减少网络传输）
   - 使用 `memcmp` 过滤器过滤特定 mint

3. **缓存策略**：
   - 使用 `DashMap` 实现并发安全的缓存
   - 设置最大缓存大小（50,000）
   - 提供强制刷新功能

4. **池选择策略**：
   - 优先选择 WSOL 交易对
   - 按 LP 供应量排序（选择流动性最好的池）
   - 支持列出所有池（供上层路由选择）

### 实现策略
1. **TDD 方法**：先编写测试，再实现功能
2. **增量开发**：先实现基础功能，再添加优化
3. **频繁提交**：每个小功能完成后立即提交
4. **代码复用**：参考 PumpSwap 的实现，保持代码一致性

## 4. 任务分解

### 阶段 1：准备工作（5 分钟）

#### 任务 1.1：验证偏移量
- **文件路径**：`src/instruction/utils/raydium_amm_v4_types.rs`
- **实现步骤**：
  1. 查看 `AmmInfo` 结构的字段顺序
  2. 计算 `coin_mint` 和 `pc_mint` 的偏移量
  3. 确认偏移量为 376 和 408
- **验证方法**：手动计算偏移量，与任务文件中的值对比
- **优先级**：高

#### 任务 1.2：创建测试文件
- **文件路径**：`tests/raydium_amm_v4_pool_tests.rs`
- **实现步骤**：
  1. 创建测试文件（如果不存在）
  2. 添加测试辅助函数
  3. 添加测试用的 RPC 客户端
- **验证方法**：运行测试文件确认可以编译
- **优先级**：高

### 阶段 2：实现缓存层（10 分钟）

#### 任务 2.1：添加 MINT_TO_POOL_CACHE 缓存
- **文件路径**：`src/instruction/utils/raydium_amm_v4.rs`
- **实现步骤**：
  1. 在缓存模块中添加 `MINT_TO_POOL_CACHE: Lazy<DashMap<Pubkey, Pubkey>>`
  2. 添加 `get_cached_pool_address_by_mint` 函数
  3. 添加 `cache_pool_address_by_mint` 函数
  4. 更新 `clear_pool_cache_internal` 函数，清除 MINT_TO_POOL_CACHE
- **验证方法**：编译通过，无错误
- **优先级**：高

#### 任务 2.2：编写缓存测试
- **文件路径**：`tests/raydium_amm_v4_pool_tests.rs`
- **实现步骤**：
  1. 编写测试 `test_cache_pool_address_by_mint`
  2. 测试缓存写入和读取
  3. 测试缓存清除
- **验证方法**：运行测试，确认通过
- **优先级**：高

### 阶段 3：实现 RPC 查询（15 分钟）

#### 任务 3.1：实现 find_pools_by_mint_offset_collect
- **文件路径**：`src/instruction/utils/raydium_amm_v4.rs`
- **实现步骤**：
  1. 添加常量 `BASE_MINT_OFFSET = 376`
  2. 添加常量 `QUOTE_MINT_OFFSET = 408`
  3. 实现 `find_pools_by_mint_offset_collect` 函数
  4. 使用 `getProgramUiAccountsWithConfig` 查询
  5. 使用 `memcmp` 过滤器过滤特定 mint
  6. 解码返回的数据为 `AmmInfo` 结构
- **验证方法**：编译通过，无错误
- **优先级**：高

#### 任务 3.2：编写 RPC 查询测试
- **文件路径**：`tests/raydium_amm_v4_pool_tests.rs`
- **实现步骤**：
  1. 编写测试 `test_find_pools_by_mint_offset_collect`
  2. 测试查询 baseMint
  3. 测试查询 quoteMint
  4. 测试查询不存在的 mint
- **验证方法**：运行测试，确认通过
- **优先级**：高

### 阶段 4：实现池选择逻辑（10 分钟）

#### 任务 4.1：实现 find_pool_by_mint_impl
- **文件路径**：`src/instruction/utils/raydium_amm_v4.rs`
- **实现步骤**：
  1. 实现 `find_pool_by_mint_impl` 函数
  2. 并行查询 baseMint 和 quoteMint
  3. 合并结果并去重
  4. 优先选择 WSOL 交易对
  5. 按 LP 供应量排序
  6. 返回最优池
- **验证方法**：编译通过，无错误
- **优先级**：高

#### 任务 4.2：编写池选择测试
- **文件路径**：`tests/raydium_amm_v4_pool_tests.rs`
- **实现步骤**：
  1. 编写测试 `test_find_pool_by_mint_impl`
  2. 测试选择 WSOL 交易对
  3. 测试按 LP 供应量排序
  4. 测试没有池的情况
- **验证方法**：运行测试，确认通过
- **优先级**：高

### 阶段 5：实现公共 API（10 分钟）

#### 任务 5.1：实现 get_pool_by_mint
- **文件路径**：`src/instruction/utils/raydium_amm_v4.rs`
- **实现步骤**：
  1. 实现 `get_pool_by_mint` 函数
  2. 检查 `MINT_TO_POOL_CACHE` 缓存
  3. 如果缓存未命中，调用 `find_pool_by_mint_impl`
  4. 更新缓存
  5. 返回结果
- **验证方法**：编译通过，无错误
- **优先级**：高

#### 任务 5.2：实现 get_pool_by_mint_force
- **文件路径**：`src/instruction/utils/raydium_amm_v4.rs`
- **实现步骤**：
  1. 实现 `get_pool_by_mint_force` 函数
  2. 从缓存中删除 mint
  3. 调用 `get_pool_by_mint` 重新查询
- **验证方法**：编译通过，无错误
- **优先级**：高

#### 任务 5.3：实现 list_pools_by_mint
- **文件路径**：`src/instruction/utils/raydium_amm_v4.rs`
- **实现步骤**：
  1. 实现 `list_pools_by_mint` 函数
  2. 并行查询 baseMint 和 quoteMint
  3. 合并结果并去重
  4. 返回所有池
- **验证方法**：编译通过，无错误
- **优先级**：高

#### 任务 5.4：编写公共 API 测试
- **文件路径**：`tests/raydium_amm_v4_pool_tests.rs`
- **实现步骤**：
  1. 编写测试 `test_get_pool_by_mint`
  2. 编写测试 `test_get_pool_by_mint_force`
  3. 编写测试 `test_list_pools_by_mint`
  4. 测试缓存功能
  5. 测试强制刷新功能
- **验证方法**：运行测试，确认通过
- **优先级**：高

### 阶段 6：集成和文档（5 分钟）

#### 任务 6.1：更新 clear_all_pool_caches
- **文件路径**：`src/common/dex_pool_cache.rs`
- **实现步骤**：
  1. 在 `clear_all_pool_caches` 中添加 Raydium AMM V4 的缓存清除
  2. 调用 `raydium_amm_v4::clear_pool_cache()`
- **验证方法**：运行测试，确认通过
- **优先级**：中

#### 任务 6.2：添加文档注释
- **文件路径**：`src/instruction/utils/raydium_amm_v4.rs`
- **实现步骤**：
  1. 为新增的函数添加详细的文档注释
  2. 说明函数用途、参数、返回值、使用示例
  3. 添加注意事项（如缓存行为、性能考虑等）
- **验证方法**：运行 `cargo doc`，确认文档生成正确
- **优先级**：中

#### 任务 6.3：创建示例代码
- **文件路径**：`examples/raydium_amm_v4_find_pool_by_mint/`
- **实现步骤**：
  1. 创建示例目录和 Cargo.toml
  2. 创建 main.rs，演示如何使用新功能
  3. 包含 `get_pool_by_mint`、`get_pool_by_mint_force`、`list_pools_by_mint` 的使用示例
- **验证方法**：运行示例，确认可以正常工作
- **优先级**：低

### 阶段 7：最终验证（5 分钟）

#### 任务 7.1：运行所有测试
- **文件路径**：`tests/`
- **实现步骤**：
  1. 运行 `cargo test --package sol-trade-sdk`
  2. 确认所有测试通过
  3. 确认没有新的警告
- **验证方法**：测试全部通过
- **优先级**：高

#### 任务 7.2：运行示例
- **文件路径**：`examples/`
- **实现步骤**：
  1. 运行 `cargo run --package raydium_amm_v4_find_pool_by_mint`
  2. 确认示例可以正常工作
  3. 确认输出符合预期
- **验证方法**：示例正常运行
- **优先级**：高

#### 任务 7.3：代码审查
- **文件路径**：`src/instruction/utils/raydium_amm_v4.rs`
- **实现步骤**：
  1. 检查代码风格是否符合项目规范
  2. 检查错误处理是否完善
  3. 检查性能是否满足要求
  4. 检查文档是否完整
- **验证方法**：手动审查代码
- **优先级**：高

## 5. Todo 列表

- [ ] 任务 1.1：验证偏移量
- [ ] 任务 1.2：创建测试文件
- [ ] 任务 2.1：添加 MINT_TO_POOL_CACHE 缓存
- [ ] 任务 2.2：编写缓存测试
- [ ] 任务 3.1：实现 find_pools_by_mint_offset_collect
- [ ] 任务 3.2：编写 RPC 查询测试
- [ ] 任务 4.1：实现 find_pool_by_mint_impl
- [ ] 任务 4.2：编写池选择测试
- [ ] 任务 5.1：实现 get_pool_by_mint
- [ ] 任务 5.2：实现 get_pool_by_mint_force
- [ ] 任务 5.3：实现 list_pools_by_mint
- [ ] 任务 5.4：编写公共 API 测试
- [ ] 任务 6.1：更新 clear_all_pool_caches
- [ ] 任务 6.2：添加文档注释
- [ ] 任务 6.3：创建示例代码
- [ ] 任务 7.1：运行所有测试
- [ ] 任务 7.2：运行示例
- [ ] 任务 7.3：代码审查

**总计**：18 个任务

## 6. 实施步骤

### 步骤 1：准备阶段（5 分钟）
1. 验证偏移量（任务 1.1）
2. 创建测试文件（任务 1.2）
3. 提交代码

### 步骤 2：实现缓存层（10 分钟）
1. 添加 MINT_TO_POOL_CACHE 缓存（任务 2.1）
2. 编写缓存测试（任务 2.2）
3. 运行测试确认通过
4. 提交代码

### 步骤 3：实现 RPC 查询（15 分钟）
1. 实现 find_pools_by_mint_offset_collect（任务 3.1）
2. 编写 RPC 查询测试（任务 3.2）
3. 运行测试确认通过
4. 提交代码

### 步骤 4：实现池选择逻辑（10 分钟）
1. 实现 find_pool_by_mint_impl（任务 4.1）
2. 编写池选择测试（任务 4.2）
3. 运行测试确认通过
4. 提交代码

### 步骤 5：实现公共 API（10 分钟）
1. 实现 get_pool_by_mint（任务 5.1）
2. 实现 get_pool_by_mint_force（任务 5.2）
3. 实现 list_pools_by_mint（任务 5.3）
4. 编写公共 API 测试（任务 5.4）
5. 运行测试确认通过
6. 提交代码

### 步骤 6：集成和文档（5 分钟）
1. 更新 clear_all_pool_caches（任务 6.1）
2. 添加文档注释（任务 6.2）
3. 创建示例代码（任务 6.3）
4. 提交代码

### 步骤 7：最终验证（5 分钟）
1. 运行所有测试（任务 7.1）
2. 运行示例（任务 7.2）
3. 代码审查（任务 7.3）
4. 提交代码

**预计总时间**：60 分钟

## 7. 风险评估

### 风险 1：偏移量计算错误
**描述**：`coin_mint` 和 `pc_mint` 的偏移量可能计算错误
**影响**：查询会返回错误的数据或无法找到池
**概率**：低
**应对措施**：
- 仔细计算偏移量
- 与 TypeScript 实现对比验证
- 编写测试验证偏移量正确性

### 风险 2：RPC 查询性能问题
**描述**：`getProgramAccounts` 查询可能返回大量数据，导致性能问题
**影响**：查询延迟高，用户体验差
**概率**：中
**应对措施**：
- 使用 `dataSlice` 只获取必要数据
- 使用缓存减少重复查询
- 设置合理的超时时间

### 风险 3：缓存一致性问题
**描述**：缓存可能与链上数据不一致
**影响**：返回过期的数据
**概率**：低
**应对措施**：
- 提供强制刷新功能
- 设置合理的缓存过期时间
- 文档中说明缓存行为

### 风险 4：并发安全问题
**描述**：多个线程同时访问缓存可能导致数据竞争
**影响**：程序崩溃或数据损坏
**概率**：低
**应对措施**：
- 使用 `DashMap` 实现并发安全的缓存
- 编写并发测试验证

### 风险 5：测试覆盖率不足
**描述**：测试可能无法覆盖所有场景
**影响**：隐藏的 bug 未被发现
**概率**：中
**应对措施**：
- 编写全面的测试用例
- 使用边界值测试
- 使用集成测试验证

## 8. 验证标准

### 功能验证
- [ ] `get_pool_by_mint` 能够正确返回最优池
- [ ] `get_pool_by_mint_force` 能够强制刷新缓存
- [ ] `list_pools_by_mint` 能够返回所有池
- [ ] 缓存功能正常工作
- [ ] 错误处理完善

### 性能验证
- [ ] 缓存命中时查询延迟 < 1ms
- [ ] 缓存未命中时查询延迟 < 500ms
- [ ] 并发查询性能良好
- [ ] 内存占用合理（< 100MB）

### 代码质量验证
- [ ] 所有测试通过
- [ ] 没有编译警告
- [ ] 代码风格符合项目规范
- [ ] 文档完整且准确
- [ ] 示例代码可以正常运行

### 兼容性验证
- [ ] 与现有代码兼容
- [ ] 不影响其他 DEX 协议的功能
- [ ] 与 PumpSwap API 保持一致

## 9. 注意事项

### 重要注意事项
1. **偏移量验证**：务必验证 `coin_mint` 和 `pc_mint` 的偏移量是否正确
2. **缓存一致性**：注意缓存与链上数据的一致性问题
3. **错误处理**：完善错误处理，避免程序崩溃
4. **性能优化**：使用缓存和数据切片优化性能
5. **代码复用**：参考 PumpSwap 的实现，保持代码一致性

### 最佳实践
1. **TDD 方法**：先编写测试，再实现功能
2. **频繁提交**：每个小功能完成后立即提交
3. **代码审查**：完成后进行代码审查
4. **文档完善**：添加详细的文档注释
5. **示例代码**：提供易于理解的示例

### 避免事项
1. **不要过度优化**：避免过早优化，先保证功能正确
2. **不要忽略错误**：不要忽略任何错误，妥善处理所有异常
3. **不要硬编码**：避免硬编码，使用常量和配置
4. **不要忽略测试**：不要忽略测试，保证测试覆盖率
5. **不要破坏兼容性**：不要破坏与现有代码的兼容性

### 调试技巧
1. **使用日志**：使用日志输出调试信息
2. **使用断言**：使用断言验证假设
3. **使用测试**：使用测试验证功能
4. **使用文档**：参考文档和示例代码
5. **使用工具**：使用调试工具和性能分析工具

---

**创建日期**：2026-01-07
**创建者**：iFlow CLI
**版本**：1.0