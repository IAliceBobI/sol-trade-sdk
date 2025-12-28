# Project Documentation Rules (Non-Obvious Only)

- DEX 协议实现分散在 `src/instruction/` 目录，每个协议有独立的指令构建器
- 计算模块位于 `src/utils/calc/`，各协议价格计算逻辑独立实现
- SWQoS 多节点实现位于 `src/swqos/`，支持 Jito、Bloxroute、Flashblock 等多个MEV保护服务
- 性能优化模块位于 `src/perf/`，包含 SIMD、编译器优化、内核旁路等低级优化
- 交易参数和常量在 `src/constants/` 中定义，而非硬编码在业务逻辑中
- 所有示例都是独立的工作区成员，需手动添加到 `Cargo.toml` 的 `workspace.members`
