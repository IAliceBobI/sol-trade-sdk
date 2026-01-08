# Raydium API 功能分析文档

## 概述

Raydium SDK V2 提供了一个完整的 API 客户端，用于访问 Raydium 协议的各种数据。本文档详细分析了 API 类提供的所有功能。

## API 类结构

```typescript
export class Api {
  public cluster: Cluster;
  public api: AxiosInstance;
  public logCount: number;
  public urlConfigs: API_URL_CONFIG;
}
```

### 构造函数参数

```typescript
interface ApiProps {
  cluster: Cluster;              // 集群类型：'mainnet' | 'devnet'
  timeout: number;               // 请求超时时间（毫秒）
  logRequests?: boolean;         // 是否记录请求日志
  logCount?: number;             // 日志记录数量
  urlConfigs?: API_URL_CONFIG;   // 自定义 URL 配置
}
```

### 使用示例

```typescript
const api = new Api({
  cluster: 'mainnet',
  timeout: 10000,
  logRequests: true,
  logCount: 1000
});
```

## 功能分类

### 1. 配置管理

#### 1.1 获取 CLMM 配置

**方法**：`getClmmConfigs()`

**返回类型**：`Promise<ApiClmmConfigInfo[]>`

**说明**：获取集中流动性做市商（CLMM）的配置信息，包括价格范围、费用层级等。

**使用示例**：
```typescript
const configs = await api.getClmmConfigs();
console.log(configs);
// 输出：[{ id: '...', baseFee: 0.0001, ... }]
```

**注意事项**：
- CLMM 是 Raydium 的高级流动性管理功能
- 配置信息用于计算价格和流动性

#### 1.2 获取 CPMM 配置

**方法**：`getCpmmConfigs()`

**返回类型**：`Promise<ApiCpmmConfigInfo[]>`

**说明**：获取恒定乘积做市商（CPMM）的配置信息，包括费用范围、最小流动性等。

**使用示例**：
```typescript
const configs = await api.getCpmmConfigs();
console.log(configs);
```

#### 1.3 获取 Launch 配置

**方法**：`fetchLaunchConfigs()`

**返回类型**：`Promise<ApiLaunchConfig[]>`

**说明**：获取 Raydium Launch 平台的配置信息，包括代币创建参数、费用结构等。

**使用示例**：
```typescript
const configs = await api.fetchLaunchConfigs();
console.log(configs);
```

**注意事项**：
- 此方法使用缓存，首次调用后会缓存结果
- 缓存按集群（mainnet/devnet）分别存储

### 2. Pool 查询

#### 2.1 获取池列表

**方法**：`getPoolList(props?: FetchPoolParams)`

**返回类型**：`Promise<PoolsApiReturn>`

**参数**：
```typescript
interface FetchPoolParams {
  type?: 'all' | 'standard' | 'concentrated' | 'stable';  // 池类型
  sort?: 'liquidity' | 'volume24h' | 'volume7d' | 'volume30d' | 'fee24h' | 'fee7d' | 'fee30d' | 'apr24h' | 'apr7d' | 'apr30d';  // 排序字段
  order?: 'desc' | 'asc';  // 排序方向
  page?: number;  // 页码
  pageSize?: number;  // 每页数量
}
```

**说明**：获取 Raydium 池列表，支持分页、排序和类型过滤。

**使用示例**：
```typescript
// 获取流动性最高的 100 个池
const pools = await api.getPoolList({
  type: 'all',
  sort: 'liquidity',
  order: 'desc',
  page: 0,
  pageSize: 100
});
```

#### 2.2 通过 ID 获取池信息

**方法**：`fetchPoolById(props: { ids: string })`

**返回类型**：`Promise<ApiV3PoolInfoItem[]>`

**说明**：根据池 ID 获取池的详细信息。

**使用示例**：
```typescript
const poolInfo = await api.fetchPoolById({
  ids: '58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2'
});
```

#### 2.3 通过 ID 获取池密钥

**方法**：`fetchPoolKeysById(props: { idList: string[] })`

**返回类型**：`Promise<PoolKeys[]>`

**说明**：根据池 ID 获取池的所有账户密钥，用于构建交易。

**注意事项**：
- 此方法使用缓存，已查询的池会缓存结果
- 缓存键为池 ID，值为池密钥对象

**使用示例**：
```typescript
const poolKeys = await api.fetchPoolKeysById({
  idList: ['pool-id-1', 'pool-id-2']
});
```

