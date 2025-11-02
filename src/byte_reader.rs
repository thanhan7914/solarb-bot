use anchor_client::solana_sdk::pubkey::Pubkey;
use anyhow::{Result, anyhow};

pub struct ByteReader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        if self.offset >= self.data.len() {
            return Err(anyhow!("Read past end of buffer"));
        }
        let val = self.data[self.offset];
        self.offset += 1;
        Ok(val)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        if self.offset + 2 > self.data.len() {
            return Err(anyhow!("Read past end of buffer"));
        }
        let val = u16::from_le_bytes([self.data[self.offset], self.data[self.offset + 1]]);
        self.offset += 2;
        Ok(val)
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        if self.offset + 4 > self.data.len() {
            return Err(anyhow!("Read past end of buffer"));
        }
        let val = u32::from_le_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ]);
        self.offset += 4;
        Ok(val)
    }

    pub fn read_u64(&mut self) -> Result<u64> {
        if self.offset + 8 > self.data.len() {
            return Err(anyhow!("Read past end of buffer"));
        }
        let val = u64::from_le_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
            self.data[self.offset + 4],
            self.data[self.offset + 5],
            self.data[self.offset + 6],
            self.data[self.offset + 7],
        ]);
        self.offset += 8;
        Ok(val)
    }

    pub fn read_u128(&mut self) -> Result<u128> {
        if self.offset + 16 > self.data.len() {
            return Err(anyhow!("Read past end of buffer"));
        }
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&self.data[self.offset..self.offset + 16]);
        let val = u128::from_le_bytes(bytes);
        self.offset += 16;
        Ok(val)
    }

    pub fn read_pubkey(&mut self) -> Result<Pubkey> {
        if self.offset + 32 > self.data.len() {
            return Err(anyhow!("Read past end of buffer"));
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&self.data[self.offset..self.offset + 32]);
        let pubkey = Pubkey::new_from_array(bytes);
        self.offset += 32;
        Ok(pubkey)
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
        if self.offset + len > self.data.len() {
            return Err(anyhow!("Read past end of buffer"));
        }
        let bytes = self.data[self.offset..self.offset + len].to_vec();
        self.offset += len;
        Ok(bytes)
    }

    pub fn read_bytes_array<const N: usize>(&mut self) -> Result<[u8; N]> {
        if self.offset + N > self.data.len() {
            return Err(anyhow!("Read past end of buffer"));
        }
        let mut array = [0u8; N];
        array.copy_from_slice(&self.data[self.offset..self.offset + N]);
        self.offset += N;
        Ok(array)
    }

    pub fn skip(&mut self, len: usize) -> Result<()> {
        if self.offset + len > self.data.len() {
            return Err(anyhow!("Skip past end of buffer"));
        }
        self.offset += len;
        Ok(())
    }

    pub fn read_pubkey_from_u64_array(&mut self) -> Result<Pubkey> {
        // Read 4 u64 values (32 bytes total) and convert to Pubkey
        let mut bytes = [0u8; 32];
        for i in 0..4 {
            let val = self.read_u64()?;
            let val_bytes = val.to_le_bytes();
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&val_bytes);
        }
        Ok(Pubkey::new_from_array(bytes))
    }

    pub fn read_i32(&mut self) -> Result<i32> {
        if self.offset + 4 > self.data.len() {
            return Err(anyhow!("Read past end of buffer"));
        }
        let bytes: [u8; 4] = self.data[self.offset..self.offset + 4]
            .try_into()
            .map_err(|_| anyhow!("Failed to convert slice to array"))?;
        let val = i32::from_le_bytes(bytes);
        self.offset += 4;
        Ok(val)
    }
}
