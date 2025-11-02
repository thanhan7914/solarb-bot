use crate::byte_reader::ByteReader;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccountFlag {
    Initialized = 1u64 << 0,
    Market = 1u64 << 1,
    OpenOrders = 1u64 << 2,
    RequestQueue = 1u64 << 3,
    EventQueue = 1u64 << 4,
    Bids = 1u64 << 5,
    Asks = 1u64 << 6,
    Disabled = 1u64 << 7,
    Closed = 1u64 << 8,
    Permissioned = 1u64 << 9,
    CrankAuthorityRequired = 1u64 << 10,
}

// MarketState struct
#[derive(Debug, Clone)]
pub struct MarketState {
    pub account_flags: u64,
    pub own_address: Pubkey,
    pub vault_signer_nonce: u64,
    pub coin_mint: Pubkey,
    pub pc_mint: Pubkey,
    pub coin_vault: Pubkey,
    pub coin_deposits_total: u64,
    pub coin_fees_accrued: u64,
    pub pc_vault: Pubkey,
    pub pc_deposits_total: u64,
    pub pc_fees_accrued: u64,
    pub pc_dust_threshold: u64,
    pub req_q: Pubkey,
    pub event_q: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub coin_lot_size: u64,
    pub pc_lot_size: u64,
    pub fee_rate_bps: u64,
    pub referrer_rebates_accrued: u64,
}

// MarketStateV2 struct
#[derive(Debug, Clone)]
pub struct MarketStateV2 {
    pub inner: MarketState,
    pub open_orders_authority: Pubkey,
    pub prune_authority: Pubkey,
    pub consume_events_authority: Pubkey,
}

// OpenOrders struct
#[derive(Debug, Clone)]
pub struct OpenOrders {
    pub account_flags: u64,
    pub market: Pubkey,
    pub owner: Pubkey,
    pub native_coin_free: u64,
    pub native_coin_total: u64,
    pub native_pc_free: u64,
    pub native_pc_total: u64,
    pub free_slot_bits: u128,
    pub is_bid_bits: u128,
    pub orders: [u128; 128],
    pub client_order_ids: [u64; 128],
    pub referrer_rebates_accrued: u64,
}

// RequestQueueHeader struct
#[derive(Debug, Clone)]
pub struct RequestQueueHeader {
    pub account_flags: u64,
    pub head: u64,
    pub count: u64,
    pub next_seq_num: u64,
}

// EventQueueHeader struct
#[derive(Debug, Clone)]
pub struct EventQueueHeader {
    pub account_flags: u64,
    pub head: u64,
    pub count: u64,
    pub seq_num: u64,
}

// Request struct
#[derive(Debug, Clone)]
pub struct Request {
    pub request_flags: u8,
    pub owner_slot: u8,
    pub fee_tier: u8,
    pub self_trade_behavior: u8,
    pub max_coin_qty_or_cancel_id: u64,
    pub native_pc_qty_locked: u64,
    pub order_id: u128,
    pub owner: Pubkey,
    pub client_order_id: u64,
}

// Event struct
#[derive(Debug, Clone)]
pub struct Event {
    pub event_flags: u8,
    pub owner_slot: u8,
    pub fee_tier: u8,
    pub native_qty_released: u64,
    pub native_qty_paid: u64,
    pub native_fee_or_rebate: u64,
    pub order_id: u128,
    pub owner: Pubkey,
    pub client_order_id: u64,
}

// OrderBookStateHeader struct
#[derive(Debug, Clone)]
pub struct OrderBookStateHeader {
    pub account_flags: u64,
}

// Deserializer implementations
impl MarketState {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 5 {
            return Err(anyhow!("Invalid data length"));
        }

        // Check header padding
        if &data[0..5] != b"serum" {
            return Err(anyhow!("Invalid header serum"));
        }

        // Check tail padding
        let tail_start = data.len() - 7;
        if &data[tail_start..] != b"padding" {
            return Err(anyhow!("Invalid tail padding"));
        }

        // Skip header padding and read the actual data
        let mut reader = ByteReader::new(&data[5..tail_start]);

