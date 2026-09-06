use crate::error::Result;
use crate::fields::FieldReader;

/// FAR — File Attributes Record (0, 10). Must be first record in file.
#[derive(Debug, Clone)]
pub struct Far {
    /// CPU type: 1=Sun(BE), 2=x86(LE)
    pub cpu_type: u8,
    /// STDF version (must be 4)
    pub stdf_ver: u8,
}

impl Far {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Far {
            cpu_type: r.read_u1()?,
            stdf_ver: r.read_u1()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ByteOrder;

    #[test]
    fn test_far_parse() {
        let data = [2, 4]; // cpu_type=2(LE), stdf_ver=4
        let mut r = FieldReader::new(&data, ByteOrder::LittleEndian);
        let far = Far::parse(&mut r).unwrap();
        assert_eq!(far.cpu_type, 2);
        assert_eq!(far.stdf_ver, 4);
    }
}
