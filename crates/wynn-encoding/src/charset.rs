/// WynnBuilder custom Base64 character set.
/// Order: 0-9 A-Z a-z + -
const CHARSET: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz+-";

/// Map a character to its 6-bit index.
pub fn char_to_index(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'Z' => Some(c - b'A' + 10),
        b'a'..=b'z' => Some(c - b'a' + 36),
        b'+' => Some(62),
        b'-' => Some(63),
        _ => None,
    }
}

/// Map a 6-bit index to its character.
pub fn index_to_char(i: u8) -> u8 {
    CHARSET[i as usize]
}

/// A bit vector for reading/writing the binary encoding.
#[derive(Debug, Clone)]
pub struct BitVec {
    bits: Vec<bool>,
    cursor: usize,
}

impl BitVec {
    /// Create an empty bit vector.
    pub fn new() -> Self {
        Self {
            bits: Vec::new(),
            cursor: 0,
        }
    }

    /// Decode a WynnBuilder hash string into a bit vector.
    /// Each character produces 6 bits (LSB first within each character).
    pub fn from_hash(hash: &str) -> Result<Self, DecodeError> {
        let mut bits = Vec::with_capacity(hash.len() * 6);
        for &byte in hash.as_bytes() {
            let idx = char_to_index(byte)
                .ok_or_else(|| DecodeError::InvalidChar(byte as char))?;
            // LSB first: bit 0 of the 6-bit value goes first
            for bit_pos in 0..6 {
                bits.push((idx >> bit_pos) & 1 == 1);
            }
        }
        Ok(Self { bits, cursor: 0 })
    }

    /// Encode the bit vector to a WynnBuilder hash string.
    /// Pads the last character with zero bits if needed.
    pub fn to_hash(&self) -> String {
        let mut result = String::with_capacity((self.bits.len() + 5) / 6);
        for chunk in self.bits.chunks(6) {
            let mut val: u8 = 0;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit {
                    val |= 1 << i;
                }
            }
            result.push(index_to_char(val) as char);
        }
        result
    }

    /// Read `n` bits as an unsigned integer.
    pub fn read_bits(&mut self, n: usize) -> Result<u64, DecodeError> {
        if self.cursor + n > self.bits.len() {
            return Err(DecodeError::UnexpectedEnd);
        }
        let mut val: u64 = 0;
        for i in 0..n {
            if self.bits[self.cursor + i] {
                val |= 1 << i;
            }
        }
        self.cursor += n;
        Ok(val)
    }

    /// Read `n` bits as a signed integer (2's complement).
    pub fn read_signed(&mut self, n: usize) -> Result<i64, DecodeError> {
        let raw = self.read_bits(n)?;
        // Sign extend
        if n > 0 && (raw >> (n - 1)) & 1 == 1 {
            // Negative: fill upper bits with 1s
            Ok(raw as i64 | (!0i64 << n))
        } else {
            Ok(raw as i64)
        }
    }

    /// Read a single bit.
    pub fn read_bit(&mut self) -> Result<bool, DecodeError> {
        if self.cursor >= self.bits.len() {
            return Err(DecodeError::UnexpectedEnd);
        }
        let bit = self.bits[self.cursor];
        self.cursor += 1;
        Ok(bit)
    }

    /// Write `n` bits from an unsigned integer (LSB first).
    pub fn write_bits(&mut self, val: u64, n: usize) {
        for i in 0..n {
            self.bits.push((val >> i) & 1 == 1);
        }
    }

    /// Write `n` bits from a signed integer.
    pub fn write_signed(&mut self, val: i64, n: usize) {
        self.write_bits(val as u64, n);
    }

    /// Write a single bit.
    pub fn write_bit(&mut self, bit: bool) {
        self.bits.push(bit);
    }

    /// Remaining bits available to read.
    pub fn remaining(&self) -> usize {
        self.bits.len().saturating_sub(self.cursor)
    }

    /// Current cursor position.
    pub fn position(&self) -> usize {
        self.cursor
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("invalid character in hash: '{0}'")]
    InvalidChar(char),
    #[error("unexpected end of data")]
    UnexpectedEnd,
    #[error("invalid version: {0}")]
    InvalidVersion(u64),
    #[error("unsupported equipment kind: {0}")]
    UnsupportedEquipmentKind(u64),
    #[error("item ID {0} not found in database")]
    ItemNotFound(u32),
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_charset_roundtrip() {
        for i in 0..64u8 {
            let c = index_to_char(i);
            assert_eq!(char_to_index(c), Some(i));
        }
    }

    #[test]
    fn test_bitvec_roundtrip() {
        let mut bv = BitVec::new();
        bv.write_bits(42, 8);
        bv.write_bits(7, 4);
        bv.write_signed(-3, 8);

        let hash = bv.to_hash();
        let mut bv2 = BitVec::from_hash(&hash).unwrap();

        assert_eq!(bv2.read_bits(8).unwrap(), 42);
        assert_eq!(bv2.read_bits(4).unwrap(), 7);
        assert_eq!(bv2.read_signed(8).unwrap(), -3);
    }

    #[test]
    fn test_bitvec_single_bits() {
        let mut bv = BitVec::new();
        bv.write_bit(true);
        bv.write_bit(false);
        bv.write_bit(true);

        let hash = bv.to_hash();
        let mut bv2 = BitVec::from_hash(&hash).unwrap();

        assert!(bv2.read_bit().unwrap());
        assert!(!bv2.read_bit().unwrap());
        assert!(bv2.read_bit().unwrap());
    }
}
