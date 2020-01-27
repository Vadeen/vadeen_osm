use std::io::Read;
use crate::osm_io::error::Result;

/// Represents a variable integer (signed or unsigned).
/// See: https://wiki.openstreetmap.org/wiki/O5m#Numbers
pub struct VarInt {
    bytes: Vec<u8>,
}

impl VarInt {
    pub fn new(bytes: Vec<u8>) -> Self {
        VarInt {
            bytes
        }
    }

    /// Reads a varint from a reader.
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut bytes = Vec::new();
        for _ in 0..9 {
            let mut buf = [0u8; 1];
            reader.read_exact(&mut buf)?;

            let byte = buf[0];
            bytes.push(byte);
            if byte & 0x80 == 0 {
                break;
            }
        }
        Ok(VarInt { bytes })
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

#[cfg(test)]
mod tests {
    use crate::osm_io::o5m::varint::VarInt;

    #[test]
    fn read_one_byte_uvarint() {
        let data = vec![0x05];
        let varint = VarInt::read(&mut data.as_slice()).unwrap();
        assert_eq!(varint.into_u64(), 5);
    }

    #[test]
    fn max_one_byte_uvarint() {
        let varint = VarInt::new(vec![0x7F]);
        assert_eq!(varint.into_u64(), 127);
    }

    #[test]
    fn read_two_bytes_uvarint() {
        let data = vec![0xC3, 0x02];
        let varint = VarInt::read(&mut data.as_slice()).unwrap();
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
        let varint = VarInt::read(&mut data.as_slice()).unwrap();
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
        let varint = VarInt::read(&mut data.as_slice()).unwrap();
        assert_eq!(varint.into_i64(), 5922698);
    }

    #[test]
    fn two_byte_negative_varint() {
        let varint = VarInt::new(vec![0x81, 0x01]);
        assert_eq!(varint.into_i64(), -65);
    }
}
