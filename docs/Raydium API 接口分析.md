# Raydium API 接口分析

**分析日期**：2026-01-09  
**分析范围**：raydium-sdk-V2-demo、raydium-sdk-V2  
**数据来源**：
- `/opt/projects/sol-trade-sdk/temp/raydium-sdk-V2-demo`
- `/opt/projects/sol-trade-sdk/temp/raydium-sdk-V2`

---

## 概述

本文档详细分析了 Raydium SDK V2 的 API 接口，包括 Pool 查询、Swap 交易等功能，涵盖 AMM V4、CPMM、CLMM 三种 DEX 协议。

---

## 1. 支持的 DEX 协议

| 协议 | 程序 ID | 说明 |
|------|---------|------|
| **AMM V4** | `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8` | 标准 AMM（自动做市商） |
| **AMM Stable** | `5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h` | 稳定币 AMM |
| **CPMM** | `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C` | 常数乘积做市商 |
| **CLMM** | `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK` | 集中流动性做市商 |

---

## 2. API 基础信息

### 2.1 API 基础 URL

```typescript
// 主网
BASE_HOST: "https://api-v3.raydium.io"

// Devnet
BASE_HOST: "https://api-v3-devnet.raydium.io"
```

### 2.2 API 类初始化

```typescript
import { Api } from '@raydium-io/raydium-sdk-v2'

const api = new Api({
  cluster: 'mainnet',  // 'mainnet' | 'devnet'
  timeout: 10000,      // 请求超时时间（毫秒）
  logRequests: false,  // 是否记录请求日志
  logCount: 1000,      // 日志记录数量
  urlConfigs: {        // 自定义 URL 配置
    BASE_HOST: 'https://api-v3.raydium.io',
  },
})
```

---

## 3. Pool 查询接口

### 3.1 获取 Pool 列表

**接口**：`/pools/info/list`

**方法**：`api.getPoolList(props)`

**参数**：
```typescript
interface FetchPoolParams {
  type?: PoolFetchType;      // Pool 类型
  sort?: string;             // 排序字段
  order?: 'desc' | 'asc';    // 排序方向
  pageSize?: number;         // 每页数量（默认 100）
  page?: number;             // 页码（默认 0）
}

enum PoolFetchType {
  All = "all",                    // 所有 Pool
  Standard = "standard",          // 标准 Pool（AMM V4、CPMM）
  Concentrated = "concentrated",  // 集中流动性 Pool（CLMM）
  AllFarm = "allFarm",            // 所有 Farm Pool
  StandardFarm = "standardFarm",  // 标准 Farm Pool
  ConcentratedFarm = "concentratedFarm",  // 集中流动性 Farm Pool
}
```

**排序字段**：
- `liquidity` - 流动性
- `volume24h` / `volume7d` / `volume30d` - 交易量
- `fee24h` / `fee7d` / `fee30d` - 手续费
- `apr24h` / `apr7d` / `apr30d` - APR

**示例**：
```typescript
const pools = await api.getPoolList({
  type: PoolFetchType.All,
  sort: 'liquidity',
  order: 'desc',
  pageSize: 100,
  page: 0,
})

console.log(pools)
// {
//   count: 1000,
//   hasNextPage: true,
//   data: [
//     {
//       id: "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj",
//       type: "Standard",
//       programId: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
//       mintA: { ... },
//       mintB: { ... },
//       price: 150.5,
//       tvl: 1000000,
//       feeRate: 0.0025,
//       // ...
//     },
//     // ...
//   ]
// }
```

---

### 3.2 根据 Mint 查询 Pool

**接口**：`/pools/info/mint`

**方法**：`api.fetchPoolByMints(props)`

**参数**：
```typescript
{
  mint1: string | PublicKey;     // 第一个 Mint
  mint2?: string | PublicKey;    // 第二个 Mint（可选）
  type?: PoolFetchType;          // Pool 类型
  sort?: string;                 // 排序字段
  order?: 'desc' | 'asc';        // 排序方向
  page?: number;                 // 页码
}
```

