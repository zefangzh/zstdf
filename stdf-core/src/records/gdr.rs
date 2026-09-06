use crate::error::Result;
use crate::fields::FieldReader;
use crate::types::VarData;

/// GDR — Generic Data Record (50, 10)
#[derive(Debug, Clone)]
pub struct Gdr {
    pub fld_cnt: u16,
    pub gen_data: Vec<VarData>,
}

impl Gdr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let fld_cnt = r.read_u2()?;
        let mut gen_data = Vec::with_capacity(fld_cnt as usize);
        for _ in 0..fld_cnt {
            gen_data.push(r.read_vn()?);
        }
        Ok(Gdr { fld_cnt, gen_data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ByteOrder;

    #[test]
    fn test_gdr_mixed_types() {
        let mut data = Vec::new();
        data.extend_from_slice(&3u16.to_le_bytes()); // fld_cnt=3
                                                     // V*n: type=1(U*1), value=42
        data.push(1);
        data.push(42);
        // V*n: type=10(C*n), len=3, "ABC"
        data.push(10);
        data.push(3);
        data.extend_from_slice(b"ABC");
        // V*n: type=7(R*4), value=1.5
        data.push(7);
        data.extend_from_slice(&1.5f32.to_le_bytes());

        let mut r = FieldReader::new(&data, ByteOrder::LittleEndian);
        let gdr = Gdr::parse(&mut r).unwrap();
        assert_eq!(gdr.fld_cnt, 3);
        assert_eq!(gdr.gen_data[0], VarData::U1(42));
        assert_eq!(gdr.gen_data[1], VarData::Cn("ABC".to_string()));
        if let VarData::R4(v) = gdr.gen_data[2] {
            assert!((v - 1.5).abs() < 0.001);
        } else {
            panic!("expected R4");
        }
    }
}
