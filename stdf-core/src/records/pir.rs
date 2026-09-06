use crate::error::Result;
use crate::fields::FieldReader;

/// PIR — Part Information Record (5, 10)
#[derive(Debug, Clone)]
pub struct Pir {
    pub head_num: u8,
    pub site_num: u8,
}

impl Pir {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Pir {
            head_num: r.read_u1()?,
            site_num: r.read_u1()?,
        })
    }
}
