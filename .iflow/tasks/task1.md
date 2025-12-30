 
 
 目前我们的项目，只支持新的pool的数据格式的解析。
 
（参考： "docs/PumpSwap池类型.md"）。
 
   我希望旧的pool也支持。
 
 
 

# 要开发测试的（/opt/projects/sol-trade-sdk/examples/pumpswap_direct_trading）
分析  ``` let (success, signatures, trade_error) =
   client.buy(buy_params).await?;```
     背后的流程. 并给一个升级计划。
   
这是老的pool:
```
    let pool = Pubkey::from_str("539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR").unwrap();
    let mint_pubkey = Pubkey::from_str("pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn").unwrap();

```

我们必须能正确的解析老pool的结构体。同时保持也能解析新pool的结构体。
目前不是找pool的逻辑有问题，而是解析老pool的结构有问题。
老pool也是重要的。


现在测试是错的
```
Compiling sol-trade-sdk v3.3.6 (/opt/projects/sol-trade-sdk)
Compiling pumpswap_direct_trading v0.1.0 (/opt/projects/sol-trade-sdk/examples/pumpswap_direct_trading)
 Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.88s
  Running `target/debug/pumpswap_direct_trading`
Testing PumpSwap trading...
🚀 Initializing SolanaTrade client...
Loading keypair from: /Users/chenwei/.config/solana/id.json
🔧 TradeConfig create_wsol_ata_on_startup default value: true
🔧 TradeConfig use_seed_optimize default value: true
🔧 TradeConfig callback_execution_mode default value: Async
✅ WSOL ATA已存在: 6eksga9k36rxve3N2bZfdGnrkuPn8GY9BbJCabaG3Tew
💸 Airdropping 10 SOL to account...
✅ Airdrop successful!
💰 Payer SOL balance: 59.991168963 SOL
✅ SolanaTrade client initialized successfully!
Buying tokens from PumpSwap...
signature: 5SKt1rip9FNqyscvrg3xr79fUBCt2uvmqJD1H3bePdpnoV1nDZXS7aMiP7HV9zK5Ch7aVGrKafrK6q7SbnJDQiqp
[rpc] Buy confirmation failed: 148.941375ms
Error: Buy failed: TradeError { code: 4, message: "Error processing Instruction 6: invalid account data for instruction \"InvalidAccountData\"", instruction: Some(6) }

Stack backtrace:
0: std::backtrace_rs::backtrace::libunwind::trace
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/../../backtrace/src/backtrace/libunwind.rs:117:9
1: std::backtrace_rs::backtrace::trace_unsynchronized
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/../../backtrace/src/backtrace/mod.rs:66:14
2: std::backtrace::Backtrace::create
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/backtrace.rs:331:13
3: std::backtrace::Backtrace::capture
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/backtrace.rs:296:9
4: anyhow::error::<impl anyhow::Error>::msg
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/anyhow-1.0.100/src/backtrace.rs:27:14
5: pumpswap_direct_trading::main::{{closure}}
          at ./examples/pumpswap_direct_trading/src/main.rs:58:24
6: <core::pin::Pin<P> as core::future::future::Future>::poll
          at /Users/chenwei/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/core/src/future/future.rs:133:9
7: tokio::runtime::park::CachedParkThread::block_on::{{closure}}
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.48.0/src/runtime/park.rs:285:71
8: tokio::task::coop::with_budget
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.48.0/src/task/coop/mod.rs:167:5
9: tokio::task::coop::budget
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.48.0/src/task/coop/mod.rs:133:5
10: tokio::runtime::park::CachedParkThread::block_on
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.48.0/src/runtime/park.rs:285:31
11: tokio::runtime::context::blocking::BlockingRegionGuard::block_on
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.48.0/src/runtime/context/blocking.rs:66:14
12: tokio::runtime::scheduler::multi_thread::MultiThread::block_on::{{closure}}
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.48.0/src/runtime/scheduler/multi_thread/mod.rs:87:22
13: tokio::runtime::context::runtime::enter_runtime
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.48.0/src/runtime/context/runtime.rs:65:16
14: tokio::runtime::scheduler::multi_thread::MultiThread::block_on
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.48.0/src/runtime/scheduler/multi_thread/mod.rs:86:9
15: tokio::runtime::runtime::Runtime::block_on_inner
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.48.0/src/runtime/runtime.rs:370:50
16: tokio::runtime::runtime::Runtime::block_on
          at /Users/chenwei/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tokio-1.48.0/src/runtime/runtime.rs:340:18
17: pumpswap_direct_trading::main
          at ./examples/pumpswap_direct_trading/src/main.rs:112:7
18: core::ops::function::FnOnce::call_once
          at /Users/chenwei/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/core/src/ops/function.rs:250:5
19: std::sys::backtrace::__rust_begin_short_backtrace
          at /Users/chenwei/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/sys/backtrace.rs:158:18
20: std::rt::lang_start::{{closure}}
          at /Users/chenwei/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/rt.rs:206:18
21: core::ops::function::impls::<impl core::ops::function::FnOnce<A> for &F>::call_once
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/core/src/ops/function.rs:287:21
22: std::panicking::catch_unwind::do_call
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/panicking.rs:590:40
23: std::panicking::catch_unwind
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/panicking.rs:553:19
24: std::panic::catch_unwind
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/panic.rs:359:14
25: std::rt::lang_start_internal::{{closure}}
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/rt.rs:175:24
26: std::panicking::catch_unwind::do_call
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/panicking.rs:590:40
27: std::panicking::catch_unwind
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/panicking.rs:553:19
28: std::panic::catch_unwind
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/panic.rs:359:14
29: std::rt::lang_start_internal
          at /rustc/ded5c06cf21d2b93bffd5d884aa6e96934ee4234/library/std/src/rt.rs:171:5
30: std::rt::lang_start
          at /Users/chenwei/.rustup/toolchains/stable-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/rt.rs:205:5
31: _main

```



# 要保持正确，不要影响
/opt/projects/sol-trade-sdk/examples/pumpswap_direct_trading_new_pool


# 最后，执行确保通过
/opt/projects/sol-trade-sdk/examples/pumpswap_direct_trading
/opt/projects/sol-trade-sdk/examples/pumpswap_direct_trading_new_pool
