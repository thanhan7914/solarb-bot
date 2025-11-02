use anchor_client::solana_sdk::{instruction::Instruction, pubkey::Pubkey};
use anyhow::Result;
use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction as token_instruction;

pub fn token_transfer_instruction(
    sender: &Pubkey,
    recipient: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Result<Instruction> {
    let token_program_id = spl_token::id();
    let sender_ata = get_associated_token_address(sender, mint);
    let recipient_ata = get_associated_token_address(recipient, mint);
    let transfer_amount = amount * 10_u64.pow(decimals as u32);

    let instruction = token_instruction::transfer(
        &token_program_id,
        &sender_ata,
        &recipient_ata,
        authority,
        &[],
        transfer_amount,
    )?;

    Ok(instruction)
}

pub fn create_ata_token_instruction(
    payer: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Instruction> {
    let token_program_id = spl_token::id();
    let instruction = spl_associated_token_account::instruction::create_associated_token_account(
        payer,
        owner,
        mint,
        &token_program_id,
    );

    Ok(instruction)
}
