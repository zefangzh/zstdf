use crate::error::Result;
use crate::fields::FieldReader;

/// PGR — Pin Group Record (1, 62)
#[derive(Debug, Clone)]
pub struct Pgr {
    pub grp_indx: u16,
    pub grp_nam: String,
    pub indx_cnt: u16,
    pub pmr_indx: Vec<u16>,
}

impl Pgr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let grp_indx = r.read_u2()?;
        let grp_nam = r.read_cn()?;
        let indx_cnt = r.read_u2()?;
        let pmr_indx = r.read_u2_array(indx_cnt as usize)?;
        Ok(Pgr {
            grp_indx,
            grp_nam,
            indx_cnt,
            pmr_indx,
        })
    }
}
