use crate::error::Result;
use crate::fields::FieldReader;

/// FTR — Functional Test Record (15, 20)
#[derive(Debug, Clone)]
pub struct Ftr {
    pub test_num: u32,
    pub head_num: u8,
    pub site_num: u8,
    pub test_flg: u8,
    pub opt_flag: Option<u8>,
    pub cycl_cnt: Option<u32>,
    pub rel_vadr: Option<u32>,
    pub rept_cnt: Option<u32>,
    pub num_fail: Option<u32>,
    pub xfail_ad: Option<i32>,
    pub yfail_ad: Option<i32>,
    pub vect_off: Option<i16>,
    pub rtn_icnt: Option<u16>,
    pub pgm_icnt: Option<u16>,
    pub rtn_indx: Option<Vec<u16>>,
    pub rtn_stat: Option<Vec<u8>>,
    pub pgm_indx: Option<Vec<u16>>,
    pub pgm_stat: Option<Vec<u8>>,
    pub fail_pin: Option<(u16, Vec<u8>)>,
    pub vect_nam: Option<String>,
    pub time_set: Option<String>,
    pub op_code: Option<String>,
    pub test_txt: Option<String>,
    pub alarm_id: Option<String>,
    pub prog_txt: Option<String>,
    pub rslt_txt: Option<String>,
    pub patg_num: Option<u8>,
    pub spin_map: Option<(u16, Vec<u8>)>,
}

