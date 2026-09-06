use crate::error::Result;
use crate::fields::FieldReader;

/// PMR — Pin Map Record (1, 60)
#[derive(Debug, Clone)]
pub struct Pmr {
    pub pmr_indx: u16,
    pub chan_typ: Option<u16>,
    pub chan_nam: Option<String>,
    pub phy_nam: Option<String>,
    pub log_nam: Option<String>,
    pub head_num: Option<u8>,
    pub site_num: Option<u8>,
}

impl Pmr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Pmr {
            pmr_indx: r.read_u2()?,
            chan_typ: if r.remaining() >= 2 {
                Some(r.read_u2()?)
            } else {
                None
            },
            chan_nam: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            phy_nam: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            log_nam: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            head_num: if r.remaining() > 0 {
                Some(r.read_u1()?)
            } else {
                None
            },
            site_num: if r.remaining() > 0 {
                Some(r.read_u1()?)
            } else {
                None
            },
        })
    }
}
