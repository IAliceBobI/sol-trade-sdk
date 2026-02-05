# CLMM 官方源码分析文档

## 源码位置

```
/opt/projects/sol-trade-sdk/temp/raydium/clmm/raydium-clmm/
```

## 核心文件结构

### 1. Program 入口
- **`programs/amm/src/lib.rs`**: Program 入口
- **`programs/amm/src/instructions/swap.rs`**: Swap 指令实现（113KB，核心文件）

### 2. 状态定义
- **`programs/amm/src/states/pool.rs`**: PoolState 结构定义
- **`programs/amm/src/states/tick_array.rs`**: TickArrayState 结构定义
- **`programs/amm/src/states/config.rs`**: AmmConfig 结构定义

### 3. 数学库
- **`programs/amm/src/libraries/swap_math.rs`**: Swap 数学计算
- **`programs/amm/src/libraries/tick_math.rs`**: Tick 数学计算
- **`programs/amm/src/libraries/liquidity_math.rs`**: 流动性数学计算
- **`programs/amm/src/libraries/sqrt_price_math.rs`**: 价格数学计算

## 核心数据结构

### PoolState
```rust
pub struct PoolState {
    pub amm_config: Pubkey,
    pub token_mint_0: Pubkey,
    pub token_mint_1: Pubkey,
    pub token_vault_0: Pubkey,
    pub token_vault_1: Pubkey,
    pub observation_key: Pubkey,

    pub mint_decimals_0: u8,
    pub mint_decimals_1: u8,

    pub tick_spacing: u16,
    pub liquidity: u128,
    pub sqrt_price_x64: u128,
    pub tick_current: i32,

    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,

    pub tick_array_bitmap: [u64; 16],
    // ... 更多字段
}
```

**关键发现**:
- `tick_array_bitmap`: 16 个 u64 的位数组，用于跟踪哪些 tick arrays 已初始化
- `tick_current`: 当前价格对应的 tick（可以是负数！）
- `tick_spacing`: tick 之间的最小间距（USDT-WSOL pool = 1）

### TickArrayState
```rust
pub struct TickArrayState {
    pub pool_id: Pubkey,
    pub start_tick_index: i32,  // 这个数组的起始 tick 索引
    pub ticks: [TickState; 60],  // 固定 60 个 tick
    pub initialized_tick_count: u8,
    // ...
}
```

**关键发现**:
- 每个 TickArray 包含固定 60 个 tick
- `start_tick_index`: 必须是 `tick_spacing * 60` 的倍数
- USDT-WSOL pool 的 tick_spacing = 1，所以每个 array 覆盖 60 个 tick

### TickState
```rust
pub struct TickState {
    pub tick: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_x64: u128,
    pub fee_growth_outside_0_x64: u128,
    pub fee_growth_outside_1_x64: u128,
    // ...
}
```

## Swap 执行流程

### swap_internal 函数（核心逻辑）

**位置**: `programs/amm/src/instructions/swap.rs:128`

**主要步骤**:

1. **初始化 SwapState**
   ```rust
   let mut state = SwapState {
       amount_specified_remaining: amount_specified,
       amount_calculated: 0,
       sqrt_price_x64: pool_state.sqrt_price_x64,
       tick: pool_state.tick_current,
       liquidity: pool_state.liquidity,
       fee_amount: 0,
       // ...
   };
   ```

2. **获取第一个初始化的 tick array**
   ```rust
   let (mut is_match_pool_current_tick_array, first_valid_tick_array_start_index) =
       pool_state.get_first_initialized_tick_array(&tickarray_bitmap_extension, zero_for_one)?;
   ```

