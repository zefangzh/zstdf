use crate::error::Result;
use crate::fields::FieldReader;

/// PCR — Part Count Record (1, 30)
#[derive(Debug, Clone)]
pub struct Pcr {
    pub head_num: u8,
    pub site_num: u8,
    pub part_cnt: u32,
    pub rtst_cnt: Option<u32>,
    pub abrt_cnt: Option<u32>,
    pub good_cnt: Option<u32>,
    pub func_cnt: Option<u32>,
}

impl Pcr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Pcr {
            head_num: r.read_u1()?,
            site_num: r.read_u1()?,
            part_cnt: r.read_u4()?,
            rtst_cnt: if r.remaining() >= 4 {
                Some(r.read_u4()?)
            } else {
                None
            },
            abrt_cnt: if r.remaining() >= 4 {
                Some(r.read_u4()?)
            } else {
                None
            },
            good_cnt: if r.remaining() >= 4 {
                Some(r.read_u4()?)
            } else {
                None
            },
            func_cnt: if r.remaining() >= 4 {
                Some(r.read_u4()?)
            } else {
                None
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ByteOrder;

    #[test]
    fn test_pcr_full_counts() {
        let mut data = Vec::new();
        data.push(1);
        data.push(0);
        data.extend_from_slice(&100u32.to_le_bytes());
        data.extend_from_slice(&5u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&94u32.to_le_bytes());
        data.extend_from_slice(&99u32.to_le_bytes());

        let mut r = FieldReader::new(&data, ByteOrder::LittleEndian);
        let pcr = Pcr::parse(&mut r).unwrap();

        assert_eq!(pcr.head_num, 1);
        assert_eq!(pcr.site_num, 0);
        assert_eq!(pcr.part_cnt, 100);
        assert_eq!(pcr.rtst_cnt, Some(5));
        assert_eq!(pcr.abrt_cnt, Some(1));
        assert_eq!(pcr.good_cnt, Some(94));
        assert_eq!(pcr.func_cnt, Some(99));
    }
}