**示例**：
```typescript
// 查询 SOL-USDC 的所有 Pool
const pools = await api.fetchPoolByMints({
  mint1: 'So11111111111111111111111111111111111111112',  // SOL
  mint2: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', // USDC
  type: PoolFetchType.All,
  sort: 'liquidity',
  order: 'desc',
  page: 1,
})

console.log(pools.data)
// [
//   {
//     id: "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj",
//     type: "Standard",
//     programId: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
//     mintA: { address: "So11111111111111111111111111111111111111112", symbol: "SOL", ... },
//     mintB: { address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", symbol: "USDC", ... },
//     price: 150.5,
//     tvl: 1000000,
//     feeRate: 0.0025,
//     // ...
//   },
//   {
//     id: "DiwsGxJYoRZURvyCtMsJVyxR86yZBBbSYeeWNm7YCmT6",
//     type: "Concentrated",
//     programId: "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",
//     config: { tradeFeeRate: 0.0001, tickSpacing: 64, ... },
//     // ...
//   }
// ]
```

---

### 3.3 根据 Pool ID 查询 Pool

**接口**：`/pools/info/ids`

**方法**：`api.fetchPoolById(props)`

**参数**：
```typescript
{
  ids: string;  // Pool ID，多个 ID 用逗号分隔
}
```

**示例**：
```typescript
const pools = await api.fetchPoolById({
  ids: '8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj,DiwsGxJYoRZURvyCtMsJVyxR86yZBBbSYeeWNm7YCmT6'
})

console.log(pools)
// [
//   {
//     id: "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj",
//     type: "Standard",
//     // ...
//   },
//   {
//     id: "DiwsGxJYoRZURvyCtMsJVyxR86yZBBbSYeeWNm7YCmT6",
//     type: "Concentrated",
//     // ...
//   }
// ]
```

---

### 3.4 获取 Pool Keys（用于构建交易）

**接口**：`/pools/key/ids`

**方法**：`api.fetchPoolKeysById(props)`

**参数**：
```typescript
{
  idList: string[];  // Pool ID 列表
}
```

**示例**：
```typescript
const poolKeys = await api.fetchPoolKeysById({
  idList: ['8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj']
})

console.log(poolKeys)
// [
//   {
//     programId: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
//     id: "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj",
//     mintA: { ... },
//     mintB: { ... },
//     vault: { A: "xxx", B: "xxx" },
//     authority: "xxx",
//     openOrders: "xxx",
//     targetOrders: "xxx",
//     mintLp: { ... },
//     marketProgramId: "xxx",
//     marketId: "xxx",
//     marketAuthority: "xxx",
//     marketBaseVault: "xxx",
//     marketQuoteVault: "xxx",
//     marketBids: "xxx",
//     marketAsks: "xxx",
//     marketEventQueue: "xxx",
//     lookupTableAccount: "xxx",
//     openTime: "xxx"
//   }
// ]
```

---

### 3.5 获取 CLMM Pool 流动性曲线

**接口**：`/pools/line/liquidity`

**方法**：`api.getClmmPoolLines(poolId)`

**参数**：
```typescript
poolId: string;  // Pool ID
```

**示例**：
```typescript
const liquidityLines = await api.getClmmPoolLines('DiwsGxJYoRZURvyCtMsJVyxR86yZBBbSYeeWNm7YCmT6')

console.log(liquidityLines)
// [
//   { price: "150.0", liquidity: "1000000" },
//   { price: "151.0", liquidity: "900000" },
//   { price: "152.0", liquidity: "800000" },
//   // ...
// ]
```

---

## 4. CLMM Pool 选择（不同费率）

### 4.1 CLMM Config 信息

**接口**：`/main/clmm-config`

**方法**：`api.getClmmConfigs()`

**示例**：
```typescript
const clmmConfigs = await api.getClmmConfigs()

console.log(clmmConfigs)
// [
//   {
//     id: "1",
//     index: 0,
//     protocolFeeRate: 0.00001,
//     tradeFeeRate: 0.0001,      // 0.01% 费率
//     tickSpacing: 1,
//     fundFeeRate: 0.0,
//     defaultRange: 1000,
//     defaultRangePoint: [-1000, 1000]
//   },
//   {
//     id: "2",
//     index: 1,
//     protocolFeeRate: 0.00001,
//     tradeFeeRate: 0.00025,     // 0.025% 费率
//     tickSpacing: 8,
//     fundFeeRate: 0.0,
//     defaultRange: 1000,
//     defaultRangePoint: [-1000, 1000]
//   },
//   {
//     id: "3",
//     index: 2,
//     protocolFeeRate: 0.00001,
//     tradeFeeRate: 0.0005,      // 0.05% 费率
//     tickSpacing: 64,
//     fundFeeRate: 0.0,
//     defaultRange: 1000,
//     defaultRangePoint: [-1000, 1000]
//   },
//   {
//     id: "4",
//     index: 3,
//     protocolFeeRate: 0.00001,
//     tradeFeeRate: 0.0025,      // 0.25% 费率
//     tickSpacing: 64,
//     fundFeeRate: 0.0,
//     defaultRange: 1000,
//     defaultRangePoint: [-1000, 1000]
//   }
// ]
```

