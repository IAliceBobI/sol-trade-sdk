# Jito Testnet Bundle 测试说明

## 概述

此测试在 Solana testnet 上实际发送一个包含 3 笔小额 SOL 转账的 Jito Bundle。

## 测试内容

- **发送方**: SOLANA_TEST_KEY_PATH1
- **接收方**: SOLANA_TEST_KEY_PATH2
- **交易数量**: 3 笔
- **每笔转账金额**: 0.000001 SOL (1,000 lamports)
- **Tip 金额**: 0.00001 SOL (10,000 lamports)
- **总花费**: 约 0.000026 SOL (转账 + tip + 交易费)

## 运行步骤

### 1. 准备测试账户

```bash
# 从 testnet faucet 获取测试 SOL
# 访问: https://faucet.solana.com/

# 或者使用命令行
solana airdrop 1 <PAYER_ADDRESS> --url https://api.testnet.solana.com
```

### 2. 设置环境变量

```bash
export SOLANA_TEST_KEY_PATH1=/path/to/sender-keypair.json
export SOLANA_TEST_KEY_PATH2=/path/to/receiver-keypair.json
export PROXY_URL=http://127.0.0.1:7891  # 可选
```

### 3. 运行测试

```bash
# 使用 cargo test
cargo test --test jito_testnet_tests -- test_jito_bundle_send_example --exact --nocapture --ignored

# 或使用 cargo nextest (推荐)
cargo nextest run --test jito_testnet_tests -- test_jito_bundle_send_example --exact --nocapture --ignored
```

## 测试流程

1. ✅ 读取发送方和接收方密钥
2. ✅ 查询发送方余额
3. ✅ 获取最新 blockhash
4. ✅ 构建 3 笔转账交易
5. ✅ 将交易转换为 VersionedTransaction
6. ✅ 序列化为 base64
7. ✅ 发送到 Jito Testnet endpoint
8. ✅ 查询交易状态

## Jito Testnet 信息

- **RPC URL**: https://api.testnet.solana.com
- **Jito Endpoint**: https://dallas.testnet.block-engine.jito.wtf
- **Tip Account**: HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe

## 注意事项

1. **余额要求**: 发送方需要至少 0.01 SOL
2. **原子性**: Bundle 中的交易要么全部成功，要么全部失败
3. **网络延迟**: Testnet 可能比 mainnet 慢，请耐心等待
4. **交易查询**: 可以在 Solscan 查看交易状态（使用 testnet cluster）

## 查看交易

测试成功后，可以通过以下链接查看交易：

```
https://solscan.io/tx/<SIGNATURE>?cluster=testnet
```

## 故障排查

### 余额不足
```
⚠️  发送方余额不足（需要至少 0.01 SOL）
💡 请从以下地址获取测试 SOL:
   https://faucet.solana.com/
```

**解决方案**: 从 testnet faucet 获取更多测试 SOL

### Bundle 发送失败
```
❌ Bundle 发送失败: <error message>
```

**可能原因**:
- Jito testnet endpoint 不可用
- 网络连接问题
- 交易格式错误

**解决方案**: 检查网络连接，稍后重试

### 交易未确认
```
⏳ 交易尚未处理
```

**原因**: Testnet 可能处理较慢

**解决方案**: 等待几分钟后在 Solscan 查询交易状态

## 相关资源

- [Jito 官方文档](https://docs.jito.wtf)
- [Solana Testnet Faucet](https://faucet.solana.com/)
- [Solana Testnet RPC](https://api.testnet.solana.com)
