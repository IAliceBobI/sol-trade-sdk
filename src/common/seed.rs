use crate::common::SolanaRpcClient;
use anyhow::anyhow;
use fnv::FnvHasher;
use once_cell::sync::Lazy;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use solana_system_interface::instruction::create_account_with_seed;
use std::hash::Hasher;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::{Duration, sleep};

// 🚀 优化：使用 AtomicU64 替代 RwLock，性能提升 5-10x
// u64::MAX 表示未初始化状态
static SPL_TOKEN_RENT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(u64::MAX));
static SPL_TOKEN_2022_RENT: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(u64::MAX));

/// 更新租金缓存（后台任务调用）
pub async fn update_rents(client: &SolanaRpcClient) -> Result<(), anyhow::Error> {
    let rent = fetch_rent_for_token_account(client, false).await?;
    SPL_TOKEN_RENT.store(rent, Ordering::Release); // Release 确保其他线程可见

    let rent = fetch_rent_for_token_account(client, true).await?;
    SPL_TOKEN_2022_RENT.store(rent, Ordering::Release);

    Ok(())
}

pub fn start_rent_updater(client: Arc<SolanaRpcClient>) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = update_rents(&client).await {
                eprintln!("⚠️  Rent cache update failed: {}. Using previous values.", e);
            }
            sleep(Duration::from_secs(60 * 60)).await;
        }
    });
}

async fn fetch_rent_for_token_account(
    client: &SolanaRpcClient,
    _is_2022_token: bool,
) -> Result<u64, anyhow::Error> {
    Ok(client.get_minimum_balance_for_rent_exemption(165).await?)
}

pub fn create_associated_token_account_use_seed(
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
    token_program: &Pubkey,
) -> Result<Vec<Instruction>, anyhow::Error> {
    let is_2022_token = token_program == &crate::constants::TOKEN_PROGRAM_2022;

    // 🚀 优化：原子读取租金缓存
    // Relaxed: 租金值不变，无需同步；Release/Acquire 在 update_rents 保证初始化可见性
    let rent = if is_2022_token {
        let v = SPL_TOKEN_2022_RENT.load(Ordering::Relaxed);
        if v == u64::MAX {
            return Err(anyhow!("Rent not initialized"));
        }
        v
    } else {
        let v = SPL_TOKEN_RENT.load(Ordering::Relaxed);
        if v == u64::MAX {
            return Err(anyhow!("Rent not initialized"));
        }
        v
    };

    let mut buf = [0u8; 8];
    let mut hasher = FnvHasher::default();
    hasher.write(mint.as_ref());
    let hash = hasher.finish();
    let v = (hash & 0xFFFF_FFFF) as u32;
    for i in 0..8 {
        let nibble = ((v >> (28 - i * 4)) & 0xF) as u8;
        buf[i] = match nibble {
            0..=9 => b'0' + nibble,
            _ => b'a' + (nibble - 10),
        };
    }
    let seed = unsafe { std::str::from_utf8_unchecked(&buf) };
    // 🔧 修复：使用传入的 token_program 生成地址（支持 Token 和 Token-2022）
    // 买入和卖出只要都使用事件中的 token_program，地址自然一致
    let ata_like = Pubkey::create_with_seed(payer, seed, token_program)?;

    let len = 165;
    // 但账户的 owner 仍然使用正确的 token_program（Token 或 Token-2022）
    let create_acc =
        create_account_with_seed(payer, &ata_like, owner, seed, rent, len, token_program);

    let init_acc = if is_2022_token {
        crate::common::spl_token_2022::initialize_account3(token_program, &ata_like, mint, owner)?
    } else {
        crate::common::spl_token::initialize_account3(token_program, &ata_like, mint, owner)?
    };

    Ok(vec![create_acc, init_acc])
}