#### 2.4 通过 Mint 地址查找池

**方法**：`fetchPoolByMints(props)`

**返回类型**：`Promise<PoolsApiReturn>`

**参数**：
```typescript
{
  mint1: string | PublicKey;  // 第一个代币地址
  mint2?: string | PublicKey;  // 第二个代币地址（可选）
  type?: PoolFetchType;       // 池类型
  sort?: string;              // 排序字段
  order?: 'desc' | 'asc';     // 排序方向
  page?: number;              // 页码
}
```

**说明**：根据代币地址查找池。如果只提供一个 mint，会返回所有包含该 mint 的池。

**注意事项**：
- 自动将 SOL 转换为 WSOL
- 自动排序 mint 地址（baseMint < quoteMint）

**使用示例**：
```typescript
// 查找 SOL-USDC 池
const pools = await api.fetchPoolByMints({
  mint1: 'So11111111111111111111111111111111111111112',  // WSOL
  mint2: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',  // USDC
  type: 'all',
  sort: 'liquidity',
  order: 'desc'
});
```

#### 2.5 获取 CLMM 池流动性曲线

**方法**：`getClmmPoolLines(poolId: string)`

**返回类型**：`Promise<{ price: string; liquidity: string }[]>`

**说明**：获取 CLMM 池的流动性曲线，表示不同价格区间内的流动性分布。

**使用示例**：
```typescript
const liquidityLines = await api.getClmmPoolLines('pool-id');
console.log(liquidityLines);
// 输出：[{ price: '1.23', liquidity: '1000000' }, ...]
```

### 3. Token 信息

#### 3.1 获取代币列表

**方法**：`getTokenList()`

**返回类型**：`Promise<{ mintList: ApiV3Token[]; blacklist: string[]; whiteList: string[] }>`

**说明**：获取 Raydium 支持的代币列表，包括白名单和黑名单。

**使用示例**：
```typescript
const tokens = await api.getTokenList();
console.log(tokens.mintList);    // 所有支持的代币
console.log(tokens.whiteList);   // 白名单代币
console.log(tokens.blacklist);   // 黑名单代币
```

#### 3.2 获取 Jupiter 代币列表

**方法**：`getJupTokenList()`

**返回类型**：`Promise<(ApiV3Token & { ... })[]>`

**说明**：获取 Jupiter DEX 支持的代币列表，比 Raydium 列表更全面。

**使用示例**：
```typescript
const jupTokens = await api.getJupTokenList();
```

#### 3.3 批量获取代币信息

**方法**：`getTokenInfo(mint: (string | PublicKey)[])`

**返回类型**：`Promise<ApiV3Token[]>`

**说明**：根据 mint 地址批量获取代币的详细信息。

**使用示例**：
```typescript
const tokenInfo = await api.getTokenInfo([
  'So11111111111111111111111111111111111111112',  // WSOL
  'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'   // USDC
]);
```

### 4. Farm 挖矿

#### 4.1 获取 Farm 信息

**方法**：`fetchFarmInfoById(props: { ids: string })`

**返回类型**：`Promise<FormatFarmInfoOut[]>`

**说明**：根据 ID 获取 Farm 池的信息，包括奖励代币、APR 等。

**使用示例**：
```typescript
const farmInfo = await api.fetchFarmInfoById({
  ids: 'farm-id'
});
```

#### 4.2 获取 Farm 密钥

**方法**：`fetchFarmKeysById(props: { ids: string })`

**返回类型**：`Promise<FormatFarmKeyOut[]>`

**说明**：根据 ID 获取 Farm 池的所有账户密钥，用于参与流动性挖矿。

**使用示例**：
```typescript
const farmKeys = await api.fetchFarmKeysById({
  ids: 'farm-id'
});
```

### 5. 系统状态

#### 5.1 获取网络 TPS

**方法**：`getBlockSlotCountForSecond(endpointUrl?: string)`

**返回类型**：`Promise<number>`

**说明**：获取 Solana 网络的每秒交易数（TPS）。

**使用示例**：
```typescript
const tps = await api.getBlockSlotCountForSecond('https://api.mainnet-beta.solana.com');
console.log(`Network TPS: ${tps}`);
```

#### 5.2 获取链时间偏移

**方法**：`getChainTimeOffset()`

**返回类型**：`Promise<{ offset: number }>`

