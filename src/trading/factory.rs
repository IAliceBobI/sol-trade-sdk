use std::sync::Arc;

use crate::constants::DexProtocol;
use crate::instruction::{
    bonk::BonkInstructionBuilder, meteora_damm_v2::MeteoraDammV2InstructionBuilder,
    pumpfun::PumpFunInstructionBuilder, pumpswap::PumpSwapInstructionBuilder,
    raydium_amm_v4::RaydiumAmmV4InstructionBuilder, raydium_clmm::RaydiumClmmInstructionBuilder,
    raydium_cpmm::RaydiumCpmmInstructionBuilder,
};

use super::core::{executor::GenericTradeExecutor, traits::TradeExecutor};

/// 支持的交易协议
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DexType {
    PumpFun,
    PumpSwap,
    Bonk,
    RaydiumCpmm,
    RaydiumAmmV4,
    RaydiumClmm,
    MeteoraDammV2,
}

impl DexType {
    /// 获取协议名称（与 DexProtocol 一致）
    pub fn name(&self) -> &'static str {
        match self {
            DexType::PumpFun => "pumpfun",
            DexType::PumpSwap => "pumpswap",
            DexType::Bonk => "bonk",
            DexType::RaydiumCpmm => "raydium_cpmm",
            DexType::RaydiumAmmV4 => "raydium_amm_v4",
            DexType::RaydiumClmm => "raydium_clmm",
            DexType::MeteoraDammV2 => "meteora_damm_v2",
        }
    }
}

/// 从 DexProtocol 转换为 DexType
/// 注意：RaydiumLaunchlab 不支持交易，会返回 None
impl TryFrom<DexProtocol> for DexType {
    type Error = &'static str;

    fn try_from(protocol: DexProtocol) -> Result<Self, Self::Error> {
        match protocol {
            DexProtocol::PumpFun => Ok(DexType::PumpFun),
            DexProtocol::PumpSwap => Ok(DexType::PumpSwap),
            DexProtocol::Bonk => Ok(DexType::Bonk),
            DexProtocol::RaydiumAmmV4 => Ok(DexType::RaydiumAmmV4),
            DexProtocol::RaydiumClmm => Ok(DexType::RaydiumClmm),
            DexProtocol::RaydiumCpmm => Ok(DexType::RaydiumCpmm),
            DexProtocol::MeteoraDammV2 => Ok(DexType::MeteoraDammV2),
            DexProtocol::RaydiumLaunchlab => Err("RaydiumLaunchlab 不支持交易"),
        }
    }
}

/// 从字符串名称解析 DexType
impl TryFrom<&str> for DexType {
    type Error = String;

    fn try_from(name: &str) -> Result<Self, Self::Error> {
        DexProtocol::from_name(name)
            .ok_or_else(|| format!("未知的 DEX 名称: {}", name))
            .and_then(|p| DexType::try_from(p).map_err(|e| e.to_string()))
    }
}

impl std::fmt::Display for DexType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// 交易工厂 - 用于创建不同协议的交易执行器
pub struct TradeFactory;

impl TradeFactory {
    /// 创建指定协议的交易执行器（零开销单例）
    pub fn create_executor(dex_type: DexType) -> Arc<dyn TradeExecutor> {
        match dex_type {
            DexType::PumpFun => Self::pumpfun_executor(),
            DexType::PumpSwap => Self::pumpswap_executor(),
            DexType::Bonk => Self::bonk_executor(),
            DexType::RaydiumCpmm => Self::raydium_cpmm_executor(),
            DexType::RaydiumAmmV4 => Self::raydium_amm_v4_executor(),
            DexType::RaydiumClmm => Self::raydium_clmm_executor(),
            DexType::MeteoraDammV2 => Self::meteora_damm_v2_executor(),
        }
    }

    // Static instances created at compile time - zero runtime overhead
    #[inline]
    fn pumpfun_executor() -> Arc<dyn TradeExecutor> {
        static INSTANCE: std::sync::LazyLock<Arc<dyn TradeExecutor>> =
            std::sync::LazyLock::new(|| {
                let instruction_builder = Arc::new(PumpFunInstructionBuilder);
                Arc::new(GenericTradeExecutor::new(instruction_builder, "PumpFun"))
            });
        INSTANCE.clone()
    }

    #[inline]
    fn pumpswap_executor() -> Arc<dyn TradeExecutor> {
        static INSTANCE: std::sync::LazyLock<Arc<dyn TradeExecutor>> =
            std::sync::LazyLock::new(|| {
                let instruction_builder = Arc::new(PumpSwapInstructionBuilder);
                Arc::new(GenericTradeExecutor::new(instruction_builder, "PumpSwap"))
            });
        INSTANCE.clone()
    }

    #[inline]
    fn bonk_executor() -> Arc<dyn TradeExecutor> {
        static INSTANCE: std::sync::LazyLock<Arc<dyn TradeExecutor>> =
            std::sync::LazyLock::new(|| {
                let instruction_builder = Arc::new(BonkInstructionBuilder);
                Arc::new(GenericTradeExecutor::new(instruction_builder, "Bonk"))
            });
        INSTANCE.clone()
    }

    #[inline]
    fn raydium_cpmm_executor() -> Arc<dyn TradeExecutor> {
        static INSTANCE: std::sync::LazyLock<Arc<dyn TradeExecutor>> =
            std::sync::LazyLock::new(|| {
                let instruction_builder = Arc::new(RaydiumCpmmInstructionBuilder);
                Arc::new(GenericTradeExecutor::new(instruction_builder, "RaydiumCpmm"))
            });
        INSTANCE.clone()
    }

    #[inline]
    fn raydium_amm_v4_executor() -> Arc<dyn TradeExecutor> {
        static INSTANCE: std::sync::LazyLock<Arc<dyn TradeExecutor>> =
            std::sync::LazyLock::new(|| {
                let instruction_builder = Arc::new(RaydiumAmmV4InstructionBuilder);
                Arc::new(GenericTradeExecutor::new(instruction_builder, "RaydiumAmmV4"))
            });
        INSTANCE.clone()
    }

    #[inline]
    fn raydium_clmm_executor() -> Arc<dyn TradeExecutor> {
        static INSTANCE: std::sync::LazyLock<Arc<dyn TradeExecutor>> =
            std::sync::LazyLock::new(|| {
                let instruction_builder = Arc::new(RaydiumClmmInstructionBuilder);
                Arc::new(GenericTradeExecutor::new(instruction_builder, "RaydiumClmm"))
            });
        INSTANCE.clone()
    }

    #[inline]
    fn meteora_damm_v2_executor() -> Arc<dyn TradeExecutor> {
        static INSTANCE: std::sync::LazyLock<Arc<dyn TradeExecutor>> =
            std::sync::LazyLock::new(|| {
                let instruction_builder = Arc::new(MeteoraDammV2InstructionBuilder);
                Arc::new(GenericTradeExecutor::new(instruction_builder, "MeteoraDammV2"))
            });
        INSTANCE.clone()
    }
}