        Ok(MarketState {
            account_flags: reader.read_u64()?,
            own_address: reader.read_pubkey_from_u64_array()?,
            vault_signer_nonce: reader.read_u64()?,
            coin_mint: reader.read_pubkey_from_u64_array()?,
            pc_mint: reader.read_pubkey_from_u64_array()?,
            coin_vault: reader.read_pubkey_from_u64_array()?,
            coin_deposits_total: reader.read_u64()?,
            coin_fees_accrued: reader.read_u64()?,
            pc_vault: reader.read_pubkey_from_u64_array()?,
            pc_deposits_total: reader.read_u64()?,
            pc_fees_accrued: reader.read_u64()?,
            pc_dust_threshold: reader.read_u64()?,
            req_q: reader.read_pubkey_from_u64_array()?,
            event_q: reader.read_pubkey_from_u64_array()?,
            bids: reader.read_pubkey_from_u64_array()?,
            asks: reader.read_pubkey_from_u64_array()?,
            coin_lot_size: reader.read_u64()?,
            pc_lot_size: reader.read_u64()?,
            fee_rate_bps: reader.read_u64()?,
            referrer_rebates_accrued: reader.read_u64()?,
        })
    }

    pub fn derive_vault_signer(market_address: &Pubkey, vault_signer_nonce: u64) -> Result<Pubkey> {
        match Pubkey::create_program_address(
            &[market_address.as_ref(), &vault_signer_nonce.to_le_bytes()],
            &super::openbook_id(),
        ) {
            Ok(pda) => Ok(pda),
            Err(e) => Err(anyhow!("Failed to create vault signer PDA: {}", e)),
        }
    }
}

impl MarketStateV2 {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 5 {
            return Err(anyhow!("Invalid data length"));
        }

        // Check header padding
        if &data[0..5] != b"serum" {
            return Err(anyhow!("Invalid header padding"));
        }

        // Check tail padding
        let tail_start = data.len() - 7;
        if &data[tail_start..] != b"padding" {
            return Err(anyhow!("Invalid tail padding"));
        }

        // Skip header padding and read the actual data
        let mut reader = ByteReader::new(&data[5..tail_start]);

        let inner = MarketState {
            account_flags: reader.read_u64()?,
            own_address: reader.read_pubkey_from_u64_array()?,
            vault_signer_nonce: reader.read_u64()?,
            coin_mint: reader.read_pubkey_from_u64_array()?,
            pc_mint: reader.read_pubkey_from_u64_array()?,
            coin_vault: reader.read_pubkey_from_u64_array()?,
            coin_deposits_total: reader.read_u64()?,
            coin_fees_accrued: reader.read_u64()?,
            pc_vault: reader.read_pubkey_from_u64_array()?,
            pc_deposits_total: reader.read_u64()?,
            pc_fees_accrued: reader.read_u64()?,
            pc_dust_threshold: reader.read_u64()?,
            req_q: reader.read_pubkey_from_u64_array()?,
            event_q: reader.read_pubkey_from_u64_array()?,
            bids: reader.read_pubkey_from_u64_array()?,
            asks: reader.read_pubkey_from_u64_array()?,
            coin_lot_size: reader.read_u64()?,
            pc_lot_size: reader.read_u64()?,
            fee_rate_bps: reader.read_u64()?,
            referrer_rebates_accrued: reader.read_u64()?,
        };

        Ok(MarketStateV2 {
            inner,
            open_orders_authority: reader.read_pubkey()?,
            prune_authority: reader.read_pubkey()?,
            consume_events_authority: reader.read_pubkey()?,
            // Skip padding bytes (992 bytes)
        })
    }
}

impl OpenOrders {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 5 {
            return Err(anyhow!("Invalid data length"));
        }

        // Check header padding
        if &data[0..5] != b"serum" {
            return Err(anyhow!("Invalid header padding"));
        }

        // Check tail padding
        let tail_start = data.len() - 7;
        if &data[tail_start..] != b"padding" {
            return Err(anyhow!("Invalid tail padding"));
        }

        // Skip header padding and read the actual data
        let mut reader = ByteReader::new(&data[5..tail_start]);

        let account_flags = reader.read_u64()?;
        let market = reader.read_pubkey_from_u64_array()?;
        let owner = reader.read_pubkey_from_u64_array()?;
        let native_coin_free = reader.read_u64()?;
        let native_coin_total = reader.read_u64()?;
        let native_pc_free = reader.read_u64()?;
        let native_pc_total = reader.read_u64()?;
        let free_slot_bits = reader.read_u128()?;
        let is_bid_bits = reader.read_u128()?;

        let mut orders = [0u128; 128];
        for i in 0..128 {
            orders[i] = reader.read_u128()?;
        }

        let mut client_order_ids = [0u64; 128];
        for i in 0..128 {
            client_order_ids[i] = reader.read_u64()?;
        }

        let referrer_rebates_accrued = reader.read_u64()?;

        Ok(OpenOrders {
            account_flags,
            market,
            owner,
            native_coin_free,
            native_coin_total,
            native_pc_free,
            native_pc_total,
            free_slot_bits,
            is_bid_bits,
            orders,
            client_order_ids,
            referrer_rebates_accrued,
        })
    }
}

impl RequestQueueHeader {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 5 {
            return Err(anyhow!("Invalid data length"));
        }

        // Check header padding
        if &data[0..5] != b"serum" {
            return Err(anyhow!("Invalid header padding"));
        }

