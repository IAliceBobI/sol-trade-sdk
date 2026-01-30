# 统一交易接口设计文档 v2.0

**日期**: 2025-01-30
**版本**: v4.0.0
**状态**: 设计完成，待实施
**Breaking Change**: 是

---

## 📋 设计目标

创建统一的交易接口，支持三种模式：
1. **本地计算** - 快速估算（`buy_quote`）
2. **链上模拟** - 准确验证（`buy_simulate`）
3. **真实执行** - 实际交易（`buy`，保持现有）

**核心原则**：
- ✅ 使用现有的 `TradeBuyParams` 参数结构
- ✅ 复用现有的 SwapParams 构建逻辑
- ✅ 自动推断交易方向（从参数直接比较）
- ✅ 最小化导出
- ✅ Breaking change，不考虑向后兼容

---

## 🏗️ 核心 API 设计

```rust
impl TradingClient {
    /// 本地计算（新增）
    pub async fn buy_quote(
        &self,
        params: TradeBuyParams,
    ) -> Result<QuoteResult, TradingError>;

    /// 链上模拟（新增）
    pub async fn buy_simulate(
        &self,
        params: TradeBuyParams,
    ) -> Result<SimulationResult, TradingError>;

    /// 真实执行（保持现有）
    pub async fn buy(
        &self,
        params: TradeBuyParams,
    ) -> Result<(bool, Vec<Signature>, Option<TradeError>), anyhow::Error>;
}
```

---

## 📦 数据结构

### QuoteResult

```rust
#[derive(Debug, Clone)]
pub struct QuoteResult {
    /// 预期输出金额（最小单位）
    pub amount_out: u64,

    /// 手续费金额（输入代币单位）
    pub fee_amount: u64,

    /// 价格影响（基点，可选）
    pub price_impact_bps: Option<u64>,

    /// 计算耗时（毫秒）
    pub calculation_time_ms: u64,

    /// 使用的 DEX 类型
    pub dex_type: DexType,
}
```

### SimulationResult

```rust
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// 模拟的输出金额
    pub amount_out: u64,

    /// 手续费金额
    pub fee_amount: u64,

    /// 计算单元消耗
    pub compute_units: u64,

    /// 交易费用
    pub transaction_fee: u64,

    /// 模拟是否成功
    pub success: bool,

    /// 错误信息（如果失败）
    pub error: Option<String>,

    /// 交易日志（用于调试）
    pub logs: Option<Vec<String>>,

    /// 使用的 DEX 类型
    pub dex_type: DexType,
}
```

---

## 🔧 核心实现

### buy_quote 实现

```rust
impl TradingClient {
    pub async fn buy_quote(
        &self,
        params: TradeBuyParams,
    ) -> Result<QuoteResult, TradingError> {
        let start = std::time::Instant::now();

        // 1. 参数验证
        if params.input_token_amount == 0 {
            return Err(TradingError::InvalidParameters("amount must be > 0".into()));
        }

        if !Self::supports_quote(&params.dex_type) {
            return Err(TradingError::UnsupportedDex(params.dex_type));
        }

        // 2. 获取 input_mint
        let input_mint = Self::get_input_mint(&params.input_token_type);

        // 3. 根据 DEX 类型调用对应的 quote_exact_in
        let (amount_out, fee_amount) = match &params.extension_params {
            DexParamEnum::RaydiumClmm(clmm_params) => {
                // 推断方向：input_mint 是否是 token0
                let zero_for_one = input_mint == clmm_params.token0_mint;

                let quote = instruction::utils::raydium_clmm::quote_exact_in(
                    &self.rpc,
                    &clmm_params.pool_state,
                    params.input_token_amount,
                    zero_for_one,
                ).await?;
                (quote.amount_out, quote.fee_amount)
            },

            DexParamEnum::RaydiumCpmm(cpmm_params) => {
                let is_token0_in = input_mint == cpmm_params.base_mint;

                let quote = instruction::utils::raydium_cpmm::quote_exact_in(
                    &self.rpc,
                    &cpmm_params.pool_state,
                    params.input_token_amount,
                    is_token0_in,
                ).await?;
                (quote.amount_out, quote.fee_amount)
            },

            DexParamEnum::RaydiumAmmV4(amm_params) => {
                let is_coin_in = input_mint == amm_params.coin_mint;

                let quote = instruction::utils::raydium_amm_v4::quote_exact_in(
                    &self.rpc,
                    &amm_params.amm,
                    params.input_token_amount,
                    is_coin_in,
                ).await?;
                (quote.amount_out, quote.fee_amount)
            },

            DexParamEnum::PumpSwap(pump_params) => {
                let is_base_in = input_mint == pump_params.base_mint;

                let quote = instruction::utils::pumpswap::quote_exact_in(
                    &self.rpc,
                    &pump_params.pool,
                    params.input_token_amount,
                    is_base_in,
                ).await?;
                (quote.amount_out, quote.fee_amount)
            },

            _ => return Err(TradingError::UnsupportedDex(params.dex_type)),
        };

        Ok(QuoteResult {
            amount_out,
            fee_amount,
            price_impact_bps: None,
            calculation_time_ms: start.elapsed().as_millis() as u64,
            dex_type: params.dex_type,
        })
    }

    // 辅助函数
    fn get_input_mint(input_token_type: &TradeTokenType) -> Pubkey {
        match input_token_type {
            TradeTokenType::SOL => SOL_TOKEN_ACCOUNT,
            TradeTokenType::WSOL => WSOL_TOKEN_ACCOUNT,
            TradeTokenType::USDC => USDC_TOKEN_ACCOUNT,
            TradeTokenType::USD1 => USD1_TOKEN_ACCOUNT,
        }
    }

    fn supports_quote(dex_type: &DexType) -> bool {
        matches!(dex_type,
            DexType::RaydiumClmm |
            DexType::RaydiumCpmm |
            DexType::RaydiumAmmV4 |
            DexType::PumpSwap
        )
    }
}
```

