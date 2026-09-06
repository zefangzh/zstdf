use crate::error::Result;
use crate::fields::FieldReader;

/// EPS — End Program Section Record (20, 20). No data fields.
#[derive(Debug, Clone)]
pub struct Eps;

impl Eps {
    pub fn parse(_r: &mut FieldReader) -> Result<Self> {
        Ok(Eps)
    }
}
