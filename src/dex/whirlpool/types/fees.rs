#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct CollectFeesQuote {
    pub fee_owed_a: u64,
    pub fee_owed_b: u64,
}
