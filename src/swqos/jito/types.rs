//! Jito 区域类型定义
//!
//! 根据 Jito 官方文档：https://docs.jito.wtf/lowlatencytxnsend/

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

/// Jito 区域枚举
///
/// Jito 在全球多个地区部署了 Block Engine，选择最近的区域可以显著降低延迟。
///
/// # 支持的区域
///
/// | 区域 | 位置 | 推荐用户 |
/// |------|------|----------|
/// | `Default` | 默认 | 大多数用户 |
/// | `Amsterdam` 🇳🇱 | 荷兰阿姆斯特丹 | 欧洲用户 |
/// | `Dublin` 🇮🇪 | 爱尔兰都柏林 | 欧洲用户 |
/// | `Frankfurt` 🇩🇪 | 德国法兰克福 | 欧洲用户 |
/// | `London` 🇬🇧 | 英国伦敦 | 欧洲用户 |
/// | `NewYork` 🇺🇸 | 美国纽约 | 美国东海岸用户 |
/// | `SLC` 🇺🇸 | 美国盐湖城 | 美国西海岸用户 |
/// | `Singapore` 🇸🇬 | 新加坡 | 亚洲用户 |
/// | `Tokyo` 🇯🇵 | 日本东京 | 亚洲用户 |
///
/// # 示例
///
/// ```rust
/// use sol_trade_sdk::swqos::jito::types::JitoRegion;
///
/// // 使用默认区域
/// let region = JitoRegion::Default;
/// println!("Endpoint: {}", region.endpoint());
///
/// // 亚洲用户使用东京区域
/// let region = JitoRegion::Tokyo;
/// println!("Endpoint: {}", region.endpoint());
///
/// // 从字符串解析
/// let region = JitoRegion::from_str("tokyo").unwrap();
/// assert_eq!(region, JitoRegion::Tokyo);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JitoRegion {
    /// 默认区域（推荐大多数用户）
    Default,

    /// 荷兰阿姆斯特丹
    Amsterdam,

    /// 爱尔兰都柏林
    Dublin,

    /// 德国法兰克福
    Frankfurt,

    /// 英国伦敦
    London,

    /// 美国纽约
    NewYork,

    /// 美国盐湖城
    SLC,

    /// 新加坡
    Singapore,

    /// 日本东京
    Tokyo,
}

impl JitoRegion {
    /// 获取该区域的 Block Engine endpoint URL
    ///
    /// # 示例
    ///
    /// ```rust
    /// use sol_trade_sdk::swqos::jito::types::JitoRegion;
    ///
    /// assert_eq!(
    ///     JitoRegion::Tokyo.endpoint(),
    ///     "https://tokyo.mainnet.block-engine.jito.wtf"
    /// );
    /// ```
    pub fn endpoint(&self) -> &'static str {
        match self {
            JitoRegion::Default => "https://mainnet.block-engine.jito.wtf",
            JitoRegion::Amsterdam => "https://amsterdam.mainnet.block-engine.jito.wtf",
            JitoRegion::Dublin => "https://dublin.mainnet.block-engine.jito.wtf",
            JitoRegion::Frankfurt => "https://frankfurt.mainnet.block-engine.jito.wtf",
            JitoRegion::London => "https://london.mainnet.block-engine.jito.wtf",
            JitoRegion::NewYork => "https://ny.mainnet.block-engine.jito.wtf",
            JitoRegion::SLC => "https://slc.mainnet.block-engine.jito.wtf",
            JitoRegion::Singapore => "https://singapore.mainnet.block-engine.jito.wtf",
            JitoRegion::Tokyo => "https://tokyo.mainnet.block-engine.jito.wtf",
        }
    }

    /// 从字符串解析区域
    ///
    /// 支持多种格式：大小写不敏感，支持常见简称
    ///
    /// # 示例
    ///
    /// ```rust
    /// use sol_trade_sdk::swqos::jito::types::JitoRegion;
    ///
    /// assert_eq!(JitoRegion::from_str("tokyo").unwrap(), JitoRegion::Tokyo);
    /// assert_eq!(JitoRegion::from_str("TOKYO").unwrap(), JitoRegion::Tokyo);
    /// assert_eq!(JitoRegion::from_str("ny").unwrap(), JitoRegion::NewYork);
    /// assert_eq!(JitoRegion::from_str("newyork").unwrap(), JitoRegion::NewYork);
    /// ```
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "default" => Ok(JitoRegion::Default),
            "amsterdam" | "ams" => Ok(JitoRegion::Amsterdam),
            "dublin" | "dub" => Ok(JitoRegion::Dublin),
            "frankfurt" | "fra" | "ffm" => Ok(JitoRegion::Frankfurt),
            "london" | "lon" => Ok(JitoRegion::London),
            "newyork" | "ny" => Ok(JitoRegion::NewYork),
            "slc" | "saltlakecity" => Ok(JitoRegion::SLC),
            "singapore" | "sgp" | "sg" => Ok(JitoRegion::Singapore),
            "tokyo" | "tyo" => Ok(JitoRegion::Tokyo),
            _ => Err(format!("Unknown Jito region: {}", s)),
        }
    }

    /// 获取所有支持的区域列表
    ///
    /// # 示例
    ///
    /// ```rust
    /// use sol_trade_sdk::swqos::jito::types::JitoRegion;
    ///
    /// let regions = JitoRegion::all_regions();
    /// assert_eq!(regions.len(), 9);
    /// ```
    pub fn all_regions() -> &'static [JitoRegion] {
        &[
            JitoRegion::Default,
            JitoRegion::Amsterdam,
            JitoRegion::Dublin,
            JitoRegion::Frankfurt,
            JitoRegion::London,
            JitoRegion::NewYork,
            JitoRegion::SLC,
            JitoRegion::Singapore,
            JitoRegion::Tokyo,
        ]
    }
}

