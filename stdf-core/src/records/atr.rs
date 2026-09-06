use crate::error::Result;
use crate::fields::FieldReader;

/// ATR — Audit Trail Record (0, 20)
#[derive(Debug, Clone)]
pub struct Atr {
    pub mod_tim: u32,
    pub cmd_line: Option<String>,
}

impl Atr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let mod_tim = r.read_u4()?;
        let cmd_line = if r.remaining() > 0 {
            Some(r.read_cn()?)
        } else {
            None
        };
        Ok(Atr { mod_tim, cmd_line })
    }
}