### 4.2 CLMM Pool 选择策略

Raydium SDK **不提供自动选择最合适 CLMM Pool 的功能**，需要开发者自己实现选择逻辑。

**推荐选择策略**：

1. **根据交易金额选择费率**：
   - 小额交易（< $100）：选择低费率 Pool（0.01%）
   - 中等交易（$100 - $10,000）：选择中等费率 Pool（0.05%）
   - 大额交易（> $10,000）：选择高费率 Pool（0.25%，通常流动性更好）

2. **根据流动性深度选择**：
   - 选择 TVL 最高的 Pool
   - 选择当前价格附近的流动性最密集的 Pool

3. **根据价格影响选择**：
   - 计算每个 Pool 的价格影响
   - 选择价格影响最小的 Pool

**示例代码**：
```typescript
import { PoolUtils } from '@raydium-io/raydium-sdk-v2'

// 1. 获取所有 CLMM Pool
const pools = await api.fetchPoolByMints({
  mint1: 'So11111111111111111111111111111111111111112',  // SOL
  mint2: 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', // USDC
  type: PoolFetchType.Concentrated,
  sort: 'liquidity',
  order: 'desc',
})

// 2. 过滤出 CLMM Pool
const clmmPools = pools.data.filter(p => p.type === 'Concentrated')

// 3. 计算每个 Pool 的价格影响
const amountIn = new BN(1000000000)  // 1 SOL
const bestPool = clmmPools[0]  // 默认选择第一个

for (const pool of clmmPools) {
  const clmmPoolInfo = await PoolUtils.fetchComputeClmmInfo({
    connection: raydium.connection,
    poolInfo: pool,
  })
  
  const tickCache = await PoolUtils.fetchMultiplePoolTickArrays({
    connection: raydium.connection,
    poolKeys: [clmmPoolInfo],
  })
  
  const { priceImpact } = await PoolUtils.computeAmountOutFormat({
    poolInfo: clmmPoolInfo,
    tickArrayCache: tickCache[pool.id],
    amountIn,
    tokenOut: pool.mintB,
    slippage: 0.01,
    epochInfo: await raydium.fetchEpochInfo(),
  })
  
  console.log(`Pool ${pool.id}: feeRate=${pool.config.tradeFeeRate}, priceImpact=${priceImpact.toNumber()}`)
  
  // 选择价格影响最小的 Pool
  if (priceImpact.toNumber() < bestPriceImpact) {
    bestPool = pool
    bestPriceImpact = priceImpact.toNumber()
  }
}

console.log('Best pool:', bestPool)
```

---

## 5. Swap 交易接口

### 5.1 AMM V4 Swap

