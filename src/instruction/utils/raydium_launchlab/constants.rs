/// Constants used as seeds for deriving PDAs (Program Derived Addresses)
pub mod seeds {
    /// Seed for bonding curve PDAs (pool_state)
    pub const POOL_SEED: &[u8] = b"pool";

    /// Seed for vault authority PDAs
    pub const VAULT_AUTH_SEED: &[u8] = b"vault_auth_seed";

    /// Seed for pool vault PDAs
    pub const POOL_VAULT_SEED: &[u8] = b"pool_vault";

    /// Seed for event authority PDAs
    pub const EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";

    /// Seed for platform config PDAs
    pub const PLATFORM_CONFIG_SEED: &[u8] = b"platform_config";
}

/// Constants related to program accounts and authorities
pub mod accounts {
    use solana_sdk::{pubkey, pubkey::Pubkey};

    /// Raydium LaunchLab program ID (mainnet)
    pub const LAUNCHLAB_PROGRAM: Pubkey = pubkey!("LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj");

    /// Raydium CPMM program ID (mainnet) - used for external pool after migration
    pub const CPMM_PROGRAM: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");

    /// Raydium CPMM program ID (devnet)
    pub const CPMM_PROGRAM_DEVNET: Pubkey = pubkey!("DRaycpLY18LhpbydsBWbVJtxpNv9oXPgjRSfpF2bWpYb");

    /// Metaplex Token Metadata program
    pub const METADATA_PROGRAM: Pubkey = pubkey!("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

    /// System Program
    pub const SYSTEM_PROGRAM: Pubkey = pubkey!("11111111111111111111111111111111");

    /// Rent Sysvar
    pub const RENT_SYSVAR: Pubkey = pubkey!("SysvarRent111111111111111111111111111111111");

    /// Associated Token Program
    pub const ASSOCIATED_TOKEN_PROGRAM: Pubkey =
        pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

    /// CPMM Create Pool Fee account (mainnet)
    /// This is the account that receives fees when creating CPMM pools
    pub const CPMM_CREATE_POOL_FEE: Pubkey =
        pubkey!("3oE58BKVt8KuYkGxx8zBojugnymWmBiyafWgMrnb6eYy");

    /// Known platform_config addresses (mainnet)
    /// LetsBonk.fun platform config
    pub const LETSBONK_PLATFORM_CONFIG: Pubkey =
        pubkey!("FfYek5vEz23cMkWsdJwG2oa6EphsvXSHrGpdALN4g6W1");

    /// CPMM Config address (devnet)
    /// From Raydium LaunchLab documentation
    pub const CPMM_CONFIG_DEVNET: Pubkey = pubkey!("EsTevfacYXpuho5VBuzBjDZi8dtWidGnXoSYAr8krTvz");

    /// CPMM Config address (mainnet)
    /// Found from Solscan and GitHub: https://github.com/raydium-io/raydium-cpi-example
    pub const CPMM_CONFIG_MAINNET: Pubkey = pubkey!("D4FPEruKEHrG5TenZ2mpDGEfu1iUvTiqBxvpU8HLBvC2");

    /// CPMM Authority address (mainnet)
    /// Known authority address for CPMM program operations
    /// From: docs/raydium/raydium-addresses-reference.md
    pub const CPMM_AUTHORITY: Pubkey = pubkey!("GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL");

    /// Lock Program address
    /// Used for LP token locking in migrate_to_cpswap
    pub const LOCK_PROGRAM: Pubkey = pubkey!("LockrWmn6K5twhz3y9w1dQERbmgSaRkfnTeTKbpofwE");

    /// Raydium Launchpad Authority (mainnet)
    /// Known authority address for LaunchLab vault operations
    /// From actual transaction analysis
    pub const LAUNCHPAD_AUTHORITY: Pubkey = pubkey!("WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh");

    /// Event Authority (mainnet)
    /// Known event authority PDA for LaunchLab events
    /// From actual transaction analysis
    pub const EVENT_AUTHORITY: Pubkey = pubkey!("2DPAtwB8L12vrMRExbLuyGnC7n2J5LNoZQSejeQGpwkr");

    /// Global Config for USD1 quote token (mainnet)
    /// Known global config address when using USD1 as quote token
    /// From actual transaction analysis: EPiZbnrThjyLnoQ6QQzkxeFqyL5uyg9RzNHHAudUPxBz
    pub const USD1_GLOBAL_CONFIG: Pubkey = pubkey!("EPiZbnrThjyLnoQ6QQzkxeFqyL5uyg9RzNHHAudUPxBz");

    /// Known migrate_to_cpswap_wallet address (mainnet)
    /// This is the wallet that must be used as payer for migrate_to_cpswap instruction
    /// From transaction: 4NkRLPVhpr2EB9mxVtf2sP7Ftn1BfxBTPw6HgK1pkPeLNbnGtSVZdVtecVJwozEgKdM6C9TAT1S1LBRmQWaovJ1a
    pub const MIGRATE_TO_CPSWAP_WALLET: Pubkey =
        pubkey!("RAYpQbFNq9i3mu6cKpTKKRwwHFDeK5AuZz8xvxUrCgw");

    /// Known lock_lp_vault address (mainnet)
    /// Used for locking LP tokens during migration
    /// From transaction: 4NkRLPVhpr2EB9mxVtf2sP7Ftn1BfxBTPw6HgK1pkPeLNbnGtSVZdVtecVJwozEgKdM6C9TAT1S1LBRmQWaovJ1a
    /// Note: This might be a PDA or fixed address. If it's a PDA, we may need to calculate it dynamically.
    pub const LOCK_LP_VAULT: Pubkey = pubkey!("B26Asj7NX4pKnx7s3jrW6CuxaRYWq8HroceTRyoxTE7b");
}

/// Instruction discriminators from IDL
pub mod discriminators {
    /// buy_exact_in discriminator: [250, 234, 13, 123, 213, 156, 19, 236]
    pub const BUY_EXACT_IN: [u8; 8] = [250, 234, 13, 123, 213, 156, 19, 236];

    /// sell_exact_in discriminator: [149, 39, 222, 155, 211, 124, 152, 26]
    pub const SELL_EXACT_IN: [u8; 8] = [149, 39, 222, 155, 211, 124, 152, 26];

    /// initialize discriminator: [175, 175, 109, 31, 13, 251, 127, 237]
    pub const INITIALIZE: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];

    /// initialize_v2 discriminator: [67, 153, 175, 39, 218, 16, 38, 32]
    pub const INITIALIZE_V2: [u8; 8] = [67, 153, 175, 39, 218, 16, 38, 32];

    /// migrate_to_cpswap discriminator: [136, 92, 200, 103, 28, 218, 144, 140]
    pub const MIGRATE_TO_CPSWAP: [u8; 8] = [136, 92, 200, 103, 28, 218, 144, 140];
}
