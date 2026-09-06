use crate::types::{ByteOrder, RecordType};

/// 4-byte STDF record header.
#[derive(Debug, Clone, Copy)]
pub struct RecordHeader {
    /// Payload length in bytes (excludes this 4-byte header).
    pub rec_len: u16,
    pub rec_typ: u8,
    pub rec_sub: u8,
}

impl RecordHeader {
    /// Parse header from exactly 4 bytes with given byte order.
    pub fn from_bytes(bytes: &[u8; 4], byte_order: ByteOrder) -> Self {
        let rec_len = match byte_order {
            ByteOrder::LittleEndian => u16::from_le_bytes([bytes[0], bytes[1]]),
            ByteOrder::BigEndian => u16::from_be_bytes([bytes[0], bytes[1]]),
        };
        RecordHeader {
            rec_len,
            rec_typ: bytes[2],
            rec_sub: bytes[3],
        }
    }

    pub fn record_type(&self) -> RecordType {
        RecordType::from_type_sub(self.rec_typ, self.rec_sub)
    }

    /// Encode header to 4 bytes.
    pub fn to_bytes(&self, byte_order: ByteOrder) -> [u8; 4] {
        let len_bytes = match byte_order {
            ByteOrder::LittleEndian => self.rec_len.to_le_bytes(),
            ByteOrder::BigEndian => self.rec_len.to_be_bytes(),
        };
        [len_bytes[0], len_bytes[1], self.rec_typ, self.rec_sub]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_far_header_le() {
        let bytes = [0x02, 0x00, 0x00, 0x0A];
        let h = RecordHeader::from_bytes(&bytes, ByteOrder::LittleEndian);
        assert_eq!(h.rec_len, 2);
        assert_eq!(h.record_type(), RecordType::Far);
    }

    #[test]
    fn test_far_header_be() {
        let bytes = [0x00, 0x02, 0x00, 0x0A];
        let h = RecordHeader::from_bytes(&bytes, ByteOrder::BigEndian);
        assert_eq!(h.rec_len, 2);
        assert_eq!(h.record_type(), RecordType::Far);
    }

    #[test]
    fn test_roundtrip() {
        let h = RecordHeader {
            rec_len: 1234,
            rec_typ: 15,
            rec_sub: 10,
        };
        for bo in [ByteOrder::LittleEndian, ByteOrder::BigEndian] {
            let bytes = h.to_bytes(bo);
            let h2 = RecordHeader::from_bytes(&bytes, bo);
            assert_eq!(h2.rec_len, 1234);
            assert_eq!(h2.rec_typ, 15);
            assert_eq!(h2.rec_sub, 10);
        }
    }
}