**示例代码**：
```typescript
import { ApiV3PoolInfoStandardItem, AmmV4Keys, AmmRpcData } from '@raydium-io/raydium-sdk-v2'
import BN from 'bn.js'
import { NATIVE_MINT } from '@solana/spl-token'

const raydium = await initSdk()
const amountIn = 500  // 0.0005 SOL
const inputMint = NATIVE_MINT.toBase58()
const poolId = '58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2'  // SOL-USDC pool

// 1. 获取 Pool 信息
const data = await raydium.api.fetchPoolById({ ids: poolId })
const poolInfo = data[0] as ApiV3PoolInfoStandardItem
const poolKeys = await raydium.liquidity.getAmmPoolKeys(poolId)
const rpcData = await raydium.liquidity.getRpcPoolInfo(poolId)

// 2. 计算输出金额
const [baseReserve, quoteReserve, status] = [
  rpcData.baseReserve,
  rpcData.quoteReserve,
  rpcData.status.toNumber(),
]

const baseIn = inputMint === poolInfo.mintA.address
const [mintIn, mintOut] = baseIn ? [poolInfo.mintA, poolInfo.mintB] : [poolInfo.mintB, poolInfo.mintA]

const out = raydium.liquidity.computeAmountOut({
  poolInfo: {
    ...poolInfo,
    baseReserve,
    quoteReserve,
    status,
    version: 4,
  },
  amountIn: new BN(amountIn),
  mintIn: mintIn.address,
  mintOut: mintOut.address,
  slippage: 0.01,  // 1% 滑点
})

console.log(`Output: ${out.amountOut.toString()}, Min: ${out.minAmountOut.toString()}`)

// 3. 构建并发送交易
const { execute } = await raydium.liquidity.swap({
  poolInfo,
  poolKeys,
  amountIn: new BN(amountIn),
  amountOut: out.minAmountOut,  // 使用最小输出金额（考虑滑点）
  fixedSide: 'in',
  inputMint: mintIn.address,
  txVersion: TxVersion.V0,
  
  // 可选：设置优先费
  computeBudgetConfig: {
    units: 600000,
    microLamports: 46591500,
  },
  
  // 可选：添加小费（如 Jito）
  txTipConfig: {
    address: new PublicKey('96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5'),
    amount: new BN(10000000),  // 0.01 SOL
  },
})

const { txId } = await execute({ sendAndConfirm: true })
console.log(`Swap successful: https://explorer.solana.com/tx/${txId}`)
```

**关键参数**：
- `amountIn` - 输入金额
- `amountOut` - 输出金额（使用 `minAmountOut` 考虑滑点）
- `fixedSide` - 固定哪一侧（`'in'` 或 `'out'`）
- `inputMint` - 输入代币地址
- `slippage` - 滑点（范围：1 ~ 0.0001，表示 100% ~ 0.01%）
- `txVersion` - 交易版本（`TxVersion.V0` 或 `TxVersion.LEGACY`）

---

### 5.2 CPMM Swap

**示例代码**：
```typescript
import { ApiV3PoolInfoStandardItemCpmm, CpmmKeys, CpmmParsedRpcData, CurveCalculator } from '@raydium-io/raydium-sdk-v2'
import BN from 'bn.js'
import { NATIVE_MINT } from '@solana/spl-token'

const raydium = await initSdk()
const inputAmount = new BN(100)  // 0.0001 SOL
const inputMint = NATIVE_MINT.toBase58()
const poolId = '7JuwJuNU88gurFnyWeiyGKbFmExMWcmRZntn9imEzdny'  // SOL-USDC pool

// 1. 获取 Pool 信息
const data = await raydium.api.fetchPoolById({ ids: poolId })
const poolInfo = data[0] as ApiV3PoolInfoStandardItemCpmm
const rpcData = await raydium.cpmm.getRpcPoolInfo(poolInfo.id, true)

// 2. 计算输出金额
const baseIn = inputMint === poolInfo.mintA.address

const swapResult = CurveCalculator.swapBaseInput(
  inputAmount,
  baseIn ? rpcData.baseReserve : rpcData.quoteReserve,
  baseIn ? rpcData.quoteReserve : rpcData.baseReserve,
  rpcData.configInfo!.tradeFeeRate,
  rpcData.configInfo!.creatorFeeRate,
  rpcData.configInfo!.protocolFeeRate,
  rpcData.configInfo!.fundFeeRate,
  rpcData.feeOn === FeeOn.BothToken || rpcData.feeOn === FeeOn.OnlyTokenB
)

console.log('Swap result:', {
  inputAmount: swapResult.inputAmount.toString(),
  outputAmount: swapResult.outputAmount.toString(),
  tradeFee: swapResult.tradeFee.toString(),
})

// 3. 构建并发送交易
const { execute, transaction } = await raydium.cpmm.swap({
  poolInfo,
  poolKeys,
  inputAmount,
  swapResult,
  slippage: 0.001,  // 0.1% 滑点
  baseIn,
  txVersion: TxVersion.V0,
  
  // 可选：设置优先费
  computeBudgetConfig: {
    units: 600000,
    microLamports: 4659150,
  },
})

