use crate::error::Result;
use crate::fields::FieldReader;

/// TSR — Test Synopsis Record (10, 30)
#[derive(Debug, Clone)]
pub struct Tsr {
    pub head_num: u8,
    pub site_num: u8,
    pub test_typ: u8,
    pub test_num: u32,
    pub exec_cnt: Option<u32>,
    pub fail_cnt: Option<u32>,
    pub alrm_cnt: Option<u32>,
    pub test_nam: Option<String>,
    pub seq_name: Option<String>,
    pub test_lbl: Option<String>,
    pub opt_flag: Option<u8>,
    pub test_tim: Option<f32>,
    pub test_min: Option<f32>,
    pub test_max: Option<f32>,
    pub tst_sums: Option<f32>,
    pub tst_sqrs: Option<f32>,
}

impl Tsr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let head_num = r.read_u1()?;
        let site_num = r.read_u1()?;
        let test_typ = r.read_c1()?;
        let test_num = r.read_u4()?;
        Ok(Tsr {
            head_num,
            site_num,
            test_typ,
            test_num,
            exec_cnt: if r.remaining() >= 4 {
                Some(r.read_u4()?)
            } else {
                None
            },
            fail_cnt: if r.remaining() >= 4 {
                Some(r.read_u4()?)
            } else {
                None
            },
            alrm_cnt: if r.remaining() >= 4 {
                Some(r.read_u4()?)
            } else {
                None
            },
            test_nam: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            seq_name: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            test_lbl: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            opt_flag: if r.remaining() > 0 {
                Some(r.read_b1()?)
            } else {
                None
            },
            test_tim: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            test_min: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            test_max: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            tst_sums: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            tst_sqrs: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
        })
    }
}
