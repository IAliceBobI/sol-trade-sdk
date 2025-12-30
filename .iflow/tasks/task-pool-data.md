# pool 相关解析的文档

/opt/projects/sol-trade-sdk/dexprojs/pump-public-docs。


# 老的pool
这个是一个pool：https://solscan.io/account/539m4mVWt6iduB6W8rDGPMarzNCMesuqY5eUTiiYHAgR。
mint1:pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn；
mint2:So11111111111111111111111111111111111111112；

pool rawdata: 
```
8ZptBBGxbbz8AABSWj6ZiH5E1LXQ8iuym3ss4LIjH+Af/9v8/l9E/LIbIwxF99+NnnKVYoSTP22Yt1cDLoPfhGBPteEX//YdWxL5BpuIV/6rgYT7aH9jRhjANdrEOdwa6ztVmKDwAAAAAAG2Iy79yKbyJqiV5Y6t7GxhIUfNWbDrm2E/WxROEX6qmb9Hzi37ol0wZrcXy+djNtjpwvpOUYc9vd1To2qW8TPHpM/nV2DKa6SC+lpwy5VuSnHEYmKwUBYuGOU3+Twb5AAmtvyaDAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
```

```json
{
  "pool_bump": {
    "type": "u8",
    "data": 252
  },
  "index": {
    "type": "u16",
    "data": "0"
  },
  "creator": {
    "type": "pubkey",
    "data": "6YUF9bvmbYSVopieXE7boHvByDP9U5ERJoUP6NfcpP9t"
  },
  "base_mint": {
    "type": "pubkey",
    "data": "pumpCmXqMfrsAkQ5r49WcJnRayYRqmXz6ae8H7H9Dfn"
  },
  "quote_mint": {
    "type": "pubkey",
    "data": "So11111111111111111111111111111111111111112"
  },
  "lp_mint": {
    "type": "pubkey",
    "data": "DFzKR7GUytmdLyvxNHFzRpsRtvFtxRWBaZUGzga4nbD2"
  },
  "pool_base_token_account": {
    "type": "pubkey",
    "data": "DsgNmawKXbcLZvLTJaw9VJGpZRKxFG1h8fmVW3P4AJPG"
  },
  "pool_quote_token_account": {
    "type": "pubkey",
    "data": "C6MjYVQjuvtCKNWCJQzSs8s2ScpVbVUTMXRnE2wNbFyy"
  },
  "lp_supply": {
    "type": "u64",
    "data": "54139860518"
  },
  "coin_creator": {
    "type": "pubkey",
    "data": "11111111111111111111111111111111"
  }
}```

# 新的pool
pool: 2xHRmdXSKURh8CkMERbNhYCiQGHGsjLWMgckEP9bmLKK
mint1: Ew8KqgSitYucieR5KnSAL2SUFspcwA8AgSuZ5xWspump
mint2: So11111111111111111111111111111111111111112

pool data : 
```
8ZptBBGxbbz/AAAdWIDT1vP9E2FsVOBr+asKUeqfBjJnI7ChLUFrksF/E88Fk3AHNufkHWdg5ZK9VSLuSHnIWgFx/C8XQfIzUJjPBpuIV/6rgYT7aH9jRhjANdrEOdwa6ztVmKDwAAAAAAHukn411jAxzGkUt+NPRJ9jdiWsBGpE75MYM2YAARp/ACLgfROK1bKohbzi59uAbrr9fM/nLYQAW+Co5F2nck2sbaJbsHe3IA3byqEgvaDkiNO/qKyYZUhir5DB0t0b/Fu0WGtZ0AMAABedOkyms0GA3twZCaP3BFxASd8TM6ukDIz0uCW63VCFAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA==
```

```json
{
  "pool_bump": {
    "type": "u8",
    "data": 255
  },
  "index": {
    "type": "u16",
    "data": "0"
  },
  "creator": {
    "type": "pubkey",
    "data": "2yZ6ZkXiRFXQBxyKoroQ3dSpwMaLFyczAVeWJhVwZJdg"
  },
  "base_mint": {
    "type": "pubkey",
    "data": "Ew8KqgSitYucieR5KnSAL2SUFspcwA8AgSuZ5xWspump"
  },
  "quote_mint": {
    "type": "pubkey",
    "data": "So11111111111111111111111111111111111111112"
  },
  "lp_mint": {
    "type": "pubkey",
    "data": "H4HbHtUQxqmwJABN21iWuGhsAdnFn4FJytrT9NJjkHxK"
  },
  "pool_base_token_account": {
    "type": "pubkey",
    "data": "3M9QEVNHFTgQXo11vSsi8zj15sS8j8aStDFX4dn3FT1M"
  },
  "pool_quote_token_account": {
    "type": "pubkey",
    "data": "8Ny2i2WiaLfLhqitqAS1CjM6LJwDTFpyLCPcgQuJLvKC"
  },
  "lp_supply": {
    "type": "u64",
    "data": "4193388288180"
  },
  "coin_creator": {
    "type": "pubkey",
    "data": "2bBRwhGoL4fRZk6g8NnhBZywsF8PdLJnBRfWDCEMogD2"
  }
}
```
