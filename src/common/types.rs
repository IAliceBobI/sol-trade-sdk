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
        Self { rpc_url, swqos_configs, commitment }
    }

    /// Create from TradeConfig (extract infrastructure-only settings)
    pub fn from_trade_config(config: &TradeConfig) -> Self {
        Self {
            rpc_url: config.rpc_url.clone(),
            swqos_configs: config.swqos_configs.clone(),
            commitment: config.commitment,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    #[default]
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
    /// 是否启用 Jito 三明治攻击防护（默认：false）
    ///
    /// # 功能说明
    ///
    /// Jito 的三明治防护通过在交易中添加以 `jitodontfront` 开头的只读账户来防止抢跑攻击。
    ///
    /// ## 启用防护 (enable_jito_sandwich_protection = true)
    ///
    /// ### 特性
    /// - ✅ **原子执行**: Bundle 内的交易要么全部成功，要么全部失败
    /// - ✅ **顺序保护**: 包含 `jitodontfront` 的交易必须在 Bundle 第一位（index 0）
    /// - ✅ **防止抢跑**: 阻止其他交易在你的交易前后插入（三明治攻击）
    ///
    /// ### 工作原理
    /// Jito Block Engine 会拒绝任何违反以下规则的 Bundle：
    /// - 包含 `jitodontfront` 账户的交易不在 Bundle 第一位
    /// - 在 `jitodontfront` 交易前后插入其他交易
    ///
    /// ### 适用场景
    /// - **套利交易**: 价格差异敏感，抢跑会让策略无利可图
    /// - **大额交易**: 容易被 MEV bot 盯上
    /// - **MEV 策略**: 需要确保执行顺序的交易
    ///
    /// ### 示例
    /// ```text
    /// # 启用防护后的 Bundle 结构
    /// Bundle: [
    ///   Swap + jitodontfront,  ← 必须在第一位，防止前后插入
    ///   tip
    /// ]
    /// ```
    ///
    /// ### 性能影响
    /// - 交易大小：+32 bytes（添加一个 Pubkey）
    /// - 执行速度：无影响（只读账户不消耗 CU）
    /// - 成功率：提高（防止三明治攻击导致的失败）
    ///
    /// ## 不启用防护 (enable_jito_sandwich_protection = false) - 推荐
    ///
    /// ### 特性
    /// - ✅ **原子执行**: Bundle 仍然是原子的（全有或全无）
    /// - ❌ **无顺序保护**: 可能被三明治攻击
    /// - ⚠️ **抢跑风险**: MEV bot 可能在你的交易前后插入交易
    ///
    /// ### 适用场景
    /// - **普通交易**: 只需要原子性，不关心顺序
    /// - **小额交易**: 不值得 MEV bot 抢跑
    /// - **已有保护**: 通过滑点限制、deadline 等方式保护
    ///
    /// ### 示例
    /// ```text
    /// # 不启用防护的 Bundle 结构
    /// Bundle: [
    ///   Swap,  ← 可能被抢跑
    ///   tip
    /// ]
    ///
    /// # 可能的攻击
    /// Bundle: [
    ///   攻击者买入,  ← 推高价格
    ///   你的 Swap,  ← 你以更高价格买入
    ///   攻击者卖出, ← 获利
    ///   tip
    /// ]
    /// ```
    ///
    /// ## 如何选择？
    ///
    /// | 交易类型 | 推荐设置 | 原因 |
    /// |----------|----------|------|
    /// | 普通 Swap | `false` | 原子性已足够，滑点保护已够用 |
    /// | 套利 | `true` | 对价格敏感，需要防抢跑 |
    /// | 大额交易 | `true` | 容易被 MEV bot 盯上 |
    /// | 小额测试 | `false` | 简单快速，不需要额外保护 |
    ///
    /// ## 官方文档
    ///
    /// 参考: https://docs.jito.wtf/lowlatencytxnsend/#sandwich-mitigation
    ///
    /// ## 注意事项
    ///
    /// 1. **只对 Jito 有效**: 这个功能只在 Jito Bundle 上生效，其他 SWQOS（如 ZeroSlot）不受影响
    /// 2. **不保证 100% 防护**: 官方文档说明此功能可能帮助减少但不能完全阻止三明治攻击
    /// 3. **账户不需要存在**: `jitodontfront` 账户只需是有效的 Pubkey，不需要在链上存在
    /// 4. **标记为只读**: 优化执行速度，不消耗额外的 Compute Unit
    pub enable_jito_sandwich_protection: bool,
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
        println!("🔧 TradeConfig enable_jito_sandwich_protection default value: false");
        Self {
            rpc_url,
            swqos_configs,
            commitment,
            create_wsol_ata_on_startup: true, // 默认：启动时检查并创建
            use_seed_optimize: false,         // 默认：禁用seed优化
            callback_execution_mode: CallbackExecutionMode::Async, // 默认：异步模式
            enable_jito_sandwich_protection: false, // 默认：禁用三明治防护（大多数场景不需要）
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

    /// 设置是否启用 Jito 三明治攻击防护
    ///
    /// # 参数
    /// * `enabled` - 是否启用防护（true 启用，false 禁用）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use sol_trade_sdk::TradingClient;
    /// # use sol_trade_sdk::common::TradeConfig;
    ///
    /// // 普通交易（不需要防护）
    /// # let config = TradeConfig::default();
    ///
    /// // 套利交易（需要防护）
    /// # let config = TradeConfig::default()
    ///     .with_jito_sandwich_protection(true);
    /// ```
    ///
    /// # 何时启用？
    ///
    /// - ✅ **套利交易**: 对价格敏感，抢跑会让策略无利可图
    /// - ✅ **大额交易**: 容易被 MEV bot 盯上
    /// - ✅ **MEV 策略**: 需要确保执行顺序
    ///
    /// # 何时不启用？
    ///
    /// - ❌ **普通 Swap**: 原子性已足够，滑点保护已够用
    /// - ❌ **小额交易**: 不值得 MEV bot 抢跑
    /// - ❌ **测试交易**: 简单快速即可
    ///
    /// # 详细说明
    ///
    /// 参见 `TradeConfig.enable_jito_sandwich_protection` 字段的详细文档。
    pub fn with_jito_sandwich_protection(mut self, enabled: bool) -> Self {
        self.enable_jito_sandwich_protection = enabled;
        self
    }
}

pub type SolanaRpcClient = solana_client::nonblocking::rpc_client::RpcClient;
pub type AnyResult<T> = anyhow::Result<T>;