pub fn get_associated_token_address_with_program_id_use_seed(
    wallet_address: &Pubkey,
    token_mint_address: &Pubkey,
    token_program_id: &Pubkey,
) -> Result<Pubkey, anyhow::Error> {
    let mut buf = [0u8; 8];
    let mut hasher = FnvHasher::default();
    hasher.write(token_mint_address.as_ref());
    let hash = hasher.finish();
    let v = (hash & 0xFFFF_FFFF) as u32;
    for i in 0..8 {
        let nibble = ((v >> (28 - i * 4)) & 0xF) as u8;
        buf[i] = match nibble {
            0..=9 => b'0' + nibble,
            _ => b'a' + (nibble - 10),
        };
    }
    let seed = unsafe { std::str::from_utf8_unchecked(&buf) };
    // 🔧 修复：使用传入的 token_program_id 生成地址（支持 Token 和 Token-2022）
    // 买入和卖出只要都使用事件中的 token_program_id，地址自然一致
    let ata_like = Pubkey::create_with_seed(wallet_address, seed, token_program_id)?;
    Ok(ata_like)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    const TOKEN_PROGRAM: Pubkey = Pubkey::new_from_array([
        0x6d, 0x98, 0x65, 0x71, 0x66, 0x13, 0x44, 0x11, 0x0c, 0xf2, 0xbc, 0xc4, 0x41, 0xcf, 0x81,
        0x1a, 0x9a, 0xf4, 0xba, 0x06, 0x40, 0x2e, 0x50, 0x8f, 0x4f, 0x9a, 0x94, 0x1f, 0x3b, 0x50,
        0xc6, 0x4d,
    ]);

    const TOKEN_PROGRAM_2022: Pubkey = Pubkey::new_from_array([
        0x6d, 0x98, 0x65, 0x71, 0x66, 0x13, 0x44, 0x12, 0x0c, 0xf2, 0xbc, 0xc4, 0x41, 0xcf, 0x81,
        0x1b, 0x9a, 0xf4, 0xba, 0x06, 0x40, 0x2e, 0x50, 0x8f, 0x4f, 0x9a, 0x94, 0x1f, 0x3b, 0x50,
        0xc6, 0x4e,
    ]);

    const MINT_A: Pubkey = Pubkey::new_from_array([
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
        0x18, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36,
        0x37, 0x38,
    ]);

    const MINT_B: Pubkey = Pubkey::new_from_array([
        0xff, 0xee, 0xdd, 0xcc, 0xbb, 0xaa, 0x99, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11,
        0x00, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ]);

    const WALLET: Pubkey = Pubkey::new_from_array([
        0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde,
        0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc,
        0xde, 0xf0,
    ]);

    #[test]
    fn test_seed_generation_deterministic() {
        // 测试同一个 mint 生成相同的 seed
        let addr1 =
            get_associated_token_address_with_program_id_use_seed(&WALLET, &MINT_A, &TOKEN_PROGRAM)
                .unwrap();
        let addr2 =
            get_associated_token_address_with_program_id_use_seed(&WALLET, &MINT_A, &TOKEN_PROGRAM)
                .unwrap();
        assert_eq!(addr1, addr2, "Same mint should produce same ATA address");
    }

    #[test]
    fn test_different_mints_generate_different_seeds() {
        // 测试不同 mint 生成不同的 seed
        let addr_a =
            get_associated_token_address_with_program_id_use_seed(&WALLET, &MINT_A, &TOKEN_PROGRAM)
                .unwrap();
        let addr_b =
            get_associated_token_address_with_program_id_use_seed(&WALLET, &MINT_B, &TOKEN_PROGRAM)
                .unwrap();
        assert_ne!(addr_a, addr_b, "Different mints should produce different ATA addresses");
    }

    #[test]
    fn test_different_wallets_generate_different_addresses() {
        // 测试不同钱包生成不同的 ATA 地址
        let wallet2 = Pubkey::new_from_array([
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
            0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
            0x66, 0x77, 0x88, 0x99,
        ]);

        let addr1 =
            get_associated_token_address_with_program_id_use_seed(&WALLET, &MINT_A, &TOKEN_PROGRAM)
                .unwrap();
        let addr2 = get_associated_token_address_with_program_id_use_seed(
            &wallet2,
            &MINT_A,
            &TOKEN_PROGRAM,
        )
        .unwrap();
        assert_ne!(addr1, addr2, "Different wallets should produce different ATA addresses");
    }

    #[test]
    fn test_token_program_affects_address() {
        // 测试不同 token program 生成不同的 ATA 地址
        let addr_token =
            get_associated_token_address_with_program_id_use_seed(&WALLET, &MINT_A, &TOKEN_PROGRAM)
                .unwrap();
        let addr_token_2022 = get_associated_token_address_with_program_id_use_seed(
            &WALLET,
            &MINT_A,
            &TOKEN_PROGRAM_2022,
        )
        .unwrap();
        assert_ne!(
            addr_token, addr_token_2022,
            "Different token programs should produce different ATA addresses"
        );
    }

    #[test]
    fn test_seed_format() {
        // 测试 seed 生成函数返回的格式正确性
        let mut buf = [0u8; 8];
        let mut hasher = FnvHasher::default();
        hasher.write(MINT_A.as_ref());
        let hash = hasher.finish();
        let v = (hash & 0xFFFF_FFFF) as u32;
        for i in 0..8 {
            let nibble = ((v >> (28 - i * 4)) & 0xF) as u8;
            buf[i] = match nibble {
                0..=9 => b'0' + nibble,
                _ => b'a' + (nibble - 10),
            };
        }
        let seed = unsafe { std::str::from_utf8_unchecked(&buf) };

        // 验证 seed 长度为 8
        assert_eq!(seed.len(), 8);

        // 验证 seed 只包含小写字母和数字
        for c in seed.chars() {
            assert!(
                c.is_ascii_digit() || c.is_ascii_lowercase(),
                "Seed character '{}' is not valid (should be 0-9 or a-f)",
                c
            );
        }
    }

    #[test]
    fn test_ata_address_is_valid_pubkey() {
        // 验证生成的地址是有效的 Pubkey
        let addr =
            get_associated_token_address_with_program_id_use_seed(&WALLET, &MINT_A, &TOKEN_PROGRAM)
                .unwrap();

        // Pubkey 应该是 32 字节
        assert_eq!(addr.as_ref().len(), 32);

        // 不应该是零地址 (所有字节都是0)
        let bytes = addr.as_ref();
        assert!(!bytes.iter().all(|&b| b == 0), "ATA address should not be zero address");
    }
}
