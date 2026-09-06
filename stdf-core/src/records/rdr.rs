use crate::error::Result;
use crate::fields::FieldReader;

/// RDR — Retest Data Record (1, 70)
#[derive(Debug, Clone)]
pub struct Rdr {
    pub num_bins: u16,
    pub rtst_bin: Vec<u16>,
}

impl Rdr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let num_bins = r.read_u2()?;
        let rtst_bin = r.read_u2_array(num_bins as usize)?;
        Ok(Rdr { num_bins, rtst_bin })
    }
}
