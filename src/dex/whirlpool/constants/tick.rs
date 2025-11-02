#![allow(non_snake_case)]

pub const TICK_ARRAY_SIZE: usize = 88;

/// Pools with tick spacing above this threshold are considered full range only.
/// This means the program rejects any non-full range positions in these pools.
pub const FULL_RANGE_ONLY_TICK_SPACING_THRESHOLD: u16 = 32768; // 2^15

/// The minimum tick index.
pub const MIN_TICK_INDEX: i32 = -443636;

/// The maximum tick index.
pub const MAX_TICK_INDEX: i32 = 443636;
