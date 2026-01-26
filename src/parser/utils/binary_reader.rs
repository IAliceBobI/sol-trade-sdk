//! BinaryReader - 二进制数据读取工具
//!
//! 参考 solana-dex-parser 的 BinaryReader 实现

use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct BinaryReader {
    buffer: Vec<u8>,
    offset: usize,
}

impl BinaryReader {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self { buffer, offset: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buffer.len().saturating_sub(self.offset)
    }

    fn check_bounds(&self, length: usize) -> Result<(), String> {
        if self.offset + length > self.buffer.len() {
            Err(format!(
                "Buffer overflow: trying to read {} bytes at offset {} in buffer of length {}",
                length,
                self.offset,
                self.buffer.len()
            ))
        } else {
            Ok(())
        }
    }

    pub fn read_u8(&mut self) -> Result<u8, String> {
        self.check_bounds(1)?;
        let value = self.buffer[self.offset];
        self.offset += 1;
        Ok(value)
    }

    pub fn read_u16(&mut self) -> Result<u16, String> {
        self.check_bounds(2)?;
        let value = u16::from_le_bytes([self.buffer[self.offset], self.buffer[self.offset + 1]]);
        self.offset += 2;
        Ok(value)
    }

    pub fn read_u32(&mut self) -> Result<u32, String> {
        self.check_bounds(4)?;
        let value = u32::from_le_bytes([
            self.buffer[self.offset],
            self.buffer[self.offset + 1],
            self.buffer[self.offset + 2],
            self.buffer[self.offset + 3],
        ]);
        self.offset += 4;
        Ok(value)
    }

    pub fn read_u64(&mut self) -> Result<u64, String> {
        self.check_bounds(8)?;
        let value = u64::from_le_bytes([
            self.buffer[self.offset],
            self.buffer[self.offset + 1],
            self.buffer[self.offset + 2],
            self.buffer[self.offset + 3],
            self.buffer[self.offset + 4],
            self.buffer[self.offset + 5],
            self.buffer[self.offset + 6],
            self.buffer[self.offset + 7],
        ]);
        self.offset += 8;
        Ok(value)
    }

    pub fn read_i64(&mut self) -> Result<i64, String> {
        self.check_bounds(8)?;
        let value = i64::from_le_bytes([
            self.buffer[self.offset],
            self.buffer[self.offset + 1],
            self.buffer[self.offset + 2],
            self.buffer[self.offset + 3],
            self.buffer[self.offset + 4],
            self.buffer[self.offset + 5],
            self.buffer[self.offset + 6],
            self.buffer[self.offset + 7],
        ]);
        self.offset += 8;
        Ok(value)
    }

    pub fn read_fixed_array(&mut self, length: usize) -> Result<Vec<u8>, String> {
        self.check_bounds(length)?;
        let array = self.buffer[self.offset..self.offset + length].to_vec();
        self.offset += length;
        Ok(array)
    }

    pub fn read_pubkey(&mut self) -> Result<Pubkey, String> {
        let bytes = self.read_fixed_array(32)?;
        Pubkey::try_from(bytes.as_slice()).map_err(|e| format!("Invalid pubkey: {}", e))
    }

    pub fn skip(&mut self, length: usize) -> Result<(), String> {
        self.check_bounds(length)?;
        self.offset += length;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u8() {
        let data = vec![0x01, 0x02];
        let mut reader = BinaryReader::new(data);
        assert_eq!(reader.read_u8().unwrap(), 0x01);
        assert_eq!(reader.read_u8().unwrap(), 0x02);
    }

    #[test]
    fn test_read_u64() {
        let data = vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let mut reader = BinaryReader::new(data);
        assert_eq!(reader.read_u64().unwrap(), 1);
    }

    #[test]
    fn test_read_pubkey() {
        let pubkey = Pubkey::new_unique();
        let data = pubkey.to_bytes().to_vec();
        let mut reader = BinaryReader::new(data);
        assert_eq!(reader.read_pubkey().unwrap(), pubkey);
    }

    #[test]
    fn test_remaining() {
        let data = vec![0x01, 0x02, 0x03];
        let reader = BinaryReader::new(data);
        assert_eq!(reader.remaining(), 3);
    }
}