### buy_simulate 实现

```rust
impl TradingClient {
    pub async fn buy_simulate(
        &self,
        params: TradeBuyParams,
    ) -> Result<SimulationResult, TradingError> {
        // 1. 参数验证（复用 buy 中的逻辑）
        if params.input_token_amount == 0 {
            return Err(TradingError::InvalidParameters("amount must be > 0".into()));
        }

        if params.input_token_type == TradeTokenType::USD1 && params.dex_type != DexType::Bonk {
            return Err(TradingError::InvalidParameters("USD1 only supported on Bonk".into()));
        }

        // 2. 构建 SwapParams（完全复用 buy 中的逻辑）
        let input_mint = Self::get_input_mint(&params.input_token_type);

        let executor = TradeFactory::create_executor(params.dex_type.clone());
        let protocol_params = params.extension_params;

        let swap_params = SwapParams {
            rpc: Some(self.rpc.clone()),
            payer: self.payer.clone(),
            trade_type: TradeType::Buy,
            input_mint,
            output_mint: params.mint,
            input_token_program: None,
            output_token_program: None,
            input_amount: Some(params.input_token_amount),
            slippage_basis_points: params.slippage_basis_points,
            address_lookup_table_account: params.address_lookup_table_account,
            recent_blockhash: params.recent_blockhash,
            wait_transaction_confirmed: false, // 模拟不需要等待确认
            protocol_params: protocol_params.clone(),
            open_seed_optimize: self.use_seed_optimize,
            swqos_clients: self.swqos_clients.clone(),
            middleware_manager: self.middleware_manager.clone(),
            durable_nonce: params.durable_nonce,
            with_tip: true,
            create_input_mint_ata: params.create_input_token_ata,
            close_input_mint_ata: params.close_input_token_ata,
            create_output_mint_ata: params.create_mint_ata,
            close_output_mint_ata: false,
            fixed_output_amount: params.fixed_output_token_amount,
            gas_fee_strategy: params.gas_fee_strategy,
            simulate: true, // 关键：设置模拟模式
            on_transaction_signed: None,
            callback_execution_mode: None,
            enable_jito_sandwich_protection: None,
        };

        // 3. 构建指令（复用现有逻辑）
        let instructions = executor.build_buy_instructions(&swap_params).await?;

        // 4. 获取用户 ATA
        let user_input_ata = get_associated_token_address(
            &self.payer.pubkey(),
            &input_mint,
        );
        let user_output_ata = get_associated_token_address(
            &self.payer.pubkey(),
            &params.mint,
        );

        // 5. 调用链上模拟
        let sim_result = simulation_based_calc::simulate_swap_transaction(
            &self.rpc,
            &self.payer,
            instructions,
            user_input_ata,
            user_output_ata,
            input_mint,
            params.mint,
        ).await?;

        // 6. 转换返回值
        Ok(SimulationResult {
            amount_out: sim_result.actual_output_amount,
            fee_amount: 0, // TODO: 从 sim_result 计算
            compute_units: sim_result.units_consumed.unwrap_or(0),
            transaction_fee: sim_result.transaction_fee,
            success: sim_result.success,
            error: sim_result.error,
            logs: sim_result.logs,
            dex_type: params.dex_type,
        })
    }
}
```

---

## ❌ 错误处理

```rust
#[derive(Debug, thiserror::Error)]
pub enum TradingError {
    #[error("Unsupported DEX for quote: {0:?}")]
    UnsupportedDex(DexType),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Quote calculation failed: {0}")]
    QuoteFailed(String),

    #[error("Simulation failed: {0}")]
    SimulationFailed(String),

    #[error("RPC error: {0}")]
    RpcError(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),
}

pub type Result<T> = std::result::Result<T, TradingError>;
```