const { txId } = await execute({ sendAndConfirm: true })
console.log(`Swap successful: https://explorer.solana.com/tx/${txId}`)
```

**关键参数**：
- `inputAmount` - 输入金额
- `swapResult` - 计算结果（包含输出金额、手续费等）
- `slippage` - 滑点（范围：1 ~ 0.0001，表示 100% ~ 0.01%）
- `baseIn` - 是否从 base token 输入
- `txVersion` - 交易版本

---

### 5.3 CLMM Swap

**示例代码**：
```typescript
import { ApiV3PoolInfoConcentratedItem, ClmmKeys, ComputeClmmPoolInfo, PoolUtils } from '@raydium-io/raydium-sdk-v2'
import BN from 'bn.js'
import { NATIVE_MINT } from '@solana/spl-token'

const raydium = await initSdk()
const poolId = '8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj'  // SOL-USDC pool
const inputMint = NATIVE_MINT.toBase58()
const inputAmount = new BN(1000000000)  // 0.1 SOL

// 1. 获取 Pool 信息
const data = await raydium.api.fetchPoolById({ ids: poolId })
const poolInfo = data[0] as ApiV3PoolInfoConcentratedItem

const clmmPoolInfo = await PoolUtils.fetchComputeClmmInfo({
  connection: raydium.connection,
  poolInfo,
})

const tickCache = await PoolUtils.fetchMultiplePoolTickArrays({
  connection: raydium.connection,
  poolKeys: [clmmPoolInfo],
})

// 2. 计算输出金额
const baseIn = inputMint === poolInfo.mintA.address

const { minAmountOut, remainingAccounts, priceImpact } = await PoolUtils.computeAmountOutFormat({
  poolInfo: clmmPoolInfo,
  tickArrayCache: tickCache[poolId],
  amountIn: inputAmount,
  tokenOut: poolInfo[baseIn ? 'mintB' : 'mintA'],
  slippage: 0.01,  // 1% 滑点
  epochInfo: await raydium.fetchEpochInfo(),
})

console.log(`Output: ${minAmountOut.amount.raw.toString()}, Price Impact: ${priceImpact.toNumber()}`)

// 3. 构建并发送交易
const { execute } = await raydium.clmm.swap({
  poolInfo,
  poolKeys,
  inputMint: poolInfo[baseIn ? 'mintA' : 'mintB'].address,
  amountIn: inputAmount,
  amountOutMin: minAmountOut.amount.raw,
  observationId: clmmPoolInfo.observationId,
  ownerInfo: {
    useSOLBalance: inputMint === NATIVE_MINT.toBase58(),  // 只有输入是 SOL 时才使用 SOL balance
  },
  remainingAccounts,
  txVersion: TxVersion.V0,
  
  // 可选：设置优先费
  computeBudgetConfig: {
    units: 600000,
    microLamports: 465915,
  },
})

const { txId } = await execute()
console.log(`Swap successful: https://explorer.solana.com/tx/${txId}`)
```

**关键参数**：
- `inputMint` - 输入代币地址
- `amountIn` - 输入金额
- `amountOutMin` - 最小输出金额（考虑滑点）
- `observationId` - 观察账户 ID
- `ownerInfo.useSOLBalance` - 是否使用 SOL 余额（而不是 WSOL）
- `remainingAccounts` - 剩余账户（tick arrays）
- `slippage` - 滑点

---

## 6. 其他辅助接口

### 6.1 获取代币列表

**接口**：`/mint/list`

**方法**：`api.getTokenList()`

**示例**：
```typescript
const { mintList, blacklist, whiteList } = await api.getTokenList()

console.log(mintList)
// [
//   {
//     chainId: 101,
//     address: "So11111111111111111111111111111111111111112",
//     programId: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
//     logoURI: "...",
//     symbol: "SOL",
//     name: "Wrapped SOL",
//     decimals: 9,
//     tags: ["hasFreeze"],
//     extensions: { ... }
//   },
//   // ...
// ]
```

---

### 6.2 获取代币信息

**接口**：`/mint/ids`

**方法**：`api.getTokenInfo(mints)`

**示例**：
```typescript
const tokens = await api.getTokenInfo([
  'So11111111111111111111111111111111111111112',
  'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',
])

console.log(tokens)
// [
//   { address: "...", symbol: "SOL", decimals: 9, ... },
//   { address: "...", symbol: "USDC", decimals: 6, ... }
// ]
```

---

### 6.3 获取 RPC 节点列表

**接口**：`/main/rpcs`

**方法**：`api.getRpcs()`

**示例**：
```typescript
const { rpcs, strategy } = await api.getRpcs()

