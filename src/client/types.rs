//! TradingClient 相关的类型定义

use solana_sdk::hash::Hash;
use solana_sdk::message::AddressLookupTableAccount;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::sync::Arc;

use crate::common::GasFeeStrategy;
use crate::{
    CallbackExecutionMode, CallbackRef, DexParamEnum, DexType, DurableNonceInfo, MiddlewareManager,
    TradeTokenType,
};

/// 买入订单的执行参数
///
/// 包含购买代币所需的所有配置，包括协议特定设置、账户管理选项和交易偏好。
#[derive(Clone)]
pub struct TradeBuyParams {
    // 交易配置
    /// 使用的 DEX 协议
    pub dex_type: DexType,
    /// 要购买的代币类型
    pub input_token_type: TradeTokenType,
    /// 要购买的代币公钥
    pub mint: Pubkey,
    /// 购买代币数量（最小代币单位）
    pub input_token_amount: u64,
    /// 可选滑点容忍度（基点，如 100 = 1%）
    pub slippage_basis_points: Option<u64>,
    /// 交易有效性的最新区块哈希
    pub recent_blockhash: Option<Hash>,
    /// 协议特定参数（PumpFun、Raydium 等）
    pub extension_params: DexParamEnum,
    // 扩展配置
    /// 交易大小优化的可选地址查找表
    pub address_lookup_table_account: Option<AddressLookupTableAccount>,
    /// 是否在返回前等待交易确认
    pub wait_transaction_confirmed: bool,
    /// 是否创建输入代币关联代币账户
    pub create_input_token_ata: bool,
    /// 是否在交易后关闭输入代币关联代币账户
    pub close_input_token_ata: bool,
    /// 是否创建代币 mint 关联代币账户
    pub create_mint_ata: bool,
    /// Durable nonce 信息
    pub durable_nonce: Option<DurableNonceInfo>,
    /// 可选的固定输出代币数量（如果设置此值，将直接分配给输出金额而不是计算）
    pub fixed_output_token_amount: Option<u64>,
    /// Gas 费策略
    pub gas_fee_strategy: GasFeeStrategy,
    /// 是否模拟交易而不是执行它
    pub simulate: bool,
    /// 交易签名后回调（可选）
    /// 用于在交易发送前获取签名后的交易实体，用于入库等操作
    pub on_transaction_signed: Option<CallbackRef>,
    /// 回调执行模式（可选，覆盖全局配置）
    ///
    /// - `Some(Async)`：异步执行，不阻塞交易发送
    /// - `Some(Sync)`：同步执行，等待回调完成后再发送交易
    /// - `None`：使用全局配置（TradeConfig.callback_execution_mode）
    pub callback_execution_mode: Option<CallbackExecutionMode>,
    /// 是否启用 Jito 三明治攻击防护（可选，覆盖全局配置）
    ///
    /// - `Some(true)`：启用防护
    /// - `Some(false)`：禁用防护
    /// - `None`：使用全局配置（TradeConfig.enable_jito_sandwich_protection）
    ///
    /// # 详细说明
    ///
    /// 参见 `TradeConfig.enable_jito_sandwich_protection` 字段的详细文档。
    pub enable_jito_sandwich_protection: Option<bool>,
}

