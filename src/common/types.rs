use crate::swqos::SwqosConfig;
use solana_commitment_config::CommitmentConfig;

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
    /// Whether to use seed optimization for all ATA operations (default: true)
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
        println!("🔧 TradeConfig use_seed_optimize default value: true");
        println!("🔧 TradeConfig callback_execution_mode default value: Async");
        Self {
            rpc_url,
            swqos_configs,
            commitment,
            create_wsol_ata_on_startup: true,  // 默认：启动时检查并创建
            use_seed_optimize: true,           // 默认：使用seed优化
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
    ///
    /// # 示例
    /// ```ignore
    /// let trade_config = TradeConfig::new(rpc_url, swqos_configs, commitment)
    ///     .with_callback_execution_mode(CallbackExecutionMode::Sync);
    /// ```
    pub fn with_callback_execution_mode(mut self, mode: CallbackExecutionMode) -> Self {
        self.callback_execution_mode = mode;
        self
    }
}

pub type SolanaRpcClient = solana_client::nonblocking::rpc_client::RpcClient;
pub type AnyResult<T> = anyhow::Result<T>;
