use crate::error::Result;
use crate::fields::FieldReader;

/// DTR — Datalog Text Record (50, 30)
#[derive(Debug, Clone)]
pub struct Dtr {
    pub text_dat: String,
}

impl Dtr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Dtr {
            text_dat: r.read_cn()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ByteOrder;

    #[test]
    fn test_dtr_text() {
        let mut data = Vec::new();
        data.push(13);
        data.extend_from_slice(b"operator note");

        let mut r = FieldReader::new(&data, ByteOrder::LittleEndian);
        let dtr = Dtr::parse(&mut r).unwrap();

        assert_eq!(dtr.text_dat, "operator note");
    }
}
