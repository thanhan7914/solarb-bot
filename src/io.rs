use anchor_client::solana_sdk::signature::{Keypair, read_keypair_file};
use anyhow::{Result, bail};
use bs58;
use std::fs;

pub fn load_key_pair_from_bs58(path: &str) -> Result<Keypair> {
    let b58_str = fs::read_to_string(path)?.trim().to_string();

    let bytes = bs58::decode(b58_str).into_vec()?;
    if bytes.len() != 64 {
        bail!("Invalid secret key");
    }

    let payer = Keypair::from_bytes(&bytes)?;
    Ok(payer)
}

pub fn load_keypair(path: &str) -> Result<Keypair> {
    let keypair = read_keypair_file(String::from(path));
    match keypair {
        std::result::Result::Ok(val) => Ok(val),
        Err(_) => load_key_pair_from_bs58(path),
    }
}
