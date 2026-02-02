// 允许在库初始化代码中使用 expect，因为：
// 1. 关键资源初始化失败是严重错误，应该 panic
// 2. 用户提供的配置已经在其他地方验证
#![allow(clippy::expect_used)]

pub mod common;
pub mod constants;
pub mod instruction;
pub mod parser;
pub mod perf;
pub mod swqos;
pub mod trading;
pub mod utils;

// 导出交易执行模式
use crate::common::CallbackExecutionMode;
use crate::common::GasFeeStrategy;
use crate::common::InfrastructureConfig;
use crate::common::TradeConfig;
use crate::common::nonce_cache::DurableNonceInfo;
use crate::constants::SOL_TOKEN_ACCOUNT;
use crate::constants::USD1_TOKEN_ACCOUNT;
use crate::constants::USDC_TOKEN_ACCOUNT;
use crate::constants::WSOL_TOKEN_ACCOUNT;
#[cfg(feature = "perf-trace")]
use crate::constants::trade_consts::DEFAULT_SLIPPAGE;
use crate::swqos::SwqosClient;
use crate::swqos::SwqosConfig;
use crate::swqos::TradeType;
use crate::swqos::common::TradeError;
pub use crate::trading::CallbackContext;
pub use crate::trading::CallbackRef;
use crate::trading::MiddlewareManager;
pub use crate::trading::NoopCallback;
use crate::trading::SwapParams;
use crate::trading::TradeFactory;
pub use crate::trading::TransactionLifecycleCallback;
use crate::trading::core::params::BonkParams;
use crate::trading::core::params::DexParamEnum;
use crate::trading::core::params::MeteoraDammV2Params;
use crate::trading::core::params::PumpFunParams;
use crate::trading::core::params::PumpSwapParams;
use crate::trading::core::params::{RaydiumAmmV4Params, RaydiumClmmParams, RaydiumCpmmParams};
pub use crate::trading::error::{Result as UnifiedResult, TradingError as UnifiedTradingError};
pub use crate::trading::factory::DexType;
pub use crate::trading::results::{QuoteResult, SimulationResult};
use common::SolanaRpcClient;
use parking_lot::Mutex;
use rustls::crypto::{CryptoProvider, ring::default_provider};
use solana_sdk::hash::Hash;
use solana_sdk::message::AddressLookupTableAccount;
use solana_sdk::signer::Signer;
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signature::Signature};
use std::sync::Arc;
pub use trading::ExecutionMode;

/// Type of the token to buy
#[derive(Clone, PartialEq)]
pub enum TradeTokenType {
    SOL,
    WSOL,
    USD1,
    USDC,
}

/// Shared infrastructure components that can be reused across multiple wallets
///
/// This struct holds the expensive-to-initialize components (RPC client, SWQOS clients)
/// that are wallet-independent and can be shared when only the trading wallet changes.
pub struct TradingInfrastructure {
    /// Shared RPC client for blockchain interactions
    pub rpc: Arc<SolanaRpcClient>,
    /// Shared SWQOS clients for transaction priority and routing
    pub swqos_clients: Vec<Arc<SwqosClient>>,
    /// Configuration used to create this infrastructure
    pub config: InfrastructureConfig,
}

impl TradingInfrastructure {
    /// Create new shared infrastructure from configuration
    ///
    /// This performs the expensive initialization:
    /// - Creates RPC client with connection pool
    /// - Creates SWQOS clients (each with their own HTTP client)
    /// - Initializes rent cache and starts background updater
    pub async fn new(config: InfrastructureConfig) -> Self {
        // Install crypto provider (idempotent)
        if CryptoProvider::get_default().is_none()
            && let Err(e) = default_provider().install_default()
        {
            eprintln!("⚠️  Failed to install crypto provider: {e:?}");
            eprintln!("    Crypto operations may fail. Continuing anyway...");
        }

        // Create RPC client
        let rpc = Arc::new(SolanaRpcClient::new_with_commitment(
            config.rpc_url.clone(),
            config.commitment,
        ));

        // Initialize rent cache and start background updater
        common::seed::update_rents(&rpc)
            .await
            .expect("Failed to initialize rent cache - this is required for trading operations");
        common::seed::start_rent_updater(rpc.clone());

        // Create SWQOS clients with blacklist checking
        let mut swqos_clients: Vec<Arc<SwqosClient>> = vec![];
        for swqos in &config.swqos_configs {
            // Check blacklist, skip disabled providers
            if swqos.is_blacklisted() {
                eprintln!(
                    "\u{26a0}\u{fe0f} SWQOS {:?} is blacklisted, skipping",
                    swqos.swqos_type()
                );
                continue;
            }
            match SwqosConfig::get_swqos_client(
                config.rpc_url.clone(),
                config.commitment,
                swqos.clone(),
            )
            .await
            {
                Ok(swqos_client) => swqos_clients.push(swqos_client),
                Err(err) => eprintln!(
                    "failed to create {:?} swqos client: {err}. Excluding from swqos list",
                    swqos.swqos_type()
                ),
            }
        }

        Self { rpc, swqos_clients, config }
    }
}

