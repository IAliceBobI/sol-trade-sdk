//! 交易参数模块
//!
//! 此模块包含所有交易参数类型的定义，按协议类型组织。

mod swap_params;
mod pump_params;
mod pumpswap_params;
mod bonk_params;
mod raydium_params;
mod meteora_params;

// 导出所有参数类型
pub use swap_params::SwapParams;
pub use pump_params::PumpFunParams;
pub use pumpswap_params::PumpSwapParams;
pub use bonk_params::BonkParams;
pub use raydium_params::{RaydiumCpmmParams, RaydiumAmmV4Params, RaydiumClmmParams};
pub use meteora_params::MeteoraDammV2Params;

/// DEX 参数枚举 - 零开销抽象替代 Box<dyn ProtocolParams>
#[derive(Clone)]
pub enum DexParamEnum {
    PumpFun(PumpFunParams),
    PumpSwap(PumpSwapParams),
    Bonk(BonkParams),
    RaydiumCpmm(RaydiumCpmmParams),
    RaydiumAmmV4(RaydiumAmmV4Params),
    RaydiumClmm(RaydiumClmmParams),
    MeteoraDammV2(MeteoraDammV2Params),
}

impl DexParamEnum {
    /// 获取内部参数的 Any 引用，用于向后兼容的类型检查
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        match self {
            DexParamEnum::PumpFun(p) => p,
            DexParamEnum::PumpSwap(p) => p,
            DexParamEnum::Bonk(p) => p,
            DexParamEnum::RaydiumCpmm(p) => p,
            DexParamEnum::RaydiumAmmV4(p) => p,
            DexParamEnum::RaydiumClmm(p) => p,
            DexParamEnum::MeteoraDammV2(p) => p,
        }
    }
}
