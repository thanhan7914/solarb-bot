use super::*;
use super::tick::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickArray {
    pub start_tick_index: i32,
    pub ticks: [Tick; 88],
    pub whirlpool: Pubkey,
}

impl TickArray {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(anyhow!("Data too short for account discriminator"));
        }

        let mut reader = ByteReader::new(&data[8..]);

        let start_tick_index = {
            let bytes = reader.read_bytes_array::<4>()?;
            i32::from_le_bytes(bytes)
        };

        // Read 88 ticks
        let mut ticks = Vec::with_capacity(88);
        for _ in 0..88 {
            ticks.push(Tick::deserialize(&mut reader)?);
        }

        let whirlpool = reader.read_pubkey()?;

        // Convert Vec to array
        let ticks_array: [Tick; 88] = ticks
            .try_into()
            .map_err(|_| anyhow!("Failed to convert ticks vector to array"))?;

        Ok(Self {
            start_tick_index,
            ticks: ticks_array,
            whirlpool,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickArrays {
    One(TickArray),
    Two(TickArray, TickArray),
    Three(TickArray, TickArray, TickArray),
    Four(
        TickArray,
        TickArray,
        TickArray,
        TickArray,
    ),
    Five(
        TickArray,
        TickArray,
        TickArray,
        TickArray,
        TickArray,
    ),
    Six(
        TickArray,
        TickArray,
        TickArray,
        TickArray,
        TickArray,
        TickArray,
    ),
}

impl TickArrays {
    pub fn into_array(self) -> [Option<TickArray>; 6] {
        match self {
            TickArrays::One(a) => [Some(a), None, None, None, None, None],
            TickArrays::Two(a, b) => [Some(a), Some(b), None, None, None, None],
            TickArrays::Three(a, b, c) => [Some(a), Some(b), Some(c), None, None, None],
            TickArrays::Four(a, b, c, d) => [Some(a), Some(b), Some(c), Some(d), None, None],
            TickArrays::Five(a, b, c, d, e) => [Some(a), Some(b), Some(c), Some(d), Some(e), None],
            TickArrays::Six(a, b, c, d, e, f) => [Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)],
        }
    }
}

impl From<TickArrays> for [Option<TickArray>; 1] {
    fn from(tick_arrays: TickArrays) -> Self {
        match tick_arrays {
            TickArrays::One(ta1) => [Some(ta1)],
            _ => [None], // If more than 1, just take the first or return None
        }
    }
}

impl From<TickArrays> for [Option<TickArray>; 2] {
    fn from(tick_arrays: TickArrays) -> Self {
        match tick_arrays {
            TickArrays::One(ta1) => [Some(ta1), None],
            TickArrays::Two(ta1, ta2) => [Some(ta1), Some(ta2)],
            _ => [None, None],
        }
    }
}

impl From<TickArrays> for [Option<TickArray>; 3] {
    fn from(tick_arrays: TickArrays) -> Self {
        match tick_arrays {
            TickArrays::One(ta1) => [Some(ta1), None, None],
            TickArrays::Two(ta1, ta2) => [Some(ta1), Some(ta2), None],
            TickArrays::Three(ta1, ta2, ta3) => [Some(ta1), Some(ta2), Some(ta3)],
            _ => [None, None, None],
        }
    }
}

impl From<TickArrays> for [Option<TickArray>; 4] {
    fn from(tick_arrays: TickArrays) -> Self {
        match tick_arrays {
            TickArrays::One(ta1) => [Some(ta1), None, None, None],
            TickArrays::Two(ta1, ta2) => [Some(ta1), Some(ta2), None, None],
            TickArrays::Three(ta1, ta2, ta3) => [Some(ta1), Some(ta2), Some(ta3), None],
            TickArrays::Four(ta1, ta2, ta3, ta4) => [Some(ta1), Some(ta2), Some(ta3), Some(ta4)],
            _ => [None, None, None, None],
        }
    }
}

impl From<TickArrays> for [Option<TickArray>; 5] {
    fn from(tick_arrays: TickArrays) -> Self {
        match tick_arrays {
            TickArrays::One(ta1) => [Some(ta1), None, None, None, None],
            TickArrays::Two(ta1, ta2) => [Some(ta1), Some(ta2), None, None, None],
            TickArrays::Three(ta1, ta2, ta3) => [Some(ta1), Some(ta2), Some(ta3), None, None],
            TickArrays::Four(ta1, ta2, ta3, ta4) => [Some(ta1), Some(ta2), Some(ta3), Some(ta4), None],
            TickArrays::Five(ta1, ta2, ta3, ta4, ta5) => [Some(ta1), Some(ta2), Some(ta3), Some(ta4), Some(ta5)],
            _ => [None, None, None, None, None],
        }
    }
}

impl From<TickArrays> for [Option<TickArray>; 6] {
    fn from(tick_arrays: TickArrays) -> Self {
        match tick_arrays {
            TickArrays::One(ta1) => [Some(ta1), None, None, None, None, None],
            TickArrays::Two(ta1, ta2) => [Some(ta1), Some(ta2), None, None, None, None],
            TickArrays::Three(ta1, ta2, ta3) => [Some(ta1), Some(ta2), Some(ta3), None, None, None],
            TickArrays::Four(ta1, ta2, ta3, ta4) => [Some(ta1), Some(ta2), Some(ta3), Some(ta4), None, None],
            TickArrays::Five(ta1, ta2, ta3, ta4, ta5) => [Some(ta1), Some(ta2), Some(ta3), Some(ta4), Some(ta5), None],
            TickArrays::Six(ta1, ta2, ta3, ta4, ta5, ta6) => [Some(ta1), Some(ta2), Some(ta3), Some(ta4), Some(ta5), Some(ta6)],
        }
    }
}

impl From<[TickArray; 1]> for TickArrays {
    fn from(arr: [TickArray; 1]) -> Self {
        let [ta1] = arr;
        TickArrays::One(ta1)
    }
}

impl From<[TickArray; 2]> for TickArrays {
    fn from(arr: [TickArray; 2]) -> Self {
        let [ta1, ta2] = arr;
        TickArrays::Two(ta1, ta2)
    }
}

impl From<[TickArray; 3]> for TickArrays {
    fn from(arr: [TickArray; 3]) -> Self {
        let [ta1, ta2, ta3] = arr;
        TickArrays::Three(ta1, ta2, ta3)
    }
}

impl From<[TickArray; 4]> for TickArrays {
    fn from(arr: [TickArray; 4]) -> Self {
        let [ta1, ta2, ta3, ta4] = arr;
        TickArrays::Four(ta1, ta2, ta3, ta4)
    }
}

impl From<[TickArray; 5]> for TickArrays {
    fn from(arr: [TickArray; 5]) -> Self {
        let [ta1, ta2, ta3, ta4, ta5] = arr;
        TickArrays::Five(ta1, ta2, ta3, ta4, ta5)
    }
}

impl From<[TickArray; 6]> for TickArrays {
    fn from(arr: [TickArray; 6]) -> Self {
        let [ta1, ta2, ta3, ta4, ta5, ta6] = arr;
        TickArrays::Six(ta1, ta2, ta3, ta4, ta5, ta6)
    }
}

// Helper function to convert tuple array to TickArrays
impl From<[(Pubkey, TickArray); 5]> for TickArrays {
    fn from(tick_data: [(Pubkey, TickArray); 5]) -> Self {
        // Extract TickArray values from tuples
        let tick_arrays: [TickArray; 5] = tick_data.map(|(_, tick_array)| tick_array);
        // Convert array to TickArrays enum
        tick_arrays.into()
    }
}

// Alternative implementations for other sizes
impl From<[(Pubkey, TickArray); 3]> for TickArrays {
    fn from(tick_data: [(Pubkey, TickArray); 3]) -> Self {
        let tick_arrays: [TickArray; 3] = tick_data.map(|(_, tick_array)| tick_array);
        tick_arrays.into()
    }
}

impl From<[(Pubkey, TickArray); 6]> for TickArrays {
    fn from(tick_data: [(Pubkey, TickArray); 6]) -> Self {
        let tick_arrays: [TickArray; 6] = tick_data.map(|(_, tick_array)| tick_array);
        tick_arrays.into()
    }
}