/// Main trading client for Solana DeFi protocols
///
/// `SolTradingSDK` provides a unified interface for trading across multiple Solana DEXs
/// including PumpFun, PumpSwap, Bonk, Raydium AMM V4, and Raydium CPMM.
/// It manages RPC connections, transaction signing, and SWQOS (Solana Web Quality of Service) settings.
pub struct TradingClient {
    /// The keypair used for signing all transactions
    /// Shared infrastructure (RPC client, SWQOS clients)
    /// Can be shared across multiple TradingClient instances with different wallets
    pub infrastructure: Option<Arc<TradingInfrastructure>>,
    pub payer: Arc<Keypair>,
    /// RPC client for blockchain interactions
    pub rpc: Arc<SolanaRpcClient>,
    /// SWQOS (Stake-Weighted Quality of Service) clients for transaction priority and routing
    pub swqos_clients: Vec<Arc<SwqosClient>>,
    /// Optional middleware manager for custom transaction processing
    pub middleware_manager: Option<Arc<MiddlewareManager>>,
    /// Whether to use seed optimization for all ATA operations (default: false)
    /// Applies to all token account creations across buy and sell operations
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

static INSTANCE: Mutex<Option<Arc<TradingClient>>> = Mutex::new(None);

/// 🔄 向后兼容：SolanaTrade 别名
pub type SolanaTrade = TradingClient;

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

/// Parameters for executing buy orders across different DEX protocols
///
/// Contains all necessary configuration for purchasing tokens, including
/// protocol-specific settings, account management options, and transaction preferences.
#[derive(Clone)]
pub struct TradeBuyParams {
    // Trading configuration
    /// The DEX protocol to use for the trade
    pub dex_type: DexType,
    /// Type of the token to buy
    pub input_token_type: TradeTokenType,
    /// Public key of the token to purchase
    pub mint: Pubkey,
    /// Amount of tokens to buy (in smallest token units)
    pub input_token_amount: u64,
    /// Optional slippage tolerance in basis points (e.g., 100 = 1%)
    pub slippage_basis_points: Option<u64>,
    /// Recent blockhash for transaction validity
    pub recent_blockhash: Option<Hash>,
    /// Protocol-specific parameters (PumpFun, Raydium, etc.)
    pub extension_params: DexParamEnum,
    // Extended configuration
    /// Optional address lookup table for transaction size optimization
    pub address_lookup_table_account: Option<AddressLookupTableAccount>,
    /// Whether to wait for transaction confirmation before returning
    pub wait_transaction_confirmed: bool,
    /// Whether to create input token associated token account
    pub create_input_token_ata: bool,
    /// Whether to close input token associated token account after trade
    pub close_input_token_ata: bool,
    /// Whether to create token mint associated token account
    pub create_mint_ata: bool,
    /// Durable nonce information
    pub durable_nonce: Option<DurableNonceInfo>,
    /// Optional fixed output token amount (If this value is set, it will be directly assigned to the output amount instead of being calculated)
    pub fixed_output_token_amount: Option<u64>,
    /// Gas fee strategy
    pub gas_fee_strategy: GasFeeStrategy,
    /// Whether to simulate the transaction instead of executing it
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

/// Parameters for executing sell orders across different DEX protocols
///
/// Contains all necessary configuration for selling tokens, including
/// protocol-specific settings, tip preferences, account management options, and transaction preferences.
#[derive(Clone)]
pub struct TradeSellParams {
    // Trading configuration
    /// The DEX protocol to use for the trade
    pub dex_type: DexType,
    /// Type of the token to sell
    pub output_token_type: TradeTokenType,
    /// Public key of the token to sell
    pub mint: Pubkey,
    /// Amount of tokens to sell (in smallest token units)
    pub input_token_amount: u64,
    /// Optional slippage tolerance in basis points (e.g., 100 = 1%)
    pub slippage_basis_points: Option<u64>,
    /// Recent blockhash for transaction validity
    pub recent_blockhash: Option<Hash>,
    /// Whether to include tip for transaction priority
    pub with_tip: bool,
    /// Protocol-specific parameters (PumpFun, Raydium, etc.)
    pub extension_params: DexParamEnum,
    // Extended configuration
    /// Optional address lookup table for transaction size optimization
    pub address_lookup_table_account: Option<AddressLookupTableAccount>,
    /// Whether to wait for transaction confirmation before returning
    pub wait_transaction_confirmed: bool,
    /// Whether to create output token associated token account
    pub create_output_token_ata: bool,
    /// Whether to close output token associated token account after trade
    pub close_output_token_ata: bool,
    /// Whether to close mint token associated token account after trade
    pub close_mint_token_ata: bool,
    /// Durable nonce information
    pub durable_nonce: Option<DurableNonceInfo>,
    /// Optional fixed output token amount (If this value is set, it will be directly assigned to the output amount instead of being calculated)
    pub fixed_output_token_amount: Option<u64>,
    /// Gas fee strategy
    pub gas_fee_strategy: GasFeeStrategy,
    /// Whether to simulate the transaction instead of executing it
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

impl TradingClient {
    /// Creates a new SolTradingSDK instance with the specified configuration
    ///
    /// 此函数负责初始化整个交易系统的所有必要组件，包括 RPC 连接、SWQOS 配置、
    /// 加密提供者、缓存预热等，确保实例创建后即可立即用于交易操作。
    ///
    /// # 参数
    /// * `payer` - 用于签名所有交易的密钥对（Keypair），此账户将用于支付交易费用和代币交易
    /// * `trade_config` - 交易配置对象，包含 RPC URL、SWQOS 配置、确认级别等设置
    ///
    /// # Returns
    /// Returns a configured `SolTradingSDK` instance ready for trading operations
    #[inline]
    pub async fn new(payer: Arc<Keypair>, trade_config: TradeConfig) -> Self {
        let pubkey = payer
            .try_pubkey()
            .expect("Failed to get pubkey from keypair - this should never happen");
        crate::common::fast_fn::fast_init(&pubkey);

        if CryptoProvider::get_default().is_none()
            && let Err(e) = default_provider().install_default()
        {
            eprintln!("⚠️  Failed to install crypto provider: {e:?}");
            eprintln!("    Crypto operations may fail. Continuing anyway...");
        }

        let rpc_url = trade_config.rpc_url.clone();
        let swqos_configs = trade_config.swqos_configs.clone();
        let commitment = trade_config.commitment;
        let mut swqos_clients: Vec<Arc<SwqosClient>> = vec![];

        for swqos in swqos_configs {
            match SwqosConfig::get_swqos_client(rpc_url.clone(), commitment, swqos.clone()).await {
                Ok(client) => swqos_clients.push(client),
                Err(e) => {
                    eprintln!("Failed to create SWQOS client {:?}: {}", swqos, e);
                },
            }
        }

        let rpc = Arc::new(SolanaRpcClient::new_with_commitment(rpc_url.clone(), commitment));
        common::seed::update_rents(&rpc)
            .await
            .expect("Failed to initialize rent cache - this is required for trading operations");
        common::seed::start_rent_updater(rpc.clone());

        // 🔧 初始化WSOL ATA：如果配置为启动时创建，则检查并创建
        if trade_config.create_wsol_ata_on_startup {
            // 根据seed配置计算WSOL ATA地址
            let wsol_ata =
                crate::common::fast_fn::get_associated_token_address_with_program_id_fast(
                    &payer.pubkey(),
                    &WSOL_TOKEN_ACCOUNT,
                    &crate::constants::TOKEN_PROGRAM,
                );

            // 查询账户是否存在
            match rpc.get_account(&wsol_ata).await {
                Ok(_) => {
                    // WSOL ATA已存在
                    println!("✅ WSOL ATA已存在: {}", wsol_ata);
                },
                Err(_) => {
                    // WSOL ATA不存在，创建它
                    println!("🔨 创建WSOL ATA: {}", wsol_ata);
                    // 使用seed优化创建WSOL ATA
                    let create_ata_ixs =
                        crate::trading::common::wsol_manager::create_wsol_ata(&payer.pubkey());

                    if !create_ata_ixs.is_empty() {
                        // 构建并发送交易
                        use solana_sdk::transaction::Transaction;
                        let recent_blockhash = rpc.get_latest_blockhash().await.expect(
                            "Failed to get recent blockhash - cannot create WSOL ATA without it",
                        );
                        let tx = Transaction::new_signed_with_payer(
                            &create_ata_ixs,
                            Some(&payer.pubkey()),
                            &[payer.as_ref()],
                            recent_blockhash,
                        );

                        match rpc.send_and_confirm_transaction(&tx).await {
                            Ok(signature) => {
                                println!("✅ WSOL ATA创建成功: {}", signature);
                            },
                            Err(e) => {
                                // 创建失败，检查是否是因为已存在
                                match rpc.get_account(&wsol_ata).await {
                                    Ok(_) => {
                                        println!(
                                            "✅ WSOL ATA已存在（交易失败但账户存在）: {}",
                                            wsol_ata
                                        );
                                    },
                                    Err(_) => {
                                        // 账户不存在且创建失败 - 这是严重错误，应该让启动失败
                                        panic!(
                                            "❌ WSOL ATA创建失败且账户不存在: {}. 错误: {}",
                                            wsol_ata, e
                                        );
                                    },
                                }
                            },
                        }
                    } else {
                        println!("ℹ️ WSOL ATA已存在（无需创建）");
                    }
                },
            }
        }

        let instance = Self {
            payer,
            rpc,
            swqos_clients,
            middleware_manager: None,
            use_seed_optimize: trade_config.use_seed_optimize,
            callback_execution_mode: trade_config.callback_execution_mode,
            enable_jito_sandwich_protection: trade_config.enable_jito_sandwich_protection,
            infrastructure: None,
        };

        let mut current = INSTANCE.lock();
        *current = Some(Arc::new(instance.clone()));

        instance
    }

