use super::lookuptable::LookupTableCache;
use crate::{usdc_mint, wsol_mint};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct EnhancedTransactionInfo {
    pub signature: String,
    pub slot: u64,
    pub block_time: Option<i64>,
    pub program_ids: Vec<Pubkey>,
    pub success: bool,
    pub fee: Option<u64>,
    pub logs: Vec<String>,
    pub err: Option<Value>,
    pub is_arbitrage: bool,

    // Account information
    pub all_accounts: Vec<AccountInfo>,
    pub writable_accounts: Vec<Pubkey>,
    pub signer_accounts: Vec<Pubkey>,

    // Lookup table accounts
    pub lookup_table_accounts: Vec<LookupTableAccount>,

    // Token balance changes (ONLY for signers)
    pub pre_token_balances: Vec<TokenBalance>,
    pub post_token_balances: Vec<TokenBalance>,
    pub signer_token_balance_changes: Vec<TokenBalanceChange>,

    // SOL balance changes (ONLY for signers)
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub signer_balance_changes: Vec<BalanceChange>,

    // Compute units
    pub compute_units_consumed: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct AccountInfo {
    pub pubkey: Pubkey,
    pub index: usize,
    pub is_signer: bool,
    pub is_writable: bool,
    pub is_executable: bool,
    pub owner: Option<Pubkey>,
    pub lamports: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct LookupTableAccount {
    pub address: Pubkey,
    pub accounts: Vec<Pubkey>,
}

#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub account_index: usize,
    pub account: Pubkey,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub amount: String,
    pub decimals: u8,
    pub ui_amount: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TokenBalanceChange {
    pub account: Pubkey,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub pre_amount: String,
    pub post_amount: String,
    pub change_amount: i128,
    pub decimals: u8,
    pub ui_change: Option<f64>,
    pub is_signer: bool,
}

#[derive(Debug, Clone)]
pub struct BalanceChange {
    pub account: Pubkey,
    pub pre_balance: u64,
    pub post_balance: u64,
    pub change: i64,
    pub is_signer: bool,
}

pub async fn fetch_transaction_details(
    rpc_endpoint: &str,
    signature: &str,
) -> Result<(EnhancedTransactionInfo, Option<Value>)> {
    let client = reqwest::Client::new();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            signature,
            {
                "encoding": "json",
                "commitment": "confirmed",
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    let response: Value = client
        .post(rpc_endpoint)
        .json(&request)
        .send()
        .await?
        .json()
        .await?;

    let mut alt_accounts: Option<Value> = None;

    if let Some(result) = response.get("result") {
        if result.is_null() {
            return Err(anyhow!("Transaction not found"));
        }

        let slot = result.get("slot").and_then(|s| s.as_u64()).unwrap_or(0);
        let block_time = result.get("blockTime").and_then(|bt| bt.as_i64());

        let mut enhanced_info = EnhancedTransactionInfo {
            signature: signature.to_string(),
            slot,
            block_time,
            program_ids: Vec::new(),
            success: true,
            fee: None,
            logs: Vec::new(),
            err: None,
            is_arbitrage: false,
            all_accounts: Vec::new(),
            writable_accounts: Vec::new(),
            signer_accounts: Vec::new(),
            lookup_table_accounts: Vec::new(),
            pre_token_balances: Vec::new(),
            post_token_balances: Vec::new(),
            signer_token_balance_changes: Vec::new(),
            pre_balances: Vec::new(),
            post_balances: Vec::new(),
            signer_balance_changes: Vec::new(),
            compute_units_consumed: None,
        };

        if let Some(meta) = result.get("meta") {
            enhanced_info.fee = meta.get("fee").and_then(|f| f.as_u64());
            enhanced_info.success = meta.get("err").map_or(true, |e| e.is_null());
            enhanced_info.err = meta.get("err").cloned();

            if let Some(log_messages) = meta.get("logMessages") {
                if let Some(log_array) = log_messages.as_array() {
                    enhanced_info.logs = log_array
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect();
                }
            }

            if let Some(pre_balances) = meta.get("preBalances") {
                if let Some(pre_array) = pre_balances.as_array() {
                    enhanced_info.pre_balances =
                        pre_array.iter().filter_map(|v| v.as_u64()).collect();
                }
            }

            if let Some(post_balances) = meta.get("postBalances") {
                if let Some(post_array) = post_balances.as_array() {
                    enhanced_info.post_balances =
                        post_array.iter().filter_map(|v| v.as_u64()).collect();
                }
            }

            enhanced_info.pre_token_balances = parse_token_balances(meta.get("preTokenBalances"))?;
            enhanced_info.post_token_balances =
                parse_token_balances(meta.get("postTokenBalances"))?;

            if let Some(compute_units) = meta.get("computeUnitsConsumed") {
                enhanced_info.compute_units_consumed = compute_units.as_u64();
            }
        }

        if let Some(transaction) = result.get("transaction") {
            if let Some(message) = transaction.get("message") {
                let mut all_accounts = Vec::new();
                if let Some(account_keys) = message.get("accountKeys") {
                    if let Some(keys_array) = account_keys.as_array() {
                        for key in keys_array {
                            if let Some(key_str) = key.as_str() {
                                if let Ok(pubkey) = Pubkey::from_str(key_str) {
                                    all_accounts.push(pubkey);
                                }
                            }
                        }
                    }
                }

                if let Some(header) = message.get("header") {
                    let num_required_signatures = header
                        .get("numRequiredSignatures")
                        .and_then(|n| n.as_u64())
                        .unwrap_or(0) as usize;

                    let num_readonly_signed_accounts = header
                        .get("numReadonlySignedAccounts")
                        .and_then(|n| n.as_u64())
                        .unwrap_or(0)
                        as usize;

                    let num_readonly_unsigned_accounts = header
                        .get("numReadonlyUnsignedAccounts")
                        .and_then(|n| n.as_u64())
                        .unwrap_or(0)
                        as usize;

                    for (i, account) in all_accounts.iter().enumerate() {
                        let is_signer = i < num_required_signatures;
                        let is_writable = if is_signer {
                            i < (num_required_signatures - num_readonly_signed_accounts)
                        } else {
                            i < (all_accounts.len() - num_readonly_unsigned_accounts)
                        };

                        if is_signer {
                            enhanced_info.signer_accounts.push(*account);
                        }
                        if is_writable {
                            enhanced_info.writable_accounts.push(*account);
                        }

                        enhanced_info.all_accounts.push(AccountInfo {
                            pubkey: *account,
                            index: i,
                            is_signer,
                            is_writable,
                            is_executable: false,
                            owner: None,
                            lamports: enhanced_info.pre_balances.get(i).copied(),
                        });
                    }
                }

                if let Some(instructions) = message.get("instructions") {
                    if let Some(inst_array) = instructions.as_array() {
                        for instruction in inst_array {
                            if let Some(program_id_index) = instruction.get("programIdIndex") {
                                if let Some(index) = program_id_index.as_u64() {
                                    if (index as usize) < all_accounts.len() {
                                        let program_id = all_accounts[index as usize];
                                        if !enhanced_info.program_ids.contains(&program_id) {
                                            enhanced_info.program_ids.push(program_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(alt) = message.get("addressTableLookups") {
                    alt_accounts = Some(alt.clone());
                }
            }
        }

        for token_balance in &mut enhanced_info.pre_token_balances {
            if let Some(account_info) = enhanced_info.all_accounts.get(token_balance.account_index)
            {
                token_balance.account = account_info.pubkey;
            }
        }

        for token_balance in &mut enhanced_info.post_token_balances {
            if let Some(account_info) = enhanced_info.all_accounts.get(token_balance.account_index)
            {
                token_balance.account = account_info.pubkey;
            }
        }

        let signer_set: HashSet<Pubkey> = enhanced_info.signer_accounts.iter().copied().collect();

        enhanced_info.signer_balance_changes =
            calculate_signer_balance_changes(&enhanced_info, &signer_set);
        enhanced_info.signer_token_balance_changes =
            calculate_signer_token_balance_changes(&enhanced_info, &signer_set);

        if enhanced_info.signer_accounts.len() == 1 {
            enhanced_info.is_arbitrage = is_arbitrage_tx(&enhanced_info, &wsol_mint())
                || is_arbitrage_tx(&enhanced_info, &usdc_mint());
        }

        Ok((enhanced_info, alt_accounts))
    } else {
        Err(anyhow!("Invalid response from RPC"))
    }
}

pub async fn fetch_accounts_from_alt(
    mut enhanced_info: EnhancedTransactionInfo,
    alt_accounts: Option<Value>,
    lookup_cache: &LookupTableCache,
) -> Result<EnhancedTransactionInfo> {
    if let Some(address_table_lookups) = alt_accounts {
        if let Some(lookups_array) = address_table_lookups.as_array() {
            for lookup in lookups_array {
                if let Some(account_key) = lookup.get("accountKey") {
                    if let Some(account_str) = account_key.as_str() {
                        if let Ok(lookup_table_key) = Pubkey::from_str(account_str) {
                            match lookup_cache
                                .get_lookup_table_accounts(&lookup_table_key)
                                .await
                            {
                                Ok(lookup_accounts) => {
                                    let writable_indexes = lookup
                                        .get("writableIndexes")
                                        .and_then(|arr| arr.as_array())
                                        .map(|arr| {
                                            arr.iter()
                                                .filter_map(|v| v.as_u64())
                                                .collect::<Vec<_>>()
                                        })
                                        .unwrap_or_default();

                                    let readonly_indexes = lookup
                                        .get("readonlyIndexes")
                                        .and_then(|arr| arr.as_array())
                                        .map(|arr| {
                                            arr.iter()
                                                .filter_map(|v| v.as_u64())
                                                .collect::<Vec<_>>()
                                        })
                                        .unwrap_or_default();

                                    let used_account_addresses: Vec<Pubkey> = writable_indexes
                                        .iter()
                                        .chain(readonly_indexes.iter())
                                        .filter_map(|&idx| lookup_accounts.get(idx as usize))
                                        .copied()
                                        .collect();

                                    enhanced_info
                                        .lookup_table_accounts
                                        .push(LookupTableAccount {
                                            address: lookup_table_key,
                                            accounts: used_account_addresses,
                                        });

                                    let mut used_accounts = Vec::new();
                                    for &index in &writable_indexes {
                                        if let Some(&lookup_account) =
                                            lookup_accounts.get(index as usize)
                                        {
                                            used_accounts.push((lookup_account, true)); // true = writable
                                            enhanced_info.writable_accounts.push(lookup_account);
                                        }
                                    }

                                    for &index in &readonly_indexes {
                                        if let Some(&lookup_account) =
                                            lookup_accounts.get(index as usize)
                                        {
                                            used_accounts.push((lookup_account, false)); // false = readonly
                                        }
                                    }

                                    for (lookup_account, is_writable) in used_accounts {
                                        enhanced_info.all_accounts.push(AccountInfo {
                                            pubkey: lookup_account,
                                            index: enhanced_info.all_accounts.len(),
                                            is_signer: false,
                                            is_writable,
                                            is_executable: false,
                                            owner: None,
                                            lamports: None,
                                        });
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        "❌ Failed to fetch lookup table {}: {}",
                                        lookup_table_key, e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(enhanced_info)
}

fn is_arbitrage_tx(tx_info: &EnhancedTransactionInfo, mint: &Pubkey) -> bool {
    if tx_info.signer_token_balance_changes.len() <= 1 {
        return false;
    }

    let mut is_contain_mint = false;

    for changed in &tx_info.signer_token_balance_changes {
        if &changed.mint != mint && changed.change_amount != 0 {
            return false;
        }

        if &changed.mint == mint && tx_info.success && changed.change_amount <= 0 {
            return false;
        }

        if &changed.mint == mint && !tx_info.success && changed.change_amount != 0 {
            return false;
        }

        if &changed.mint == mint {
            is_contain_mint = true;
        }
    }

    is_contain_mint
}

fn parse_token_balances(balances_value: Option<&Value>) -> Result<Vec<TokenBalance>> {
    let mut token_balances = Vec::new();

    if let Some(balances) = balances_value {
        if let Some(balances_array) = balances.as_array() {
            for balance in balances_array {
                if let (Some(account_index), Some(mint), Some(owner), Some(ui_token_amount)) = (
                    balance.get("accountIndex").and_then(|i| i.as_u64()),
                    balance.get("mint").and_then(|m| m.as_str()),
                    balance.get("owner").and_then(|o| o.as_str()),
                    balance.get("uiTokenAmount"),
                ) {
                    let amount = ui_token_amount
                        .get("amount")
                        .and_then(|a| a.as_str())
                        .unwrap_or("0")
                        .to_string();

                    let decimals = ui_token_amount
                        .get("decimals")
                        .and_then(|d| d.as_u64())
                        .unwrap_or(0) as u8;

                    let ui_amount = ui_token_amount.get("uiAmount").and_then(|ua| ua.as_f64());

                    if let (Ok(mint_pubkey), Ok(owner_pubkey)) =
                        (Pubkey::from_str(mint), Pubkey::from_str(owner))
                    {
                        token_balances.push(TokenBalance {
                            account_index: account_index as usize,
                            account: Pubkey::default(),
                            mint: mint_pubkey,
                            owner: owner_pubkey,
                            amount,
                            decimals,
                            ui_amount,
                        });
                    }
                }
            }
        }
    }

    Ok(token_balances)
}

fn calculate_signer_balance_changes(
    tx_info: &EnhancedTransactionInfo,
    signer_set: &HashSet<Pubkey>,
) -> Vec<BalanceChange> {
    let mut changes = Vec::new();

    for (i, account_info) in tx_info.all_accounts.iter().enumerate() {
        if !signer_set.contains(&account_info.pubkey) {
            continue;
        }

        if let (Some(pre_balance), Some(post_balance)) =
            (tx_info.pre_balances.get(i), tx_info.post_balances.get(i))
        {
            let change = (*post_balance as i64) - (*pre_balance as i64);

            changes.push(BalanceChange {
                account: account_info.pubkey,
                pre_balance: *pre_balance,
                post_balance: *post_balance,
                change,
                is_signer: true,
            });
        }
    }

    changes
}

fn calculate_signer_token_balance_changes(
    tx_info: &EnhancedTransactionInfo,
    signer_set: &HashSet<Pubkey>,
) -> Vec<TokenBalanceChange> {
    let mut changes = Vec::new();
    let mut pre_by_account: HashMap<usize, &TokenBalance> = HashMap::new();

    for pre_balance in &tx_info.pre_token_balances {
        pre_by_account.insert(pre_balance.account_index, pre_balance);
    }

    for post_balance in &tx_info.post_token_balances {
        let account_index = post_balance.account_index;
        let account_key = post_balance.account;
        let owner_key = post_balance.owner;

        if !signer_set.contains(&owner_key) {
            continue;
        }

        if let Some(pre_balance) = pre_by_account.get(&account_index) {
            if let (Ok(pre_amount), Ok(post_amount)) = (
                pre_balance.amount.parse::<i128>(),
                post_balance.amount.parse::<i128>(),
            ) {
                let change_amount = post_amount - pre_amount;

                let ui_change = if let (Some(pre_ui), Some(post_ui)) =
                    (pre_balance.ui_amount, post_balance.ui_amount)
                {
                    Some(post_ui - pre_ui)
                } else {
                    None
                };

                changes.push(TokenBalanceChange {
                    account: account_key,
                    mint: post_balance.mint,
                    owner: post_balance.owner,
                    pre_amount: pre_balance.amount.clone(),
                    post_amount: post_balance.amount.clone(),
                    change_amount,
                    decimals: post_balance.decimals,
                    ui_change,
                    is_signer: true,
                });
            }
        }
    }

    changes
}

pub fn extract_pubkeys(alt_accounts: Option<Value>) -> Vec<Pubkey> {
    alt_accounts
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|lookup| {
            lookup
                .get("accountKey")
                .and_then(|v| v.as_str())
                .and_then(|s| Pubkey::from_str(s).ok())
        })
        .collect()
}
