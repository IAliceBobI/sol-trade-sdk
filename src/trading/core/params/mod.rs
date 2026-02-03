//! 交易参数模块
//!
//! 此模块包含所有交易参数类型的定义，按协议类型组织。

mod bonk_params;
mod meteora_params;
mod pump_params;
mod pumpswap_params;
mod raydium_params;
mod swap_params;

// 导出所有参数类型
pub use bonk_params::BonkParams;
pub use meteora_params::MeteoraDammV2Params;
pub use pump_params::PumpFunParams;
pub use pumpswap_params::PumpSwapParams;
pub use raydium_params::{RaydiumAmmV4Params, RaydiumClmmParams, RaydiumCpmmParams};
pub use swap_params::SwapParams;

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
