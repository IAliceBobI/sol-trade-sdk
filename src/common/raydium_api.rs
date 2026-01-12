use anyhow::Result;
use reqwest::{Client, Proxy};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, time::Duration};

use crate::constants::tokens::SOL_MINT;

/// Raydium API 客户端配置
#[derive(Debug, Clone)]
pub struct RaydiumApiConfig {
    /// 基础地址，例如 `https://api-v3.raydium.io`
    pub base_host: String,
    /// 请求超时时间（毫秒）
    pub timeout_millis: u64,
}

impl Default for RaydiumApiConfig {
    fn default() -> Self {
        Self {
            base_host: "https://api-v3.raydium.io".to_string(),
            timeout_millis: 10_000,
        }
    }
}

/// Raydium HTTP API 客户端（仅负责 REST 查询，不参与交易构建）
#[derive(Clone)]
pub struct RaydiumApiClient {
    http: Client,
    pub config: RaydiumApiConfig,
}

impl RaydiumApiClient {
    /// 使用给定配置创建新的 Raydium API 客户端
    pub fn new(config: RaydiumApiConfig) -> Result<Self> {
        let timeout = Duration::from_millis(config.timeout_millis);
        let mut builder = Client::builder()
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(64)
            .tcp_nodelay(true)
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(5));

        // 优先使用 HTTPS_PROXY，其次 HTTP_PROXY（从 .env 或进程环境变量读取）
        if let Ok(https_proxy) = env::var("HTTPS_PROXY").or_else(|_| env::var("https_proxy")) {
            builder = builder.proxy(Proxy::https(&https_proxy)?);
        } else if let Ok(http_proxy) = env::var("HTTP_PROXY").or_else(|_| env::var("http_proxy")) {
            builder = builder.proxy(Proxy::http(&http_proxy)?);
        }

        let http = builder.build()?;

        Ok(Self { http, config })
    }

    /// 使用默认配置（主网 BASE_HOST + 10s 超时）创建客户端
    pub fn mainnet_default() -> Result<Self> {
        Self::new(RaydiumApiConfig::default())
    }

    #[inline]
    fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.config.base_host.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    // ===================== 3.1 获取 Pool 列表 =====================

    /// 对应文档 3.1：`/pools/info/list` → `api.getPoolList(props)`
    pub async fn get_pool_list(&self, params: &GetPoolListRequest) -> Result<PoolListPage> {
        let url = self.endpoint("/pools/info/list");
        let resp = self
            .http
            .get(url)
            .query(params)
            .send()
            .await?
            .error_for_status()?;

        let api_resp = resp.json::<ApiResponse<PoolListPage>>().await?;
        Ok(api_resp.data)
    }

    // ===================== 3.2 根据 Mint 查询 Pool =====================

    /// 对应文档 3.2：`/pools/info/mint` → `api.fetchPoolByMints(props)`
    pub async fn fetch_pools_by_mints(
        &self,
        params: &FetchPoolsByMintsRequest,
    ) -> Result<PoolListPage> {
        // 对标 raydium-sdk-V2：
        // - 将 SOL 视为 WSOL（这里通过 normalize_mint_address 处理）
        // - 对 mint1/mint2 做排序，生成 baseMint/quoteMint，确保查询稳定
        let mut mint1 = normalize_mint_address(&params.mint1);
        let mut mint2_opt = params
            .mint2
            .as_ref()
            .map(|m| normalize_mint_address(m));

        if let Some(ref mut mint2) = mint2_opt {
            if !mint2.is_empty() && mint1 > *mint2 {
                std::mem::swap(&mut mint1, mint2);
            }
        }

        let effective = FetchPoolsByMintsRequest {
            mint1,
            mint2: mint2_opt,
            r#type: params.r#type.clone(),
            sort: params.sort.clone(),
            order: params.order.clone(),
            page: params.page,
        };

        let url = self.endpoint("/pools/info/mint");
        let resp = self
            .http
            .get(url)
            .query(&effective)
            .send()
            .await?
            .error_for_status()?;

        let api_resp = resp.json::<ApiResponse<PoolListPage>>().await?;
        Ok(api_resp.data)
    }

    // ===================== 3.3 根据 Pool ID 查询 Pool =====================

    /// 对应文档 3.3：`/pools/info/ids` → `api.fetchPoolById(props)`
    ///
    /// 注意：
    /// - `ids` 为多个 pool id 的列表，内部会自动拼接为逗号分隔字符串
    pub async fn fetch_pools_by_ids(&self, ids: &[String]) -> Result<Vec<Value>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let ids_param = ids.join(",");
        let url = self.endpoint("/pools/info/ids");
        let resp = self
            .http
            .get(url)
            .query(&[("ids", ids_param)])
            .send()
            .await?
            .error_for_status()?;

        let pools = resp.json::<Vec<Value>>().await?;
        Ok(pools)
    }

    // ===================== 3.4 获取 Pool Keys（用于构建交易） =====================

    /// 对应文档 3.4：`/pools/key/ids` → `api.fetchPoolKeysById(props)`
    ///
    /// 这里使用 GET 方式：`?ids=idList.join(',')`，与官方 SDK 行为保持一致。
    pub async fn fetch_pool_keys_by_ids(&self, id_list: &[String]) -> Result<Vec<Value>> {
        if id_list.is_empty() {
            return Ok(Vec::new());
        }

        let url = self.endpoint("/pools/key/ids");
        let ids_param = id_list.join(",");
        let resp = self
            .http
            .get(url)
            .query(&[("ids", ids_param)])
            .send()
            .await?
            .error_for_status()?;

        let keys = resp.json::<Vec<Value>>().await?;
        Ok(keys)
    }

    // ===================== 3.5 获取 CLMM Pool 流动性曲线 =====================

    /// 对应文档 3.5：`/pools/line/liquidity` → `api.getClmmPoolLines(poolId)`
    pub async fn get_clmm_pool_liquidity_lines(
        &self,
        pool_id: &str,
    ) -> Result<Vec<ClmmLiquidityPoint>> {
        let url = self.endpoint("/pools/line/liquidity");
        let resp = self
            .http
            .get(url)
            .query(&[("id", pool_id)])
            .send()
            .await?
            .error_for_status()?;

        let lines = resp.json::<Vec<ClmmLiquidityPoint>>().await?;
        Ok(lines)
    }
}