console.log(rpcs)
// [
//   {
//     url: "https://api.mainnet-beta.solana.com",
//     weight: 1,
//     batch: true,
//     name: "Solana Official"
//   },
//   // ...
// ]
```

---

### 6.4 获取链上时间偏移

**接口**：`/main/chain-time`

**方法**：`api.getChainTimeOffset()`

**示例**：
```typescript
const { offset } = await api.getChainTimeOffset()

console.log(`Chain time offset: ${offset} seconds`)
```

---

## 7. 直接从 RPC 查询 Pool

除了使用 API，也可以直接从 RPC 查询 Pool 信息。

### 7.1 获取所有 AMM V4 Pool

```typescript
import { Connection, PublicKey } from '@solana/web3.js'
import { ALL_PROGRAM_ID, liquidityStateV4Layout, struct, publicKey } from '@raydium-io/raydium-sdk-v2'

const connection = new Connection('rpc url')

// 获取所有 AMM V4 Pool（只获取 baseMint 和 quoteMint）
const layoutAmm = struct([publicKey('baseMint'), publicKey('quoteMint')])
const ammPools: (ReturnType<typeof layoutAmm.decode> & { poolId: PublicKey })[] = []

const ammPoolsData = await connection.getProgramAccounts(ALL_PROGRAM_ID.AMM_V4, {
  filters: [{ dataSize: liquidityStateV4Layout.span }],
  dataSlice: { offset: liquidityStateV4Layout.offsetOf('baseMint'), length: 64 },
  encoding: 'base64' as any,
})

ammPoolsData.forEach((a) => {
  ammPools.push({
    poolId: a.pubkey,
    ...layoutAmm.decode(a.account.data),
  })
})

console.log(`Found ${ammPools.length} AMM V4 pools`)
```

---

### 7.2 获取所有 CLMM Pool

```typescript
import { PoolInfoLayout } from '@raydium-io/raydium-sdk-v2'

const clmmPools: (ReturnType<typeof PoolInfoLayout.decode> & { poolId: PublicKey })[] = []

const clmmPoolsData = await connection.getProgramAccounts(ALL_PROGRAM_ID.CLMM_PROGRAM_ID, {
  filters: [{ dataSize: PoolInfoLayout.span }],
})

clmmPoolsData.forEach((c) => {
  clmmPools.push({
    poolId: c.pubkey,
    ...PoolInfoLayout.decode(c.account.data),
  })
})

console.log(`Found ${clmmPools.length} CLMM pools`)
```

---

### 7.3 获取所有 CPMM Pool

```typescript
import { CpmmPoolInfoLayout } from '@raydium-io/raydium-sdk-v2'

const cpmmPools: (ReturnType<typeof CpmmPoolInfoLayout.decode> & { poolId: PublicKey })[] = []

const cpmmPoolsData = await connection.getProgramAccounts(ALL_PROGRAM_ID.CREATE_CPMM_POOL_PROGRAM, {
  filters: [{ dataSize: CpmmPoolInfoLayout.span }],
})

cpmmPoolsData.forEach((c) => {
  cpmmPools.push({
    poolId: c.pubkey,
    ...CpmmPoolInfoLayout.decode(c.account.data),
  })
})