    /// Adds a middleware manager to the SolanaTrade instance
    ///
    /// Middleware managers can be used to implement custom logic that runs before or after trading operations,
    /// such as logging, monitoring, or custom validation.
    ///
    /// # Arguments
    /// * `middleware_manager` - The middleware manager to attach
    ///
    /// # Returns
    /// Returns the modified SolanaTrade instance with middleware manager attached
    pub fn with_middleware_manager(mut self, middleware_manager: MiddlewareManager) -> Self {
        self.middleware_manager = Some(Arc::new(middleware_manager));
        self
    }

    /// Gets the RPC client instance for direct Solana blockchain interactions
    ///
    /// This provides access to the underlying Solana RPC client that can be used
    /// for custom blockchain operations outside of the trading framework.
    ///
    /// # Returns
    /// Returns a reference to the Arc-wrapped SolanaRpcClient instance
    pub fn get_rpc(&self) -> &Arc<SolanaRpcClient> {
        &self.rpc
    }

    /// Gets the current globally shared SolanaTrade instance
    ///
    /// This provides access to the singleton instance that was created with `new()`.
    /// Useful for accessing the trading instance from different parts of the application.
    ///
    /// # Returns
    /// Returns the Arc-wrapped SolanaTrade instance
    ///
    /// # Panics
    /// Panics if no instance has been initialized yet. Make sure to call `new()` first.
    pub fn get_instance() -> Arc<Self> {
        let instance = INSTANCE.lock();
        instance
            .as_ref()
            .expect("SolanaTrade instance not initialized. Please call new() first.")
            .clone()
    }

