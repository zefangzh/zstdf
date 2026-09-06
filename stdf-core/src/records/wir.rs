use crate::error::Result;
use crate::fields::FieldReader;

/// WIR — Wafer Information Record (2, 10)
#[derive(Debug, Clone)]
pub struct Wir {
    pub head_num: u8,
    pub site_grp: Option<u8>,
    pub start_t: Option<u32>,
    pub wafer_id: Option<String>,
}

impl Wir {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Wir {
            head_num: r.read_u1()?,
            site_grp: if r.remaining() > 0 {
                Some(r.read_u1()?)
            } else {
                None
            },
            start_t: if r.remaining() >= 4 {
                Some(r.read_u4()?)
            } else {
                None
            },
            wafer_id: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
        })
    }
}
