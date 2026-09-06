use crate::error::Result;
use crate::fields::FieldReader;

/// PRR — Part Results Record (5, 20)
#[derive(Debug, Clone)]
pub struct Prr {
    pub head_num: u8,
    pub site_num: u8,
    pub part_flg: u8,
    pub num_test: u16,
    pub hard_bin: u16,
    pub soft_bin: u16,
    pub x_coord: Option<i16>,
    pub y_coord: Option<i16>,
    pub test_t: Option<u32>,
    pub part_id: Option<String>,
    pub part_txt: Option<String>,
    pub part_fix: Option<Vec<u8>>,
}

impl Prr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Prr {
            head_num: r.read_u1()?,
            site_num: r.read_u1()?,
            part_flg: r.read_b1()?,
            num_test: r.read_u2()?,
            hard_bin: r.read_u2()?,
            soft_bin: r.read_u2()?,
            x_coord: if r.remaining() >= 2 {
                Some(r.read_i2()?)
            } else {
                None
            },
            y_coord: if r.remaining() >= 2 {
                Some(r.read_i2()?)
            } else {
                None
            },
            test_t: if r.remaining() >= 4 {
                Some(r.read_u4()?)
            } else {
                None
            },
            part_id: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            part_txt: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            part_fix: if r.remaining() > 0 {
                Some(r.read_bn()?)
            } else {
                None
            },
        })
    }

    /// Check if part passed (PART_FLG bit 3 = 0 means pass when bit 4 = 0)
    pub fn passed(&self) -> bool {
        // Bit 4: 1 = no pass/fail indication (abnormal completion)
        // Bit 3: 0 = part passed, 1 = part failed
        (self.part_flg & 0x10) == 0 && (self.part_flg & 0x08) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ByteOrder;

    #[test]
    fn test_prr_mandatory_only() {
        let mut data = Vec::new();
        data.push(1); // head
        data.push(0); // site
        data.push(0x00); // part_flg (pass)
        data.extend_from_slice(&5u16.to_le_bytes()); // num_test
        data.extend_from_slice(&1u16.to_le_bytes()); // hard_bin
        data.extend_from_slice(&1u16.to_le_bytes()); // soft_bin

        let mut r = FieldReader::new(&data, ByteOrder::LittleEndian);
        let prr = Prr::parse(&mut r).unwrap();
        assert_eq!(prr.num_test, 5);
        assert!(prr.passed());
        assert!(prr.x_coord.is_none());
    }
}
