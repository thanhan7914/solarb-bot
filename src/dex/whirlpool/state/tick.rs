use super::*;

pub const MAX_TICK_INDEX: i32 = 443636;
pub const MIN_TICK_INDEX: i32 = -443636;

#[derive(Debug, Default, Clone, PartialEq, Eq, Copy)]
pub struct Tick {
    pub initialized: bool,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub fee_growth_outside_a: u128,
    pub fee_growth_outside_b: u128,
    pub reward_growths_outside: [u128; 3],
}

impl Tick {
    pub fn deserialize(reader: &mut ByteReader) -> Result<Self> {
        Ok(Self {
            initialized: reader.read_u8()? != 0,
            liquidity_net: {
                let bytes = reader.read_bytes_array::<16>()?;
                i128::from_le_bytes(bytes)
            },
            liquidity_gross: reader.read_u128()?,
            fee_growth_outside_a: reader.read_u128()?,
            fee_growth_outside_b: reader.read_u128()?,
            reward_growths_outside: [
                reader.read_u128()?,
                reader.read_u128()?,
                reader.read_u128()?,
            ],
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct TickRange {
    pub tick_lower_index: i32,
    pub tick_upper_index: i32,
}