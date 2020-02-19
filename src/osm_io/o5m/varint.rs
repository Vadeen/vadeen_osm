//! In the o5m format the integers are encoded as variable integers. Small ints require fewer bytes.
//!
//! Unsigned integers uses the most significant bit of every byte to determine if there is more
//! bytes remaining.
//!
//! Signed integers is similar to unsigned, but they also uses the least significant bit of the
//! least significant byte to determine if the number is negative or not.
//!
//! This module can encode and decode both unsigned and signed integers. The internal representation
//! is the encoded byte vector.
//!
//! The `From` trait implementations respects the signedness of the input type. I.e. i32 and i64
//! are encoded as a signed variable integer, u32 and u64 as unsigned.
//!
//! The trait `ReadVarInt` adds read_varint() to all readers for easy reading of varints.
//!
//! See: https://wiki.openstreetmap.org/wiki/O5m#Numbers

use crate::osm_io::error::{Error, ErrorKind, Result};
use std::io::{Read, Write};

/// Represents a variable integer (signed or unsigned).
#[derive(Debug)]
pub struct VarInt {
    bytes: Vec<u8>,
}

impl VarInt {
    pub fn new(bytes: Vec<u8>) -> Self {
        VarInt { bytes }
    }

    /// Convenience function for creating a (u)varint and returning its bytes.
    /// The function respects the signedness of the input type.
    ///
    /// Equivalent to: `VarInt::from(value).bytes()`.
    pub fn create_bytes<T: Into<VarInt>>(value: T) -> Vec<u8> {
        let varint: VarInt = value.into();
        varint.bytes()
    }

    pub fn bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Turns VarInt into an signed int.
    pub fn into_i64(mut self) -> i64 {
        let (first, rest) = self.bytes.split_first().unwrap();
        let byte = *first as u64;
        let negative = (byte & 0x01) != 0x00;
        let mut value = (byte & 0x7E) >> 1;

        // If first bit is set, there is more bytes in the same format as uvarint.
        if (byte & 0x80) != 0x00 {
            self.bytes = rest.to_vec();
            value |= self.into_u64() << 6;
        }

        let value = value as i64;
        if negative {
            -value - 1
        } else {
            value
        }
    }

    /// Turns VarInt into an unsigned int.
    pub fn into_u64(self) -> u64 {
        let mut value = 0;
        for (n, _) in self.bytes.iter().enumerate() {
            // 9*7 = 63 bits. We can store 64 without overflow.
            let byte = self.bytes[n] as u64;
            value |= (byte & 0x7F) << (7 * (n as u64));

            if byte & 0x80 == 0 {
                break;
            }
        }
        value
    }
}

/// Extends [`Read`] with methods for reading varints.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
pub trait ReadVarInt: Read {
    fn read_varint(&mut self) -> Result<VarInt> {
        let mut bytes = Vec::new();
        for i in 0..10 {
            // If we get to byte 9 we have more bits than 64.
            if i == 9 {
                return Err(Error::new(
                    ErrorKind::ParseError,
                    Some("Varint overflow, read 9 bytes.".to_owned()),
                ));
            }

            let mut buf = [0u8; 1];
            self.read_exact(&mut buf)?;

            let byte = buf[0];
            bytes.push(byte);
            if byte & 0x80 == 0 {
                break;
            }
        }
        Ok(VarInt { bytes })
    }
}

/// All types that implements the Read trait gets the ReadVarInt methods.
impl<R: Read + ?Sized> ReadVarInt for R {}

pub trait WriteVarInt: Write {
    fn write_varint<T: Into<VarInt>>(&mut self, i: T) -> Result<()> {
        let varint: VarInt = i.into();
        self.write_all(&varint.bytes)?;
        Ok(())
    }
}

impl<W: Write + ?Sized> WriteVarInt for W {}

impl From<u32> for VarInt {
    fn from(value: u32) -> Self {
        VarInt::from(value as u64)
    }
}

impl From<u64> for VarInt {
    fn from(mut value: u64) -> Self {
        let mut bytes = Vec::new();

        while value > 0x7F {
            bytes.push(((value & 0x7F) | 0x80) as u8);
            value >>= 7;
        }

        if value > 0 {
            bytes.push(value as u8);
        }

        VarInt::new(bytes)
    }
}