impl Display for JitoRegion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            JitoRegion::Default => "Default",
            JitoRegion::Amsterdam => "Amsterdam",
            JitoRegion::Dublin => "Dublin",
            JitoRegion::Frankfurt => "Frankfurt",
            JitoRegion::London => "London",
            JitoRegion::NewYork => "NewYork",
            JitoRegion::SLC => "SLC",
            JitoRegion::Singapore => "Singapore",
            JitoRegion::Tokyo => "Tokyo",
        };
        write!(f, "{}", name)
    }
}

impl Default for JitoRegion {
    fn default() -> Self {
        JitoRegion::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_endpoints() {
        let test_cases = vec![
            (JitoRegion::Default, "https://mainnet.block-engine.jito.wtf"),
            (JitoRegion::Amsterdam, "https://amsterdam.mainnet.block-engine.jito.wtf"),
            (JitoRegion::Dublin, "https://dublin.mainnet.block-engine.jito.wtf"),
            (JitoRegion::Frankfurt, "https://frankfurt.mainnet.block-engine.jito.wtf"),
            (JitoRegion::London, "https://london.mainnet.block-engine.jito.wtf"),
            (JitoRegion::NewYork, "https://ny.mainnet.block-engine.jito.wtf"),
            (JitoRegion::SLC, "https://slc.mainnet.block-engine.jito.wtf"),
            (JitoRegion::Singapore, "https://singapore.mainnet.block-engine.jito.wtf"),
            (JitoRegion::Tokyo, "https://tokyo.mainnet.block-engine.jito.wtf"),
        ];

        for (region, expected) in test_cases {
            assert_eq!(region.endpoint(), expected);
        }
    }

    #[test]
    fn test_from_str() {
        assert_eq!(JitoRegion::from_str("tokyo").unwrap(), JitoRegion::Tokyo);
        assert_eq!(JitoRegion::from_str("TOKYO").unwrap(), JitoRegion::Tokyo);
        assert_eq!(JitoRegion::from_str("ny").unwrap(), JitoRegion::NewYork);
        assert_eq!(JitoRegion::from_str("newyork").unwrap(), JitoRegion::NewYork);
        assert_eq!(JitoRegion::from_str("singapore").unwrap(), JitoRegion::Singapore);
        assert_eq!(JitoRegion::from_str("sg").unwrap(), JitoRegion::Singapore);

        assert!(JitoRegion::from_str("invalid").is_err());
        assert!(JitoRegion::from_str("losangeles").is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(JitoRegion::Tokyo.to_string(), "Tokyo");
        assert_eq!(JitoRegion::NewYork.to_string(), "NewYork");
        assert_eq!(JitoRegion::Singapore.to_string(), "Singapore");
    }

    #[test]
    fn test_all_regions() {
        let regions = JitoRegion::all_regions();
        assert_eq!(regions.len(), 9);
        assert!(regions.contains(&JitoRegion::Default));
        assert!(regions.contains(&JitoRegion::Tokyo));
        assert!(regions.contains(&JitoRegion::Singapore));
    }
}
