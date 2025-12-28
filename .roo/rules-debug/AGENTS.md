# Project Debug Rules (Non-Obvious Only)

- 性能追踪特性 `perf-trace` 在生产环境应禁用（默认关闭），启用时会影响延迟
- `fast_init()` 函数会预热所有缓存 PDA 和 ATA，需在交易前调用以避免首笔交易延迟
- SWQoS 并发交易失败时会返回部分成功的签名列表，需检查返回的 `Vec<Signature>` 而非仅检查成功标志
- 交易模块使用 `Arc<dyn TradeExecutor>` 单例，修改后需重启程序才能生效
- 缓存键基于 Pubkey 的部分字节进行哈希，避免在热路径中克隆完整 Pubkey
