# Jito Testnet 与 Mainnet Tip Accounts 区别

## ⚠️ 重要提示

**Jito Testnet 和 Mainnet 使用完全不同的 Tip 账户地址！**

使用错误的 tip accounts 会导致 Bundle 提交失败，错误信息：
```
"Bundles must write lock at least one tip account to be eligible for the auction."
```

## 为什么不同？

1. **独立环境**: Jito Testnet 是完全独立的测试环境
   - 使用独立的验证者节点
   - 运行在单独的区块链网络上
   - 有自己的基础设施和经济系统

2. **收益分配**:
   - **Testnet tips** 分配给 Testnet 验证者
   - **Mainnet tips** 分配给 Mainnet 验证者
   - Tip 是激励验证者打包你交易的关键机制

3. **账户隔离**:
   - Testnet 账户余额是虚拟的测试 SOL
   - Mainnet 账户涉及真实资产
   - 混用会导致交易失败或资产损失

## 如何获取正确的 Tip Accounts？

### Testnet Tip Accounts

**API 端点**:
```
https://dallas.testnet.block-engine.jito.wtf/api/v1/getTipAccounts
https://ny.testnet.block-engine.jito.wtf/api/v1/getTipAccounts
https://testnet.block-engine.jito.wtf/api/v1/getTipAccounts
```

**获取方式**:
```bash
curl -X POST "https://dallas.testnet.block-engine.jito.wtf/api/v1/getTipAccounts" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getTipAccounts","params":[]}'
```

**Testnet Tip Accounts (8 个)**:
```
1. 7aewvu8fMf1DK4fKoMXKfs3h3wpAQ7r7D8T1C71LmMF
2. 84DrGKhycCUGfLzw8hXsUYX9SnWdh2wW3ozsTPrC5xyg
3. BkMx5bRzQeP6tUZgzEs3xeDWJfQiLYvNDqSgmGZKYJDq
4. 4uRnem4BfVpZBv7kShVxUYtcipscgZMSHi3B9CSL6gAA
5. G2d63CEgKBdgtpYT2BuheYQ9HFuFCenuHLNyKVpqAuSD
6. AzfhMPcx3qjbvCK3UUy868qmc5L451W341cpFqdL3EBe
7. F7ThiQUBYiEcyaxpmMuUeACdoiSLKg4SZZ8JSfpFNwAf
8. CwWZzvRgmxj9WLLhdoWUVrHZ1J8db3w2iptKuAitHqoC
```

### Mainnet Tip Accounts

**API 端点**:
```
https://mainnet.block-engine.jito.wtf/api/v1/getTipAccounts
https://amsterdam.mainnet.block-engine.jito.wtf/api/v1/getTipAccounts
https://tokyo.mainnet.block-engine.jito.wtf/api/v1/getTipAccounts
... 等多个区域
```

**获取方式**:
```bash
curl -X POST "https://mainnet.block-engine.jito.wtf/api/v1/getTipAccounts" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getTipAccounts","params":[]}'
```

**Mainnet Tip Accounts (8 个)**:
```
1. 96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5
2. HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe
3. Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY
4. ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49
5. DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh
6. ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt
7. DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL
8. 3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT
```

## 代码示例

### ❌ 错误示例（使用 Mainnet Tip Accounts 在 Testnet）

```rust
// 错误！在 Testnet 上使用 Mainnet 的 tip accounts
let jito_endpoint = "https://dallas.testnet.block-engine.jito.wtf/api/v1/bundles";
let tip_accounts = vec![
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5", // Mainnet!
    // ...
];

// 结果：Bundle 被拒绝，错误："Bundles must write lock at least one tip account"
```

### ✅ 正确示例（使用对应网络的 Tip Accounts）

```rust
// Testnet
let jito_endpoint = "https://dallas.testnet.block-engine.jito.wtf/api/v1/bundles";
let tip_accounts = vec![
    "7aewvu8fMf1DK4fKoMXKfs3h3wpAQ7r7D8T1C71LmMF", // Testnet ✓
    "84DrGKhycCUGfLzw8hXsUYX9SnWdh2wW3ozsTPrC5xyg", // Testnet ✓
    // ...
];

// Mainnet
let jito_endpoint = "https://mainnet.block-engine.jito.wtf/api/v1/bundles";
let tip_accounts = vec![
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5", // Mainnet ✓
    "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe", // Mainnet ✓
    // ...
];
```

### ✅ 最佳实践（动态获取）

```rust
async fn get_jito_tip_accounts(network: JitoNetwork) -> Result<Vec<String>> {
    let endpoint = match network {
        JitoNetwork::Mainnet => "https://mainnet.block-engine.jito.wtf",
        JitoNetwork::Testnet => "https://dallas.testnet.block-engine.jito.wtf",
    };

    let url = format!("{}/api/v1/getTipAccounts", endpoint);

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"getTipAccounts","params":[]}"#)
        .send()
        .await?;

    let json: serde_json::Value = response.json().await?;
    let accounts: Vec<String> = json["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    Ok(accounts)
}
```

## SDK 配置

### 本项目 SDK 中的配置

**文件位置**: `src/constants/swqos.rs`

**定义的常量**:
- `JITO_TIP_ACCOUNTS`: Mainnet tip accounts（已添加详细注释说明）
- `SWQOS_ENDPOINTS_JITO`: Mainnet endpoints

**测试代码**:
- `tests/jito_testnet_tests.rs`: 使用正确的 Testnet tip accounts

## 其他 MEV 服务

**注意**: 其他 MEV 服务（如 ZeroSlot, Bloxroute 等）可能也有 Testnet 和 Mainnet 的区别，使用前请查看对应服务的文档。

## 参考资源

- **Jito 官方文档**: https://docs.jito.wtf/lowlatencytxnsend/
- **Jito GitHub**: https://github.com/jito-foundation
- **Solana Testnet Faucet**: https://faucet.solana.com/

## 常见错误

### 错误 1: Bundle 被拒绝
```
"Bundles must write lock at least one tip account to be eligible for the auction."
```

**原因**: 使用了错误网络的 tip accounts

**解决**: 检查 tip accounts 是否与网络（Testnet/Mainnet）匹配

### 错误 2: 交易未上链
```
Bundle 提交成功，但交易长时间未确认
```

**原因**:
1. Tip 金额太低（最小 1000 lamports）
2. 使用了错误的网络配置
3. Testnet 网络拥堵或维护中

**解决**:
1. 检查并增加 tip 金额
2. 验证使用正确的端点和 tip accounts
3. 使用 `getBundleStatuses` 检查 Bundle 状态

## 总结

⚠️ **核心要点**:
1. Testnet 和 Mainnet 的 tip accounts **完全不同**
2. **永远不要混用** tip accounts
3. **使用前验证**网络配置
4. **最佳实践**: 从对应网络的 API 动态获取 tip accounts

这样可以避免交易失败和资产损失！
