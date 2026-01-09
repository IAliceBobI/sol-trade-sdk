use crate::swqos::SwqosConfig;
use solana_commitment_config::CommitmentConfig;
use std::hash::{Hash, Hasher};

/// Infrastructure-only configuration (wallet-independent)
/// Can be shared across multiple wallets using the same RPC/SWQOS setup
#[derive(Debug, Clone)]
pub struct InfrastructureConfig {
    pub rpc_url: String,
    pub swqos_configs: Vec<SwqosConfig>,
    pub commitment: CommitmentConfig,
}

impl InfrastructureConfig {
    pub fn new(
        rpc_url: String,
        swqos_configs: Vec<SwqosConfig>,
        commitment: CommitmentConfig,
    ) -> Self {
        Self {
            rpc_url,
            swqos_configs,
            commitment,
        }
    }

    /// Create from TradeConfig (extract infrastructure-only settings)
    pub fn from_trade_config(config: &TradeConfig) -> Self {
        Self {
            rpc_url: config.rpc_url.clone(),
            swqos_configs: config.swqos_configs.clone(),
            commitment: config.commitment.clone(),
        }
    }

    /// Generate a cache key for this infrastructure configuration
    pub fn cache_key(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

// Manual Hash implementation since CommitmentConfig doesn't implement Hash
impl Hash for InfrastructureConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.rpc_url.hash(state);
        self.swqos_configs.hash(state);
        // Hash commitment level as string since CommitmentConfig doesn't impl Hash
        format!("{:?}", self.commitment).hash(state);
    }
}

impl PartialEq for InfrastructureConfig {
    fn eq(&self, other: &Self) -> bool {
        self.rpc_url == other.rpc_url
            && self.swqos_configs == other.swqos_configs
            && self.commitment == other.commitment
    }
}

impl Eq for InfrastructureConfig {}

/// 回调执行模式
///
/// 控制交易生命周期回调的执行方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackExecutionMode {
    /// 异步模式：不阻塞交易发送（默认）
    ///
    /// # 特性
    /// - 回调失败不影响交易发送
    /// - 使用 `tokio::spawn` 异步执行
    /// - 适合：监控、日志、非关键业务
    ///
    /// # 性能
    /// - 交易延迟：0ms（不阻塞）
    /// - 失败影响：不影响交易
    Async,

    /// 同步模式：等待回调完成后再发送交易
    ///
    /// # 特性
    /// - 回调失败会阻止交易发送
    /// - 使用 `.await` 同步等待
    /// - 适合：入库、审计、关键业务
    ///
    /// # 性能
    /// - 交易延迟：取决于回调执行时间
    /// - 失败影响：阻止交易发送
    Sync,
}

impl Default for CallbackExecutionMode {
    fn default() -> Self {
        Self::Async
    }
}

#[derive(Debug, Clone)]
pub struct TradeConfig {
    pub rpc_url: String,
    pub swqos_configs: Vec<SwqosConfig>,
    pub commitment: CommitmentConfig,
    /// Whether to create WSOL ATA on startup (default: true)
    /// If true, SDK will check WSOL ATA on initialization and create if not exists
    pub create_wsol_ata_on_startup: bool,
    /// Whether to use seed optimization for all ATA operations (default: false)
    pub use_seed_optimize: bool,
    /// 回调执行模式（默认：异步）
    ///
    /// - `Async`：异步执行，不阻塞交易发送
    /// - `Sync`：同步执行，等待回调完成后再发送交易
    pub callback_execution_mode: CallbackExecutionMode,
}

impl TradeConfig {
    pub fn new(
        rpc_url: String,
        swqos_configs: Vec<SwqosConfig>,
        commitment: CommitmentConfig,
    ) -> Self {
        println!("🔧 TradeConfig create_wsol_ata_on_startup default value: true");
        println!("🔧 TradeConfig use_seed_optimize default value: false");
        println!("🔧 TradeConfig callback_execution_mode default value: Async");
        Self {
            rpc_url,
            swqos_configs,
            commitment,
            create_wsol_ata_on_startup: true,  // 默认：启动时检查并创建
            use_seed_optimize: false,          // 默认：禁用seed优化
            callback_execution_mode: CallbackExecutionMode::Async,  // 默认：异步模式
        }
    }

    /// Create a TradeConfig with custom WSOL ATA settings
    pub fn with_wsol_ata_config(
        mut self,
        create_wsol_ata_on_startup: bool,
        use_seed_optimize: bool,
    ) -> Self {
        self.create_wsol_ata_on_startup = create_wsol_ata_on_startup;
        self.use_seed_optimize = use_seed_optimize;
        self
    }

    /// 设置回调执行模式
    ///
    /// # 参数
    /// * `mode` - 回调执行模式（Async 或 Sync）
    pub fn with_callback_execution_mode(mut self, mode: CallbackExecutionMode) -> Self {
        self.callback_execution_mode = mode;
        self
    }
}

pub type SolanaRpcClient = solana_client::nonblocking::rpc_client::RpcClient;
pub type AnyResult<T> = anyhow::Result<T>;
