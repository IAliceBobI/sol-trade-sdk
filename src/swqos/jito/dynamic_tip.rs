//! Jito 动态 Tip 调整功能
//!
//! 从 Jito Tip Floor API 获取实时 tip 数据，并根据配置自动计算最优 tip。

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::Instant;

/// Jito Tip Floor API 响应
#[derive(Debug, Clone, Deserialize)]
pub struct JitoTipFloorResponse {
    #[serde(default)]
    pub time: String,
    pub landed_tips_25th_percentile: f64,
    pub landed_tips_50th_percentile: f64,
    pub landed_tips_75th_percentile: f64,
    pub landed_tips_95th_percentile: f64,
    pub landed_tips_99th_percentile: f64,
    pub ema_landed_tips_50th_percentile: f64,
}

/// Tip 百分位选择
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipPercentile {
    P25,
    P50,
    P75,
    P95,
    P99,
}

impl TipPercentile {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "25th" => Ok(TipPercentile::P25),
            "50th" => Ok(TipPercentile::P50),
            "75th" => Ok(TipPercentile::P75),
            "95th" => Ok(TipPercentile::P95),
            "99th" => Ok(TipPercentile::P99),
            _ => anyhow::bail!("Invalid tip percentile: {}", s),
        }
    }
}

/// 动态 Tip 配置
#[derive(Debug, Clone)]
pub struct DynamicTipConfig {
    /// 是否启用动态 tip
    pub enabled: bool,
    /// Tip 百分位
    pub percentile: TipPercentile,
    /// Tip 倍数
    pub multiplier: f64,
    /// 最小 tip（SOL）
    pub min_tip: f64,
    /// 最大 tip（SOL）
    pub max_tip: f64,
}

impl Default for DynamicTipConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            percentile: TipPercentile::P50,
            multiplier: 1.0,
            min_tip: 0.00001,
            max_tip: 0.001,
        }
    }
}

/// Jito Tip Floor 客户端
pub struct JitoTipFloorClient {
    client: Client,
    endpoint: String,
}

impl JitoTipFloorClient {
    const DEFAULT_ENDPOINT: &'static str = "https://bundles.jito.wtf/api/v1/bundles/tip_floor";

    pub fn new() -> Self {
        let client = Client::builder().timeout(Duration::from_millis(2000)).build().unwrap();

        Self { client, endpoint: Self::DEFAULT_ENDPOINT.to_string() }
    }

    /// 使用代理创建客户端
    ///
    /// ## 参数
    /// - `proxy_url`: 代理地址（例如 "http://127.0.0.1:7891"）
    pub fn with_proxy(proxy_url: &str) -> Result<Self> {
        use reqwest::Proxy;
        let proxy = Proxy::all(proxy_url).context("Failed to create proxy")?;

        let client = Client::builder()
            .timeout(Duration::from_millis(2000))
            .proxy(proxy)
            .build()
            .context("Failed to build client")?;

        Ok(Self { client, endpoint: Self::DEFAULT_ENDPOINT.to_string() })
    }

    /// 从环境变量 PROXY_URL 创建客户端（如果设置）
    pub fn from_env_proxy() -> Self {
        use std::env;

        if let Ok(proxy_url) = env::var("PROXY_URL") {
            Self::with_proxy(&proxy_url).unwrap_or_else(|_| Self::new())
        } else {
            Self::new()
        }
    }

    /// 获取 Tip Floor 数据
    pub async fn get_tip_floor(&self) -> Result<JitoTipFloorResponse> {
        let start = Instant::now();

        let response = self
            .client
            .get(&self.endpoint)
            .send()
            .await
            .context("Failed to fetch tip floor")?;

        if !response.status().is_success() {
            anyhow::bail!("Tip floor API returned status: {}", response.status());
        }

        let text = response.text().await.context("Failed to read response")?;

        // API 返回的是数组，取第一个元素
        let tip_data_array: Vec<JitoTipFloorResponse> =
            serde_json::from_str(&text).context("Failed to parse tip floor response")?;

        if tip_data_array.is_empty() {
            anyhow::bail!("Tip floor API returned empty array");
        }

        let tip_data = tip_data_array[0].clone();

        println!(
            "[jito] Tip floor fetched in {:?}: 50th = {} SOL",
            start.elapsed(),
            tip_data.landed_tips_50th_percentile
        );

        Ok(tip_data)
    }

    /// 计算最优 tip
    pub fn calculate_tip(
        &self,
        tip_floor: &JitoTipFloorResponse,
        config: &DynamicTipConfig,
    ) -> f64 {
        // 根据配置选择百分位
        let base_tip = match config.percentile {
            TipPercentile::P25 => tip_floor.landed_tips_25th_percentile,
            TipPercentile::P50 => tip_floor.landed_tips_50th_percentile,
            TipPercentile::P75 => tip_floor.landed_tips_75th_percentile,
            TipPercentile::P95 => tip_floor.landed_tips_95th_percentile,
            TipPercentile::P99 => tip_floor.landed_tips_99th_percentile,
        };

        // 应用倍数
        let calculated_tip = base_tip * config.multiplier;

        // 限制在最小和最大值之间
        let final_tip = calculated_tip.clamp(config.min_tip, config.max_tip);

        println!(
            "[jito] Dynamic tip: base={} SOL, multiplier={}, final={} SOL",
            base_tip, config.multiplier, final_tip
        );

        final_tip
    }

    /// 自动获取并计算最优 tip
    pub async fn get_optimal_tip(&self, config: &DynamicTipConfig) -> Result<f64> {
        if !config.enabled {
            // 动态 tip 未启用，返回最小值
            return Ok(config.min_tip);
        }

        let tip_floor = self.get_tip_floor().await?;
        Ok(self.calculate_tip(&tip_floor, config))
    }
}

impl Default for JitoTipFloorClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_parsing() {
        assert_eq!(TipPercentile::from_str("50th").unwrap(), TipPercentile::P50);
        assert_eq!(TipPercentile::from_str("75th").unwrap(), TipPercentile::P75);
        assert!(TipPercentile::from_str("invalid").is_err());
    }

    #[test]
    fn test_tip_clamping() {
        let config = DynamicTipConfig {
            enabled: true,
            percentile: TipPercentile::P50,
            multiplier: 1.0,
            min_tip: 0.00001,
            max_tip: 0.001,
        };

        let client = JitoTipFloorClient::new();

        // 测试最小值
        let tip_floor = JitoTipFloorResponse {
            time: "2024-01-01".to_string(),
            landed_tips_25th_percentile: 0.000001,
            landed_tips_50th_percentile: 0.000001,
            landed_tips_75th_percentile: 0.000001,
            landed_tips_95th_percentile: 0.000001,
            landed_tips_99th_percentile: 0.000001,
            ema_landed_tips_50th_percentile: 0.000001,
        };

        let tip = client.calculate_tip(&tip_floor, &config);
        assert_eq!(tip, 0.00001); // 应该返回最小值

        // 测试最大值
        let tip_floor = JitoTipFloorResponse {
            time: "2024-01-01".to_string(),
            landed_tips_25th_percentile: 0.01,
            landed_tips_50th_percentile: 0.01,
            landed_tips_75th_percentile: 0.01,
            landed_tips_95th_percentile: 0.01,
            landed_tips_99th_percentile: 0.01,
            ema_landed_tips_50th_percentile: 0.01,
        };

        let tip = client.calculate_tip(&tip_floor, &config);
        assert_eq!(tip, 0.001); // 应该返回最大值
    }
}