/// 卖出订单的执行参数
///
/// 包含卖出代币所需的所有配置，包括协议特定设置、小费偏好、账户管理选项和交易偏好。
#[derive(Clone)]
pub struct TradeSellParams {
    // 交易配置
    /// 使用的 DEX 协议
    pub dex_type: DexType,
    /// 要卖出的代币类型
    pub output_token_type: TradeTokenType,
    /// 要卖出的代币公钥
    pub mint: Pubkey,
    /// 卖出代币数量（最小代币单位）
    pub input_token_amount: u64,
    /// 可选滑点容忍度（基点，如 100 = 1%）
    pub slippage_basis_points: Option<u64>,
    /// 交易有效性的最新区块哈希
    pub recent_blockhash: Option<Hash>,
    /// 是否为交易优先级包含小费
    pub with_tip: bool,
    /// 协议特定参数（PumpFun、Raydium 等）
    pub extension_params: DexParamEnum,
    // 扩展配置
    /// 交易大小优化的可选地址查找表
    pub address_lookup_table_account: Option<AddressLookupTableAccount>,
    /// 是否在返回前等待交易确认
    pub wait_transaction_confirmed: bool,
    /// 是否创建输出代币关联代币账户
    pub create_output_token_ata: bool,
    /// 是否在交易后关闭输出代币关联代币账户
    pub close_output_token_ata: bool,
    /// 是否在交易后关闭 mint 代币关联代币账户
    pub close_mint_token_ata: bool,
    /// Durable nonce 信息
    pub durable_nonce: Option<DurableNonceInfo>,
    /// 可选的固定输出代币数量（如果设置此值，将直接分配给输出金额而不是计算）
    pub fixed_output_token_amount: Option<u64>,
    /// Gas 费策略
    pub gas_fee_strategy: GasFeeStrategy,
    /// 是否模拟交易而不是执行它
    pub simulate: bool,
    /// 交易签名后回调（可选）
    /// 用于在交易发送前获取签名后的交易实体，用于入库等操作
    pub on_transaction_signed: Option<CallbackRef>,
    /// 回调执行模式（可选，覆盖全局配置）
    ///
    /// - `Some(Async)`：异步执行，不阻塞交易发送
    /// - `Some(Sync)`：同步执行，等待回调完成后再发送交易
    /// - `None`：使用全局配置（TradeConfig.callback_execution_mode）
    pub callback_execution_mode: Option<CallbackExecutionMode>,
    /// 是否启用 Jito 三明治攻击防护（可选，覆盖全局配置）
    ///
    /// - `Some(true)`：启用防护
    /// - `Some(false)`：禁用防护
    /// - `None`：使用全局配置（TradeConfig.enable_jito_sandwich_protection）
    ///
    /// # 详细说明
    ///
    /// 参见 `TradeConfig.enable_jito_sandwich_protection` 字段的详细文档。
    pub enable_jito_sandwich_protection: Option<bool>,
}

/// 主交易客户端，用于 Solana DeFi 协议
///
/// `TradingClient` 为跨多个 Solana DEX（包括 PumpFun、PumpSwap、Bonk、Raydium AMM V4 和 Raydium CPMM）
/// 的交易提供统一接口。它管理 RPC 连接、交易签名和 SWQOS（Solana Web Quality of Service）设置。
pub struct TradingClient {
    /// 用于签名所有交易的密钥对
    /// 共享基础设施（RPC 客户端、SWQOS 客户端）
    /// 可在具有不同钱包的多个 TradingClient 实例之间共享
    pub infrastructure: Option<Arc<crate::TradingInfrastructure>>,
    pub payer: Arc<Keypair>,
    /// 区块链交互的 RPC 客户端
    pub rpc: Arc<crate::SolanaRpcClient>,
    /// 交易优先级和路由的 SWQOS（Stake-Weighted Quality of Service）客户端
    pub swqos_clients: Vec<Arc<crate::SwqosClient>>,
    /// 可选的中间件管理器，用于自定义交易处理
    pub middleware_manager: Option<Arc<MiddlewareManager>>,
    /// 是否对所有 ATA 操作使用 seed 优化（默认：false）
    /// 应用于买入和卖出操作中的所有代币账户创建
    pub use_seed_optimize: bool,
    /// 回调执行模式（全局默认配置）
    pub callback_execution_mode: CallbackExecutionMode,
    /// 是否启用 Jito 三明治攻击防护（全局默认配置）
    ///
    /// # 优先级
    ///
    /// 1. **交易级别**: TradeBuyParams/TradeSellParams.enable_jito_sandwich_protection
    /// 2. **全局级别**: TradingClient.enable_jito_sandwich_protection (这里)
    /// 3. **默认值**: false
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use sol_trade_sdk::TradingClient;
    /// # use sol_trade_sdk::TradeConfig;
    /// // 全局禁用（默认）
    /// # let config = TradeConfig::default();
    ///
    /// // 全局启用
    /// # let config = TradeConfig::default().with_jito_sandwich_protection(true);
    ///
    /// // 单次交易覆盖全局配置
    /// # let mut buy_params = TradeBuyParams::default();
    /// buy_params.enable_jito_sandwich_protection = Some(true); // 强制启用
    /// ```
    pub enable_jito_sandwich_protection: bool,
}

impl Clone for TradingClient {
    fn clone(&self) -> Self {
        Self {
            payer: self.payer.clone(),
            rpc: self.rpc.clone(),
            swqos_clients: self.swqos_clients.clone(),
            middleware_manager: self.middleware_manager.clone(),
            use_seed_optimize: self.use_seed_optimize,
            callback_execution_mode: self.callback_execution_mode,
            enable_jito_sandwich_protection: self.enable_jito_sandwich_protection,
            infrastructure: self.infrastructure.clone(),
        }
    }
}
