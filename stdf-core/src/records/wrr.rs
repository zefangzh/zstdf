use crate::error::Result;
use crate::fields::FieldReader;

/// WRR — Wafer Results Record (2, 20)
#[derive(Debug, Clone)]
pub struct Wrr {
    pub head_num: u8,
    pub site_grp: u8,
    pub finish_t: u32,
    pub part_cnt: u32,
    pub rtst_cnt: Option<u32>,
    pub abrt_cnt: Option<u32>,
    pub good_cnt: Option<u32>,
    pub func_cnt: Option<u32>,
    pub wafer_id: Option<String>,
    pub fabwf_id: Option<String>,
    pub frame_id: Option<String>,
    pub mask_id: Option<String>,
    pub usr_desc: Option<String>,
    pub exc_desc: Option<String>,
}

impl Wrr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let head_num = r.read_u1()?;
        let site_grp = r.read_u1()?;
        let finish_t = r.read_u4()?;
        let part_cnt = r.read_u4()?;
        macro_rules! opt_u4 {
            ($r:expr) => {
                if $r.remaining() >= 4 {
                    Some($r.read_u4()?)
                } else {
                    None
                }
            };
        }
        macro_rules! opt_cn {
            ($r:expr) => {
                if $r.remaining() > 0 {
                    Some($r.read_cn()?)
                } else {
                    None
                }
            };
        }
        Ok(Wrr {
            head_num,
            site_grp,
            finish_t,
            part_cnt,
            rtst_cnt: opt_u4!(r),
            abrt_cnt: opt_u4!(r),
            good_cnt: opt_u4!(r),
            func_cnt: opt_u4!(r),
            wafer_id: opt_cn!(r),
            fabwf_id: opt_cn!(r),
            frame_id: opt_cn!(r),
            mask_id: opt_cn!(r),
            usr_desc: opt_cn!(r),
            exc_desc: opt_cn!(r),
        })
    }
}
