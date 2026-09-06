use crate::error::Result;
use crate::fields::FieldReader;

/// BPS — Begin Program Section Record (20, 10)
#[derive(Debug, Clone)]
pub struct Bps {
    pub seq_name: Option<String>,
}

impl Bps {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Bps {
            seq_name: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
        })
    }
}
