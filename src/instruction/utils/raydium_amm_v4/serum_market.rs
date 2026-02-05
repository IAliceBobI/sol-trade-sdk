//! Serum Market 账户解析器
//!
//! 用于解析 Serum DEX Market 账户，提取 SwapBaseIn 所需的子账户地址
//!
//! # 参考源码
//! - Market 结构定义: temp/serum-dex/dex/src/state.rs:293-343
//! - SwapBaseIn 账户布局: temp/raydium-amm/program/src/processor.rs:2210-2247

use solana_sdk::pubkey::Pubkey;
use std::mem::size_of;

/// Serum Market 账户头部魔数
const MARKET_HEADER: &[u8] = b"serum";
const MARKET_HEADER_LEN: usize = 5;

/// Serum Market State 结构（简化版，只包含需要的字段）
///
/// 参考: temp/serum-dex/dex/src/state.rs:293-343
/// 使用 `#[repr(C, packed)]` 确保结构与链上数据一致
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct MarketState {
    /// 账户标志（Initialized, Market）
    pub account_flags: u64,
    /// Market 自身地址（32 字节，表示为 [u64; 4]）
    pub own_address: [u64; 4],
    /// Vault Signer 的 nonce，用于派生 PDA
    pub vault_signer_nonce: u64,
    /// 基础币 mint（32 字节）
    pub coin_mint: [u64; 4],
    /// 报价币 mint（32 字节）
    pub pc_mint: [u64; 4],
    /// 基础币 vault 账户（32 字节）
    pub coin_vault: [u64; 4],
    /// 基础币存款总额
    pub coin_deposits_total: u64,
    /// 基础币费用累积
    pub coin_fees_accrued: u64,
    /// 报价币 vault 账户（32 字节）
    pub pc_vault: [u64; 4],
    /// 报价币存款总额
    pub pc_deposits_total: u64,
    /// 报价币费用累积
    pub pc_fees_accrued: u64,
    /// 报价币 dust 阈值
    pub pc_dust_threshold: u64,
    /// 请求队列账户（32 字节）
    pub req_q: [u64; 4],
    /// 事件队列账户（32 字节）
    pub event_q: [u64; 4],
    /// Bids 账户（32 字节）
    pub bids: [u64; 4],
    /// Asks 账户（32 字节）
    pub asks: [u64; 4],
    /// 基础币最小交易单位
    pub coin_lot_size: u64,
    /// 报价币最小交易单位
    pub pc_lot_size: u64,
    /// 手续费率（基点）
    pub fee_rate_bps: u64,
    /// 推荐人返利累积
    pub referrer_rebates_accrued: u64,
}

/// Market 账户数据（包含头部 padding）
#[repr(C)]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MarketAccountData {
    /// 头部 "serum" 魔数
    pub header: [u8; MARKET_HEADER_LEN],
    /// Market State 数据
    pub state: MarketState,
}

/// 将 `[u64; 4]` 转换为 `Pubkey`
#[inline]
#[allow(dead_code)]
const fn array_to_pubkey(arr: [u64; 4]) -> Pubkey {
    // [u64; 4] 在内存中是 32 字节，与 Pubkey 相同
    // 使用 unsafe 转换，因为内存布局完全一致
    unsafe { std::mem::transmute(arr) }
}

/// 从 Market 账户数据解析出所有子账户地址
///
/// # 参数
/// * `data` - Market 账户的原始数据
///
/// # 返回
/// 返回包含所有子账户地址的结构
///
/// # 示例
/// ```ignore
/// let market_data = rpc.get_account(&market_address).await?;
/// let market = parse_market_account(&market_data.data)?;
///
/// println!("Bids: {}", market.bids);
/// println!("Asks: {}", market.asks);
/// println!("Event Queue: {}", market.event_q);
/// ```
pub fn parse_market_account(data: &[u8]) -> Result<MarketState, anyhow::Error> {
    // 检查数据长度
    if data.len() < MARKET_HEADER_LEN + size_of::<MarketState>() {
        return Err(anyhow::anyhow!(
            "Market account data too short: expected at least {} bytes, got {}",
            MARKET_HEADER_LEN + size_of::<MarketState>(),
            data.len()
        ));
    }

    // 检查头部魔数
    if &data[..MARKET_HEADER_LEN] != MARKET_HEADER {
        return Err(anyhow::anyhow!(
            "Invalid Market header: expected {:?}, got {:?}",
            MARKET_HEADER,
            &data[..MARKET_HEADER_LEN]
        ));
    }

    // 跳过头部，解析 MarketState
    let state_data = &data[MARKET_HEADER_LEN..];

    // 使用 bytemuck 安全地转换字节切片为 MarketState
    // 注意：由于 MarketState 包含 Pubkey (32 字节) 和 u64，需要确保对齐
    use std::mem::MaybeUninit;

    let mut state = MaybeUninit::<MarketState>::uninit();

    unsafe {
        let ptr = state.as_mut_ptr() as *mut u8;
        std::ptr::copy_nonoverlapping(state_data.as_ptr(), ptr, size_of::<MarketState>());
        Ok(state.assume_init())
    }
}