---

## 🧪 测试策略

### 单元测试

```rust
// tests/unified_interface_test.rs

#[tokio::test]
async fn test_buy_quote_clmm() {
    let client = setup_test_client().await;
    let params = create_clmm_test_params();

    let quote = client.buy_quote(params).await.unwrap();

    assert!(quote.amount_out > 0);
    assert!(quote.calculation_time_ms < 100);
    assert_eq!(quote.dex_type, DexType::RaydiumClmm);
}

#[tokio::test]
async fn test_buy_quote_unsupported_dex() {
    let client = setup_test_client().await;
    let mut params = create_test_params();
    params.dex_type = DexType::PumpFun; // 不支持

    let result = client.buy_quote(params).await;

    assert!(matches!(result, Err(TradingError::UnsupportedDex(_))));
}

#[tokio::test]
async fn test_buy_simulate_vs_quote_accuracy() {
    let client = setup_test_client().await;
    let params = create_test_params(DexType::RaydiumClmm);

    let quote = client.buy_quote(params.clone()).await.unwrap();
    let sim = client.buy_simulate(params).await.unwrap();

    // 误差应该 < 0.1%
    let error_rate = (quote.amount_out as f64 - sim.amount_out as f64)
        .abs() / sim.amount_out as f64;
    assert!(error_rate < 0.001);

    assert!(sim.success);
    assert!(sim.compute_units > 0);
}

#[tokio::test]
async fn test_progressive_workflow() {
    let client = setup_test_client().await;
    let params = create_test_params(DexType::RaydiumClmm);

    // quote → simulate
    let quick = client.buy_quote(params.clone()).await.unwrap();
    let verified = client.buy_simulate(params.clone()).await.unwrap();

    assert!(verified.success);
}
```

---

## 📤 导出策略

```rust
// src/lib.rs

// ✅ 导出核心类型
pub use crate::trading::TradingClient;
pub use crate::trading::TradeBuyParams;
pub use crate::trading::DexType;
pub use crate::trading::DexParamEnum;
pub use crate::trading::TradeTokenType;

// ✅ 导出结果类型
pub use crate::trading::results::{QuoteResult, SimulationResult};

// ✅ 导出错误类型
pub use crate::trading::TradingError;

// ✅ 导出必要的配置类型
pub use crate::common::{SolanaRpcClient, TradeConfig, GasFeeStrategy};

// ❌ 不再导出 DEX 特定函数
// 不导出 instruction::utils::* 的细节
```

---

## 📦 实施计划

### Phase 1: 基础设施（2 小时）
- [ ] 创建 `src/trading/results.rs`
- [ ] 定义 `QuoteResult` 和 `SimulationResult`
- [ ] 定义 `TradingError`（或集成到现有错误类型）
- [ ] 更新 `src/trading/mod.rs` 导出

### Phase 2: 实现 buy_quote（2 小时）
- [ ] 实现 `TradingClient::buy_quote()`
- [ ] 实现 `get_input_mint()` 辅助函数
- [ ] 实现 `supports_quote()` 辅助函数
- [ ] 为 4 个 DEX 实现 quote 逻辑和方向推断

### Phase 3: 实现 buy_simulate（2-3 小时）
- [ ] 实现 `TradingClient::buy_simulate()`
- [ ] 复用现有的 SwapParams 构建逻辑
- [ ] 复用 `simulate_swap_transaction`
- [ ] 实现结果转换

### Phase 4: 更新导出（30 分钟）
- [ ] 更新 `src/lib.rs` 导出
- [ ] 确保只导出必要的类型

### Phase 5: 测试（2-3 小时）
- [ ] 创建 `tests/unified_interface_test.rs`
- [ ] 添加单元测试（覆盖 4 个 DEX）
- [ ] 添加准确性测试
- [ ] 运行现有测试确保无回归

### Phase 6: 文档（1 小时）
- [ ] 编写迁移指南 `docs/migration-v4.md`
- [ ] 更新 `CLAUDE.md`
- [ ] 更新 `CHANGELOG.md`

### Phase 7: 发布（30 分钟）
- [ ] 更新版本号为 4.0.0
- [ ] 提交 git commit
- [ ] 创建 git tag

**总预估时间**: 1-2 天

---

## ✅ 验收标准

- [ ] `buy_quote` 对 4 个 DEX 都能正常工作
- [ ] `buy_simulate` 对 4 个 DEX 都能正常工作
- [ ] 误差测试通过（quote vs simulate < 0.1%）
- [ ] 所有测试通过
- [ ] 文档完整
- [ ] Breaking change 明确标注

---

**设计完成时间**: 2025-01-30
**目标版本**: v4.0.0
**Breaking Change**: 是
**向后兼容**: 否
**实施时间**: 1-2 天
