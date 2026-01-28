pub mod types;
pub mod dynamic_tip;

pub use dynamic_tip::{DynamicTipConfig, JitoTipFloorClient, TipPercentile};
pub use types::JitoRegion;

use crate::swqos::common::{
    FormatBase64VersionedTransaction, poll_transaction_confirmation,
    serialize_transaction_and_encode,
};
use rand::seq::IndexedRandom;
use reqwest::Client;
use serde_json::json;
use std::{sync::Arc, time::Instant};

use solana_transaction_status::UiTransactionEncoding;
use std::time::Duration;

use crate::swqos::SwqosClientTrait;
use crate::swqos::{SwqosType, TradeType};
use anyhow::Result;
use solana_sdk::transaction::VersionedTransaction;

use crate::{common::SolanaRpcClient, constants::swqos::JITO_TIP_ACCOUNTS};

/// Jito Sandwich 保护账户前缀
///
/// 在交易中添加以 `jitodontfront` 开头的只读账户可以防止 sandwich attacks
/// 参考：https://docs.jito.wtf/lowlatencytxnsend/#sandwich-mitigation
pub const JITO_DONT_FRONT_PREFIX: &str = "jitodontfront";

/// 默认的 jitodontfront 账户
pub const JITO_DONT_FRONT_DEFAULT: &str = "jitodontfront111111111111111111111111111111";

/// 生成 jitodontfront 账户
///
/// # 参数
///
/// * `custom_suffix` - 自定义后缀（可选）
///
/// # 示例
///
/// ```rust
/// use sol_trade_sdk::swqos::jito::generate_dont_front_account;
///
/// // 使用默认账户
/// let account = generate_dont_front_account(None);
///
/// // 使用自定义后缀
/// let account = generate_dont_front_account(Some("myapp"));
/// ```
pub fn generate_dont_front_account(custom_suffix: Option<&str>) -> String {
    match custom_suffix {
        Some(suffix) => format!("{}{}", JITO_DONT_FRONT_PREFIX, suffix),
        None => JITO_DONT_FRONT_DEFAULT.to_string(),
    }
}

pub struct JitoClient {
    pub endpoint: String,
    pub auth_token: String,
    pub rpc_client: Arc<SolanaRpcClient>,
    pub http_client: Client,
}

#[async_trait::async_trait]
impl SwqosClientTrait for JitoClient {
    async fn send_transaction(
        &self,
        trade_type: TradeType,
        transaction: &VersionedTransaction,
        wait_confirmation: bool,
    ) -> Result<()> {
        self.send_transaction_impl(trade_type, transaction, wait_confirmation).await
    }

    async fn send_transactions(
        &self,
        trade_type: TradeType,
        transactions: &[VersionedTransaction],
        wait_confirmation: bool,
    ) -> Result<()> {
        self.send_transactions_impl(trade_type, transactions, wait_confirmation).await
    }

    fn get_tip_account(&self) -> Result<String> {
        if let Some(acc) = JITO_TIP_ACCOUNTS.choose(&mut rand::rng()) {
            Ok(acc.to_string())
        } else {
            Err(anyhow::anyhow!("no valid tip accounts found"))
        }
    }

    fn get_swqos_type(&self) -> SwqosType {
        SwqosType::Jito
    }
}

impl JitoClient {
    /// 创建新的 Jito Client
    ///
    /// # 参数
    ///
    /// * `rpc_url` - Solana RPC URL
    /// * `region` - Jito 区域（选择最近的区域以降低延迟）
    /// * `auth_token` - Jito 认证令牌（可选，用于更高的速率限制）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use sol_trade_sdk::swqos::jito::{JitoClient, types::JitoRegion};
    ///
    /// // 使用默认区域
    /// let client = JitoClient::new(
    ///     "http://127.0.0.1:8899".to_string(),
    ///     JitoRegion::Default,
    ///     String::new(),
    /// );
    ///
    /// // 亚洲用户使用东京区域
    /// let client = JitoClient::new(
    ///     "http://127.0.0.1:8899".to_string(),
    ///     JitoRegion::Tokyo,
    ///     String::new(),
    /// );
    /// ```
    pub fn new(rpc_url: String, region: JitoRegion, auth_token: String) -> Self {
        let endpoint = region.endpoint().to_string();
        let rpc_client = SolanaRpcClient::new(rpc_url);
        let http_client = Client::builder()
            // Optimized connection pool settings for high performance
            .pool_idle_timeout(Duration::from_secs(120))
            .pool_max_idle_per_host(256) // Increased from 64 to 256
            .tcp_keepalive(Some(Duration::from_secs(60))) // Reduced from 1200 to 60
            .tcp_nodelay(true) // Disable Nagle's algorithm for lower latency
            .http2_keep_alive_interval(Duration::from_secs(10))
            .http2_keep_alive_timeout(Duration::from_secs(5))
            .http2_adaptive_window(true) // Enable adaptive flow control
            .timeout(Duration::from_millis(3000)) // Reduced from 10s to 3s
            .connect_timeout(Duration::from_millis(2000)) // Reduced from 5s to 2s
            .build()
            .unwrap();
        Self {
            rpc_client: Arc::new(rpc_client),
            endpoint,
            auth_token,
            http_client,
        }
    }

