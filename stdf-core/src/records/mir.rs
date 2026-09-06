use crate::error::Result;
use crate::fields::FieldReader;

/// MIR — Master Information Record (1, 10). ~38 fields, mostly C*n strings.
#[derive(Debug, Clone)]
pub struct Mir {
    // ── Mandatory fields ──
    pub setup_t: u32,
    pub start_t: u32,
    pub stat_num: u8,
    pub mode_cod: u8,
    pub rtst_cod: u8,
    pub prot_cod: u8,
    pub burn_tim: u16,
    pub cmod_cod: u8,
    pub lot_id: String,
    pub part_typ: String,
    pub node_nam: String,
    pub tstr_typ: String,
    pub job_nam: String,
    // ── Optional fields (all C*n) ──
    pub job_rev: Option<String>,
    pub sblot_id: Option<String>,
    pub oper_nam: Option<String>,
    pub exec_typ: Option<String>,
    pub exec_ver: Option<String>,
    pub test_cod: Option<String>,
    pub tst_temp: Option<String>,
    pub user_txt: Option<String>,
    pub aux_file: Option<String>,
    pub pkg_typ: Option<String>,
    pub famly_id: Option<String>,
    pub date_cod: Option<String>,
    pub facil_id: Option<String>,
    pub floor_id: Option<String>,
    pub proc_id: Option<String>,
    pub oper_frq: Option<String>,
    pub spec_nam: Option<String>,
    pub spec_ver: Option<String>,
    pub flow_id: Option<String>,
    pub setup_id: Option<String>,
    pub dsgn_rev: Option<String>,
    pub eng_id: Option<String>,
    pub rom_cod: Option<String>,
    pub serl_num: Option<String>,
    pub supr_nam: Option<String>,
}

impl Mir {
    pub fn parse(r: &mut FieldReader) -> Result<Self> {
        let setup_t = r.read_u4()?;
        let start_t = r.read_u4()?;
        let stat_num = r.read_u1()?;
        let mode_cod = r.read_c1()?;
        let rtst_cod = r.read_c1()?;
        let prot_cod = r.read_c1()?;
        let burn_tim = r.read_u2()?;
        let cmod_cod = r.read_c1()?;
        let lot_id = r.read_cn()?;
        let part_typ = r.read_cn()?;
        let node_nam = r.read_cn()?;
        let tstr_typ = r.read_cn()?;
        let job_nam = r.read_cn()?;

        // Helper: read optional C*n
        macro_rules! opt_cn {
            ($r:expr) => {
                if $r.remaining() > 0 {
                    Some($r.read_cn()?)
                } else {
                    None
                }
            };
        }

        Ok(Mir {
            setup_t,
            start_t,
            stat_num,
            mode_cod,
            rtst_cod,
            prot_cod,
            burn_tim,
            cmod_cod,
            lot_id,
            part_typ,
            node_nam,
            tstr_typ,
            job_nam,
            job_rev: opt_cn!(r),
            sblot_id: opt_cn!(r),
            oper_nam: opt_cn!(r),
            exec_typ: opt_cn!(r),
            exec_ver: opt_cn!(r),
            test_cod: opt_cn!(r),
            tst_temp: opt_cn!(r),
            user_txt: opt_cn!(r),
            aux_file: opt_cn!(r),
            pkg_typ: opt_cn!(r),
            famly_id: opt_cn!(r),
            date_cod: opt_cn!(r),
            facil_id: opt_cn!(r),
            floor_id: opt_cn!(r),
            proc_id: opt_cn!(r),
            oper_frq: opt_cn!(r),
            spec_nam: opt_cn!(r),
            spec_ver: opt_cn!(r),
            flow_id: opt_cn!(r),
            setup_id: opt_cn!(r),
            dsgn_rev: opt_cn!(r),
            eng_id: opt_cn!(r),
            rom_cod: opt_cn!(r),
            serl_num: opt_cn!(r),
            supr_nam: opt_cn!(r),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ByteOrder;

    #[test]
    fn test_mir_mandatory_only() {
        let mut data = Vec::new();
        data.extend_from_slice(&1000u32.to_le_bytes()); // setup_t
        data.extend_from_slice(&2000u32.to_le_bytes()); // start_t
        data.push(1); // stat_num
        data.push(b'P'); // mode_cod
        data.push(b' '); // rtst_cod
        data.push(b' '); // prot_cod
        data.extend_from_slice(&0u16.to_le_bytes()); // burn_tim
        data.push(b' '); // cmod_cod
        data.push(6);
        data.extend_from_slice(b"LOT001"); // lot_id
        data.push(6);
        data.extend_from_slice(b"CHIP_A"); // part_typ
        data.push(4);
        data.extend_from_slice(b"ND01"); // node_nam
        data.push(2);
        data.extend_from_slice(b"J7"); // tstr_typ
        data.push(8);
        data.extend_from_slice(b"TEST_001"); // job_nam

        let mut r = FieldReader::new(&data, ByteOrder::LittleEndian);
        let mir = Mir::parse(&mut r).unwrap();
        assert_eq!(mir.lot_id, "LOT001");
        assert_eq!(mir.part_typ, "CHIP_A");
        assert_eq!(mir.job_nam, "TEST_001");
        assert_eq!(mir.mode_cod, b'P');
        assert!(mir.job_rev.is_none());
        assert!(mir.supr_nam.is_none());
    }
}
