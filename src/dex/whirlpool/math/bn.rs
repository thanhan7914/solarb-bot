#![allow(clippy::assign_op_pattern)]
#![allow(clippy::ptr_offset_with_cast)]
#![allow(clippy::manual_range_contains)]

use anyhow::{Result, anyhow};
/// The following code is referenced from drift-labs:
/// https://github.com/drift-labs/protocol-v1/blob/3da78f1f03b66a273fc50818323ac62874abd1d8/programs/clearing_house/src/math/bn.rs
///
/// Based on parity's uint crate
/// https://github.com/paritytech/parity-common/tree/master/uint
///
/// Note: We cannot use U256 from primitive-types (default u256 from parity's uint) because we need to extend the U256 struct to
/// support the Borsh serial/deserialize traits.
///
/// The reason why this custom U256 impl does not directly impl TryInto traits is because of this:
/// https://stackoverflow.com/questions/37347311/how-is-there-a-conflicting-implementation-of-from-when-using-a-generic-type
///
/// As a result, we have to define our own custom Into methods
///
/// U256 reference:
/// https://crates.parity.io/sp_core/struct.U256.html
///
use std::borrow::BorrowMut;
use std::convert::TryInto;
use std::io::{Error, ErrorKind, Write};
use std::mem::size_of;
use uint::construct_uint;

construct_uint! {
    // U256 of [u64; 4]
    pub struct U256(4);
}

impl U256 {
    pub fn try_into_u64(self) -> Result<u64> {
        self.try_into().map_err(|_| anyhow!("NumberCastError"))
    }

    pub fn try_into_u128(self) -> Result<u128> {
        self.try_into().map_err(|_| anyhow!("NumberCastError"))
    }

    pub fn from_le_bytes(bytes: [u8; 32]) -> Self {
        U256::from_little_endian(&bytes)
    }

    pub fn to_le_bytes(self) -> [u8; 32] {
        let mut buf: Vec<u8> = Vec::with_capacity(size_of::<Self>());
        self.to_little_endian(buf.borrow_mut());

        let mut bytes: [u8; 32] = [0u8; 32];
        bytes.copy_from_slice(buf.as_slice());
        bytes
    }
}