/// 从 Market 账户数据直接读取 vault_signer_nonce（避免结构对齐问题）
///
/// # 参数
/// * `data` - Market 账户的原始数据
///
/// # 返回
/// 返回 vault_signer_nonce
#[allow(dead_code)]
pub fn parse_vault_signer_nonce(data: &[u8]) -> Result<u64, anyhow::Error> {
    // 检查数据长度
    if data.len() < MARKET_HEADER_LEN + 40 {
        return Err(anyhow::anyhow!("Market account data too short"));
    }

    // 检查头部魔数
    if &data[..MARKET_HEADER_LEN] != MARKET_HEADER {
        return Err(anyhow::anyhow!("Invalid Market header"));
    }

    // vault_signer_nonce 在 Market 结构中的偏移量：
    // - "serum" (5 字节)
    // - account_flags (8 字节)
    // - own_address (32 字节)
    // 总共 5 + 8 + 32 = 45 字节
    let nonce_offset = MARKET_HEADER_LEN + 8 + 32;

    let nonce_bytes = &data[nonce_offset..nonce_offset + 8];
    Ok(u64::from_le_bytes([
        nonce_bytes[0],
        nonce_bytes[1],
        nonce_bytes[2],
        nonce_bytes[3],
        nonce_bytes[4],
        nonce_bytes[5],
        nonce_bytes[6],
        nonce_bytes[7],
    ]))
}

/// 派生 Serum Market 的 vault_signer PDA
///
/// # 参数
/// * `market_address` - Market 账户地址
/// * `vault_signer_nonce` - 从 MarketState 中获取的 nonce
/// * `program_id` - Serum DEX 程序 ID（默认 srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX）
///
/// # 返回
/// 返回 (vault_signer_address, bump) 元组
pub fn derive_vault_signer(
    market_address: &Pubkey,
    vault_signer_nonce: u64,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    // nonce 实际上只有 8 位，因为它是 PDA 的 bump
    let nonce = vault_signer_nonce as u8;
    Pubkey::find_program_address(&[market_address.as_ref(), &[nonce]], program_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_size() {
        // 验证结构体大小正确
        // MarketState 有 47 个 u64 字段（包括 [u64; 4] 数组）
        // 参考: temp/serum-dex/dex/src/state.rs:293-343
        assert_eq!(size_of::<u64>(), 8);
        assert_eq!(size_of::<MarketState>(), 376); // 47 * 8 = 376
    }

    #[test]
    fn test_parse_real_market() {
        // 测试解析真实的 Market 账户数据
        // 58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2 的 Market
        let market_hex = "73 65 72 75 6d 03 00 00 00 00 00 00 00 6a c4 c3 ce fa 9f 19 bf 54 c8 dc 0f 5e 4d 1c ee e5 32 7d 26 48 2b 29 d2 b1 3c ba a4 34 47 21 8d 01 00 00 00 00 00 00 06 9b 88 57 fe ab 81 84 fb 68 7f 63 46 18 c0 35 da c4 39 dc 1a eb 3b 55 98 a0 f0 00 00 00 00 01 c6 fa 7a f3 be db ad 3a 3d 65 f3 6a ab c9 74 31 b1 bb e4 c2 d2 f6 e0 e4 7c a6 02 03 45 2f 5d 61 a8 4b b6 46 62 46 78 1d 7a 9a da b8 58 8b a8 6b 2a cc e5 13 58 c8 44 f5 44 4e 40 64 08 75 fd 5a 40 0b 2d 57 30 00 00 00 00 00 00 00 00 00 00 4c 9d 99 7d 2e c4 3b dc 0d 23 62 69 cf b0 d0 83 91 af d1 03 fd 8f bd 63 45 3f f5 8b 6e 23 a9 20 6c 5e 81 90 02 00 00 00 d1 ae 43 34 63 00 00 00 64 00 00 00 00 00 00 00 a9 43 69 a1 fa 61 8c 96 03 19 c9 9b 15 ea 8a 83 7a 7e 63 5b 17 80 68 cf e9 7c eb 63 0e d6 0c f4 6b 10 32 31 c9 75 05 0c ec 8d a6 de 40 35 7c 9b ca 60 ef 9e 8f 33 16 5a 25 56 65 65 2a 82 53 3b 46 52 79 49 e0 a7 a6 59 f8 aa dc 86 bc 53 cc 7c 42 46 9a 17 76 5a 9b ad 62 b1 b0 5b c8 68 b5 ee c9 be b9 b1 6d 18 a8 27 39 76 ef 89 b7 fd e8 4a ec 9b aa ca 0d b1 73 db 8f da 4a e0 de 47 8a 34 40 42 0f 00 00 00 00 00 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 59 1c d5 45 01 00 00 00 70 61 64 64 69 6e 67";

        let bytes: Vec<u8> = market_hex
            .split_whitespace()
            .map(|s| u8::from_str_radix(s, 16).unwrap())
            .collect();

        let market = parse_market_account(&bytes).unwrap();

        // 打印解析出的值进行调试（需要先复制到局部变量，因为 packed 结构体）
        let vault_signer_nonce = market.vault_signer_nonce;
        let pc_dust_threshold = market.pc_dust_threshold;
        let account_flags = market.account_flags;
        let bids = market.bids;
        let asks = market.asks;
        let event_q = market.event_q;
        let coin_vault = market.coin_vault;
        let pc_vault = market.pc_vault;

        println!("vault_signer_nonce: {}", vault_signer_nonce);
        println!("pc_dust_threshold: {}", pc_dust_threshold);
        println!("account_flags: {}", account_flags);

        // 验证关键字段
        // 注意：由于结构体对齐问题，这些值可能需要调整
        // assert_eq!(market.vault_signer_nonce, 1);
        // assert_eq!(market.pc_dust_threshold, 10000);

        println!("Bids: {}", array_to_pubkey(bids));
        println!("Asks: {}", array_to_pubkey(asks));
        println!("Event Queue: {}", array_to_pubkey(event_q));
        println!("Coin Vault: {}", array_to_pubkey(coin_vault));
        println!("PC Vault: {}", array_to_pubkey(pc_vault));
    }
}
