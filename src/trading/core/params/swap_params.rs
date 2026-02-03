use crate::common::nonce_cache::DurableNonceInfo;
use crate::common::{GasFeeStrategy, SolanaRpcClient};
use crate::swqos::{SwqosClient, TradeType};
use crate::trading::MiddlewareManager;
use crate::trading::core::params::DexParamEnum;
use solana_hash::Hash;
use solana_sdk::message::AddressLookupTableAccount;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use std::sync::Arc;

/// Swap parameters
#[derive(Clone)]
pub struct SwapParams {
    pub rpc: Option<Arc<SolanaRpcClient>>,
    pub payer: Arc<Keypair>,
    pub trade_type: TradeType,
    pub input_mint: Pubkey,
    pub input_token_program: Option<Pubkey>,
    pub output_mint: Pubkey,
    pub output_token_program: Option<Pubkey>,
    pub input_amount: Option<u64>,
    pub slippage_basis_points: Option<u64>,
    pub address_lookup_table_account: Option<AddressLookupTableAccount>,
    pub recent_blockhash: Option<Hash>,
    pub wait_transaction_confirmed: bool,
    pub protocol_params: DexParamEnum,
    pub open_seed_optimize: bool,
    pub swqos_clients: Vec<Arc<SwqosClient>>,
    pub middleware_manager: Option<Arc<MiddlewareManager>>,
    pub durable_nonce: Option<DurableNonceInfo>,
    pub with_tip: bool,
    pub create_input_mint_ata: bool,
    pub close_input_mint_ata: bool,
    pub create_output_mint_ata: bool,
    pub close_output_mint_ata: bool,
    pub fixed_output_amount: Option<u64>,
    pub gas_fee_strategy: GasFeeStrategy,
    pub simulate: bool,
    /// 交易签名后回调（可选）
    /// 用于在交易发送前获取签名后的交易实体，用于入库等操作
    pub on_transaction_signed: Option<crate::trading::CallbackRef>,
    /// 回调执行模式（可选，覆盖全局配置）
    ///
    /// - `Some(Async)`：异步执行，不阻塞交易发送
    /// - `Some(Sync)`：同步执行，等待回调完成后再发送交易
    /// - `None`：使用全局配置（TradeConfig.callback_execution_mode）
    pub callback_execution_mode: Option<crate::common::CallbackExecutionMode>,
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

impl std::fmt::Debug for SwapParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SwapParams: ...")
    }
}
