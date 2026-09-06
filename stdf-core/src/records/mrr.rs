use crate::error::Result;
use crate::fields::FieldReader;

/// MRR — Master Results Record (1, 20)
#[derive(Debug, Clone)]
pub struct Mrr {
    pub finish_t: u32,
    pub disp_cod: Option<u8>,
    pub usr_desc: Option<String>,
    pub exc_desc: Option<String>,
}

impl Mrr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let finish_t = r.read_u4()?;
        let disp_cod = if r.remaining() > 0 {
            Some(r.read_c1()?)
        } else {
            None
        };
        let usr_desc = if r.remaining() > 0 {
            Some(r.read_cn()?)
        } else {
            None
        };
        let exc_desc = if r.remaining() > 0 {
            Some(r.read_cn()?)
        } else {
            None
        };
        Ok(Mrr {
            finish_t,
            disp_cod,
            usr_desc,
            exc_desc,
        })
    }
}
