//! 基础解析器 trait 定义

use async_trait::async_trait;

use super::{
    transaction_adapter::TransactionAdapter,
    types::{DexProtocol, ParsedTradeInfo},
};

/// 解析器错误
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("不支持的协议: {0}")]
    UnsupportedProtocol(String),
    #[error("指令数据缺失")]
    MissingInstructionData,
    #[error("账户数据缺失")]
    MissingAccountData,
    #[error("代币信息缺失")]
    MissingTokenInfo,
    #[error("解析失败: {0}")]
    ParseFailed(String),
}

/// 基础解析器 trait
///
/// 所有 DEX 协议解析器必须实现此 trait
#[async_trait]
pub trait DexParserTrait: Send + Sync {
    /// 解析交易并返回交易信息列表
    ///
    /// # 参数
    /// - `adapter`: 交易适配器，提供统一的交易数据访问接口
    ///
    /// # 返回
    /// 解析后的交易信息列表
    async fn parse(&self, adapter: &TransactionAdapter)
    -> Result<Vec<ParsedTradeInfo>, ParseError>;

    /// 获取解析器支持的协议
    fn protocol(&self) -> DexProtocol;

    /// 判断是否可以解析此交易
    ///
    /// # 参数
    /// - `adapter`: 交易适配器
    ///
    /// # 返回
    /// 如果可以解析返回 true
    fn can_parse(&self, adapter: &TransactionAdapter) -> bool {
        let program_id = self.protocol().program_id();
        adapter
            .instructions
            .iter()
            .any(|instr| instr.program_id.to_string() == program_id)
    }
}

/// 基础解析器实现
///
/// 提供通用的解析逻辑和辅助方法
pub struct BaseParser {
    protocol: DexProtocol,
}

impl BaseParser {
    /// 创建新的基础解析器
    pub fn new(protocol: DexProtocol) -> Self {
        Self { protocol }
    }

    /// 获取协议
    pub fn protocol(&self) -> DexProtocol {
        self.protocol
    }

    /// 从账户列表中提取池地址
    ///
    /// 不同协议的池地址在账户列表中的位置不同
    /// 子类可以重写此方法以自定义逻辑
    pub fn extract_pool_address(
        &self,
        accounts: &[solana_sdk::pubkey::Pubkey],
    ) -> Option<solana_sdk::pubkey::Pubkey> {
        match self.protocol {
            DexProtocol::PumpSwap => accounts.get(1).copied(),
            DexProtocol::RaydiumV4 => accounts.get(1).copied(),
            DexProtocol::RaydiumClmm => accounts.get(2).copied(),
            DexProtocol::RaydiumCpmm => accounts.get(3).copied(),
        }
    }

    /// 从账户列表中提取用户地址
    ///
    /// 子类可以重写此方法以自定义逻辑
    pub fn extract_user_address(
        &self,
        accounts: &[solana_sdk::pubkey::Pubkey],
    ) -> Option<solana_sdk::pubkey::Pubkey> {
        accounts.first().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_base_parser_extract_pool() {
        let parser = BaseParser::new(DexProtocol::PumpSwap);
        let accounts = vec![
            Pubkey::new_unique(), // 用户
            Pubkey::new_unique(), // 池
            Pubkey::new_unique(),
        ];

        let pool = parser.extract_pool_address(&accounts);
        assert!(pool.is_some());
        assert_eq!(pool.unwrap(), accounts[1]);
    }

    #[test]
    fn test_base_parser_extract_user() {
        let parser = BaseParser::new(DexProtocol::PumpSwap);
        let accounts = vec![
            Pubkey::new_unique(), // 用户
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        ];

        let user = parser.extract_user_address(&accounts);
        assert!(user.is_some());
        assert_eq!(user.unwrap(), accounts[0]);
    }
}