    /// 使用指定区域创建 Jito Client（推荐）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use sol_trade_sdk::swqos::jito::{JitoClient, types::JitoRegion};
    ///
    /// let client = JitoClient::with_region(JitoRegion::Tokyo);
    /// ```
    pub fn with_region(region: JitoRegion) -> Self {
        Self::new(
            "http://127.0.0.1:8899".to_string(),
            region,
            String::new(),
        )
    }

    pub async fn send_transaction_impl(
        &self,
        trade_type: TradeType,
        transaction: &VersionedTransaction,
        wait_confirmation: bool,
    ) -> Result<()> {
        let start_time = Instant::now();
        let (content, signature) =
            serialize_transaction_and_encode(transaction, UiTransactionEncoding::Base64).await?;

        let request_body = serde_json::to_string(&json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": "sendTransaction",
            "params": [
                content,
                {
                    "encoding": "base64"
                }
            ]
        }))?;

        let endpoint = if self.auth_token.is_empty() {
            format!("{}/api/v1/transactions", self.endpoint)
        } else {
            format!("{}/api/v1/transactions?uuid={}", self.endpoint, self.auth_token)
        };
        let response = if self.auth_token.is_empty() {
            self.http_client.post(&endpoint)
        } else {
            self.http_client.post(&endpoint).header("x-jito-auth", &self.auth_token)
        };
        let response_text = response
            .body(request_body)
            .header("Content-Type", "application/json")
            .send()
            .await?
            .text()
            .await?;

        if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            if response_json.get("result").is_some() {
                println!(" [jito] {} submitted: {:?}", trade_type, start_time.elapsed());
            } else if let Some(_error) = response_json.get("error") {
                eprintln!(" [jito] {} submission failed: {:?}", trade_type, _error);
            }
        } else {
            eprintln!(" [jito] {} submission failed: {:?}", trade_type, response_text);
        }

        let start_time: Instant = Instant::now();
        match poll_transaction_confirmation(&self.rpc_client, signature, wait_confirmation).await {
            Ok(_) => (),
            Err(e) => {
                println!(" signature: {:?}", signature);
                println!(" [jito] {} confirmation failed: {:?}", trade_type, start_time.elapsed());
                return Err(e);
            },
        }
        if wait_confirmation {
            println!(" signature: {:?}", signature);
            println!(" [jito] {} confirmed: {:?}", trade_type, start_time.elapsed());
        }

        Ok(())
    }

    pub async fn send_transactions_impl(
        &self,
        trade_type: TradeType,
        transactions: &[VersionedTransaction],
        _wait_confirmation: bool,
    ) -> Result<()> {
        let start_time = Instant::now();
        let txs_base64 =
            transactions.iter().map(|tx| tx.to_base64_string()).collect::<Vec<String>>();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "sendBundle",
            "params": [
                txs_base64,
                { "encoding": "base64" }
            ],
            "id": 1,
        });

        let endpoint = if self.auth_token.is_empty() {
            format!("{}/api/v1/bundles", self.endpoint)
        } else {
            format!("{}/api/v1/bundles?uuid={}", self.endpoint, self.auth_token)
        };
        let response = if self.auth_token.is_empty() {
            self.http_client.post(&endpoint)
        } else {
            self.http_client.post(&endpoint).header("x-jito-auth", &self.auth_token)
        };
        let response_text = response
            .body(body.to_string())
            .header("Content-Type", "application/json")
            .send()
            .await?
            .text()
            .await?;

        if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(&response_text) {
            if response_json.get("result").is_some() {
                println!(" jito {} submitted: {:?}", trade_type, start_time.elapsed());
            } else if let Some(_error) = response_json.get("error") {
                eprintln!(" jito {} submission failed: {:?}", trade_type, _error);
            }
        }

        Ok(())
    }
}
