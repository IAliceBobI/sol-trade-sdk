//! DEX 验证框架核心类型定义

use sol_trade_sdk::DexType;
use solana_sdk::pubkey::Pubkey;
use std::fmt;

/// Token Program 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenProgramType {
    /// SPL Token Program
    Token,
    /// Token-2022 Program
    Token2022,
}

impl fmt::Display for TokenProgramType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Token => write!(f, "Token"),
            Self::Token2022 => write!(f, "Token2022"),
        }
    }
}

/// Pool 配置（包含两个 Token 的 Program 类型信息）
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Pool 地址
    pub pool_address: Pubkey,
    /// Pool 名称（用于显示）
    pub pool_name: &'static str,
    /// Token0 Mint 地址
    pub token0_mint: Pubkey,
    /// Token0 使用的 Program 类型
    pub token0_program: TokenProgramType,
    /// Token1 Mint 地址
    pub token1_mint: Pubkey,
    /// Token1 使用的 Program 类型
    pub token1_program: TokenProgramType,
    /// 流动性数量（用于初始化）
    pub liquidity_amount: u64,
}

impl PoolConfig {
    /// 创建新的 Pool 配置
    pub fn new(
        pool_address: Pubkey,
        pool_name: &'static str,
        token0_mint: Pubkey,
        token0_program: TokenProgramType,
        token1_mint: Pubkey,
        token1_program: TokenProgramType,
        liquidity_amount: u64,
    ) -> Self {
        Self {
            pool_address,
            pool_name,
            token0_mint,
            token0_program,
            token1_mint,
            token1_program,
            liquidity_amount,
        }
    }

    /// 判断是否是混合 Pool（Token + Token-2022）
    pub fn is_mixed_pool(&self) -> bool {
        self.token0_program != self.token1_program
    }

    /// 判断是否需要 Token-2022 支持
    pub fn requires_token2022(&self) -> bool {
        matches!(self.token0_program, TokenProgramType::Token2022)
            || matches!(self.token1_program, TokenProgramType::Token2022)
    }

    /// 获取 Pool 类型描述
    pub fn pool_type_description(&self) -> String {
        if self.is_mixed_pool() {
            format!("Mixed ({} + {})", self.token0_program, self.token1_program)
        } else if self.requires_token2022() {
            "Token2022".to_string()
        } else {
            "Token".to_string()
        }
    }
}

impl fmt::Display for PoolConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.pool_name, self.pool_type_description())
    }
}

/// 操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    /// 买入（已知输入，计算输出）
    BuyExactIn,
    /// 卖出（已知输入，计算输出）
    SellExactIn,
    /// 买入（已知输出，计算输入）
    BuyExactOut,
    /// 卖出（已知输出，计算输入）
    SellExactOut,
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuyExactIn => write!(f, "Buy Exact In"),
            Self::SellExactIn => write!(f, "Sell Exact In"),
            Self::BuyExactOut => write!(f, "Buy Exact Out"),
            Self::SellExactOut => write!(f, "Sell Exact Out"),
        }
    }
}

impl OperationType {
    /// 判断是否是 Exact In 操作
    pub fn is_exact_in(&self) -> bool {
        matches!(self, Self::BuyExactIn | Self::SellExactIn)
    }

    /// 判断是否是 Exact Out 操作
    pub fn is_exact_out(&self) -> bool {
        matches!(self, Self::BuyExactOut | Self::SellExactOut)
    }

    /// 判断是否是买入操作
    pub fn is_buy(&self) -> bool {
        matches!(self, Self::BuyExactIn | Self::BuyExactOut)
    }

    /// 判断是否是卖出操作
    pub fn is_sell(&self) -> bool {
        matches!(self, Self::SellExactIn | Self::SellExactOut)
    }
}

/// 交易方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeDirection {
    /// Token0 -> Token1
    Token0ToToken1,
    /// Token1 -> Token0
    Token1ToToken0,
}

impl fmt::Display for TradeDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Token0ToToken1 => write!(f, "Token0 -> Token1"),
            Self::Token1ToToken0 => write!(f, "Token1 -> Token0"),
        }
    }
}

/// 完整的 DEX 验证配置
#[derive(Debug, Clone)]
pub struct DexVerifyConfig {
    /// DEX 类型
    pub dex_type: DexType,
    /// Pool 配置
    pub pool: PoolConfig,
    /// 操作类型
    pub operation: OperationType,
    /// 交易方向
    pub direction: TradeDirection,
    /// 输入金额
    pub input_amount: u64,
    /// 是否跳过本地 Quote 计算（适用于本地计算不准确的场景，如 CLMM 负数 tick）
    pub skip_local_quote: bool,
}

impl fmt::Display for DexVerifyConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} - {} - {} - {}", self.dex_type, self.pool, self.operation, self.direction)
    }
}

impl DexVerifyConfig {
    /// 获取输入 Token Mint
    pub fn input_mint(&self) -> Pubkey {
        match self.direction {
            TradeDirection::Token0ToToken1 => self.pool.token0_mint,
            TradeDirection::Token1ToToken0 => self.pool.token1_mint,
        }
    }

    /// 获取输出 Token Mint
    pub fn output_mint(&self) -> Pubkey {
        match self.direction {
            TradeDirection::Token0ToToken1 => self.pool.token1_mint,
            TradeDirection::Token1ToToken0 => self.pool.token0_mint,
        }
    }

    /// 获取输入 Token Program 类型
    pub fn input_token_program(&self) -> TokenProgramType {
        match self.direction {
            TradeDirection::Token0ToToken1 => self.pool.token0_program,
            TradeDirection::Token1ToToken0 => self.pool.token1_program,
        }
    }

    /// 获取输出 Token Program 类型
    pub fn output_token_program(&self) -> TokenProgramType {
        match self.direction {
            TradeDirection::Token0ToToken1 => self.pool.token1_program,
            TradeDirection::Token1ToToken0 => self.pool.token0_program,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn test_pool_config_detection() {
        let token_only = PoolConfig::new(
            Pubkey::default(),
            "Test",
            Pubkey::default(),
            TokenProgramType::Token,
            Pubkey::new_unique(),
            TokenProgramType::Token,
            10,
        );
        assert!(!token_only.is_mixed_pool());
        assert!(!token_only.requires_token2022());

        let token2022_only = PoolConfig::new(
            Pubkey::default(),
            "Test",
            Pubkey::default(),
            TokenProgramType::Token2022,
            Pubkey::new_unique(),
            TokenProgramType::Token2022,
            10,
        );
        assert!(!token2022_only.is_mixed_pool());
        assert!(token2022_only.requires_token2022());

        let mixed = PoolConfig::new(
            Pubkey::default(),
            "Test",
            Pubkey::default(),
            TokenProgramType::Token,
            Pubkey::new_unique(),
            TokenProgramType::Token2022,
            10,
        );
        assert!(mixed.is_mixed_pool());
        assert!(mixed.requires_token2022());
    }

    #[test]
    fn test_operation_type() {
        assert!(OperationType::BuyExactIn.is_exact_in());
        assert!(OperationType::SellExactIn.is_exact_in());
        assert!(!OperationType::BuyExactIn.is_exact_out());
        assert!(OperationType::BuyExactIn.is_buy());
        assert!(!OperationType::BuyExactIn.is_sell());
    }
}
