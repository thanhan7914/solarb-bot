#![allow(non_snake_case)]

pub type CoreError = &'static str;

pub const TICK_ARRAY_NOT_EVENLY_SPACED: CoreError = "Tick array not evenly spaced";

pub const TICK_INDEX_OUT_OF_BOUNDS: CoreError = "Tick index out of bounds";

pub const INVALID_TICK_INDEX: CoreError = "Invalid tick index";

pub const ARITHMETIC_OVERFLOW: CoreError = "Arithmetic over- or underflow";

pub const AMOUNT_EXCEEDS_MAX_U64: CoreError = "Amount exceeds max u64";

pub const SQRT_PRICE_OUT_OF_BOUNDS: CoreError = "Sqrt price out of bounds";

pub const TICK_SEQUENCE_EMPTY: CoreError = "Tick sequence empty";

pub const SQRT_PRICE_LIMIT_OUT_OF_BOUNDS: CoreError = "Sqrt price limit out of bounds";

pub const INVALID_SQRT_PRICE_LIMIT_DIRECTION: CoreError = "Invalid sqrt price limit direction";

pub const ZERO_TRADABLE_AMOUNT: CoreError = "Zero tradable amount";

pub const INVALID_TIMESTAMP: CoreError = "Invalid timestamp";

pub const INVALID_TRANSFER_FEE: CoreError = "Invalid transfer fee";

pub const INVALID_SLIPPAGE_TOLERANCE: CoreError = "Invalid slippage tolerance";

pub const TICK_INDEX_NOT_IN_ARRAY: CoreError = "Tick index not in array";

pub const INVALID_TICK_ARRAY_SEQUENCE: CoreError = "Invalid tick array sequence";

pub const INVALID_ADAPTIVE_FEE_INFO: CoreError = "Invalid adaptive fee info";
