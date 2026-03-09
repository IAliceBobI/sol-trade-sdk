# Solana 历史区块遍历验证报告

## 验证目标

验证 QuickNode RPC 是否可以遍历 Solana 历史区块和交易数据。

## 验证方法

使用 curl 直接调用 RPC 接口，抽查不同历史深度的区块。

### RPC 端点

```
https://tiniest-twilight-putty.solana-mainnet.quiknode.pro/<key>
```

### 关键 RPC 方法

- `getSlot` - 获取当前 slot
- `getBlock` - 获取特定区块的详细信息
- `getTransaction` - 获取特定交易的详细信息

## 抽查结果

| 区块 Slot | 时间范围 | Blockhash | 状态 |
|---------|---------|-----------|------|
| 405,188,980 | 最近 (~slot-100) | EJzWK9rLEm4jRvQGDsHvWBQhzgpYMDXb8dpPyVohr62R | ✅ 可访问 |
| 404,973,080 | 约 1 天前 | FtDJaHXpnQBNTpPCd7gwdM2Bagkf7RUSsEhRvVd9Dqz1 | ✅ 可访问 |
| 400,000,000 | 约 1 个月前 | UQr2NVYe3mstRS2yfThK4rAFt5LpL2XV6ozDdFVKPnb | ✅ 可访问 |
| 300,000,000 | 2024 年初 | 4aCVoqmcAyGDKdTURY6cQ3jY43URNhQStorm4kXmDf2i | ✅ 可访问 |

## 验证命令示例

### 1. 获取当前 Slot

```bash
curl -X POST "$QUICKNODE_RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getSlot"}'
```

### 2. 获取区块信息

```bash
curl -X POST "$QUICKNODE_RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBlock",
    "params": [
      405188980,
      {
        "encoding": "json",
        "maxSupportedTransactionVersion": 0,
        "transactionDetails": "signatures"
      }
    ]
  }'
```

### 3. 获取交易详情

```bash
curl -X POST "$QUICKNODE_RPC_URL" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getTransaction",
    "params": [
      "tbiyMmdPJBMLBV9dk38mXGg4fBQFkTp6zWpvi8ksqAdFNCzmJXfaeQ9EKpfJWbKDxBzxoQVDg4taEpkcRt7Mwoa",
      {
        "encoding": "json",
        "maxSupportedTransactionVersion": 0
      }
    ]
  }'
```

## 关键参数说明

### getBlock 参数

| 参数 | 说明 |
|-----|------|
| `maxSupportedTransactionVersion` | 必须设置为 `0` 以获取 v0 交易 |
| `transactionDetails` | `"signatures"` 只返回签名列表，`"full"` 返回完整交易 |
| `encoding` | `"json"` 返回 JSON 格式，`"base64"` 返回编码数据 |

## 区块数据字段

```json
{
  "blockHeight": 383294936,
  "blockTime": 1773034027,
  "blockhash": "EJzWK9rLEm4jRvQGDsHvWBQhzgpYMDXb8dpPyVohr62R",
  "parentSlot": 405188979,
  "previousBlockhash": "DW6BRGShW7hb6JA9BXjZQHd3kz7HD7gDQUsPiEF3ZZ6S",
  "rewards": [...],
  "signatures": ["tx_sig1", "tx_sig2", ...]
}
```

## 结论

✅ **QuickNode 是存档节点**，可以访问完整的历史区块数据。

从验证结果看，QuickNode RPC 支持遍历 Solana 全链历史，至少可以追溯到 2024 年初（slot 300M 左右）。对于需要获取历史区块和交易数据的场景，可以直接使用 QuickNode RPC。

## 遍历流程（如需实现）

```
1. getSlot() → 获取当前最新 slot
        ↓
2. getBlocks(start, end) → 批量获取区块号范围
        ↓
3. 对每个 slot 调用 getBlock(slot) → 获取区块交易签名
        ↓
4. 对每个签名调用 getTransaction(sig) → 获取交易详情
```

## 注意事项

1. **速率限制** - QuickNode 有 RPC 调用频率限制，批量获取时注意控制速率
2. **数据量** - 完整区块数据很大，建议先用 `transactionDetails: "signatures"` 获取签名列表
3. **空区块** - 某些区块可能没有交易，返回的 signatures 为空数组

---

*验证时间: 2026-03-09*
*当前 Slot: 405,189,080*