        // Skip header padding and read the actual data
        let mut reader = ByteReader::new(&data[5..]);

        Ok(RequestQueueHeader {
            account_flags: reader.read_u64()?,
            head: reader.read_u64()?,
            count: reader.read_u64()?,
            next_seq_num: reader.read_u64()?,
        })
    }
}

impl EventQueueHeader {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 5 {
            return Err(anyhow!("Invalid data length"));
        }

        // Check header padding
        if &data[0..5] != b"serum" {
            return Err(anyhow!("Invalid header padding"));
        }

        // Skip header padding and read the actual data
        let mut reader = ByteReader::new(&data[5..]);

        Ok(EventQueueHeader {
            account_flags: reader.read_u64()?,
            head: reader.read_u64()?,
            count: reader.read_u64()?,
            seq_num: reader.read_u64()?,
        })
    }
}

impl Request {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data);

        Ok(Request {
            request_flags: reader.read_u8()?,
            owner_slot: reader.read_u8()?,
            fee_tier: reader.read_u8()?,
            self_trade_behavior: reader.read_u8()?,
            // Skip padding (4 bytes)
            max_coin_qty_or_cancel_id: {
                reader.skip(4)?;
                reader.read_u64()?
            },
            native_pc_qty_locked: reader.read_u64()?,
            order_id: reader.read_u128()?,
            owner: reader.read_pubkey_from_u64_array()?,
            client_order_id: reader.read_u64()?,
        })
    }
}

impl Event {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut reader = ByteReader::new(data);

        Ok(Event {
            event_flags: reader.read_u8()?,
            owner_slot: reader.read_u8()?,
            fee_tier: reader.read_u8()?,
            // Skip padding (5 bytes)
            native_qty_released: {
                reader.skip(5)?;
                reader.read_u64()?
            },
            native_qty_paid: reader.read_u64()?,
            native_fee_or_rebate: reader.read_u64()?,
            order_id: reader.read_u128()?,
            owner: reader.read_pubkey_from_u64_array()?,
            client_order_id: reader.read_u64()?,
        })
    }
}

impl OrderBookStateHeader {
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        if data.len() < 5 {
            return Err(anyhow!("Invalid data length"));
        }

        // Check header padding
        if &data[0..5] != b"serum" {
            return Err(anyhow!("Invalid header padding"));
        }

        // Skip header padding and read the actual data
        let mut reader = ByteReader::new(&data[5..]);

        Ok(OrderBookStateHeader {
            account_flags: reader.read_u64()?,
        })
    }
}

pub fn has_flag(account_flags: u64, flag: AccountFlag) -> bool {
    (account_flags & (flag as u64)) != 0
}

pub fn is_market_v2(account_flags: u64) -> bool {
    has_flag(account_flags, AccountFlag::Permissioned)
}

pub fn deserialize_serum_account(data: &[u8]) -> Result<SerumAccount> {
    if data.len() < 13 {
        return Err(anyhow!("Data too short"));
    }

    // Check header padding
    if &data[0..5] != b"serum" {
        return Err(anyhow!("Invalid header padding"));
    }

    // Read account flags to determine account type
    let account_flags = u64::from_le_bytes([
        data[5], data[6], data[7], data[8], data[9], data[10], data[11], data[12],
    ]);

    if has_flag(account_flags, AccountFlag::Market) {
        if is_market_v2(account_flags) {
            Ok(SerumAccount::MarketV2(MarketStateV2::deserialize(data)?))
        } else {
            Ok(SerumAccount::Market(MarketState::deserialize(data)?))
        }
    } else if has_flag(account_flags, AccountFlag::OpenOrders) {
        Ok(SerumAccount::OpenOrders(OpenOrders::deserialize(data)?))
    } else if has_flag(account_flags, AccountFlag::RequestQueue) {
        Ok(SerumAccount::RequestQueue(RequestQueueHeader::deserialize(
            data,
        )?))
    } else if has_flag(account_flags, AccountFlag::EventQueue) {
        Ok(SerumAccount::EventQueue(EventQueueHeader::deserialize(
            data,
        )?))
    } else if has_flag(account_flags, AccountFlag::Bids)
        || has_flag(account_flags, AccountFlag::Asks)
    {
        Ok(SerumAccount::OrderBook(OrderBookStateHeader::deserialize(
            data,
        )?))
    } else {
        Err(anyhow!("Unknown account type"))
    }
}

#[derive(Debug)]
pub enum SerumAccount {
    Market(MarketState),
    MarketV2(MarketStateV2),
    OpenOrders(OpenOrders),
    RequestQueue(RequestQueueHeader),
    EventQueue(EventQueueHeader),
    OrderBook(OrderBookStateHeader),
}
