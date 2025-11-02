use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::Result;
use std::str::FromStr;

#[inline]
pub fn pubkeys_to_strings(pubkeys: &[Pubkey]) -> Vec<String> {
    pubkeys.iter().map(|pk| pk.to_string()).collect()
}

#[inline]
pub fn strings_to_pubkeys(
    pubkeys: &[String],
) -> Result<Vec<Pubkey>> {
    pubkeys
        .iter()
        .map(|s| Pubkey::from_str(s).map_err(|e| e.into()))
        .collect()
}

#[inline]
pub fn strings_to_pubkeys_unwrap(pubkeys: &[String]) -> Vec<Pubkey> {
    pubkeys
        .iter()
        .map(|s| Pubkey::from_str(s).unwrap())
        .collect()
}

#[inline]
pub fn concat<T: Clone>(vec1: &Vec<T>, vec2: &Vec<T>) -> Vec<T> {
    let mut result = Vec::with_capacity(vec1.len() + vec2.len());
    result.extend_from_slice(vec1);
    result.extend_from_slice(vec2);
    result
}

#[inline]
pub fn merge<T: Clone>(vecs: &[&Vec<T>]) -> Vec<T> {
    let total_len: usize = vecs.iter().map(|v| v.len()).sum();
    let mut result = Vec::with_capacity(total_len);

    for vec in vecs {
        result.extend_from_slice(vec);
    }

    result
}
