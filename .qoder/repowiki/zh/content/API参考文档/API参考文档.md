# API参考文档

<cite>
**本文档中引用的文件**   
- [lib.rs](file://src/lib.rs)
- [trading/factory.rs](file://src/trading/factory.rs)
- [trading/core/traits.rs](file://src/trading/core/traits.rs)
- [trading/core/params.rs](file://src/trading/core/params.rs)
- [common/types.rs](file://src/common/types.rs)
- [constants/trade.rs](file://src/constants/trade.rs)
- [trading/core/executor.rs](file://src/trading/core/executor.rs)
- [instruction/raydium_amm_v4.rs](file://src/instruction/raydium_amm_v4.rs)
- [instruction/pumpfun.rs](file://src/instruction/pumpfun.rs)
- [instruction/bonk.rs](file://src/instruction/bonk.rs)
</cite>

## 目录
1. [简介](#简介)
2. [核心组件](#核心组件)
3. [TradingClient公共接口](#tradingclient公共接口)
4. [TradeFactory工厂模式](#tradefactory工厂模式)
5. [DexExecutor交易执行器](#dexecutor交易执行器)
6. [TradeParams和TradeConfig结构体](#tradeparams和tradeconfig结构体)
7. [API调用示例](#api调用示例)
8. [错误处理](#错误处理)

## 简介

sol-trade-sdk是一个为Solana区块链设计的高性能交易SDK，提供统一的接口来与多个去中心化交易所（DEX）进行交互。该SDK支持PumpFun、PumpSwap、Bonk、Raydium AMM V4、Raydium CPMM和Meteora DAMM V2等多种交易协议，通过优化的交易构建和执行流程，实现低延迟的交易操作。

SDK的核心设计理念是提供一个简洁而强大的API，使开发者能够轻松地在不同DEX协议之间进行交易，同时保持高性能和可靠性。通过工厂模式和策略模式的结合，SDK实现了协议无关的交易接口，同时保留了各协议的特定功能。

**Section sources**
- [lib.rs](file://src/lib.rs#L49-L53)

## 核心组件

sol-trade-sdk的核心架构由以下几个关键组件构成：`TradingClient`作为主要的交易客户端，`TradeFactory`负责创建特定协议的执行器，`DexExecutor`定义了交易执行的通用接口，以及各种参数结构体来配置交易行为。

这些组件共同工作，提供了一个分层的架构，其中高层API隐藏了底层协议的复杂性，同时仍然允许对交易细节进行精细控制。这种设计使得SDK既易于使用，又具有足够的灵活性来满足高级交易策略的需求。

**Section sources**
- [lib.rs](file://src/lib.rs#L54-L66)
- [trading/factory.rs](file://src/trading/factory.rs#L11-L20)
- [trading/core/traits.rs](file://src/trading/core/traits.rs#L5-L15)

## TradingClient公共接口

`TradingClient`是sol-trade-sdk的主要入口点，提供了与Solana DEX协议交互的统一接口。该结构体封装了交易所需的所有必要组件，包括支付密钥对、RPC客户端、SWQOS客户端和中间件管理器。

### new方法

创建一个新的`TradingClient`实例，初始化所有必要的组件。

**参数**
- `payer`: 用于签名所有交易的密钥对（Arc<Keypair>）
- `trade_config`: 交易配置，包含RPC URL、SWQOS配置和提交级别

**返回值**
- 返回配置好的`TradingClient`实例

**Section sources**
- [lib.rs](file://src/lib.rs#L186-L298)

### buy方法

执行买入订单，购买指定的代币。

**参数**
- `params`: `TradeBuyParams`结构体，包含所有必要的交易配置

**返回值**
- `Result<(bool, Vec<Signature>, Option<TradeError>), anyhow::Error>`: 包含成功标志、所有提交的交易签名和可能的错误信息

**可能抛出的错误**
- 提供的协议参数对指定的DEX类型无效
- 交易执行失败
- 发生网络或RPC错误
- 购买所需的SOL余额不足
- 无法创建或访问必需的账户

**Section sources**
- [lib.rs](file://src/lib.rs#L369-L456)

### sell方法

执行卖出订单，出售指定的代币。

**参数**
- `params`: `TradeSellParams`结构体，包含所有必要的交易配置

**返回值**
- `Result<(bool, Vec<Signature>, Option<TradeError>), anyhow::Error>`: 包含成功标志、所有提交的交易签名和可能的错误信息

**可能抛出的错误**
- 提供的协议参数对指定的DEX类型无效
- 交易执行失败
- 发生网络或RPC错误
- 出售所需的代币余额不足
- 代币账户不存在或未正确初始化
- 无法创建或访问必需的账户

**Section sources**
- [lib.rs](file://src/lib.rs#L484-L572)

### sell_by_percent方法

根据指定百分比执行卖出订单。

**参数**
- `params`: `TradeSellParams`结构体（将用计算出的代币数量修改）
- `amount_token`: 可用的代币总数量（以最小代币单位）
- `percent`: 要出售的代币百分比（1-100，其中100=100%）

**返回值**
- `Result<(bool, Vec<Signature>, Option<TradeError>), anyhow::Error>`: 包含成功标志、所有提交的交易签名和可能的错误信息

**可能抛出的错误**
- `percent`为0或大于100
- 提供的协议参数对指定的DEX类型无效
- 交易执行失败
- 发生网络或RPC错误
- 计算出的出售数量所需的代币余额不足
- 代币账户不存在或未正确初始化
- 无法创建或访问必需的账户

**Section sources**
- [lib.rs](file://src/lib.rs#L600-L612)

### wrap_sol_to_wsol方法

将原生SOL包装为wSOL（Wrapped SOL）以用于SPL代币操作。

**参数**
- `amount`: 要包装的SOL数量（以lamports为单位）

**返回值**
- `Result<String, anyhow::Error>`: 成功时返回交易签名，失败时返回错误

**可能抛出的错误**
- 包装操作所需的SOL余额不足
- wSOL关联代币账户创建失败
- 交易执行或确认失败
- 发生网络或RPC错误

**Section sources**
- [lib.rs](file://src/lib.rs#L634-L644)

### close_wsol方法

关闭wSOL关联代币账户并将剩余余额解包装为原生SOL。

**返回值**
- `Result<String, anyhow::Error>`: 成功时返回交易签名，失败时返回错误

**可能抛出的错误**
- wSOL关联代币账户不存在
- 由于权限不足导致账户关闭失败
- 交易执行或确认失败
- 发生网络或RPC错误

**Section sources**
- [lib.rs](file://src/lib.rs#L645-L672)

### create_wsol_ata方法

创建wSOL关联代币账户而不包装任何SOL。

**返回值**
- `Result<String, anyhow::Error>`: 成功时返回交易签名，失败时返回错误

**可能抛出的错误**
- wSOL ATA账户已存在（幂等，将静默成功）
- 交易执行或确认失败
- 发生网络或RPC错误
- 交易费用所需的SOL不足

**Section sources**
- [lib.rs](file://src/lib.rs#L691-L708)

### wrap_wsol_to_sol方法

将WSOL转换为SOL，使用seed账户。

**参数**
- `amount`: 要转换的WSOL数量（以lamports为单位）

**返回值**
- `Result<String, anyhow::Error>`: 成功时返回交易签名，失败时返回错误

**可能抛出的错误**
- 用户WSOL ATA余额不足
- seed账户创建失败
- 转账指令执行失败
- 交易执行或确认失败
- 发生网络或RPC错误

**Section sources**
- [lib.rs](file://src/lib.rs#L733-L760)

## TradeFactory工厂模式

`TradeFactory`实现了工厂模式，根据交易平台类型创建相应的DEX执行器。这种设计模式允许SDK在运行时动态选择和实例化适当的交易执行器，而无需客户端代码了解具体实现细节。

### create_executor方法

创建指定协议的交易执行器。

**参数**
- `dex_type`: `DexType`枚举，指定要创建的执行器类型

**返回值**
- `Arc<dyn TradeExecutor>`: 指定协议的交易执行器的智能指针

**支持的DEX类型**
- `PumpFun`: PumpFun协议执行器
- `PumpSwap`: PumpSwap协议执行器
- `Bonk`: Bonk协议执行器
- `RaydiumCpmm`: Raydium CPMM协议执行器
- `RaydiumAmmV4`: Raydium AMM V4协议执行器
- `MeteoraDammV2`: Meteora DAMM V2协议执行器

**实现细节**
`TradeFactory`使用`LazyLock`静态实例来确保每个协议的执行器只创建一次，从而实现零运行时开销的单例模式。这不仅提高了性能，还确保了跨多个交易的一致性。

**Section sources**
- [trading/factory.rs](file://src/trading/factory.rs#L27-L98)

## DexExecutor交易执行器

`DexExecutor` trait定义了所有交易协议必须实现的核心方法，为不同DEX协议提供了一致的接口。

### TradeExecutor trait

```rust
#[async_trait::async_trait]
pub trait TradeExecutor: Send + Sync {
    async fn swap(&self, params: SwapParams) -> Result<(bool, Vec<Signature>, Option<anyhow::Error>)>;
    fn protocol_name(&self) -> &'static str;
}
```

**swap方法**
- 执行交易的核心方法，接受`SwapParams`参数并返回交易结果
- 返回值包含成功标志、所有提交的交易签名和可能的最后一个错误

**protocol_name方法**
- 返回协议名称的静态字符串
- 用于日志记录和调试

### 具体实现

#### RaydiumAmmV4执行器

处理Raydium AMM V4协议的交易，通过`RaydiumAmmV4InstructionBuilder`构建特定于协议的指令。

**Section sources**
- [trading/core/traits.rs](file://src/trading/core/traits.rs#L5-L15)
- [instruction/raydium_amm_v4.rs](file://src/instruction/raydium_amm_v4.rs#L17-L252)

#### PumpFun执行器

处理PumpFun协议的交易，通过`PumpFunInstructionBuilder`构建特定于协议的指令。

**Section sources**
- [trading/core/traits.rs](file://src/trading/core/traits.rs#L5-L15)
- [instruction/pumpfun.rs](file://src/instruction/pumpfun.rs#L24-L291)

#### Bonk执行器

处理Bonk协议的交易，通过`BonkInstructionBuilder`构建特定于协议的指令。

**Section sources**
- [trading/core/traits.rs](file://src/trading/core/traits.rs#L5-L15)
- [instruction/bonk.rs](file://src/instruction/bonk.rs)

## TradeParams和TradeConfig结构体

### TradeBuyParams结构体

包含执行买入订单所需的所有配置。

**字段**
- `dex_type`: 使用的DEX协议
- `input_token_type`: 要购买的代币类型
- `mint`: 要购买的代币的公钥
- `input_token_amount`: 要购买的代币数量（以最小代币单位）
- `slippage_basis_points`: 可选的滑点容忍度（以基点为单位，例如100=1%）
- `recent_blockhash`: 交易有效性的最近区块哈希
- `extension_params`: 协议特定参数（PumpFun、Raydium等）
- `address_lookup_table_account`: 可选的地址查找表以优化交易大小
- `wait_transaction_confirmed`: 是否在返回前等待交易确认
- `create_input_token_ata`: 是否创建输入代币关联代币账户
- `close_input_token_ata`: 是否在交易后关闭输入代币关联代币账户
- `create_mint_ata`: 是否创建代币铸币关联代币账户
- `durable_nonce`: 耐用的nonce信息
- `fixed_output_token_amount`: 可选的固定输出代币数量
- `gas_fee_strategy`: 燃气费策略
- `simulate`: 是否模拟交易而不是执行它

**Section sources**
- [lib.rs](file://src/lib.rs#L89-L125)

### TradeSellParams结构体

包含执行卖出订单所需的所有配置。

**字段**
- `dex_type`: 使用的DEX协议
- `output_token_type`: 要出售的代币类型
- `mint`: 要出售的代币的公钥
- `input_token_amount`: 要出售的代币数量（以最小代币单位）
- `slippage_basis_points`: 可选的滑点容忍度（以基点为单位，例如100=1%）
- `recent_blockhash`: 交易有效性的最近区块哈希
- `with_tip`: 是否包含小费以提高交易优先级
- `extension_params`: 协议特定参数（PumpFun、Raydium等）
- `address_lookup_table_account`: 可选的地址查找表以优化交易大小
- `wait_transaction_confirmed`: 是否在返回前等待交易确认
- `create_output_token_ata`: 是否创建输出代币关联代币账户
- `close_output_token_ata`: 是否在交易后关闭输出代币关联代币账户
- `close_mint_token_ata`: 是否在交易后关闭铸币代币关联代币账户
- `durable_nonce`: 耐用的nonce信息
- `fixed_output_token_amount`: 可选的固定输出代币数量
- `gas_fee_strategy`: 燃气费策略
- `simulate`: 是否模拟交易而不是执行它

**Section sources**
- [lib.rs](file://src/lib.rs#L131-L169)

### TradeConfig结构体

定义交易的全局配置。

**字段**
- `rpc_url`: Solana RPC端点URL
- `swqos_configs`: SWQOS（Solana Web Quality of Service）配置列表
- `commitment`: RPC调用的交易提交级别
- `create_wsol_ata_on_startup`: 启动时是否创建WSOL ATA（默认：true）
- `use_seed_optimize`: 是否对所有ATA操作使用seed优化（默认：true）

**Section sources**
- [common/types.rs](file://src/common/types.rs#L5-L14)

## API调用示例

### Raydium AMM V4交易示例

```rust
let client = create_solana_trade_client().await?;
let params = RaydiumAmmV4Params::new(
    trade_info.amm,
    amm_info.coin_mint,
    amm_info.pc_mint,
    amm_info.token_coin,
    amm_info.token_pc,
    coin_reserve,
    pc_reserve,
);
let buy_params = sol_trade_sdk::TradeBuyParams {
    dex_type: DexType::RaydiumAmmV4,
    input_token_type: if is_wsol { TradeTokenType::WSOL } else { TradeTokenType::USDC },
    mint: mint_pubkey,
    input_token_amount: input_token_amount,
    slippage_basis_points: Some(100),
    recent_blockhash: Some(recent_blockhash),
    extension_params: DexParamEnum::RaydiumAmmV4(params),
    address_lookup_table_account: None,
    wait_transaction_confirmed: true,
    create_input_token_ata: is_wsol,
    close_input_token_ata: is_wsol,
    create_mint_ata: true,
    durable_nonce: None,
    fixed_output_token_amount: None,
    gas_fee_strategy: gas_fee_strategy.clone(),
    simulate: false,
};
client.buy(buy_params).await?;
```

**Section sources**
- [examples/raydium_amm_v4_trading/src/main.rs](file://examples/raydium_amm_v4_trading/src/main.rs#L156-L179)

### PumpFun狙击交易示例

```rust
let buy_params = sol_trade_sdk::TradeBuyParams {
    dex_type: DexType::PumpFun,
    input_token_type: TradeTokenType::SOL,
    mint: mint_pubkey,
    input_token_amount: buy_sol_amount,
    slippage_basis_points: Some(300),
    recent_blockhash: Some(recent_blockhash),
    extension_params: DexParamEnum::PumpFun(PumpFunParams::from_dev_trade(
        trade_info.mint,
        trade_info.token_amount,
        trade_info.max_sol_cost,
        trade_info.creator,
        trade_info.bonding_curve,
        trade_info.associated_bonding_curve,
        trade_info.creator_vault,
        None,
        trade_info.fee_recipient,
        trade_info.token_program,
    )),
    address_lookup_table_account: None,
    wait_transaction_confirmed: true,
    create_input_token_ata: true,
    close_input_token_ata: true,
    create_mint_ata: true,
    durable_nonce: None,
    fixed_output_token_amount: None,
    gas_fee_strategy: gas_fee_strategy.clone(),
    simulate: false,
};
client.buy(buy_params).await?;
```

**Section sources**
- [examples/pumpfun_sniper_trading/src/main.rs](file://examples/pumpfun_sniper_trading/src/main.rs#L94-L126)

### Bonk复制交易示例

```rust
let buy_params = sol_trade_sdk::TradeBuyParams {
    dex_type: DexType::Bonk,
    input_token_type: input_token_type.clone(),
    mint: mint_pubkey,
    input_token_amount: buy_sol_amount,
    slippage_basis_points: Some(100),
    recent_blockhash: Some(recent_blockhash),
    extension_params: DexParamEnum::Bonk(BonkParams::from_trade(
        trade_info.virtual_base,
        trade_info.virtual_quote,
        trade_info.real_base_after,
        trade_info.real_quote_after,
        trade_info.pool_state,
        trade_info.base_vault,
        trade_info.quote_vault,
        trade_info.base_token_program,
        trade_info.platform_config,
        trade_info.platform_associated_account,
        trade_info.creator_associated_account,
        trade_info.global_config,
    )),
    address_lookup_table_account: None,
    wait_transaction_confirmed: true,
    create_input_token_ata: true,
    close_input_token_ata: false,
    create_mint_ata: true,
    durable_nonce: None,
    fixed_output_token_amount: None,
    gas_fee_strategy: gas_fee_strategy.clone(),
    simulate: false,
};
client.buy(buy_params).await?;
```

**Section sources**
- [examples/bonk_copy_trading/src/main.rs](file://examples/bonk_copy_trading/src/main.rs#L150-L181)

## 错误处理

sol-trade-sdk提供了全面的错误处理机制，确保交易操作的可靠性和可预测性。

### 通用错误类型

- `Invalid protocol params for Trade`: 提供的协议参数对指定的DEX类型无效
- `Percentage must be between 1 and 100`: `sell_by_percent`方法的百分比参数无效
- `Amount cannot be zero`: 交易金额为零
- `Token amount is required`: 卖出操作缺少代币数量
- `Pool must contain WSOL or USDC`: Raydium池必须包含WSOL或USDC

### 网络和RPC错误

所有网络和RPC相关的错误都会通过`anyhow::Error`类型返回，允许调用者进行适当的错误处理和重试逻辑。

### 交易执行错误

交易执行错误通过返回值中的`Option<TradeError>`字段传递，包含最后一个错误（如果所有尝试都失败）。这允许客户端代码检查交易结果并采取适当的措施。

**Section sources**
- [lib.rs](file://src/lib.rs#L360-L368)
- [lib.rs](file://src/lib.rs#L474-L483)
- [lib.rs](file://src/lib.rs#L591-L599)
- [instruction/raydium_amm_v4.rs](file://src/instruction/raydium_amm_v4.rs#L25-L27)
- [instruction/pumpfun.rs](file://src/instruction/pumpfun.rs#L38-L40)