use crate::error::StdfError;

/// STDF's numeric fields can be little- or big-endian depending on the
/// ATE that wrote the file. The FAR record's CPU_TYPE byte declares which,
/// so this is detected once per file rather than checked per field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endianness {
    Little,
    Big,
}

/// Every STDF record starts with this 4-byte header:
/// 2-byte record length (of the payload that follows), 1-byte type,
/// 1-byte subtype. E.g. PTR (Parametric Test Record) is type 15, subtype 10.
#[derive(Debug, Clone, Copy)]
pub struct RecordHeader {
    pub len: u16,
    pub rec_typ: u8,
    pub rec_sub: u8,
}

pub fn read_header(bytes: &[u8], endianness: Endianness) -> Result<RecordHeader, StdfError> {
    if bytes.len() < 4 {
        return Err(StdfError::UnexpectedEof {
            needed: 4,
            available: bytes.len(),
        });
    }
    let len = match endianness {
        Endianness::Little => u16::from_le_bytes([bytes[0], bytes[1]]),
        Endianness::Big => u16::from_be_bytes([bytes[0], bytes[1]]),
    };
    Ok(RecordHeader {
        len,
        rec_typ: bytes[2],
        rec_sub: bytes[3],
    })
}

/// FAR is always REC_TYP=0, REC_SUB=10. Its first payload byte (the 5th
/// byte of the file) is CPU_TYPE, which declares endianness:
/// 1 = big-endian, 2 = little-endian. We read this once per file.
pub fn detect_endianness(bytes: &[u8]) -> Result<Endianness, StdfError> {
    if bytes.len() < 6 {
        return Err(StdfError::InvalidFar);
    }
    if bytes[2] != 0 || bytes[3] != 10 {
        return Err(StdfError::InvalidFar);
    }
    let cpu_type = bytes[4];
    Ok(match cpu_type {
        1 => Endianness::Big,
        2 => Endianness::Little,
        _ => return Err(StdfError::UnsupportedCpuType(cpu_type)),
    })
}
