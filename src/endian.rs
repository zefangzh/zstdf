#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
    pub fn u16(self, b: [u8; 2]) -> u16 {
        match self {
            Self::Little => u16::from_le_bytes(b),
            Self::Big    => u16::from_be_bytes(b),
        }
    }
    pub fn u32(self, b: [u8; 4]) -> u32 {
        match self {
            Self::Little => u32::from_le_bytes(b),
            Self::Big    => u32::from_be_bytes(b),
        }
    }
    pub fn f32(self, b: [u8; 4]) -> f32 {
        match self {
            Self::Little => f32::from_le_bytes(b),
            Self::Big    => f32::from_be_bytes(b),
        }
    }
    pub fn f64(self, b: [u8; 8]) -> f64 {
        match self {
            Self::Little => f64::from_le_bytes(b),
            Self::Big    => f64::from_be_bytes(b),
        }
    }
    pub fn u16_to_bytes(self, v: u16) -> [u8; 2] {
        match self {
            Self::Little => v.to_le_bytes(),
            Self::Big    => v.to_be_bytes(),
        }
    }
}
