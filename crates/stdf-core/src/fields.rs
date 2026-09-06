use crate::error::StdfError;
use crate::header::Endianness;

/// Sequential binary field reader for STDF record payloads.
pub struct FieldReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    endianness: Endianness,
}

impl<'a> FieldReader<'a> {
    pub fn new(bytes: &'a [u8], endianness: Endianness) -> Self {
        Self {
            bytes,
            offset: 0,
            endianness,
        }
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    fn ensure(&self, len: usize) -> Result<(), StdfError> {
        if self.remaining() < len {
            return Err(StdfError::UnexpectedEof {
                needed: len,
                available: self.remaining(),
            });
        }
        Ok(())
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], StdfError> {
        self.ensure(len)?;
        let start = self.offset;
        self.offset += len;
        Ok(&self.bytes[start..self.offset])
    }

    pub fn read_u1(&mut self) -> Result<u8, StdfError> {
        Ok(self.read_exact(1)?[0])
    }

    pub fn read_u2(&mut self) -> Result<u16, StdfError> {
        let bytes = self.read_exact(2)?;
        Ok(match self.endianness {
            Endianness::Little => u16::from_le_bytes([bytes[0], bytes[1]]),
            Endianness::Big => u16::from_be_bytes([bytes[0], bytes[1]]),
        })
    }

    pub fn read_u4(&mut self) -> Result<u32, StdfError> {
        let bytes = self.read_exact(4)?;
        Ok(match self.endianness {
            Endianness::Little => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Endianness::Big => u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
    }

    pub fn read_i1(&mut self) -> Result<i8, StdfError> {
        Ok(self.read_u1()? as i8)
    }

    pub fn read_i2(&mut self) -> Result<i16, StdfError> {
        let bytes = self.read_exact(2)?;
        Ok(match self.endianness {
            Endianness::Little => i16::from_le_bytes([bytes[0], bytes[1]]),
            Endianness::Big => i16::from_be_bytes([bytes[0], bytes[1]]),
        })
    }

    pub fn read_i4(&mut self) -> Result<i32, StdfError> {
        let bytes = self.read_exact(4)?;
        Ok(match self.endianness {
            Endianness::Little => i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            Endianness::Big => i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        })
    }

    pub fn read_r4(&mut self) -> Result<f32, StdfError> {
        Ok(f32::from_bits(self.read_u4()?))
    }

    pub fn read_c1(&mut self) -> Result<char, StdfError> {
        Ok(self.read_u1()? as char)
    }

    pub fn read_cn(&mut self) -> Result<String, StdfError> {
        let len = self.read_u1()? as usize;
        let bytes = self.read_exact(len)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    pub fn read_bn(&mut self) -> Result<Vec<u8>, StdfError> {
        let len = self.read_u1()? as usize;
        Ok(self.read_exact(len)?.to_vec())
    }

    pub fn read_optional_u1(&mut self) -> Result<Option<u8>, StdfError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.read_u1()?))
    }

    pub fn read_optional_u2(&mut self) -> Result<Option<u16>, StdfError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.read_u2()?))
    }

    pub fn read_optional_u4(&mut self) -> Result<Option<u32>, StdfError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.read_u4()?))
    }

    pub fn read_optional_i1(&mut self) -> Result<Option<i8>, StdfError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.read_i1()?))
    }

    pub fn read_optional_i2(&mut self) -> Result<Option<i16>, StdfError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.read_i2()?))
    }

    pub fn read_optional_i4(&mut self) -> Result<Option<i32>, StdfError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.read_i4()?))
    }

    pub fn read_optional_r4(&mut self) -> Result<Option<f32>, StdfError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.read_r4()?))
    }

    pub fn read_optional_c1(&mut self) -> Result<Option<char>, StdfError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.read_c1()?))
    }

    pub fn read_optional_cn(&mut self) -> Result<Option<String>, StdfError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.read_cn()?))
    }

    pub fn read_optional_bn(&mut self) -> Result<Option<Vec<u8>>, StdfError> {
        if self.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.read_bn()?))
    }
}