3. **主循环**: 遍历 tick arrays 直到输入耗尽或达到价格限制
   ```rust
   while state.amount_specified_remaining != 0
       && state.sqrt_price_x64 != sqrt_price_limit_x64
       && state.tick < MAX_TICK
       && state.tick > MIN_TICK
   {
       // 3.1 找到下一个初始化的 tick
       let next_initialized_tick = tick_array_current
           .next_initialized_tick(state.tick, pool_state.tick_spacing, zero_for_one)?;

       // 3.2 计算目标价格
       let target_price = if (zero_for_one && sqrt_price_next_x64 < sqrt_price_limit_x64)
           || (!zero_for_one && sqrt_price_next_x64 > sqrt_price_limit_x64) {
           sqrt_price_limit_x64
       } else {
           sqrt_price_next_x64
       };

       // 3.3 调用数学库计算 swap step
       let swap_step = swap_math::compute_swap_step(
           sqrt_price_current_x64,
           target_price,
           liquidity,
           amount_remaining,
           fee_rate,
           is_base_input,
           zero_for_one,
           block_timestamp,
       )?;

       // 3.4 更新状态
       state.sqrt_price_x64 = swap_step.sqrt_price_next_x64;
       state.amount_specified_remaining -= swap_step.amount_in + swap_step.fee_amount;
       state.amount_calculated += swap_step.amount_out;

       // 3.5 如果达到下一个 tick，更新流动性
       if state.sqrt_price_x64 == sqrt_price_next_x64 {
           let liquidity_delta = if zero_for_one { -liquidity_net } else { liquidity_net };
           state.liquidity = add_delta(state.liquidity, liquidity_delta)?;
           state.tick = if zero_for_one { tick_next - 1 } else { tick_next };
       }

       // 3.6 如果需要，移动到下一个 tick array
       if needs_next_tick_array(...) {
           tick_array_current = tick_array_states.pop_front()?;
       }
   }
   ```

## 如何解析 Simulate Log

### 启用日志功能

官方代码中有日志输出功能（通过 `#[cfg(feature = "enable-log")]`）:

```rust
#[cfg(feature = "enable-log")]
msg!(
    "while begin, is_base_input:{},fee_growth_global_x32:{}, state_sqrt_price_x64:{}, state_tick:{},state_liquidity:{},state.protocol_fee:{}, protocol_fee_rate:{}",
    is_base_input,
    state.fee_growth_global_x64,
    state.sqrt_price_x64,
    state.tick,
    state.liquidity,
    state.protocol_fee,
    amm_config.protocol_fee_rate
);
```

### 关键日志输出

1. **循环开始**: 打印当前状态
2. **next_initialized_tick**: 下一个 tick 的信息
3. **sqrt_price_current_x64**: 当前价格和目标价格
4. **swap_step**: 计算步骤的详细信息
5. **fee_growth_global**: 手续费增长信息

### 调试本地计算

要对比本地计算和链上执行：

1. **获取 simulate 结果中的日志**
   ```rust
   let simulation_result = rpc.simulate_transaction(transaction)?;
   let logs = simulation_result.logs;
   ```

2. **解析关键信息**
   - 查找 "while begin" 日志：获取初始状态
   - 查找 "sqrt_price_current_x64" 日志：获取每步的价格变化
   - 查找 "next_initialized_tick" 日志：获取 tick 转换信息

3. **对比本地计算**
   ```rust
   // 本地计算
   let local_quote = quote_exact_in(&rpc, params).await?;

   // 链上模拟
   let simulation_result = simulate_swap(...).await?;

   // 对比
   println!("本地计算: {}", local_quote.amount_out);
   println!("链上模拟: {}", simulation_result.amount_out);
   ```

## USDT-WSOL Pool 特殊性

### Pool 配置
```
Pool: ExcBWu8fGPdJiaF1b1z3iEef38sjQJks8xvj6M85pPY6
Token0 (WSOL): So11111111111111111111111111111111111111112
Token1 (USDT): Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB

tick_current: -23964 (负数！)
tick_spacing: 1 (非常小的间距)
liquidity: 504998593108
```

### 特殊问题

1. **负数 tick**: `-23964`
   - 需要特殊的数学处理（有符号 vs 无符号）
   - 可能导致本地计算错误

2. **小 tick_spacing**: `1`
   - 需要更多的 tick arrays
   - 每个 tick array 只覆盖 60 个 tick（因为 `tick_spacing * 60 = 60`）
   - 对于大额交易，可能需要跨越多个 tick arrays

