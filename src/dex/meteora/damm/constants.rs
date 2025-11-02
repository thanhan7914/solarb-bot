pub const Q64_64_SCALE: f64 = 18446744073709551616.0; // 2^64

pub mod fee {
    /// Default fee denominator. DO NOT simply update it as it will break logic that depends on it as default value.
    pub const FEE_DENOMINATOR: u64 = 1_000_000_000;

    /// Max fee BPS
    pub const MAX_FEE_BPS: u64 = 5000; // 50%
    pub const MAX_FEE_NUMERATOR: u64 = 500_000_000; // 50%

    /// Max basis point. 100% in pct
    pub const MAX_BASIS_POINT: u64 = 10000;

    pub const MIN_FEE_BPS: u64 = 1; // 0.01%
    pub const MIN_FEE_NUMERATOR: u64 = 100_000;
}
