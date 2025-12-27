# Raydium交易支持

<cite>
**本文档中引用的文件**  
- [raydium_amm_v4.rs](file://src/instruction/raydium_amm_v4.rs)
- [raydium_cpmm.rs](file://src/instruction/raydium_cpmm.rs)
- [raydium_amm_v4_types.rs](file://src/instruction/utils/raydium_amm_v4_types.rs)
- [raydium_cpmm_types.rs](file://src/instruction/utils/raydium_cpmm_types.rs)
- [raydium_amm_v4.rs](file://src/utils/calc/raydium_amm_v4.rs)
- [raydium_cpmm.rs](file://src/utils/calc/raydium_cpmm.rs)
- [price/raydium_amm_v4.rs](file://src/utils/price/raydium_amm_v4.rs)
- [price/raydium_cpmm.rs](file://src/utils/price/raydium_cpmm.rs)
- [compute_budget_manager.rs](file://src/trading/common/compute_budget_manager.rs)
- [raydium_amm_v4_trading/main.rs](file://examples/raydium_amm_v4_trading/src/main.rs)
- [raydium_cpmm_trading/main.rs](file://examples/raydium_cpmm_trading/src/main.rs)
- [ADDRESS_LOOKUP_TABLE_CN.md](file://docs/ADDRESS_LOOKUP_TABLE_CN.md)
</cite>

## 目录
1. [介绍](#介绍)
2. [AMM V4与CPMM流动性池模型](#amm-v4与cpmm流动性池模型)
3. [交换指令构建差异](#交换指令构建差异)
4. [状态账户处理](#状态账户处理)
5. [流动性计算与费用模型](#流动性计算与费用模型)
6. [价格预测实现](#价格预测实现)
7. [计算预算优化](#计算预算优化)
8. [交易压缩（地址查找表）](#交易压缩地址查找表)
9. [完整交易流程示例](#完整交易流程示例)
10. [性能比较与策略选择](#性能比较与策略选择)

## 介绍
sol-trade-sdk为Raydium协议提供了全面的支持，涵盖AMM V4和CPMM两种流动性池模型。本SDK通过优化的指令构建、精确的价格预测和高效的交易执行，为开发者提供了强大的工具来实现自动化交易策略。AMM V4基于订单簿模型，而CPMM采用恒定乘积做市商模型，两者在架构和性能特征上有显著差异。

**本节不分析具体源文件**

## AMM V4与CPMM流动性池模型
sol-trade-sdk支持Raydium的两种主要流动性池模型：AMM V4和CPMM。AMM V4（Automated Market Maker Version 4）是一种结合了订单簿和自动化做市商特性的混合模型，它使用Serum DEX作为底层订单匹配引擎，同时提供AMM流动性。CPMM（Constant Product Market Maker）则是一种纯粹的恒定乘积做市商模型，遵循x*y=k的数学公式，其中x和y是池中两种资产的数量，k是常数。

AMM V4模型更适合高流动性交易对，因为它可以利用订单簿的深度，而CPMM模型则更适合长尾资产和新上线的代币，因为它提供了简单的定价机制和较低的进入门槛。在sol-trade-sdk中，这两种模型通过不同的指令构建器和参数结构进行区分，确保了交易逻辑的清晰分离。

**Section sources**
- [raydium_amm_v4.rs](file://src/instruction/raydium_amm_v4.rs#L1-L252)
- [raydium_cpmm.rs](file://src/instruction/raydium_cpmm.rs#L1-L309)

## 交换指令构建差异
AMM V4和CPMM在交换指令的构建上存在显著差异，主要体现在指令数据结构、账户列表和指令标识符上。AMM V4的交换指令使用单字节的判别器`[9]`表示基础代币输入交换，而CPMM使用8字节的判别器`[143, 190, 90, 218, 196, 30, 51, 222]`。这种差异反映了两种协议不同的序列化格式和指令分派机制。

在账户列表方面，AMM V4需要17个账户，包括Serum市场、出价、要价、事件队列等订单簿相关账户，而CPMM只需要13个账户，主要关注池状态、金库和观察状态账户。这种简化使得CPMM交易更轻量，交易费用更低。指令数据的构建也不同：AMM V4使用17字节的数据（1字节判别器+8字节输入金额+8字节最小输出金额），而CPMM使用24字节的数据（8字节判别器+8字节输入金额+8字节最小输出金额）。

```mermaid
flowchart TD
A["AMM V4指令构建"] --> B["单字节判别器 [9]"]
A --> C["17字节指令数据"]
A --> D["17个账户"]
A --> E["包含Serum订单簿账户"]
F["CPMM指令构建"] --> G["8字节判别器 [143,190,90,218,196,30,51,222]"]
F --> H["24字节指令数据"]
F --> I["13个账户"]
F --> J["包含观察状态账户"]
style A fill:#f9f,stroke:#333
style F fill:#f9f,stroke:#333
```

**Diagram sources**
- [raydium_amm_v4.rs](file://src/instruction/raydium_amm_v4.rs#L120-L123)
- [raydium_cpmm.rs](file://src/instruction/raydium_cpmm.rs#L150-L153)

**Section sources**
- [raydium_amm_v4.rs](file://src/instruction/raydium_amm_v4.rs#L120-L129)
- [raydium_cpmm.rs](file://src/instruction/raydium_cpmm.rs#L150-L159)

## 状态账户处理
在状态账户处理方面，AMM V4和CPMM采用了不同的架构。AMM V4使用一个包含752字节数据的`AmmInfo`结构体，该结构体包含了订单簿状态、流动性池信息和费用参数。这个结构体通过`amm_info_decode`函数从账户数据中解析，包含了丰富的市场信息，如订单数量、深度、最小交易量等。

相比之下，CPMM使用一个629字节的`PoolState`结构体，该结构体更加专注于流动性池的核心参数，如代币金库、LP代币、协议费用和观察状态。CPMM还引入了观察状态账户（Observation State Account），用于支持时间加权平均价格（TWAP）计算，这是AMM V4所不具备的功能。在sol-trade-sdk中，这些状态账户通过PDA（程序派生地址）机制生成，AMM V4使用`pool`种子，而CPMM使用`pool`、`pool_vault`和`observation`三种种子。

```mermaid
classDiagram
class AmmInfo {
+status : u64
+nonce : u64
+order_num : u64
+depth : u64
+coin_decimals : u64
+pc_decimals : u64
+token_coin : Pubkey
+token_pc : Pubkey
+coin_mint : Pubkey
+pc_mint : Pubkey
+open_orders : Pubkey
+market : Pubkey
+serum_dex : Pubkey
+fees : Fees
+out_put : OutPutData
}
class PoolState {
+amm_config : Pubkey
+pool_creator : Pubkey
+token0_vault : Pubkey
+token1_vault : Pubkey
+lp_mint : Pubkey
+token0_mint : Pubkey
+token1_mint : Pubkey
+token0_program : Pubkey
+token1_program : Pubkey
+observation_key : Pubkey
+auth_bump : u8
+status : u8
+lp_mint_decimals : u8
+mint0_decimals : u8
+mint1_decimals : u8
+lp_supply : u64
+protocol_fees_token0 : u64
+protocol_fees_token1 : u64
+fund_fees_token0 : u64
+fund_fees_token1 : u64
+open_time : u64
+recent_epoch : u64
}
class Fees {
+min_separate_numerator : u64
+min_separate_denominator : u64
+trade_fee_numerator : u64
+trade_fee_denominator : u64
+pnl_numerator : u64
+pnl_denominator : u64
+swap_fee_numerator : u64
+swap_fee_denominator : u64
}
AmmInfo *-- Fees
AmmInfo : "752 bytes"
PoolState : "629 bytes"
```

**Diagram sources**
- [raydium_amm_v4_types.rs](file://src/instruction/utils/raydium_amm_v4_types.rs#L5-L80)
- [raydium_cpmm_types.rs](file://src/instruction/utils/raydium_cpmm_types.rs#L5-L40)

**Section sources**
- [raydium_amm_v4_types.rs](file://src/instruction/utils/raydium_amm_v4_types.rs#L5-L80)
- [raydium_cpmm_types.rs](file://src/instruction/utils/raydium_cpmm_types.rs#L5-L40)

## 流动性计算与费用模型
AMM V4和CPMM在流动性计算和费用模型上采用了不同的数学公式和费用结构。AMM V4的流动性计算基于`swap_base_input`函数，该函数首先计算交易费用（25/10000），然后从输入金额中扣除费用，最后使用恒定乘积公式计算输出金额。AMM V4的费用模型包括交易费用（25/10000）和交换费用（25/10000），其中交换费用是交易费用的一部分。

CPMM的流动性计算也基于`swap_base_input`函数，但其费用模型更加复杂，包括交易费用（2500/1000000）、协议费用（120000/1000000）、基金费用（40000/1000000）和创建者费用（0/1000000）。CPMM使用1,000,000作为分母基数，提供了更高的费用精度。在计算过程中，CPMM首先计算交易费用，然后根据`is_creator_fee_on_input`标志决定创建者费用是应用于输入还是输出代币。

```mermaid
flowchart TD
A["AMM V4流动性计算"] --> B["输入金额"]
B --> C["计算交易费用 = 输入金额 * 25/10000"]
C --> D["扣除交易费用"]
D --> E["使用恒定乘积公式计算输出"]
E --> F["计算交换费用 = 交易费用 * 25/10000"]
F --> G["最终输出金额"]
H["CPMM流动性计算"] --> I["输入金额"]
I --> J["计算交易费用 = 输入金额 * 2500/1000000"]
J --> K["扣除交易费用"]
K --> L["使用恒定乘积公式计算输出"]
L --> M["计算协议费用 = 交易费用 * 120000/1000000"]
M --> N["计算基金费用 = 交易费用 * 40000/1000000"]
N --> O["计算创建者费用"]
O --> P["最终输出金额"]
style A fill:#f9f,stroke:#333
style H fill:#f9f,stroke:#333
```

**Diagram sources**
- [raydium_amm_v4.rs](file://src/utils/calc/raydium_amm_v4.rs#L76-L104)
- [raydium_cpmm.rs](file://src/utils/calc/raydium_cpmm.rs#L100-L146)

**Section sources**
- [raydium_amm_v4.rs](file://src/utils/calc/raydium_amm_v4.rs#L76-L104)
- [raydium_cpmm.rs](file://src/utils/calc/raydium_cpmm.rs#L100-L146)

## 价格预测实现
sol-trade-sdk通过`utils/price/raydium_amm_v4.rs`和`utils/price/raydium_cpmm.rs`模块实现了精确的价格预测功能。这两个模块提供了相同的API接口，包括`price_base_in_quote`和`price_quote_in_base`函数，但它们都委托给`utils/price/common.rs`中的通用实现。这种设计确保了代码的复用性和一致性。

价格预测的实现考虑了代币的小数位数，通过调整数量来计算实际价格。例如，`price_base_in_quote`函数将基础代币储备和报价代币储备分别除以10的相应小数位数次方，然后计算报价代币相对于基础代币的价格。这种精确的计算对于避免滑点和确保交易执行的准确性至关重要。在实际使用中，开发者可以通过调用这些函数来获取当前市场价格，从而做出更明智的交易决策。

**Section sources**
- [raydium_amm_v4.rs](file://src/utils/price/raydium_amm_v4.rs#L1-L48)
- [raydium_cpmm.rs](file://src/utils/price/raydium_cpmm.rs#L1-L48)

## 计算预算优化
sol-trade-sdk通过`trading/common/compute_budget_manager.rs`模块实现了计算预算优化。该模块使用`DashMap`和`Lazy`静态变量来缓存计算预算指令，避免了重复创建相同的指令。计算预算指令包括设置计算单元限制、计算单元价格和加载账户数据大小限制。

该优化机制根据`unit_price`、`unit_limit`、`data_size_limit`和`is_buy`参数生成唯一的缓存键，然后检查缓存中是否存在对应的指令。如果存在，则直接返回缓存的指令；否则，创建新的指令并存入缓存。这种缓存机制显著减少了内存分配和指令创建的开销，特别是在高频交易场景下。对于买入交易，SDK还会设置加载账户数据大小限制，以优化地址查找表的使用。

```mermaid
sequenceDiagram
participant Developer
participant ComputeBudgetManager
participant Cache
Developer->>ComputeBudgetManager : 请求计算预算指令
ComputeBudgetManager->>Cache : 检查缓存
alt 缓存命中
Cache-->>ComputeBudgetManager : 返回缓存指令
ComputeBudgetManager-->>Developer : 返回指令
else 缓存未命中
ComputeBudgetManager->>ComputeBudgetManager : 创建新指令
ComputeBudgetManager->>Cache : 存储指令到缓存
ComputeBudgetManager-->>Developer : 返回新指令
end
```

**Diagram sources**
- [compute_budget_manager.rs](file://src/trading/common/compute_budget_manager.rs#L1-L65)

**Section sources**
- [compute_budget_manager.rs](file://src/trading/common/compute_budget_manager.rs#L1-L65)

## 交易压缩（地址查找表）
sol-trade-sdk支持通过地址查找表（Address Lookup Table, ALT）实现交易压缩。地址查找表是Solana的一项功能，允许将经常使用的地址存储在紧凑的表格中，并通过索引引用它们，从而显著减少交易大小和成本。在使用地址查找表时，交易中的完整32字节地址被替换为1字节的索引，这可以将交易大小减少35%以上，交易费用节省高达30%。

在sol-trade-sdk中，开发者可以通过在交易参数中包含`address_lookup_table_account`来启用地址查找表。SDK会自动处理地址的压缩和解压，确保交易的正确执行。为了获得最佳效果，建议在高频交易场景中使用地址查找表，特别是在需要引用大量地址的复杂交易中。使用地址查找表还需要确保RPC提供商支持该功能，并在主网使用前在开发网进行充分测试。

**Section sources**
- [ADDRESS_LOOKUP_TABLE_CN.md](file://docs/ADDRESS_LOOKUP_TABLE_CN.md#L1-L67)

## 完整交易流程示例
sol-trade-sdk提供了完整的交易流程示例，从价格查询、交易构建到广播和确认。以`examples/raydium_amm_v4_trading/src/main.rs`为例，交易流程包括：首先通过gRPC订阅Raydium AMM V4的交换事件，然后在事件回调中创建SolanaTrade客户端，获取最新的区块哈希，计算滑点，构建买入和卖出指令，并最终广播交易。

在买入阶段，SDK会检查是否需要创建输入代币的关联代币账户（ATA），计算最小输出金额，构建交换指令，并根据需要添加计算预算指令。在卖出阶段，SDK会查询用户的代币余额，确保有足够的代币进行卖出，然后构建相应的卖出指令。整个流程通过异步执行，确保了高性能和低延迟。交易确认后，SDK会自动处理关联代币账户的关闭，回收租金。

**Section sources**
- [raydium_amm_v4_trading/main.rs](file://examples/raydium_amm_v4_trading/src/main.rs#L1-L222)
- [raydium_cpmm_trading/main.rs](file://examples/raydium_cpmm_trading/src/main.rs#L1-L202)

## 性能比较与策略选择
AMM V4和CPMM在交易延迟、滑点和成本方面表现出不同的性能特征。AMM V4由于依赖Serum订单簿，通常具有更低的滑点和更好的价格执行，但交易延迟可能稍高，因为需要与订单簿交互。CPMM由于其简单的恒定乘积模型，交易执行速度更快，延迟更低，但在高波动性市场中可能产生更高的滑点。

在交易成本方面，CPMM通常更便宜，因为其交易更轻量，账户更少。然而，AMM V4可能在某些情况下提供更好的整体成本效益，因为它可以利用订单簿的深度。对于开发者来说，选择最优策略取决于具体的交易场景：对于高频交易和低延迟要求，CPMM可能是更好的选择；而对于大额交易和价格敏感型交易，AMM V4可能更合适。此外，开发者还应考虑流动性、代币对的成熟度和市场条件等因素。

```mermaid
graph TB
A[交易策略选择] --> B{交易类型}
B --> C[高频交易]
B --> D[大额交易]
B --> E[新代币交易]
C --> F[选择CPMM]
C --> G[低延迟]
C --> H[低成本]
D --> I[选择AMM V4]
D --> J[低滑点]
D --> K[好价格执行]
E --> L[选择CPMM]
E --> M[简单定价]
E --> N[低进入门槛]
style A fill:#f9f,stroke:#333
```

**Diagram sources**
- [raydium_amm_v4.rs](file://src/instruction/raydium_amm_v4.rs#L1-L252)
- [raydium_cpmm.rs](file://src/instruction/raydium_cpmm.rs#L1-L309)

**Section sources**
- [raydium_amm_v4.rs](file://src/instruction/raydium_amm_v4.rs#L1-L252)
- [raydium_cpmm.rs](file://src/instruction/raydium_cpmm.rs#L1-L309)