impl Ftr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let test_num = r.read_u4()?;
        let head_num = r.read_u1()?;
        let site_num = r.read_u1()?;
        let test_flg = r.read_b1()?;

        let opt_flag = if r.remaining() > 0 {
            Some(r.read_b1()?)
        } else {
            None
        };
        let cycl_cnt = if r.remaining() >= 4 {
            Some(r.read_u4()?)
        } else {
            None
        };
        let rel_vadr = if r.remaining() >= 4 {
            Some(r.read_u4()?)
        } else {
            None
        };
        let rept_cnt = if r.remaining() >= 4 {
            Some(r.read_u4()?)
        } else {
            None
        };
        let num_fail = if r.remaining() >= 4 {
            Some(r.read_u4()?)
        } else {
            None
        };
        let xfail_ad = if r.remaining() >= 4 {
            Some(r.read_i4()?)
        } else {
            None
        };
        let yfail_ad = if r.remaining() >= 4 {
            Some(r.read_i4()?)
        } else {
            None
        };
        let vect_off = if r.remaining() >= 2 {
            Some(r.read_i2()?)
        } else {
            None
        };

        let rtn_icnt = if r.remaining() >= 2 {
            Some(r.read_u2()?)
        } else {
            None
        };
        let pgm_icnt = if r.remaining() >= 2 {
            Some(r.read_u2()?)
        } else {
            None
        };

        let ri = rtn_icnt.unwrap_or(0) as usize;
        let pi = pgm_icnt.unwrap_or(0) as usize;

        let rtn_indx = if r.remaining() > 0 && ri > 0 {
            Some(r.read_u2_array(ri)?)
        } else {
            None
        };
        let rtn_stat = if r.remaining() > 0 && ri > 0 {
            Some(r.read_nibble_array(ri)?)
        } else {
            None
        };
        let pgm_indx = if r.remaining() > 0 && pi > 0 {
            Some(r.read_u2_array(pi)?)
        } else {
            None
        };
        let pgm_stat = if r.remaining() > 0 && pi > 0 {
            Some(r.read_nibble_array(pi)?)
        } else {
            None
        };

        let fail_pin = if r.remaining() > 0 {
            Some(r.read_dn()?)
        } else {
            None
        };
        let vect_nam = if r.remaining() > 0 {
            Some(r.read_cn()?)
        } else {
            None
        };
        let time_set = if r.remaining() > 0 {
            Some(r.read_cn()?)
        } else {
            None
        };
        let op_code = if r.remaining() > 0 {
            Some(r.read_cn()?)
        } else {
            None
        };
        let test_txt = if r.remaining() > 0 {
            Some(r.read_cn()?)
        } else {
            None
        };
        let alarm_id = if r.remaining() > 0 {
            Some(r.read_cn()?)
        } else {
            None
        };
        let prog_txt = if r.remaining() > 0 {
            Some(r.read_cn()?)
        } else {
            None
        };
        let rslt_txt = if r.remaining() > 0 {
            Some(r.read_cn()?)
        } else {
            None
        };
        let patg_num = if r.remaining() > 0 {
            Some(r.read_u1()?)
        } else {
            None
        };
        let spin_map = if r.remaining() > 0 {
            Some(r.read_dn()?)
        } else {
            None
        };

        Ok(Ftr {
            test_num,
            head_num,
            site_num,
            test_flg,
            opt_flag,
            cycl_cnt,
            rel_vadr,
            rept_cnt,
            num_fail,
            xfail_ad,
            yfail_ad,
            vect_off,
            rtn_icnt,
            pgm_icnt,
            rtn_indx,
            rtn_stat,
            pgm_indx,
            pgm_stat,
            fail_pin,
            vect_nam,
            time_set,
            op_code,
            test_txt,
            alarm_id,
            prog_txt,
            rslt_txt,
            patg_num,
            spin_map,
        })
    }

    pub fn passed(&self) -> bool {
        (self.test_flg & 0x80) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ByteOrder;

    #[test]
    fn test_ftr_with_pin_arrays_and_text() {
        let mut data = Vec::new();
        data.extend_from_slice(&4001u32.to_le_bytes());
        data.push(1);
        data.push(0);
        data.push(0x80);
        data.push(0x00);
        data.extend_from_slice(&10u32.to_le_bytes());
        data.extend_from_slice(&20u32.to_le_bytes());
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&(-3i32).to_le_bytes());
        data.extend_from_slice(&4i32.to_le_bytes());
        data.extend_from_slice(&(-1i16).to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&11u16.to_le_bytes());
        data.extend_from_slice(&12u16.to_le_bytes());
        data.push(0x21);
        data.extend_from_slice(&21u16.to_le_bytes());
        data.extend_from_slice(&22u16.to_le_bytes());
        data.push(0x43);
        data.extend_from_slice(&9u16.to_le_bytes());
        data.extend_from_slice(&[0b1010_0101, 0b0000_0001]);
        data.push(4);
        data.extend_from_slice(b"VEC1");
        data.push(3);
        data.extend_from_slice(b"TS1");
        data.push(2);
        data.extend_from_slice(b"OP");
        data.push(8);
        data.extend_from_slice(b"FUNCFAIL");
        data.push(0);
        data.push(4);
        data.extend_from_slice(b"PROG");
        data.push(4);
        data.extend_from_slice(b"RSLT");
        data.push(7);
        data.extend_from_slice(&3u16.to_le_bytes());
        data.push(0b0000_0101);

        let mut r = FieldReader::new(&data, ByteOrder::LittleEndian);
        let ftr = Ftr::parse(&mut r).unwrap();

        assert_eq!(ftr.test_num, 4001);
        assert!(!ftr.passed());
        assert_eq!(ftr.rtn_indx.as_deref(), Some(&[11, 12][..]));
        assert_eq!(ftr.rtn_stat.as_deref(), Some(&[1, 2][..]));
        assert_eq!(ftr.pgm_indx.as_deref(), Some(&[21, 22][..]));
        assert_eq!(ftr.pgm_stat.as_deref(), Some(&[3, 4][..]));
        assert_eq!(ftr.fail_pin.as_ref().unwrap().0, 9);
        assert_eq!(ftr.vect_nam.as_deref(), Some("VEC1"));
        assert_eq!(ftr.test_txt.as_deref(), Some("FUNCFAIL"));
        assert_eq!(ftr.patg_num, Some(7));
        assert_eq!(ftr.spin_map.as_ref().unwrap().0, 3);
    }
}