    /// 本地计算（快速估算）
    ///
    /// 提供快速的本地价格估算，不发送交易到链上。
    /// 支持的 DEX：Raydium CLMM, Raydium CPMM, Raydium AMM V4, PumpSwap
    ///
    /// # 参数
    ///
    /// * `params` - 交易参数
    ///
    /// # 返回
    ///
    /// 返回 `QuoteResult` 包含预期的输出金额、手续费等信息
    pub async fn buy_quote(&self, params: TradeBuyParams) -> UnifiedResult<QuoteResult> {
        let start = std::time::Instant::now();

        // 1. 参数验证
        if params.input_token_amount == 0 {
            return Err(UnifiedTradingError::InvalidParameters("amount must be > 0".into()));
        }

        if !Self::supports_quote(&params.dex_type) {
            return Err(UnifiedTradingError::UnsupportedDex(params.dex_type));
        }

        // 2. 获取 input_mint
        let input_mint = Self::get_input_mint(&params.input_token_type);

        // 3. 根据 DEX 类型调用对应的 quote_exact_in
        let (amount_out, fee_amount) = match &params.extension_params {
            DexParamEnum::RaydiumClmm(clmm_params) => {
                // 推断方向：input_mint 是否是 token0
                let zero_for_one = input_mint == clmm_params.token0_mint;

                let quote = crate::instruction::utils::raydium_clmm::quote_exact_in(
                    &self.rpc,
                    &clmm_params.pool_state,
                    params.input_token_amount,
                    zero_for_one,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            DexParamEnum::RaydiumCpmm(cpmm_params) => {
                let is_token0_in = input_mint == cpmm_params.base_mint;

                let quote = crate::instruction::utils::raydium_cpmm::quote_exact_in(
                    &self.rpc,
                    &cpmm_params.pool_state,
                    params.input_token_amount,
                    is_token0_in,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            DexParamEnum::RaydiumAmmV4(amm_params) => {
                let is_coin_in = input_mint == amm_params.coin_mint;

                let quote = crate::instruction::utils::raydium_amm_v4::quote_exact_in(
                    &self.rpc,
                    &amm_params.amm,
                    params.input_token_amount,
                    is_coin_in,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            DexParamEnum::PumpSwap(pump_params) => {
                let is_base_in = input_mint == pump_params.base_mint;

                let quote = crate::instruction::utils::pumpswap::quote_exact_in(
                    &self.rpc,
                    &pump_params.pool,
                    params.input_token_amount,
                    is_base_in,
                )
                .await
                .map_err(|e| UnifiedTradingError::QuoteFailed(e.to_string()))?;
                (quote.amount_out, quote.fee_amount)
            },

            _ => return Err(UnifiedTradingError::UnsupportedDex(params.dex_type)),
        };

        Ok(QuoteResult {
            amount_out,
            fee_amount,
            price_impact_bps: None,
            calculation_time_ms: start.elapsed().as_millis() as u64,
            dex_type: params.dex_type,
        })
    }

    /// 链上模拟（准确验证）
    ///
    /// 通过链上模拟提供准确的交易结果，不发送真实交易。
    /// 支持所有 DEX。
    ///
    /// # 参数
    ///
    /// * `params` - 交易参数
    ///
    /// # 返回
    ///
    /// 返回 `SimulationResult` 包含模拟的输出金额、CU 消耗、交易费用等
    pub async fn buy_simulate(&self, params: TradeBuyParams) -> UnifiedResult<SimulationResult> {
        // 1. 参数验证（支持 exact_in 和 exact_out）
        if let Some(fixed_output) = params.fixed_output_token_amount {
            // exact_out 模式验证
            if fixed_output == 0 {
                return Err(UnifiedTradingError::InvalidParameters(
                    "fixed_output_token_amount must be > 0".into(),
                ));
            }
        } else {
            // exact_in 模式验证（现有逻辑）
            if params.input_token_amount == 0 {
                return Err(UnifiedTradingError::InvalidParameters("amount must be > 0".into()));
            }
        }

        if params.input_token_type == TradeTokenType::USD1 && params.dex_type != DexType::Bonk {
            return Err(UnifiedTradingError::InvalidParameters(
                "USD1 only supported on Bonk".into(),
            ));
        }

        // 2. 获取 input_mint
        let input_mint = Self::get_input_mint(&params.input_token_type);

        // 3. 构建 SwapParams（完全复用 buy 中的逻辑）
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

        // 4. 构建指令（根据 DEX 类型使用对应的 InstructionBuilder）
        use crate::trading::core::traits::InstructionBuilder;
        let instructions = match params.dex_type {
            DexType::RaydiumClmm => {
                crate::instruction::raydium_clmm::RaydiumClmmInstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::RaydiumCpmm => {
                crate::instruction::raydium_cpmm::RaydiumCpmmInstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::RaydiumAmmV4 => {
                crate::instruction::raydium_amm_v4::RaydiumAmmV4InstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::PumpSwap => {
                crate::instruction::pumpswap::PumpSwapInstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::PumpFun => {
                crate::instruction::pumpfun::PumpFunInstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::Bonk => {
                crate::instruction::bonk::BonkInstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
            DexType::MeteoraDammV2 => {
                crate::instruction::meteora_damm_v2::MeteoraDammV2InstructionBuilder
                    .build_buy_instructions(&swap_params)
                    .await
            },
        }
        .map_err(|e| UnifiedTradingError::TransactionBuildError(e.to_string()))?;

        // 5. 获取用户 ATA
        let user_input_ata = spl_associated_token_account::get_associated_token_address(
            &self.payer.pubkey(),
            &input_mint,
        );
        let user_output_ata = spl_associated_token_account::get_associated_token_address(
            &self.payer.pubkey(),
            &params.mint,
        );

        // 6. 调用链上模拟
        let sim_result = crate::utils::simulation_based_calc::simulate_swap_transaction(
            &self.rpc,
            &self.payer,
            instructions,
            user_input_ata,
            user_output_ata,
            input_mint,
            params.mint,
        )
        .await
        .map_err(|e| UnifiedTradingError::SimulationFailed(e.to_string()))?;

        // 7. 转换返回值
        Ok(SimulationResult {
            amount_out: sim_result.actual_output_amount,
            amount_in: params.input_token_amount, // 添加此行：支持 exact_out 模式
            fee_amount: 0,                        // TODO: 从 sim_result 计算
            compute_units: sim_result.units_consumed.unwrap_or(0),
            transaction_fee: sim_result.transaction_fee,
            success: sim_result.success,
            error: sim_result.error,
            logs: sim_result.logs,
            dex_type: params.dex_type,
        })
    }

    /// 卖出模拟（exact_in 和 exact_out）
    ///
    /// 模拟卖出操作，返回链上模拟结果。
    ///
    /// # 参数
    ///
    /// * `params` - 卖出参数
    ///   - `input_token_amount`: 要卖出的代币数量（exact_in 模式）
    ///   - `fixed_output_token_amount`: 期望获得的输出代币数量（exact_out 模式，可选）
    ///
    /// # 返回
    ///
    /// 返回 `SimulationResult` 包含：
    /// - `amount_in`: 实际卖出的数量
    /// - `amount_out`: 获得的输出数量
    /// - `compute_units`: 计算单元消耗
    /// - `transaction_fee`: 交易费用
    pub async fn sell_simulate(&self, params: TradeSellParams) -> UnifiedResult<SimulationResult> {
        // 1. 参数验证
        if let Some(fixed_output) = params.fixed_output_token_amount {
            if fixed_output == 0 {
                return Err(UnifiedTradingError::InvalidParameters(
                    "fixed_output_token_amount must be > 0".into(),
                ));
            }
        } else {
            if params.input_token_amount == 0 {
                return Err(UnifiedTradingError::InvalidParameters(
                    "input_token_amount must be > 0".into(),
                ));
            }
        }

        // 2. 获取 output_mint
        let output_mint = Self::get_output_mint(&params.output_token_type);

        // 3. 构建 SwapParams
        let swap_params = SwapParams {
            rpc: Some(self.rpc.clone()),
            payer: self.payer.clone(),
            trade_type: TradeType::Sell,
            input_mint: params.mint,
            output_mint,
            input_token_program: None,
            output_token_program: None,
            input_amount: Some(params.input_token_amount),
            slippage_basis_points: params.slippage_basis_points,
            address_lookup_table_account: params.address_lookup_table_account,
            recent_blockhash: params.recent_blockhash,
            wait_transaction_confirmed: false,
            protocol_params: params.extension_params.clone(),
            open_seed_optimize: self.use_seed_optimize,
            swqos_clients: self.swqos_clients.clone(),
            middleware_manager: self.middleware_manager.clone(),
            durable_nonce: params.durable_nonce,
            with_tip: params.with_tip,
            create_input_mint_ata: false,
            close_input_mint_ata: false,
            create_output_mint_ata: params.create_output_token_ata,
            close_output_mint_ata: params.close_output_token_ata,
            fixed_output_amount: params.fixed_output_token_amount,
            gas_fee_strategy: params.gas_fee_strategy,
            simulate: true,
            on_transaction_signed: params.on_transaction_signed,
            callback_execution_mode: params.callback_execution_mode,
            enable_jito_sandwich_protection: None,
        };

        // 4. 构建指令
        use crate::trading::core::traits::InstructionBuilder;
        let instructions = match params.dex_type {
            DexType::RaydiumClmm => {
                crate::instruction::raydium_clmm::RaydiumClmmInstructionBuilder
                    .build_sell_instructions(&swap_params)
                    .await
            },
            DexType::RaydiumCpmm => {
                crate::instruction::raydium_cpmm::RaydiumCpmmInstructionBuilder
                    .build_sell_instructions(&swap_params)
                    .await
            },
            DexType::RaydiumAmmV4 => {
                crate::instruction::raydium_amm_v4::RaydiumAmmV4InstructionBuilder
                    .build_sell_instructions(&swap_params)
                    .await
            },
            DexType::PumpSwap => {
                crate::instruction::pumpswap::PumpSwapInstructionBuilder
                    .build_sell_instructions(&swap_params)
                    .await
            },
            _ => {
                return Err(UnifiedTradingError::UnsupportedDex(params.dex_type));
            },
        }
        .map_err(|e| UnifiedTradingError::TransactionBuildError(e.to_string()))?;

        // 5. 获取用户 ATA
        let user_input_ata = spl_associated_token_account::get_associated_token_address(
            &self.payer.pubkey(),
            &params.mint,
        );
        let user_output_ata = spl_associated_token_account::get_associated_token_address(
            &self.payer.pubkey(),
            &output_mint,
        );

        // 6. 调用链上模拟
        let sim_result = crate::utils::simulation_based_calc::simulate_swap_transaction(
            &self.rpc,
            &self.payer,
            instructions,
            user_input_ata,
            user_output_ata,
            params.mint,
            output_mint,
        )
        .await
        .map_err(|e| UnifiedTradingError::SimulationFailed(e.to_string()))?;

        // 7. 转换返回值
        Ok(SimulationResult {
            amount_out: sim_result.actual_output_amount,
            amount_in: params.input_token_amount,
            fee_amount: 0,
            compute_units: sim_result.units_consumed.unwrap_or(0),
            transaction_fee: sim_result.transaction_fee,
            success: sim_result.success,
            error: sim_result.error,
            logs: sim_result.logs,
            dex_type: params.dex_type,
        })
    }

    /// Execute a buy order for a specified token
    ///
    /// 🔧 修复：返回Vec<Signature>支持多SWQOS并发交易
    /// - bool: 是否至少有一个交易成功
    /// - Vec<Signature>: 所有提交的交易签名（按SWQOS顺序）
    /// - Option<TradeError>: 最后一个错误（如果全部失败）
    ///
    /// # Arguments
    ///
    /// * `params` - Buy trade parameters containing all necessary trading configuration
    ///
    /// # Returns
    ///
    /// Returns `Ok((bool, Vec<Signature>, Option<TradeError>))` with success flag and all transaction signatures,
    /// or an error if the transaction fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Invalid protocol parameters are provided for the specified DEX type
    /// - The transaction fails to execute
    /// - Network or RPC errors occur
    /// - Insufficient SOL balance for the purchase
    /// - Required accounts cannot be created or accessed
    #[inline]
    pub async fn buy(
        &self,
        params: TradeBuyParams,
    ) -> Result<(bool, Vec<Signature>, Option<TradeError>), anyhow::Error> {
        #[cfg(feature = "perf-trace")]
        if params.slippage_basis_points.is_none() {
            log::debug!(
                "slippage_basis_points is none, use default slippage basis points: {}",
                DEFAULT_SLIPPAGE
            );
        }
        if params.input_token_type == TradeTokenType::USD1 && params.dex_type != DexType::Bonk {
            return Err(anyhow::anyhow!(
                " Current version only support USD1 trading on Bonk protocols"
            ));
        }
        let input_token_mint = if params.input_token_type == TradeTokenType::SOL {
            SOL_TOKEN_ACCOUNT
        } else if params.input_token_type == TradeTokenType::WSOL {
            WSOL_TOKEN_ACCOUNT
        } else if params.input_token_type == TradeTokenType::USDC {
            USDC_TOKEN_ACCOUNT
        } else {
            USD1_TOKEN_ACCOUNT
        };
        let executor = TradeFactory::create_executor(params.dex_type.clone());
        let protocol_params = params.extension_params;
        let buy_params = SwapParams {
            rpc: Some(self.rpc.clone()),
            payer: self.payer.clone(),
            trade_type: TradeType::Buy,
            input_mint: input_token_mint,
            output_mint: params.mint,
            input_token_program: None,
            output_token_program: None,
            input_amount: Some(params.input_token_amount),
            slippage_basis_points: params.slippage_basis_points,
            address_lookup_table_account: params.address_lookup_table_account,
            recent_blockhash: params.recent_blockhash,
            wait_transaction_confirmed: params.wait_transaction_confirmed,
            protocol_params: protocol_params.clone(),
            open_seed_optimize: self.use_seed_optimize, // 使用全局seed优化配置
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
            simulate: params.simulate,
            on_transaction_signed: params.on_transaction_signed,
            callback_execution_mode: params
                .callback_execution_mode
                .or(Some(self.callback_execution_mode)),
            enable_jito_sandwich_protection: params
                .enable_jito_sandwich_protection
                .or(Some(self.enable_jito_sandwich_protection)),
        };

        // Validate protocol params
        let is_valid_params = match params.dex_type {
            DexType::PumpFun => protocol_params.as_any().downcast_ref::<PumpFunParams>().is_some(),
            DexType::PumpSwap => {
                protocol_params.as_any().downcast_ref::<PumpSwapParams>().is_some()
            },
            DexType::Bonk => protocol_params.as_any().downcast_ref::<BonkParams>().is_some(),
            DexType::RaydiumCpmm => {
                protocol_params.as_any().downcast_ref::<RaydiumCpmmParams>().is_some()
            },
            DexType::RaydiumAmmV4 => {
                protocol_params.as_any().downcast_ref::<RaydiumAmmV4Params>().is_some()
            },
            DexType::RaydiumClmm => {
                protocol_params.as_any().downcast_ref::<RaydiumClmmParams>().is_some()
            },
            DexType::MeteoraDammV2 => {
                protocol_params.as_any().downcast_ref::<MeteoraDammV2Params>().is_some()
            },
        };

        if !is_valid_params {
            return Err(anyhow::anyhow!("Invalid protocol params for Trade"));
        }

        let swap_result = executor.swap(buy_params).await;

        swap_result.map(|(success, sigs, err)| (success, sigs, err.map(TradeError::from)))
    }

    /// Execute a sell order for a specified token
    ///
    /// 🔧 修复：返回Vec<Signature>支持多SWQOS并发交易
    /// - bool: 是否至少有一个交易成功
    /// - Vec<Signature>: 所有提交的交易签名（按SWQOS顺序）
    /// - Option<TradeError>: 最后一个错误（如果全部失败）
    ///
    /// # Arguments
    ///
    /// * `params` - Sell trade parameters containing all necessary trading configuration
    ///
    /// # Returns
    ///
    /// Returns `Ok((bool, Vec<Signature>, Option<TradeError>))` with success flag and all transaction signatures,
    /// or an error if the transaction fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Invalid protocol parameters are provided for the specified DEX type
    /// - The transaction fails to execute
    /// - Network or RPC errors occur
    /// - Insufficient token balance for the sale
    /// - Token account doesn't exist or is not properly initialized
    /// - Required accounts cannot be created or accessed
    #[inline]
    pub async fn sell(
        &self,
        params: TradeSellParams,
    ) -> Result<(bool, Vec<Signature>, Option<TradeError>), anyhow::Error> {
        #[cfg(feature = "perf-trace")]
        if params.slippage_basis_points.is_none() {
            log::debug!(
                "slippage_basis_points is none, use default slippage basis points: {}",
                DEFAULT_SLIPPAGE
            );
        }
        if params.output_token_type == TradeTokenType::USD1 && params.dex_type != DexType::Bonk {
            return Err(anyhow::anyhow!(
                " Current version only support USD1 trading on Bonk protocols"
            ));
        }
        let executor = TradeFactory::create_executor(params.dex_type.clone());
        let protocol_params = params.extension_params;
        let output_token_mint = if params.output_token_type == TradeTokenType::SOL {
            SOL_TOKEN_ACCOUNT
        } else if params.output_token_type == TradeTokenType::WSOL {
            WSOL_TOKEN_ACCOUNT
        } else if params.output_token_type == TradeTokenType::USDC {
            USDC_TOKEN_ACCOUNT
        } else {
            USD1_TOKEN_ACCOUNT
        };
        let sell_params = SwapParams {
            rpc: Some(self.rpc.clone()),
            payer: self.payer.clone(),
            trade_type: TradeType::Sell,
            input_mint: params.mint,
            output_mint: output_token_mint,
            input_token_program: None,
            output_token_program: None,
            input_amount: Some(params.input_token_amount),
            slippage_basis_points: params.slippage_basis_points,
            address_lookup_table_account: params.address_lookup_table_account,
            recent_blockhash: params.recent_blockhash,
            wait_transaction_confirmed: params.wait_transaction_confirmed,
            protocol_params: protocol_params.clone(),
            with_tip: params.with_tip,
            open_seed_optimize: self.use_seed_optimize, // 使用全局seed优化配置
            swqos_clients: self.swqos_clients.clone(),
            middleware_manager: self.middleware_manager.clone(),
            durable_nonce: params.durable_nonce,
            create_input_mint_ata: false,
            close_input_mint_ata: params.close_mint_token_ata,
            create_output_mint_ata: params.create_output_token_ata,
            close_output_mint_ata: params.close_output_token_ata,
            fixed_output_amount: params.fixed_output_token_amount,
            gas_fee_strategy: params.gas_fee_strategy,
            simulate: params.simulate,
            on_transaction_signed: params.on_transaction_signed,
            callback_execution_mode: params
                .callback_execution_mode
                .or(Some(self.callback_execution_mode)),
            enable_jito_sandwich_protection: params
                .enable_jito_sandwich_protection
                .or(Some(self.enable_jito_sandwich_protection)),
        };

        // Validate protocol params
        let is_valid_params = match params.dex_type {
            DexType::PumpFun => protocol_params.as_any().downcast_ref::<PumpFunParams>().is_some(),
            DexType::PumpSwap => {
                protocol_params.as_any().downcast_ref::<PumpSwapParams>().is_some()
            },
            DexType::Bonk => protocol_params.as_any().downcast_ref::<BonkParams>().is_some(),
            DexType::RaydiumCpmm => {
                protocol_params.as_any().downcast_ref::<RaydiumCpmmParams>().is_some()
            },
            DexType::RaydiumAmmV4 => {
                protocol_params.as_any().downcast_ref::<RaydiumAmmV4Params>().is_some()
            },
            DexType::RaydiumClmm => {
                protocol_params.as_any().downcast_ref::<RaydiumClmmParams>().is_some()
            },
            DexType::MeteoraDammV2 => {
                protocol_params.as_any().downcast_ref::<MeteoraDammV2Params>().is_some()
            },
        };

        if !is_valid_params {
            return Err(anyhow::anyhow!("Invalid protocol params for Trade"));
        }

        // Execute sell based on tip preference
        let swap_result = executor.swap(sell_params).await;

        swap_result.map(|(success, sigs, err)| (success, sigs, err.map(TradeError::from)))
    }

    // 辅助函数：获取 input token 的 mint 地址
    fn get_input_mint(input_token_type: &TradeTokenType) -> Pubkey {
        match input_token_type {
            TradeTokenType::SOL => SOL_TOKEN_ACCOUNT,
            TradeTokenType::WSOL => WSOL_TOKEN_ACCOUNT,
            TradeTokenType::USDC => USDC_TOKEN_ACCOUNT,
            TradeTokenType::USD1 => USD1_TOKEN_ACCOUNT,
        }
    }

    /// 获取输出代币的 mint 地址
    fn get_output_mint(output_type: &TradeTokenType) -> Pubkey {
        match output_type {
            TradeTokenType::SOL => SOL_TOKEN_ACCOUNT,
            TradeTokenType::WSOL => WSOL_TOKEN_ACCOUNT,
            TradeTokenType::USDC => USDC_TOKEN_ACCOUNT,
            TradeTokenType::USD1 => USD1_TOKEN_ACCOUNT,
        }
    }

    // 辅助函数：检查 DEX 是否支持 quote
    fn supports_quote(dex_type: &DexType) -> bool {
        matches!(
            dex_type,
            DexType::RaydiumClmm | DexType::RaydiumCpmm | DexType::RaydiumAmmV4 | DexType::PumpSwap
        )
    }

    /// Execute a sell order for a percentage of the specified token amount
    ///
    /// This is a convenience function that calculates the exact amount to sell based on
    /// a percentage of the total token amount and then calls the `sell` function.
    ///
    /// # Arguments
    ///
    /// * `params` - Sell trade parameters (will be modified with calculated token amount)
    /// * `amount_token` - Total amount of tokens available (in smallest token units)
    /// * `percent` - Percentage of tokens to sell (1-100, where 100 = 100%)
    ///
    /// # Returns
    ///
    /// Returns `Ok((bool, Vec<Signature>, Option<TradeError>))` with success flag and all transaction signatures,
    /// or an error if the transaction fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - `percent` is 0 or greater than 100
    /// - Invalid protocol parameters are provided for the specified DEX type
    /// - The transaction fails to execute
    /// - Network or RPC errors occur
    /// - Insufficient token balance for the calculated sale amount
    /// - Token account doesn't exist or is not properly initialized
    /// - Required accounts cannot be created or accessed
    pub async fn sell_by_percent(
        &self,
        mut params: TradeSellParams,
        amount_token: u64,
        percent: u64,
    ) -> Result<(bool, Vec<Signature>, Option<TradeError>), anyhow::Error> {
        if percent == 0 || percent > 100 {
            return Err(anyhow::anyhow!("Percentage must be between 1 and 100"));
        }
        let amount = amount_token * percent / 100;
        params.input_token_amount = amount;
        self.sell(params).await
    }

    /// Wraps native SOL into wSOL (Wrapped SOL) for use in SPL token operations
    ///
    /// This function creates a wSOL associated token account (if it doesn't exist),
    /// transfers the specified amount of SOL to that account, and then syncs the native
    /// token balance to make SOL usable as an SPL token in trading operations.
    ///
    /// # Arguments
    /// * `amount` - The amount of SOL to wrap (in lamports)
    ///
    /// # Returns
    /// * `Ok(String)` - Transaction signature if successful
    /// * `Err(anyhow::Error)` - If the transaction fails to execute
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Insufficient SOL balance for the wrap operation
    /// - wSOL associated token account creation fails
    /// - Transaction fails to execute or confirm
    /// - Network or RPC errors occur
    pub async fn wrap_sol_to_wsol(&self, amount: u64) -> Result<String, anyhow::Error> {
        use crate::trading::common::wsol_manager::handle_wsol;
        use solana_sdk::transaction::Transaction;
        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let instructions = handle_wsol(&self.payer.pubkey(), amount);
        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey()));
        transaction.sign(&[&*self.payer], recent_blockhash);
        let signature = self.rpc.send_and_confirm_transaction(&transaction).await?;
        Ok(signature.to_string())
    }
    /// Closes the wSOL associated token account and unwraps remaining balance to native SOL
    ///
    /// This function closes the wSOL associated token account, which automatically
    /// transfers any remaining wSOL balance back to the account owner as native SOL.
    /// This is useful for cleaning up wSOL accounts and recovering wrapped SOL after trading operations.
    ///
    /// # Returns
    /// * `Ok(String)` - Transaction signature if successful
    /// * `Err(anyhow::Error)` - If the transaction fails to execute
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - wSOL associated token account doesn't exist
    /// - Account closure fails due to insufficient permissions
    /// - Transaction fails to execute or confirm
    /// - Network or RPC errors occur
    pub async fn close_wsol(&self) -> Result<String, anyhow::Error> {
        use crate::trading::common::wsol_manager::close_wsol;
        use solana_sdk::transaction::Transaction;
        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let instructions = close_wsol(&self.payer.pubkey());
        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey()));
        transaction.sign(&[&*self.payer], recent_blockhash);
        let signature = self.rpc.send_and_confirm_transaction(&transaction).await?;
        Ok(signature.to_string())
    }

    /// Creates a wSOL associated token account (ATA) without wrapping any SOL
    ///
    /// This function only creates the wSOL associated token account for the payer
    /// without transferring any SOL into it. This is useful when you want to set up
    /// the account infrastructure in advance without committing funds yet.
    ///
    /// # Returns
    /// * `Ok(String)` - Transaction signature if successful
    /// * `Err(anyhow::Error)` - If the transaction fails to execute
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - wSOL ATA account already exists (idempotent, will succeed silently)
    /// - Transaction fails to execute or confirm
    /// - Network or RPC errors occur
    /// - Insufficient SOL for transaction fees
    pub async fn create_wsol_ata(&self) -> Result<String, anyhow::Error> {
        use crate::trading::common::wsol_manager::create_wsol_ata;
        use solana_sdk::transaction::Transaction;

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let instructions = create_wsol_ata(&self.payer.pubkey());

        // If instructions are empty, ATA already exists
        if instructions.is_empty() {
            return Err(anyhow::anyhow!("wSOL ATA already exists or no instructions needed"));
        }

        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey()));
        transaction.sign(&[&*self.payer], recent_blockhash);
        let signature = self.rpc.send_and_confirm_transaction(&transaction).await?;
        Ok(signature.to_string())
    }

    /// 将 WSOL 转换为 SOL，使用 seed 账户
    ///
    /// 这个函数实现以下步骤：
    /// 1. 使用 super::seed::create_associated_token_account_use_seed 创建 WSOL seed 账号
    /// 2. 使用 get_associated_token_address_with_program_id_use_seed 获取该账号的 ATA 地址
    /// 3. 添加从用户 WSOL ATA 转账到该 seed ATA 账号的指令
    /// 4. 添加关闭 WSOL seed 账号的指令
    ///
    /// # Arguments
    /// * `amount` - 要转换的 WSOL 数量（以 lamports 为单位）
    ///
    /// # Returns
    /// * `Ok(String)` - 交易签名
    /// * `Err(anyhow::Error)` - 如果交易执行失败
    ///
    /// # Errors
    ///
    /// 此函数在以下情况下会返回错误：
    /// - 用户 WSOL ATA 中余额不足
    /// - seed 账户创建失败
    /// - 转账指令执行失败
    /// - 交易执行或确认失败
    /// - 网络或 RPC 错误
    pub async fn wrap_wsol_to_sol(&self, amount: u64) -> Result<String, anyhow::Error> {
        use crate::common::seed::get_associated_token_address_with_program_id_use_seed;
        use crate::trading::common::wsol_manager::{
            wrap_wsol_to_sol as wrap_wsol_to_sol_internal, wrap_wsol_to_sol_without_create,
        };
        use solana_sdk::transaction::Transaction;

        // 检查临时seed账户是否已存在
        let seed_ata_address = get_associated_token_address_with_program_id_use_seed(
            &self.payer.pubkey(),
            &crate::constants::WSOL_TOKEN_ACCOUNT,
            &crate::constants::TOKEN_PROGRAM,
        )?;

        let account_exists = self.rpc.get_account(&seed_ata_address).await.is_ok();

        let instructions = if account_exists {
            // 如果账户已存在，使用不创建账户的版本
            wrap_wsol_to_sol_without_create(&self.payer.pubkey(), amount)?
        } else {
            // 如果账户不存在，使用创建账户的版本
            wrap_wsol_to_sol_internal(&self.payer.pubkey(), amount)?
        };

        let recent_blockhash = self.rpc.get_latest_blockhash().await?;
        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey()));
        transaction.sign(&[&*self.payer], recent_blockhash);
        let signature = self.rpc.send_and_confirm_transaction(&transaction).await?;
        Ok(signature.to_string())
    }

    /// Creates a new token on PumpFun bonding curve
    ///
    /// This function creates a new SPL token and initializes its bonding curve on PumpFun.
    /// You can choose between the traditional `create` instruction (Token program) or
    /// the newer `create_v2` instruction (Token2022 with Mayhem mode support).
    ///
    /// # Arguments
    /// * `name` - Token name
    /// * `symbol` - Token symbol (max 10 characters)
    /// * `uri` - Metadata URI (JSON metadata URL)
    /// * `use_v2` - Whether to use create_v2 (Token2022 + Mayhem support). If false, uses traditional create
    /// * `is_mayhem_mode` - Whether to enable Mayhem mode (only for create_v2)
    ///
    /// # Returns
    /// * `Ok((Pubkey, String))` - Tuple of (mint address, transaction signature) if successful
    /// * `Err(anyhow::Error)` - If the transaction fails to execute
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Token name or symbol is empty
    /// - Symbol exceeds 10 characters
    /// - Mint keypair generation fails
    /// - Transaction fails to execute or confirm
    /// - Network or RPC errors occur
    pub async fn create_pumpfun_token(
        &self,
        name: String,
        symbol: String,
        uri: String,
        use_v2: bool,
        is_mayhem_mode: bool,
    ) -> Result<(Pubkey, String), anyhow::Error> {
        use crate::instruction::pumpfun::{CreateTokenParams, PumpFunInstructionBuilder};
        use solana_sdk::transaction::Transaction;

        // Validate inputs
        if name.trim().is_empty() {
            return Err(anyhow::anyhow!("Token name cannot be empty"));
        }
        if symbol.trim().is_empty() {
            return Err(anyhow::anyhow!("Token symbol cannot be empty"));
        }
        if symbol.len() > 10 {
            return Err(anyhow::anyhow!("Token symbol must be 10 characters or less"));
        }
        if use_v2 && is_mayhem_mode {
            // Mayhem mode is experimental and high-risk
            // We allow it but don't enforce any restrictions here
        }

        // Generate mint keypair
        let mint = Arc::new(Keypair::new());

        // Build create instruction
        let create_params = CreateTokenParams {
            mint: mint.clone(),
            name,
            symbol,
            uri,
            creator: self.payer.pubkey(),
            use_v2,
            is_mayhem_mode,
        };

        let instruction = if use_v2 {
            PumpFunInstructionBuilder::build_create_v2_instruction(&create_params)?
        } else {
            PumpFunInstructionBuilder::build_create_instruction(&create_params)?
        };

        // Build and send transaction
        // Reference: pumpfun-bonkfun-bot uses Transaction([payer, mint_keypair], message, recent_blockhash)
        // Signers order: payer first (as fee payer), then mint (as instruction signer)
        let recent_blockhash = self.rpc.get_latest_blockhash().await?;

        // Build message first, then create transaction with signers
        // Reference: pumpfun-bonkfun-bot uses Transaction([payer, mint_keypair], message, recent_blockhash)
        // Signers order: payer first (as fee payer), then mint (as instruction signer)

        // 为什么需要 Message？
        // 在 Solana 中，Transaction 由两部分组成：
        // 1. Message: 包含交易的逻辑信息（指令、账户、fee payer、blockhash 等）
        // 2. signatures: 签名数组
        //
        // 为什么使用 Message::new() + Transaction::new_unsigned()？
        // - 需要精确控制签名者顺序：payer 作为 fee payer（必须在 message.account_keys[0]），
        //   mint 作为 instruction signer（在指令账户列表中标记为 signer）
        // - 如果使用 Transaction::new_with_payer()，签名顺序可能不符合要求
        //
        // 与 IDL 的关系：
        // - IDL 文件定义了程序的接口（指令名称、参数、账户结构），主要用于代码生成和接口定义
        // - 这里手动构建了 instruction（通过 build_create_instruction），不依赖 IDL 来创建 Message
        // - IDL 不直接参与运行时交易构建，Message 的创建使用的是 Solana SDK 的底层 API
        use solana_sdk::message::Message;
        let message = Message::new(&[instruction], Some(&self.payer.pubkey()));

        // Create transaction with signers in correct order: [payer, mint]
        // payer is fee payer (first in message.account_keys), mint is instruction signer
        let mut transaction = Transaction::new_unsigned(message);

        // Sign transaction: payer first (as fee payer), then mint (as instruction signer)
        transaction.sign(&[&*self.payer, &*mint], recent_blockhash);

        let signature = self.rpc.send_and_confirm_transaction(&transaction).await?;

        Ok((mint.pubkey(), signature.to_string()))
    }
}
