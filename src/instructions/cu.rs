use anchor_client::solana_sdk::{
    compute_budget::ComputeBudgetInstruction, instruction::Instruction,
};

pub fn limit_instruction(units: u32) -> Instruction {
    ComputeBudgetInstruction::set_compute_unit_limit(units)
}

pub fn price_instruction(micro_lamports: u64) -> Instruction {
    ComputeBudgetInstruction::set_compute_unit_price(micro_lamports)
}

pub fn loaded_accounts_data_size_limit_instruction(bytes: u32) -> Instruction {
    ComputeBudgetInstruction::set_loaded_accounts_data_size_limit(bytes)
}
