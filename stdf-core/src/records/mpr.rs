use crate::error::Result;
use crate::fields::FieldReader;

/// MPR — Multiple-Result Parametric Record (15, 15)
#[derive(Debug, Clone)]
pub struct Mpr {
    pub test_num: u32,
    pub head_num: u8,
    pub site_num: u8,
    pub test_flg: u8,
    pub parm_flg: u8,
    pub rtn_icnt: u16,
    pub rslt_cnt: u16,
    pub rtn_stat: Option<Vec<u8>>,
    pub rtn_rslt: Option<Vec<f32>>,
    pub test_txt: Option<String>,
    pub alarm_id: Option<String>,
    pub opt_flag: Option<u8>,
    pub res_scal: Option<i8>,
    pub llm_scal: Option<i8>,
    pub hlm_scal: Option<i8>,
    pub lo_limit: Option<f32>,
    pub hi_limit: Option<f32>,
    pub start_in: Option<f32>,
    pub incr_in: Option<f32>,
    pub rtn_indx: Option<Vec<u16>>,
    pub units: Option<String>,
    pub units_in: Option<String>,
    pub c_resfmt: Option<String>,
    pub c_llmfmt: Option<String>,
    pub c_hlmfmt: Option<String>,
    pub lo_spec: Option<f32>,
    pub hi_spec: Option<f32>,
}

impl Mpr {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let test_num = r.read_u4()?;
        let head_num = r.read_u1()?;
        let site_num = r.read_u1()?;
        let test_flg = r.read_b1()?;
        let parm_flg = r.read_b1()?;
        let rtn_icnt = r.read_u2()?;
        let rslt_cnt = r.read_u2()?;

        // RTN_STAT: nibble array of length rtn_icnt
        let rtn_stat = if r.remaining() > 0 && rtn_icnt > 0 {
            Some(r.read_nibble_array(rtn_icnt as usize)?)
        } else {
            None
        };

        // RTN_RSLT: R*4 array of length rslt_cnt
        let rtn_rslt = if r.remaining() > 0 && rslt_cnt > 0 {
            Some(r.read_r4_array(rslt_cnt as usize)?)
        } else {
            None
        };

        Ok(Mpr {
            test_num,
            head_num,
            site_num,
            test_flg,
            parm_flg,
            rtn_icnt,
            rslt_cnt,
            rtn_stat,
            rtn_rslt,
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
            start_in: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            incr_in: if r.remaining() >= 4 {
                Some(r.read_r4()?)
            } else {
                None
            },
            rtn_indx: if r.remaining() > 0 && rtn_icnt > 0 {
                Some(r.read_u2_array(rtn_icnt as usize)?)
            } else {
                None
            },
            units: if r.remaining() > 0 {
                Some(r.read_cn()?)
            } else {
                None
            },
            units_in: if r.remaining() > 0 {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ByteOrder;

    #[test]
    fn test_mpr_with_arrays() {
        let mut data = Vec::new();
        data.extend_from_slice(&3001u32.to_le_bytes());
        data.push(1);
        data.push(0);
        data.push(0x00);
        data.push(0x00);
        data.extend_from_slice(&3u16.to_le_bytes()); // rtn_icnt=3
        data.extend_from_slice(&3u16.to_le_bytes()); // rslt_cnt=3
                                                     // rtn_stat: 3 nibbles = 2 bytes: [0x10, 0x02] → nibbles 0,1,2
        data.push(0x10);
        data.push(0x02);
        // rtn_rslt: 3 × R*4
        data.extend_from_slice(&1.0f32.to_le_bytes());
        data.extend_from_slice(&2.0f32.to_le_bytes());
        data.extend_from_slice(&3.0f32.to_le_bytes());

        let mut r = FieldReader::new(&data, ByteOrder::LittleEndian);
        let mpr = Mpr::parse(&mut r).unwrap();
        assert_eq!(mpr.test_num, 3001);
        assert_eq!(mpr.rtn_icnt, 3);
        assert_eq!(mpr.rtn_stat.as_ref().unwrap().len(), 3);
        let rslt = mpr.rtn_rslt.unwrap();
        assert_eq!(rslt.len(), 3);
        assert!((rslt[0] - 1.0).abs() < 0.001);
        assert!((rslt[2] - 3.0).abs() < 0.001);
    }
}
