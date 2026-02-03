//! 共享基础设施模块
//!
//! 提供多钱包场景下可共享的基础组件，包括 RPC 客户端和 SWQOS 客户端。

use crate::common::SolanaRpcClient;
use crate::{InfrastructureConfig, SwqosClient, SwqosConfig};
use rustls::crypto::{CryptoProvider, ring::default_provider};
use std::sync::Arc;

/// 共享的基础设施组件，可在多个钱包之间共享
///
/// 此结构体持有昂贵的初始化组件（RPC 客户端、SWQOS 客户端），
/// 这些组件与钱包无关，可以在仅更改交易钱包时共享。
pub struct TradingInfrastructure {
    /// 区块链交互的共享 RPC 客户端
    pub rpc: Arc<SolanaRpcClient>,
    /// 交易优先级和路由的共享 SWQOS 客户端
    pub swqos_clients: Vec<Arc<SwqosClient>>,
    /// 用于创建此基础设施的配置
    pub config: InfrastructureConfig,
}

impl TradingInfrastructure {
    /// 从配置创建新的共享基础设施
    ///
    /// 此方法执行昂贵的初始化：
    /// - 创建带连接池的 RPC 客户端
    /// - 创建 SWQOS 客户端（每个都有自己的 HTTP 客户端）
    /// - 初始化 rent 缓存并启动后台更新器
    pub async fn new(config: InfrastructureConfig) -> Self {
        // 安装加密提供者（幂等操作）
        if CryptoProvider::get_default().is_none()
            && let Err(e) = default_provider().install_default()
        {
            eprintln!("⚠️  Failed to install crypto provider: {e:?}");
            eprintln!("    Crypto operations may fail. Continuing anyway...");
        }

        // 创建 RPC 客户端
        let rpc = Arc::new(SolanaRpcClient::new_with_commitment(
            config.rpc_url.clone(),
            config.commitment,
        ));

        // 初始化 rent 缓存并启动后台更新器
        crate::common::seed::update_rents(&rpc)
            .await
            .expect("Failed to initialize rent cache - this is required for trading operations");
        crate::common::seed::start_rent_updater(rpc.clone());

        // 创建带有黑名单检查的 SWQOS 客户端
        let mut swqos_clients: Vec<Arc<SwqosClient>> = vec![];
        for swqos in &config.swqos_configs {
            // 检查黑名单，跳过已禁用的提供商
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
