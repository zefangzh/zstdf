use crate::error::Result;
use crate::fields::FieldReader;

/// PTR — Parametric Test Record (15, 10). The most common record type.
#[derive(Debug, Clone)]
pub struct Ptr {
    // ── Mandatory ──
    pub test_num: u32,
    pub head_num: u8,
    pub site_num: u8,
    /// Bit 0: alarm, bit 6: pass/fail valid, bit 7: 0=pass 1=fail
    pub test_flg: u8,
    pub parm_flg: u8,
    pub result: f32,
    // ── Optional ──
    pub test_txt: Option<String>,
    pub alarm_id: Option<String>,
    pub opt_flag: Option<u8>,
    pub res_scal: Option<i8>,
    pub llm_scal: Option<i8>,
    pub hlm_scal: Option<i8>,
    pub lo_limit: Option<f32>,
    pub hi_limit: Option<f32>,
    pub units: Option<String>,
    pub c_resfmt: Option<String>,
    pub c_llmfmt: Option<String>,
    pub c_hlmfmt: Option<String>,
    pub lo_spec: Option<f32>,
    pub hi_spec: Option<f32>,
}

impl Ptr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let test_num = r.read_u4()?;
        let head_num = r.read_u1()?;
        let site_num = r.read_u1()?;
        let test_flg = r.read_b1()?;
        let parm_flg = r.read_b1()?;
        let result = r.read_r4()?;

        Ok(Ptr {
            test_num,
            head_num,
            site_num,
            test_flg,
            parm_flg,
            result,
            test_txt: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            alarm_id: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            opt_flag: if r.remaining() > 0 {
                Some(r.read_b1()?)
            } else {
                None
            },
            res_scal: if r.remaining() > 0 {
                Some(r.read_i1()?)
            } else {
                None
            },
            llm_scal: if r.remaining() > 0 {
                Some(r.read_i1()?)
            } else {
                None
            },
            hlm_scal: if r.remaining() > 0 {
                Some(r.read_i1()?)
            } else {
                None
            },
            lo_limit: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            hi_limit: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            units: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            c_resfmt: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            c_llmfmt: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            c_hlmfmt: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            lo_spec: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            hi_spec: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
        })
    }

    /// Did this test pass? (TEST_FLG bit 7: 0=pass, 1=fail)
    pub fn passed(&self) -> bool {
        (self.test_flg & 0x80) == 0
    }

    /// Is the pass/fail indication valid? (TEST_FLG bit 6)
    pub fn pass_fail_valid(&self) -> bool {
        (self.test_flg & 0x40) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ByteOrder;

    #[test]
    fn test_ptr_mandatory_only() {
        let mut data = Vec::new();
        data.extend_from_slice(&1001u32.to_le_bytes());
        data.push(1); // head
        data.push(0); // site
        data.push(0x00); // test_flg (pass)
        data.push(0x00); // parm_flg
        data.extend_from_slice(&3.14f32.to_le_bytes());

        let mut r = FieldReader::new(&data, ByteOrder::LittleEndian);
        let ptr = Ptr::parse(&mut r).unwrap();
        assert_eq!(ptr.test_num, 1001);
        assert!((ptr.result - 3.14).abs() < 0.01);
        assert!(ptr.passed());
        assert!(ptr.test_txt.is_none());
    }

    #[test]
    fn test_ptr_with_text_and_limits() {
        let mut data = Vec::new();
        data.extend_from_slice(&2002u32.to_le_bytes());
        data.push(1);
        data.push(0);
        data.push(0x80); // test_flg = fail
        data.push(0x00);
        data.extend_from_slice(&5.5f32.to_le_bytes());
        // test_txt
        data.push(7);
        data.extend_from_slice(b"VDD_MAX");
        // alarm_id
        data.push(0);
        // opt_flag
        data.push(0x00);
        // res_scal, llm_scal, hlm_scal
        data.push(0);
        data.push(0);
        data.push(0);
        // lo_limit, hi_limit
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&5.0f32.to_le_bytes());
        // units
        data.push(1);
        data.push(b'V');

        let mut r = FieldReader::new(&data, ByteOrder::LittleEndian);
        let ptr = Ptr::parse(&mut r).unwrap();
        assert_eq!(ptr.test_num, 2002);
        assert!(!ptr.passed());
        assert_eq!(ptr.test_txt, Some("VDD_MAX".to_string()));
        assert!((ptr.lo_limit.unwrap() - 1.0).abs() < 0.001);
        assert!((ptr.hi_limit.unwrap() - 5.0).abs() < 0.001);
        assert_eq!(ptr.units, Some("V".to_string()));
    }
}