3. **初始化的 tick 较少**
   - 可能导致"找不到下一个初始化 tick"的错误
   - 需要正确处理边界情况

## 修复本地计算的建议

### 1. 使用官方数学库

我们已经复制了官方的数学库到：
```
/opt/projects/sol-trade-sdk/src/utils/calc/clmm_math/
```

**已复制的文件**:
- `swap_math.rs`: Swap 计算
- `tick_math.rs`: Tick 计算
- `liquidity_math.rs`: 流动性计算
- `sqrt_price_math.rs`: 价格计算
- `full_math.rs`: 完整数学运算

### 2. 检查 tick array 查找逻辑

**问题**: `find_next_initialized_tick` 函数可能对负数 tick 处理不正确

**位置**: `src/utils/calc/raydium_clmm.rs:303`

```rust
fn find_next_initialized_tick(
    tick_arrays: &[(i32, Vec<(i32, i128, u128)>)],
    current_tick: i32,
    _tick_spacing: u16,
    zero_for_one: bool,
) -> Option<(i32, bool, i128)> {
    for (_start_index, ticks) in tick_arrays {
        for &(tick, liquidity_net, liquidity_gross) in ticks {
            let is_initialized = liquidity_gross > 0;

            if zero_for_one {
                if tick <= current_tick && is_initialized {  // 可能有问题
                    return Some((tick, is_initialized, liquidity_net));
                }
            } else if tick > current_tick && is_initialized {
                return Some((tick, is_initialized, liquidity_net));
            }
        }
    }
    None
}
```

**对比官方实现**（`tick_array.rs`）:
```rust
pub fn next_initialized_tick(
    &self,
    tick_index: i32,
    tick_spacing: u16,
    zero_for_one: bool,
) -> Result<Option<&TickState>> {
    // ... 更复杂的逻辑
}
```

### 3. 添加调试日志

在本地计算代码中添加详细日志：

```rust
use log::info;

info!("CLMM Quote 计算:");
info!("  current_tick: {}", tick_current);
info!("  sqrt_price_x64: {}", sqrt_price_x64);
info!("  liquidity: {}", liquidity);
info!("  zero_for_one: {}", zero_for_one);

for step in swap_steps {
    info!("Step:");
    info!("  tick_next: {}", step.tick_next);
    info!("  amount_in: {}", step.amount_in);
    info!("  amount_out: {}", step.amount_out);
    info!("  fee_amount: {}", step.fee_amount);
}
```

### 4. 使用 Simulate Result 验证

创建一个测试工具，对比：
1. 本地计算结果
2. 链上模拟结果
3. 实际执行结果

```rust
pub async fn verify_clmm_calculation(
    rpc: &SolanaRpcClient,
    pool_address: &Pubkey,
    amount_in: u64,
) -> Result<()> {
    // 1. 本地计算
    let local_quote = quote_exact_in(...).await?;

    // 2. 链上模拟
    let simulation_result = simulate_swap(...).await?;

    // 3. 对比
    println!("本地计算: {}", local_quote.amount_out);
    println!("链上模拟: {}", simulation_result.amount_out);
    println!("误差率: {}%",
        (local_quote.amount_out as f64 - simulation_result.amount_out as f64)
        / simulation_result.amount_out as f64 * 100.0
    );

    Ok(())
}
```

## 总结

1. **官方源码位置**: `/opt/projects/sol-trade-sdk/temp/raydium/clmm/raydium-clmm/`
2. **核心文件**: `programs/amm/src/instructions/swap.rs`
3. **数据结构**: `PoolState`, `TickArrayState`, `TickState`
4. **数学库**: 已复制到 `src/utils/calc/clmm_math/`
5. **调试方法**: 对比本地计算、链上模拟和实际执行结果
6. **USDT-WSOL 特殊性**: 负数 tick + 小 tick_spacing

通过参考官方源码，我们可以：
- 理解正确的 swap 执行流程
- 验证本地计算的准确性
- 修复 tick array 查找和处理的 bug
- 解析 simulate log 来调试问题
