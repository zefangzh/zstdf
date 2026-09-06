use crate::error::Result;
use crate::fields::FieldReader;

/// SBR — Software Bin Record (1, 50)
#[derive(Debug, Clone)]
pub struct Sbr {
    pub head_num: u8,
    pub site_num: u8,
    pub sbin_num: u16,
    pub sbin_cnt: u32,
    pub sbin_pf: Option<u8>,
    pub sbin_nam: Option<String>,
}

impl Sbr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Sbr {
            head_num: r.read_u1()?,
            site_num: r.read_u1()?,
            sbin_num: r.read_u2()?,
            sbin_cnt: r.read_u4()?,
            sbin_pf: if r.remaining() > 0 {
                Some(r.read_c1()?)
            } else {
                None
            },
            sbin_nam: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
        })
    }
}