**说明**：获取链时间与本地时间的偏移量，用于时间同步。

**使用示例**：
```typescript
const { offset } = await api.getChainTimeOffset();
console.log(`Time offset: ${offset}ms`);
```

#### 5.3 获取 RPC 列表

**方法**：`getRpcs()`

**返回类型**：`Promise<{ rpcs: { ... }[]; strategy: string }>`

**说明**：获取推荐的 RPC 节点列表，包括权重和负载均衡策略。

**使用示例**：
```typescript
const { rpcs, strategy } = await api.getRpcs();
console.log(rpcs);  // RPC 节点列表
console.log(strategy);  // 负载均衡策略
```

#### 5.4 检查 API 可用性

**方法**：`fetchAvailabilityStatus()`

**返回类型**：`Promise<AvailabilityCheckAPI3>`

**说明**：检查 Raydium API 的可用性状态。

**使用示例**：
```typescript
const status = await api.fetchAvailabilityStatus();
console.log(status);
```

## 缓存机制

### Pool Keys 缓存

- **缓存位置**：`poolKeysCache: Map<string, PoolKeys>`
- **缓存键**：池 ID
- **缓存值**：池密钥对象
- **使用方法**：`fetchPoolKeysById`

### Launch Configs 缓存

- **缓存位置**：`cacheLaunchConfigs: Map<Cluster, ApiLaunchConfig[]>`
- **缓存键**：集群类型（mainnet/devnet）
- **缓存值**：Launch 配置数组
- **使用方法**：`fetchLaunchConfigs`

## 错误处理

### 无限重试机制

API 类提供了一个 `endlessRetry` 工具函数，用于在请求失败时自动重试：

```typescript
async function endlessRetry<T>(
  name: string,
  call: () => Promise<T>,
  interval = 1000
): Promise<T>
```

**使用示例**：
```typescript
const result = await endlessRetry('getPoolList', async () => {
  return await api.getPoolList();
});
```

## 请求日志

### 日志记录

通过设置 `logRequests: true`，可以记录所有 API 请求的详细信息：

```typescript
const api = new Api({
  cluster: 'mainnet',
  timeout: 10000,
  logRequests: true,
  logCount: 1000  // 只记录最近 1000 条请求
});
```

### 日志内容

每条日志包含：
- 请求状态码
- 请求 URL
- 请求参数
- 响应数据或错误信息

## 最佳实践

### 1. 使用池密钥缓存

```typescript
// 第一次调用会从 API 获取并缓存
const poolKeys1 = await api.fetchPoolKeysById({
  idList: ['pool-id']
});

// 第二次调用会从缓存返回
const poolKeys2 = await api.fetchPoolKeysById({
  idList: ['pool-id']
});
```

### 2. 自动处理 WSOL 转换

```typescript
// API 会自动将 SOL 转换为 WSOL
const pools = await api.fetchPoolByMints({
  mint1: 'So11111111111111111111111111111111111111112',  // SOL
  mint2: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v'
});
```

### 3. 批量查询优化

```typescript
// 批量获取多个池的信息
const poolKeys = await api.fetchPoolKeysById({
  idList: ['pool-1', 'pool-2', 'pool-3', 'pool-4', 'pool-5']
});
```

### 4. 分页查询大量数据

```typescript
// 分页获取所有池
let page = 0;
let allPools = [];

while (true) {
  const result = await api.getPoolList({
    page,
    pageSize: 100
  });

  allPools = allPools.concat(result.data);

  if (result.data.length < 100) break;
  page++;
}
```

## 注意事项

1. **速率限制**：Raydium API 可能有速率限制，建议适当控制请求频率
2. **错误处理**：使用 try-catch 捕获 API 错误，并进行适当的重试
3. **缓存失效**：池密钥缓存不会自动失效，如有需要可以手动清除
4. **网络代理**：如果你的网络需要代理，请确保代理配置正确
5. **超时设置**：根据网络情况调整超时时间，避免请求长时间挂起

## 总结

Raydium SDK V2 的 API 类提供了全面的功能，包括：
- 5 大类功能（配置管理、Pool 查询、Token 信息、Farm 挖矿、系统状态）
- 17 个公共方法
- 完善的缓存机制
- 请求日志记录
- 错误处理和重试机制

这些功能为开发者提供了强大的工具，可以轻松地与 Raydium 协议交互。