console.log(`Found ${cpmmPools.length} CPMM pools`)
```

---

## 8. 最佳实践

### 8.1 Pool 查询策略

1. **优先使用 API**：
   - API 返回的数据更完整（包含 TVL、交易量、APR 等）
   - API 支持排序和分页
   - API 有缓存机制，速度更快

2. **RPC 查询适用场景**：
   - 需要实时数据（API 可能有延迟）
   - 需要查询所有 Pool（API 可能有限制）
   - 需要自定义过滤条件

### 8.2 CLMM Pool 选择策略

1. **根据交易金额选择**：
   ```typescript
   const selectBestClmmPool = (pools: ApiV3PoolInfoConcentratedItem[], amountIn: BN) => {
     const amountInUsd = amountIn.div(new BN(10 ** 9)).toNumber() * 150  // 假设 SOL 价格 $150
     
     if (amountInUsd < 100) {
       // 小额交易：选择低费率 Pool
       return pools.filter(p => p.config.tradeFeeRate <= 0.0001).sort((a, b) => b.tvl - a.tvl)[0]
     } else if (amountInUsd < 10000) {
       // 中等交易：选择中等费率 Pool
       return pools.filter(p => p.config.tradeFeeRate <= 0.0005).sort((a, b) => b.tvl - a.tvl)[0]
     } else {
       // 大额交易：选择高费率 Pool（流动性更好）
       return pools.sort((a, b) => b.tvl - a.tvl)[0]
     }
   }
   ```

2. **根据价格影响选择**：
   ```typescript
   const selectBestClmmPoolByPriceImpact = async (pools: ApiV3PoolInfoConcentratedItem[], amountIn: BN) => {
     let bestPool = pools[0]
     let bestPriceImpact = Infinity
     
     for (const pool of pools) {
       const clmmPoolInfo = await PoolUtils.fetchComputeClmmInfo({
         connection: raydium.connection,
         poolInfo: pool,
       })
       
       const tickCache = await PoolUtils.fetchMultiplePoolTickArrays({
         connection: raydium.connection,
         poolKeys: [clmmPoolInfo],
       })
       
       const { priceImpact } = await PoolUtils.computeAmountOutFormat({
         poolInfo: clmmPoolInfo,
         tickArrayCache: tickCache[pool.id],
         amountIn,
         tokenOut: pool.mintB,
         slippage: 0.01,
         epochInfo: await raydium.fetchEpochInfo(),
       })
       
       if (priceImpact.toNumber() < bestPriceImpact) {
         bestPool = pool
         bestPriceImpact = priceImpact.toNumber()
       }
     }
     
     return bestPool
   }
   ```

### 8.3 Swap 交易最佳实践

1. **设置合理的滑点**：
   - 小额交易：0.1% - 0.5%
   - 中等交易：0.5% - 1%
   - 大额交易：1% - 3%

2. **使用优先费**：
   ```typescript
   computeBudgetConfig: {
     units: 600000,
     microLamports: 46591500,  // 根据网络拥堵情况调整
   }
   ```

3. **使用小费加速**（如 Jito）：
   ```typescript
   txTipConfig: {
     address: new PublicKey('96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5'),
     amount: new BN(10000000),  // 0.01 SOL
   }
   ```

4. **模拟交易**：
   ```typescript
   const { execute, transaction } = await raydium.clmm.swap({ ... })
   
   // 模拟交易
   const simulation = await raydium.connection.simulateTransaction(transaction)
   if (simulation.value.err) {
     console.error('Simulation failed:', simulation.value.err)
     throw new Error('Transaction simulation failed')
   }
   
   // 发送交易
   const { txId } = await execute({ sendAndConfirm: true })
   ```

---

## 9. 总结

### 9.1 支持的 DEX

| DEX | Pool 类型 | 查询方法 | Swap 方法 |
|-----|-----------|----------|-----------|
| **AMM V4** | 标准 | `fetchPoolByMints(type: 'standard')` | `raydium.liquidity.swap()` |
| **CPMM** | 标准 | `fetchPoolByMints(type: 'standard')` | `raydium.cpmm.swap()` |
| **CLMM** | 集中流动性 | `fetchPoolByMints(type: 'concentrated')` | `raydium.clmm.swap()` |

### 9.2 CLMM Pool 选择

- **不提供自动选择功能**
- **需要开发者自己实现选择逻辑**
- **推荐策略**：
  1. 根据交易金额选择费率
  2. 根据流动性深度选择
  3. 根据价格影响选择

### 9.3 Swap 参数

| 参数 | AMM V4 | CPMM | CLMM |
|------|--------|------|------|
| `amountIn` | ✅ | ✅ | ✅ |
| `amountOut` / `amountOutMin` | ✅ | ❌ | ✅ |
| `fixedSide` | ✅ | ❌ | ❌ |
| `baseIn` | ❌ | ✅ | ❌ |
| `inputMint` | ✅ | ❌ | ✅ |
| `slippage` | ✅ | ✅ | ✅ |
| `swapResult` | ❌ | ✅ | ❌ |
| `remainingAccounts` | ❌ | ❌ | ✅ |
| `observationId` | ❌ | ❌ | ✅ |

---

## 10. 相关文档

- [Pool 查询方法参考](./Pool查询方法.md)
- [Raydium CPMM Pool 查找技术分析](./raydium-cpmm-pool-lookup.md)
- [Raydium API 功能分析](./raydium-api-analysis.md)
- [Pool 选择算法分析](./Pool选择算法分析.md)

---

**创建日期**：2026-01-09  
**最后更新**：2026-01-09  
**作者**：iFlow CLI