// ===================== 请求/响应类型定义 =====================

/// Pool 类型枚举，对应文档中的 `PoolFetchType`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PoolFetchType {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "standard")]
    Standard,
    #[serde(rename = "concentrated")]
    Concentrated,
    #[serde(rename = "allFarm")]
    AllFarm,
    #[serde(rename = "standardFarm")]
    StandardFarm,
    #[serde(rename = "concentratedFarm")]
    ConcentratedFarm,
}

/// 排序方向
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// 3.1 获取 Pool 列表 请求参数
#[derive(Debug, Clone, Serialize, Default)]
pub struct GetPoolListRequest {
    /// Pool 类型
    #[serde(rename = "poolType", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<PoolFetchType>,
    /// 排序字段，如 `liquidity` / `volume24h` 等
    #[serde(rename = "poolSortField", skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// 排序方向 `asc` / `desc`
    #[serde(rename = "sortType", skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
    /// 每页数量（默认 100）
    #[serde(rename = "pageSize", skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
    /// 页码（默认 0）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

/// 3.2 根据 Mint 查询 Pool 请求参数
#[derive(Debug, Clone, Serialize)]
pub struct FetchPoolsByMintsRequest {
    /// 第一个 Mint（必填），字符串形式的地址
    #[serde(rename = "mint1")]
    pub mint1: String,
    /// 第二个 Mint（可选）
    #[serde(rename = "mint2", skip_serializing_if = "Option::is_none")]
    pub mint2: Option<String>,
    /// Pool 类型
    #[serde(rename = "poolType", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<PoolFetchType>,
    /// 排序字段
    #[serde(rename = "poolSortField", skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// 排序方向
    #[serde(rename = "sortType", skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
    /// 页码
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}


/// 通用的 Raydium API 响应外层结构
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct ApiResponse<T> {
    pub id: String,
    pub success: bool,
    pub data: T,
}

/// 通用的 Pool 列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct PoolListPage {
    pub count: u64,
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
    /// 由于不同协议返回字段差异较大，这里保留原始 JSON，交由上层按需解析
    pub data: Vec<Value>,
}

/// 3.5 CLMM Pool 流动性曲线中的一个点
#[derive(Debug, Clone, Deserialize)]
pub struct ClmmLiquidityPoint {
    pub price: String,
    pub liquidity: String,
}

fn normalize_mint_address(input: &str) -> String {
    match input {
        // 对标 Raydium TS SDK 中的 solToWSol：如果外部显式传入 "SOL"/"sol"，统一映射为 WSOL mint
        "SOL" | "sol" => SOL_MINT.to_string(),
        _ => input.to_string(),
    }
}
