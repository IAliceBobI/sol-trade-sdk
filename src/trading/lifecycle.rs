//! 交易生命周期回调模块
//!
//! 提供交易生命周期钩子，允许上游应用在关键节点拦截和处理交易

use anyhow::Result;
use solana_sdk::transaction::VersionedTransaction;
use crate::swqos::{SwqosType, TradeType};
use base64::Engine;
use std::sync::Arc;
use tracing::warn;

/// 交易生命周期回调 Trait
///
/// 允许上游应用在交易签名后、发送前进行自定义处理
/// 例如：入库、审计、日志记录等
pub trait TransactionLifecycleCallback: Send + Sync {
    /// 交易签名后、发送前的回调
    ///
    /// # 参数
    /// * `context` - 回调上下文，包含交易和元数据
    ///
    /// # 返回
    /// * `Ok(())` - 回调成功，不影响交易发送
    /// * `Err(e)` - 回调失败，**不会**阻止交易发送（仅记录错误）
    ///
    /// # 性能说明
    /// - 此回调在异步任务中执行，**不会阻塞**交易发送
    /// - 建议使用异步非阻塞方式处理（如发送到消息队列）
    /// - 如果回调耗时较长，建议使用 `tokio::spawn` 在后台处理
    fn on_transaction_signed(&self, context: CallbackContext) -> futures::future::BoxFuture<'static, Result<()>>;
}

/// 回调上下文
///
/// 包含签名后的交易和完整的元数据
#[derive(Clone)]
pub struct CallbackContext {
    /// 签名后的交易
    pub transaction: VersionedTransaction,

    /// SWQOS 服务类型
    pub swqos_type: SwqosType,

    /// 交易类型（买入/卖出）
    pub trade_type: TradeType,

    /// 交易签名
    pub signature: String,

    /// 时间戳（纳秒）
    pub timestamp_ns: u64,

    /// 是否包含小费
    pub with_tip: bool,

    /// 小费金额（SOL）
    pub tip_amount: f64,
}

impl CallbackContext {
    /// 创建新的回调上下文
    pub fn new(
        transaction: VersionedTransaction,
        swqos_type: SwqosType,
        trade_type: TradeType,
        with_tip: bool,
        tip_amount: f64,
    ) -> Self {
        let signature = transaction
            .signatures
            .first()
            .map(|sig| sig.to_string())
            .unwrap_or_else(|| {
                warn!("交易没有签名，使用空字符串作为默认签名");
                String::new()
            });

        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|e| {
                warn!("获取系统时间失败: {}，使用 0 作为默认时间戳", e);
                std::time::Duration::from_secs(0)
            })
            .as_nanos() as u64;

        Self {
            transaction,
            swqos_type,
            trade_type,
            signature,
            timestamp_ns,
            with_tip,
            tip_amount,
        }
    }

    /// 获取交易的 Base64 编码
    pub fn to_base64(&self) -> String {
        use bincode::serialize;
        serialize(&self.transaction)
            .ok()
            .and_then(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes).into())
            .unwrap_or_else(|| {
                warn!("交易序列化为 Base64 失败，返回空字符串");
                String::new()
            })
    }

    /// 获取交易的 JSON 表示（用于日志）
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "signature": self.signature,
            "swqos_type": format!("{:?}", self.swqos_type),
            "trade_type": format!("{:?}", self.trade_type),
            "timestamp_ns": self.timestamp_ns,
            "with_tip": self.with_tip,
            "tip_amount": self.tip_amount,
            "transaction_base64": self.to_base64(),
        })
    }
}

/// 空回调实现（默认实现，不做任何操作）
#[derive(Clone)]
pub struct NoopCallback;

impl TransactionLifecycleCallback for NoopCallback {
    fn on_transaction_signed(&self, _context: CallbackContext) -> futures::future::BoxFuture<'static, Result<()>> {
        Box::pin(async { Ok(()) })
    }
}

/// Arc 包装的回调（便于共享）
pub type CallbackRef = Arc<dyn TransactionLifecycleCallback>;