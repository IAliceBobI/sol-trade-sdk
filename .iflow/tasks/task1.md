 
 
 目前我们的项目，只支持新的pool的数据格式的解析。
 
（参考： "docs/PumpSwap池类型.md"）。
 
   我希望旧的pool也支持。
 
 
 

# 要开发测试的（/opt/projects/sol-trade-sdk/examples/pumpswap_direct_trading）
分析  ``` let (success, signatures, trade_error) =
   client.buy(buy_params).await?;```
     背后的流程. 并给一个升级计划。



# 要保持正确，不要影响
/opt/projects/sol-trade-sdk/examples/pumpswap_direct_trading_new_pool
