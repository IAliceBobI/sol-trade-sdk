//! TradingClient 构造函数相关方法

use super::types::TradingClient;
use crate::common::fast_fn;
use crate::constants::{TOKEN_PROGRAM, WSOL_TOKEN_ACCOUNT};
use crate::{SolanaRpcClient, SwqosClient, SwqosConfig, TradeConfig};
use parking_lot::Mutex;
use rustls::crypto::{CryptoProvider, ring::default_provider};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::sync::Arc;

use super::super::trading::common::wsol_manager;

static INSTANCE: Mutex<Option<Arc<TradingClient>>> = Mutex::new(None);

/// 🔄 向后兼容：SolanaTrade 别名
pub type SolanaTrade = TradingClient;

impl TradingClient {
    /// 创建具有指定配置的新 SolTradingSDK 实例
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
        fast_fn::fast_init(&pubkey);

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
        crate::common::seed::update_rents(&rpc)
            .await
            .expect("Failed to initialize rent cache - this is required for trading operations");
        crate::common::seed::start_rent_updater(rpc.clone());

        // 🔧 初始化WSOL ATA：如果配置为启动时创建，则检查并创建
        if trade_config.create_wsol_ata_on_startup {
            // 根据seed配置计算WSOL ATA地址
            let wsol_ata = fast_fn::get_associated_token_address_with_program_id_fast(
                &payer.pubkey(),
                &WSOL_TOKEN_ACCOUNT,
                &TOKEN_PROGRAM,
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
                    let create_ata_ixs = wsol_manager::create_wsol_ata(&payer.pubkey());

                    if !create_ata_ixs.is_empty() {
                        // 构建并发送交易
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

    /// 添加中间件管理器到 SolanaTrade 实例
    ///
    /// 中间件管理器可用于实现自定义逻辑，在交易操作之前或之后运行，
    /// 例如日志记录、监控或自定义验证。
    ///
    /// # Arguments
    /// * `middleware_manager` - 要附加的中间件管理器
    ///
    /// # Returns
    /// Returns the modified SolanaTrade instance with middleware manager attached
    pub fn with_middleware_manager(mut self, middleware_manager: crate::MiddlewareManager) -> Self {
        self.middleware_manager = Some(Arc::new(middleware_manager));
        self
    }

    /// 获取 RPC 客户端实例，用于直接的 Solana 区块链交互
    ///
    /// 这提供对底层 Solana RPC 客户端的访问，可用于交易框架之外的自定义区块链操作。
    ///
    /// # Returns
    /// Returns a reference to the Arc-wrapped SolanaRpcClient instance
    pub fn get_rpc(&self) -> &Arc<SolanaRpcClient> {
        &self.rpc
    }

    /// 获取当前全局共享的 SolanaTrade 实例
    ///
    /// 这提供对使用 `new()` 创建的单例实例的访问。
    /// 对于从应用程序的不同部分访问交易实例非常有用。
    ///
    /// # Returns
    /// Returns the Arc-wrapped SolanaTrade instance
    ///
    /// # Panics
    /// 如果尚未初始化任何实例，则会 panic。请确保先调用 `new()`。
    pub fn get_instance() -> Arc<Self> {
        let instance = INSTANCE.lock();
        instance
            .as_ref()
            .expect("SolanaTrade instance not initialized. Please call new() first.")
            .clone()
    }
}
