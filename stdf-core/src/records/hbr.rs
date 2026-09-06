use crate::error::Result;
use crate::fields::FieldReader;

/// HBR — Hardware Bin Record (1, 40)
#[derive(Debug, Clone)]
pub struct Hbr {
    pub head_num: u8,
    pub site_num: u8,
    pub hbin_num: u16,
    pub hbin_cnt: u32,
    pub hbin_pf: Option<u8>,
    pub hbin_nam: Option<String>,
}

impl Hbr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Hbr {
            head_num: r.read_u1()?,
            site_num: r.read_u1()?,
            hbin_num: r.read_u2()?,
            hbin_cnt: r.read_u4()?,
            hbin_pf: if r.remaining() > 0 {
                Some(r.read_c1()?)
            } else {
                None
            },
            hbin_nam: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
        })
    }
}
