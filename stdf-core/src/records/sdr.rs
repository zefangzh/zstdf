use crate::error::Result;
use crate::fields::FieldReader;

/// SDR — Site Description Record (1, 80)
#[derive(Debug, Clone)]
pub struct Sdr {
    pub head_num: u8,
    pub site_grp: u8,
    pub site_cnt: u8,
    pub site_num: Vec<u8>,
    pub hand_typ: Option<String>,
    pub hand_id: Option<String>,
    pub card_typ: Option<String>,
    pub card_id: Option<String>,
    pub load_typ: Option<String>,
    pub load_id: Option<String>,
    pub dib_typ: Option<String>,
    pub dib_id: Option<String>,
    pub cabl_typ: Option<String>,
    pub cabl_id: Option<String>,
    pub cont_typ: Option<String>,
    pub cont_id: Option<String>,
    pub lasr_typ: Option<String>,
    pub lasr_id: Option<String>,
    pub extr_typ: Option<String>,
    pub extr_id: Option<String>,
}

impl Sdr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let head_num = r.read_u1()?;
        let site_grp = r.read_u1()?;
        let site_cnt = r.read_u1()?;
        let site_num = r.read_u1_array(site_cnt as usize)?;
        macro_rules! opt_cn {
            ($r:expr) => {
                if $r.remaining() > 0 {
                    Some($r.read_cn()?)
                } else {
                    None
                }
            };
        }
        Ok(Sdr {
            head_num,
            site_grp,
            site_cnt,
            site_num,
            hand_typ: opt_cn!(r),
            hand_id: opt_cn!(r),
            card_typ: opt_cn!(r),
            card_id: opt_cn!(r),
            load_typ: opt_cn!(r),
            load_id: opt_cn!(r),
            dib_typ: opt_cn!(r),
            dib_id: opt_cn!(r),
            cabl_typ: opt_cn!(r),
            cabl_id: opt_cn!(r),
            cont_typ: opt_cn!(r),
            cont_id: opt_cn!(r),
            lasr_typ: opt_cn!(r),
            lasr_id: opt_cn!(r),
            extr_typ: opt_cn!(r),
            extr_id: opt_cn!(r),
        })
    }
}
