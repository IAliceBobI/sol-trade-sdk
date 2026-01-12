// Copyright (c) Raydium Foundation
// Licensed under Apache 2.0
// Source: https://github.com/raydium-io/raydium-clmm/programs/amm/src/libraries/big_num.rs
// Simplified for client-side use - only U128/U256/U512 types, macro removed

///! 128 and 256 bit numbers
///! U128 is more efficient that u128
///! https://github.com/solana-labs/solana/issues/19549
use uint::construct_uint;

construct_uint! {
    pub struct U128(2);
}

construct_uint! {
    pub struct U256(4);
}

construct_uint! {
    pub struct U512(8);
}
