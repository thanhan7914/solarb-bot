use super::{
    CoreError, INVALID_TICK_ARRAY_SEQUENCE, INVALID_TICK_INDEX, MAX_TICK_INDEX, MIN_TICK_INDEX,
    TICK_ARRAY_NOT_EVENLY_SPACED, TICK_ARRAY_SIZE, TICK_INDEX_OUT_OF_BOUNDS, TICK_SEQUENCE_EMPTY,
    state::tick::*, state::tick_array::*,
};

use super::{
    get_initializable_tick_index, get_next_initializable_tick_index,
    get_prev_initializable_tick_index,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickArraySequence<const SIZE: usize> {
    pub tick_arrays: [Option<TickArray>; SIZE],
    pub tick_spacing: u16,
}

impl<const SIZE: usize> TickArraySequence<SIZE> {
    pub fn new(
        tick_arrays: [Option<TickArray>; SIZE],
        tick_spacing: u16,
    ) -> Result<Self, CoreError> {
        let mut tick_arrays = tick_arrays;
        tick_arrays.sort_by_key(start_tick_index);

        if tick_arrays.is_empty() || tick_arrays[0].is_none() {
            return Err(TICK_SEQUENCE_EMPTY);
        }

        let required_tick_array_spacing = TICK_ARRAY_SIZE as i32 * tick_spacing as i32;
        for i in 0..tick_arrays.len() - 1 {
            let current_start_tick_index = start_tick_index(&tick_arrays[i]);
            let next_start_tick_index = start_tick_index(&tick_arrays[i + 1]);
            if next_start_tick_index != <i32>::MAX
                && next_start_tick_index - current_start_tick_index != required_tick_array_spacing
            {
                return Err(TICK_ARRAY_NOT_EVENLY_SPACED);
            }
        }

        Ok(Self {
            tick_arrays,
            tick_spacing,
        })
    }

    /// Returns the first valid tick index in the sequence.
    pub fn start_index(&self) -> i32 {
        start_tick_index(&self.tick_arrays[0]).max(MIN_TICK_INDEX)
    }

    /// Returns the last valid tick index in the sequence.
    pub fn end_index(&self) -> i32 {
        let mut last_valid_start_index = self.start_index();
        for i in 0..self.tick_arrays.len() {
            if start_tick_index(&self.tick_arrays[i]) != <i32>::MAX {
                last_valid_start_index = start_tick_index(&self.tick_arrays[i]);
            }
        }
        let end_index =
            last_valid_start_index + TICK_ARRAY_SIZE as i32 * self.tick_spacing as i32 - 1;
        end_index.min(MAX_TICK_INDEX)
    }

    pub fn tick(&self, tick_index: i32) -> Result<&Tick, CoreError> {
        if (tick_index < self.start_index()) || (tick_index > self.end_index()) {
            return Err(TICK_INDEX_OUT_OF_BOUNDS);
        }
        if (tick_index % self.tick_spacing as i32) != 0 {
            return Err(INVALID_TICK_INDEX);
        }
        let first_index = start_tick_index(&self.tick_arrays[0]);
        let tick_array_index = ((tick_index - first_index)
            / (TICK_ARRAY_SIZE as i32 * self.tick_spacing as i32))
            as usize;
        let tick_array_start_index = start_tick_index(&self.tick_arrays[tick_array_index]);
        let tick_array_ticks = ticks(&self.tick_arrays[tick_array_index]);
        let index_in_array = (tick_index - tick_array_start_index) / self.tick_spacing as i32;
        Ok(&tick_array_ticks[index_in_array as usize])
    }

    pub fn next_initialized_tick(
        &self,
        tick_index: i32,
    ) -> Result<(Option<&Tick>, i32), CoreError> {
        let array_end_index = self.end_index();
        if tick_index >= array_end_index {
            return Err(INVALID_TICK_ARRAY_SEQUENCE);
        }
        let mut next_index = tick_index;
        loop {
            next_index = get_next_initializable_tick_index(next_index, self.tick_spacing);
            // If at the end of the sequence, we don't have tick info but can still return the next tick index
            if next_index > array_end_index {
                return Ok((None, array_end_index));
            }
            let tick = self.tick(next_index)?;
            if tick.initialized {
                return Ok((Some(tick), next_index));
            }
        }
    }

    pub fn prev_initialized_tick(
        &self,
        tick_index: i32,
    ) -> Result<(Option<&Tick>, i32), CoreError> {
        let array_start_index = self.start_index();
        if tick_index < array_start_index {
            return Err(INVALID_TICK_ARRAY_SEQUENCE);
        }
        let mut prev_index =
            get_initializable_tick_index(tick_index, self.tick_spacing, Some(false));
        loop {
            // If at the start of the sequence, we don't have tick info but can still return the previous tick index
            if prev_index < array_start_index {
                return Ok((None, array_start_index));
            }
            let tick = self.tick(prev_index)?;
            if tick.initialized {
                return Ok((Some(tick), prev_index));
            }
            prev_index = get_prev_initializable_tick_index(prev_index, self.tick_spacing);
        }
    }
}

// internal functions

fn start_tick_index(tick_array: &Option<TickArray>) -> i32 {
    if let Some(tick_array) = tick_array {
        tick_array.start_tick_index
    } else {
        <i32>::MAX
    }
}

fn ticks(tick_array: &Option<TickArray>) -> &[Tick] {
    if let Some(tick_array) = tick_array {
        &tick_array.ticks
    } else {
        &[]
    }
}
