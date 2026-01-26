use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub enum TradeDirection {
    #[default]
    Buy,
    Sell,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub enum PoolStatus {
    #[default]
    Fund,
    Migrate,
    Trade,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct MintParams {
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct VestingParams {
    pub total_locked_amount: u64,
    pub cliff_period: u64,
    pub unlock_period: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub enum AmmFeeOn {
    #[default]
    QuoteToken,
    BothToken,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct ConstantCurve {
    pub supply: u64,
    pub total_base_sell: u64,
    pub total_quote_fund_raising: u64,
    pub migrate_type: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct FixedCurve {
    pub supply: u64,
    pub total_quote_fund_raising: u64,
    pub migrate_type: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct LinearCurve {
    pub supply: u64,
    pub total_quote_fund_raising: u64,
    pub migrate_type: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub enum CurveParams {
    Constant { data: ConstantCurve },
    Fixed { data: FixedCurve },
    Linear { data: LinearCurve },
}
impl Default for CurveParams {
    fn default() -> Self {
        Self::Constant { data: ConstantCurve::default() }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct VestingSchedule {
    pub total_locked_amount: u64,
    pub cliff_period: u64,
    pub unlock_period: u64,
    pub start_time: u64,
    pub allocated_share_amount: u64,
}

// ==============
// BONKswap AMM types (from official IDL)
// ==============

/// Fixed point type used in BONKswap (u128 with 10^12 precision)
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedPoint {
    pub v: u128,
}

impl borsh::BorshDeserialize for FixedPoint {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let v = u128::deserialize_reader(reader)?;
        Ok(Self { v })
    }
}

/// Token amount type (u64)
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub v: u64,
}

impl borsh::BorshDeserialize for Token {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let v = u64::deserialize_reader(reader)?;
        Ok(Self { v })
    }
}

/// Product type for constant K (u128)
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    pub v: u128,
}

impl borsh::BorshDeserialize for Product {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let v = u128::deserialize_reader(reader)?;
        Ok(Self { v })
    }
}

/// BONKswap Pool State (from official IDL)
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoolState {
    // New field names (from official BONKswap IDL)
    pub token_x: Pubkey,              // 0:  Token X mint
    pub token_y: Pubkey,              // 32: Token Y mint
    pub pool_x_account: Pubkey,       // 64: Pool's Token X account
    pub pool_y_account: Pubkey,       // 96: Pool's Token Y account
    pub admin: Pubkey,                // 128: Admin pubkey
    pub project_owner: Pubkey,        // 160: Project owner pubkey
    pub token_x_reserve: Token,       // 192: Token X reserve amount
    pub token_y_reserve: Token,       // 200: Token Y reserve amount
    pub self_shares: Token,           // 208: Self shares
    pub all_shares: Token,            // 216: All shares
    pub buyback_amount_x: Token,      // 224: Buyback amount X
    pub buyback_amount_y: Token,      // 232: Buyback amount Y
    pub project_amount_x: Token,      // 240: Project amount X
    pub project_amount_y: Token,      // 248: Project amount Y
    pub mercanti_amount_x: Token,     // 256: Mercanti amount X
    pub mercanti_amount_y: Token,     // 264: Mercanti amount Y
    pub lp_accumulator_x: FixedPoint, // 272: LP accumulator X
    pub lp_accumulator_y: FixedPoint, // 280: LP accumulator Y
    pub const_k: Product,             // 288: Constant K (invariant)
    pub price: FixedPoint,            // 296: Current price (Y/X ratio)
    pub lp_fee: FixedPoint,           // 304: LP fee
    pub buyback_fee: FixedPoint,      // 312: Buyback fee
    pub project_fee: FixedPoint,      // 320: Project fee
    pub mercanti_fee: FixedPoint,     // 328: Mercanti fee
    pub farm_count: u64,              // 336: Farm count
    pub bump: u8,                     // 344: Bump seed

    // Old field names (for backward compatibility - calculated values)
    #[serde(skip)]
    pub base_mint: Pubkey, // Alias for token_x
    #[serde(skip)]
    pub quote_mint: Pubkey, // Alias for token_y
    #[serde(skip)]
    pub base_vault: Pubkey, // Alias for pool_x_account
    #[serde(skip)]
    pub quote_vault: Pubkey, // Alias for pool_y_account
    #[serde(skip)]
    pub creator: Pubkey, // Alias for admin
    #[serde(skip)]
    pub platform_config: Pubkey, // Alias for project_owner
    #[serde(skip)]
    pub global_config: Pubkey, // Alias for admin
    #[serde(skip)]
    pub virtual_base: u128, // Calculated: K / y
    #[serde(skip)]
    pub virtual_quote: u128, // Calculated: price * x
    #[serde(skip)]
    pub real_base: u64, // Alias for token_x_reserve.v
    #[serde(skip)]
    pub real_quote: u64, // Alias for token_y_reserve.v
    #[serde(skip)]
    pub supply: u64, // Alias for all_shares.v
    #[serde(skip)]
    pub total_base_sell: u64, // Not available in new format
    #[serde(skip)]
    pub status: u8, // Not available in new format
}

impl PoolState {
    /// Update backward compatibility fields after deserialization
    fn update_compat_fields(&mut self) {
        // Set old field names as aliases for new ones
        self.base_mint = self.token_x;
        self.quote_mint = self.token_y;
        self.base_vault = self.pool_x_account;
        self.quote_vault = self.pool_y_account;
        self.creator = self.admin;
        self.platform_config = self.project_owner;
        self.global_config = self.admin;
        self.real_base = self.token_x_reserve.v;
        self.real_quote = self.token_y_reserve.v;
        self.supply = self.all_shares.v;

        // Calculate virtual reserves from K and price
        // K = x * y (constant product)
        // virtual_x = K / y, virtual_y = price * x (with 10^12 precision)
        if self.token_y_reserve.v > 0 {
            self.virtual_base = self.const_k.v / self.token_y_reserve.v as u128;
        }
        // virtual_quote = price * x / 10^12
        self.virtual_quote = (self.price.v * self.token_x_reserve.v as u128) / (10_u128.pow(12));
        self.total_base_sell = 0; // Not available
        self.status = 2; // Trade status (2 = Trade)
    }
}

impl borsh::BorshDeserialize for PoolState {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut state = Self {
            token_x: Pubkey::deserialize_reader(reader)?,
            token_y: Pubkey::deserialize_reader(reader)?,
            pool_x_account: Pubkey::deserialize_reader(reader)?,
            pool_y_account: Pubkey::deserialize_reader(reader)?,
            admin: Pubkey::deserialize_reader(reader)?,
            project_owner: Pubkey::deserialize_reader(reader)?,
            token_x_reserve: Token::deserialize_reader(reader)?,
            token_y_reserve: Token::deserialize_reader(reader)?,
            self_shares: Token::deserialize_reader(reader)?,
            all_shares: Token::deserialize_reader(reader)?,
            buyback_amount_x: Token::deserialize_reader(reader)?,
            buyback_amount_y: Token::deserialize_reader(reader)?,
            project_amount_x: Token::deserialize_reader(reader)?,
            project_amount_y: Token::deserialize_reader(reader)?,
            mercanti_amount_x: Token::deserialize_reader(reader)?,
            mercanti_amount_y: Token::deserialize_reader(reader)?,
            lp_accumulator_x: FixedPoint::deserialize_reader(reader)?,
            lp_accumulator_y: FixedPoint::deserialize_reader(reader)?,
            const_k: Product::deserialize_reader(reader)?,
            price: FixedPoint::deserialize_reader(reader)?,
            lp_fee: FixedPoint::deserialize_reader(reader)?,
            buyback_fee: FixedPoint::deserialize_reader(reader)?,
            project_fee: FixedPoint::deserialize_reader(reader)?,
            mercanti_fee: FixedPoint::deserialize_reader(reader)?,
            farm_count: u64::deserialize_reader(reader)?,
            bump: u8::deserialize_reader(reader)?,
            // Initialize old fields
            base_mint: Pubkey::default(),
            quote_mint: Pubkey::default(),
            base_vault: Pubkey::default(),
            quote_vault: Pubkey::default(),
            creator: Pubkey::default(),
            platform_config: Pubkey::default(),
            global_config: Pubkey::default(),
            virtual_base: 0,
            virtual_quote: 0,
            real_base: 0,
            real_quote: 0,
            supply: 0,
            total_base_sell: 0,
            status: 0,
        };

        // Update backward compatibility fields
        state.update_compat_fields();

        Ok(state)
    }
}

/// Old PoolState (for backward compatibility with existing code)
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, BorshDeserialize)]
pub struct OldPoolState {
    pub epoch: u64,
    pub auth_bump: u8,
    pub status: u8,
    pub base_decimals: u8,
    pub quote_decimals: u8,
    pub migrate_type: u8,
    pub supply: u64,
    pub total_base_sell: u64,
    pub virtual_base: u64,
    pub virtual_quote: u64,
    pub real_base: u64,
    pub real_quote: u64,
    pub total_quote_fund_raising: u64,
    pub quote_protocol_fee: u64,
    pub platform_fee: u64,
    pub migrate_fee: u64,
    pub vesting_schedule: VestingSchedule,
    pub global_config: Pubkey,
    pub platform_config: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub creator: Pubkey,
    pub padding: [u64; 8],
}

/// Size of BONKswap PoolState
pub const BONK_POOL_STATE_SIZE: usize = 345;

/// Size of old PoolState
pub const OLD_POOL_STATE_SIZE: usize = 8 + 5 + 8 * 10 + 32 * 7 + 8 * 8 + 8 * 5;

/// Try to decode pool state, supporting both BONKswap and old formats
pub fn pool_state_decode(data: &[u8]) -> Option<PoolState> {
    if data.len() < OLD_POOL_STATE_SIZE {
        // Try BONKswap format first if data is large enough
        if data.len() >= BONK_POOL_STATE_SIZE {
            return pool_state_decode_bonkswap(data);
        }
        return None;
    }

    // If data is large enough for old format, try it
    pool_state_decode_old(data)
}

/// Decode BONKswap format pool state using manual deserialization
fn pool_state_decode_bonkswap(data: &[u8]) -> Option<PoolState> {
    let mut reader = std::io::Cursor::new(data);
    PoolState::deserialize_reader(&mut reader).ok()
}

/// Decode old format pool state
fn pool_state_decode_old(data: &[u8]) -> Option<PoolState> {
    if data.len() < OLD_POOL_STATE_SIZE {
        return None;
    }

    let old: OldPoolState = borsh::from_slice(&data[..OLD_POOL_STATE_SIZE]).ok()?;

    // Convert old format to new format
    Some(PoolState {
        token_x: old.base_mint,
        token_y: old.quote_mint,
        pool_x_account: old.base_vault,
        pool_y_account: old.quote_vault,
        admin: old.global_config,
        project_owner: old.platform_config,
        token_x_reserve: Token { v: old.real_base },
        token_y_reserve: Token { v: old.real_quote },
        self_shares: Token { v: 0 },
        all_shares: Token { v: old.supply },
        buyback_amount_x: Token { v: 0 },
        buyback_amount_y: Token { v: 0 },
        project_amount_x: Token { v: 0 },
        project_amount_y: Token { v: 0 },
        mercanti_amount_x: Token { v: 0 },
        mercanti_amount_y: Token { v: 0 },
        lp_accumulator_x: FixedPoint { v: 0 },
        lp_accumulator_y: FixedPoint { v: 0 },
        const_k: Product { v: 0 },
        price: FixedPoint { v: 0 },
        lp_fee: FixedPoint { v: 0 },
        buyback_fee: FixedPoint { v: 0 },
        project_fee: FixedPoint { v: 0 },
        mercanti_fee: FixedPoint { v: 0 },
        farm_count: 0,
        bump: old.auth_bump,
        // Old compatibility fields
        base_mint: old.base_mint,
        quote_mint: old.quote_mint,
        base_vault: old.base_vault,
        quote_vault: old.quote_vault,
        creator: old.creator,
        platform_config: old.platform_config,
        global_config: old.global_config,
        virtual_base: old.virtual_base as u128,
        virtual_quote: old.virtual_quote as u128,
        real_base: old.real_base,
        real_quote: old.real_quote,
        supply: old.supply,
        total_base_sell: old.total_base_sell,
        status: old.status,
    })
}
