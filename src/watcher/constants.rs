use anchor_client::solana_sdk::pubkey::Pubkey;
use serde::{Deserialize, Deserializer};
use std::fs;
use std::str::FromStr;

fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
struct Program {
    #[serde(deserialize_with = "deserialize_pubkey")]
    address: Pubkey,
    name: String,
    program_type: String,
    dex: bool,
}

#[derive(Debug, Deserialize)]
struct WatchPrograms {
    #[serde(default)]
    programs: Vec<Program>,
}

fn _load_programs(path: &str) -> anyhow::Result<Vec<(Pubkey, String, Option<String>, bool)>> {
    let raw = fs::read_to_string(path)?;
    let data: WatchPrograms = toml::from_str(&raw)?;

    let programs: Vec<(Pubkey, String, Option<String>, bool)> = data
        .programs
        .into_iter()
        .map(|program| {
            (
                program.address,
                program.name,
                Some(program.program_type),
                program.dex,
            )
        })
        .collect();

    Ok(programs)
}

lazy_static::lazy_static! {
    pub static ref PROGRAMS_TO_WATCH: Vec<(Pubkey, String, Option<String>, bool)> = _load_programs("programs.toml").unwrap();
}