impl From<i32> for VarInt {
    fn from(value: i32) -> Self {
        VarInt::from(value as i64)
    }
}

impl From<i64> for VarInt {
    fn from(mut value: i64) -> Self {
        let mut sign_bit = 0x00;
        if value < 0 {
            sign_bit = 0x01;

            // We handle the sign our selves, negative range is shifted by 1.
            value = -value - 1;
        }

        let value = value as u64;
        let least_significant = (((value << 1) & 0x7F) | sign_bit) as u8;

        let mut bytes = Vec::new();
        // We can only fit 6 bits in first byte since we use 1 bit for sign and 1 for length.
        if value > 0x3F {
            bytes.push(least_significant | 0x80);

            // Only first byte is special, rest is same as uvarint.
            let mut rest = Self::from((value >> 6) as u64);
            bytes.append(&mut rest.bytes);
        } else {
            bytes.push(least_significant);
        }
        VarInt::new(bytes)
    }
}

#[cfg(test)]
mod test_from_bytes {
    use crate::osm_io::o5m::varint::ReadVarInt;
    use crate::osm_io::o5m::varint::VarInt;

    #[test]
    fn max_one_byte_uvarint() {
        let varint = VarInt::new(vec![0x7F]);
        assert_eq!(varint.into_u64(), 127);
    }

    #[test]
    fn read_two_bytes_uvarint() {
        let data = vec![0xC3, 0x02];
        let varint = data.as_slice().read_varint().unwrap();
        assert_eq!(varint.into_u64(), 323);
    }

    #[test]
    fn three_byte_uvarint() {
        let varint = VarInt::new(vec![0x80, 0x80, 0x01]);
        assert_eq!(varint.into_u64(), 16384);
    }

    #[test]
    fn read_one_byte_positive_varint() {
        let data = vec![0x08];
        let varint = data.as_slice().read_varint().unwrap();
        assert_eq!(varint.into_i64(), 4);
    }

    #[test]
    fn one_byte_negative_varint() {
        let varint = VarInt::new(vec![0x03]);
        assert_eq!(varint.into_i64(), -2);
    }

    #[test]
    fn read_four_byte_positive_varint() {
        let data = vec![0x94, 0xfe, 0xd2, 0x05];
        let varint = data.as_slice().read_varint().unwrap();
        assert_eq!(varint.into_i64(), 5922698);
    }

    #[test]
    fn two_byte_negative_varint() {
        let varint = VarInt::new(vec![0x81, 0x01]);
        assert_eq!(varint.into_i64(), -65);
    }

    #[test]
    fn too_many_bytes() {
        let data = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let error = data.as_slice().read_varint().unwrap_err();
        assert_eq!(error.to_string(), "Varint overflow, read 9 bytes.")
    }
}

#[cfg(test)]
mod test_to_bytes {
    use crate::osm_io::o5m::varint::VarInt;

    #[test]
    fn one_byte_uvarint() {
        let varint = VarInt::from(5 as u64);
        assert_eq!(varint.bytes, vec![0x05]);
    }

    #[test]
    fn max_one_byte_uvarint() {
        let varint = VarInt::from(127 as u64);
        assert_eq!(varint.bytes, vec![0x7F]);
    }

    #[test]
    fn two_byte_uvarint() {
        let varint = VarInt::from(323 as u64);
        assert_eq!(varint.bytes, vec![0xC3, 0x02]);
    }

    #[test]
    fn three_byte_uvarint() {
        let varint = VarInt::from(16384 as u64);
        assert_eq!(varint.bytes, vec![0x80, 0x80, 0x01]);
    }

    #[test]
    fn one_byte_positive_varint() {
        let varint = VarInt::from(4 as i64);
        assert_eq!(varint.bytes, vec![0x08]);
    }

    #[test]
    fn one_byte_negative_varint() {
        let varint = VarInt::from(-3 as i64);
        assert_eq!(varint.bytes, vec![0x05]);
    }

    #[test]
    fn two_byte_positive_varint() {
        let varint = VarInt::from(64 as i64);
        assert_eq!(varint.bytes, vec![0x80, 0x01]);
    }

    #[test]
    fn two_byte_negative_varint() {
        let varint = VarInt::from(-65 as i64);
        assert_eq!(varint.bytes, vec![0x81, 0x01]);
    }
}
