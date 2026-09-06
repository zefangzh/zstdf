use crate::error::Result;
use crate::fields::FieldReader;

/// PLR — Pin List Record (1, 63)
#[derive(Debug, Clone)]
pub struct Plr {
    pub grp_cnt: u16,
    pub grp_indx: Vec<u16>,
    pub grp_mode: Vec<u16>,
    pub grp_radx: Vec<u8>,
    pub pgm_char: Vec<String>,
    pub rtn_char: Vec<String>,
    pub pgm_chal: Vec<String>,
    pub rtn_chal: Vec<String>,
}

impl Plr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let grp_cnt = r.read_u2()?;
        let k = grp_cnt as usize;
        let grp_indx = if r.remaining() > 0 {
            r.read_u2_array(k)?
        } else {
            Vec::new()
        };
        let grp_mode = if r.remaining() > 0 {
            r.read_u2_array(k)?
        } else {
            Vec::new()
        };
        let grp_radx = if r.remaining() > 0 {
            r.read_u1_array(k)?
        } else {
            Vec::new()
        };
        let pgm_char = if r.remaining() > 0 {
            r.read_cn_array(k)?
        } else {
            Vec::new()
        };
        let rtn_char = if r.remaining() > 0 {
            r.read_cn_array(k)?
        } else {
            Vec::new()
        };
        let pgm_chal = if r.remaining() > 0 {
            r.read_cn_array(k)?
        } else {
            Vec::new()
        };
        let rtn_chal = if r.remaining() > 0 {
            r.read_cn_array(k)?
        } else {
            Vec::new()
        };
        Ok(Plr {
            grp_cnt,
            grp_indx,
            grp_mode,
            grp_radx,
            pgm_char,
            rtn_char,
            pgm_chal,
            rtn_chal,
        })
    }
}
