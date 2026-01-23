//! RPC 客户端包装器
//!
//! 支持标准 RpcClient 和 AutoMockRpcClient

use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::sync::Arc;

use super::auto_mock_rpc::AutoMockRpcClient;

/// RPC 客户端包装器枚举
#[derive(Clone)]
pub enum RpcClientWrapper {
    /// 标准 RPC 客户端（生产环境）
    Standard(Arc<solana_rpc_client::rpc_client::RpcClient>),
    /// Auto Mock RPC 客户端（测试环境）
    AutoMock(Arc<AutoMockRpcClient>),
}

impl RpcClientWrapper {
    /// 获取交易（异步）
    pub async fn get_transaction_with_config(
        &self,
        sig: &Signature,
        config: RpcTransactionConfig,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta, String> {
        match self {
            RpcClientWrapper::Standard(client) => {
                let client = client.clone();
                let sig = *sig;

                tokio::task::spawn_blocking(move || {
                    client
                        .get_transaction_with_config(&sig, config)
                        .map_err(|e| format!("RPC 调用失败: {}", e))
                })
                .await
                .map_err(|e| format!("任务执行失败: {}", e))?
            }
            RpcClientWrapper::AutoMock(client) => {
                client.get_transaction(sig, config).await
            }
        }
    }
}
