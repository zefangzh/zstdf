use crate::error::Result;
use crate::fields::FieldReader;

/// WCR — Wafer Configuration Record (2, 30). All fields optional.
#[derive(Debug, Clone)]
pub struct Wcr {
    pub wafr_siz: Option<f32>,
    pub die_ht: Option<f32>,
    pub die_wid: Option<f32>,
    pub wf_units: Option<u8>,
    pub wf_flat: Option<u8>,
    pub center_x: Option<i16>,
    pub center_y: Option<i16>,
    pub pos_x: Option<u8>,
    pub pos_y: Option<u8>,
}

impl Wcr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        Ok(Wcr {
            wafr_siz: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            die_ht: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            die_wid: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            wf_units: if r.remaining() > 0 {
                Some(r.read_u1()?)
            } else {
                None
            },
            wf_flat: if r.remaining() > 0 {
                Some(r.read_c1()?)
            } else {
                None
            },
            center_x: if r.remaining() >= 2 {
                Some(r.read_i2()?)
            } else {
                None
            },
            center_y: if r.remaining() >= 2 {
                Some(r.read_i2()?)
            } else {
                None
            },
            pos_x: if r.remaining() > 0 {
                Some(r.read_c1()?)
            } else {
                None
            },
            pos_y: if r.remaining() > 0 {
                Some(r.read_c1()?)
            } else {
                None
            },
        })
    }
}
