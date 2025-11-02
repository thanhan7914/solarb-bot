use anyhow::{anyhow, Result};

// Helper function for ceiling division
#[inline]
pub fn ceil_div(a: u128, b: u128) -> Result<u128> {
    if b == 0 {
        return Err(anyhow!("Cannot divide by zero."));
    }
    Ok((a + b - 1) / b)
}

// Calculate fee using basis points
#[inline]
pub fn fee(amount: u128, basis_points: u128) -> Result<u128> {
    ceil_div(amount * basis_points, 10_000)
}
