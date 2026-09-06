use crate::{endian::Endian, error::{Result, StdfError}, reader::RecordReader};

#[derive(Debug, Clone)]
pub struct FarRecord { pub cpu_typ: u8, pub stdf_ver: u8 }

impl FarRecord {
    pub const TYP: u8 = 0;
    pub const SUB: u8 = 10;

    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        let cpu_typ  = r.read_u1()?;
        let stdf_ver = r.read_u1()?;
        if stdf_ver != 4 {
            return Err(StdfError::UnsupportedVersion(stdf_ver));
        }
        Ok(Self { cpu_typ, stdf_ver })
    }

    pub fn endian(&self) -> Result<Endian> {
        match self.cpu_typ {
            1 => Ok(Endian::Big),
            2 => Ok(Endian::Little),
            n => Err(StdfError::InvalidCpuType(n)),
        }
    }
}

/// Audit Trail Record.
#[derive(Debug, Clone)]
pub struct AtrRecord {
    pub mod_tim:  u32,
    pub cmd_line: Option<String>,
}
impl AtrRecord {
    pub const TYP: u8 = 0;
    pub const SUB: u8 = 20;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self { mod_tim: r.read_u4()?, cmd_line: r.maybe_cn()? })
    }
}

#[derive(Debug, Clone, Default)]
pub struct MirRecord {
    pub setup_t:  u32,
    pub start_t:  u32,
    pub stat_num: u8,
    pub mode_cod: Option<char>,
    pub rtst_cod: Option<char>,
    pub prot_cod: Option<char>,
    pub burn_tim: Option<u16>,
    pub cmod_cod: Option<char>,
    pub lot_id:   Option<String>,
    pub part_typ: Option<String>,
    pub node_nam: Option<String>,
    pub tstr_typ: Option<String>,
    pub job_nam:  Option<String>,
    pub job_rev:  Option<String>,
    pub sblot_id: Option<String>,
    pub oper_nam: Option<String>,
    pub exec_typ: Option<String>,
    pub exec_ver: Option<String>,
    pub test_cod: Option<String>,
    pub tst_temp: Option<String>,
    pub user_txt: Option<String>,
    pub aux_file: Option<String>,
    pub pkg_typ:  Option<String>,
    pub famly_id: Option<String>,
    pub date_cod: Option<String>,
    pub facil_id: Option<String>,
    pub floor_id: Option<String>,
    pub proc_id:  Option<String>,
    pub oper_frq: Option<String>,
    pub spec_nam: Option<String>,
    pub spec_ver: Option<String>,
    pub flow_id:  Option<String>,
    pub setup_id: Option<String>,
    pub dsgn_rev: Option<String>,
    pub eng_id:   Option<String>,
    pub rom_cod:  Option<String>,
    pub serl_num: Option<String>,
    pub supr_nam: Option<String>,
}

/// Historically this collapsed space/NUL bytes to `None` on the assumption
/// that a blank flag character meant "not set". The reference ASCII dump
/// (`/opt/hp93000/soc/formatter/bin/STDFreader`) instead prints any C1 byte
/// that was actually present in the record — including a literal space,
/// e.g. `CMOD_COD: ' '` — so presence in the byte stream (not the byte's
/// value) is what determines `Some`/`None` here.
fn opt_char(b: u8) -> Option<char> { Some(b as char) }

macro_rules! read_cn_if {
    ($r:expr, $field:expr) => {
        if !$r.is_empty() { $field = $r.read_cn()?; }
    };
}

impl MirRecord {
    pub const TYP: u8 = 1;
    pub const SUB: u8 = 10;

    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        let mut m = MirRecord::default();
        m.setup_t  = r.read_u4()?;
        m.start_t  = r.read_u4()?;
        m.stat_num = r.read_u1()?;
        m.mode_cod = r.maybe_u1()?.and_then(opt_char);
        m.rtst_cod = r.maybe_u1()?.and_then(opt_char);
        m.prot_cod = r.maybe_u1()?.and_then(opt_char);
        if !r.is_empty() { m.burn_tim = Some(r.read_u2()?); }
        m.cmod_cod = r.maybe_u1()?.and_then(opt_char);
        read_cn_if!(r, m.lot_id);
        read_cn_if!(r, m.part_typ);
        read_cn_if!(r, m.node_nam);
        read_cn_if!(r, m.tstr_typ);
        read_cn_if!(r, m.job_nam);
        read_cn_if!(r, m.job_rev);
        read_cn_if!(r, m.sblot_id);
        read_cn_if!(r, m.oper_nam);
        read_cn_if!(r, m.exec_typ);
        read_cn_if!(r, m.exec_ver);
        read_cn_if!(r, m.test_cod);
        read_cn_if!(r, m.tst_temp);
        read_cn_if!(r, m.user_txt);
        read_cn_if!(r, m.aux_file);
        read_cn_if!(r, m.pkg_typ);
        read_cn_if!(r, m.famly_id);
        read_cn_if!(r, m.date_cod);
        read_cn_if!(r, m.facil_id);
        read_cn_if!(r, m.floor_id);
        read_cn_if!(r, m.proc_id);
        read_cn_if!(r, m.oper_frq);
        read_cn_if!(r, m.spec_nam);
        read_cn_if!(r, m.spec_ver);
        read_cn_if!(r, m.flow_id);
        read_cn_if!(r, m.setup_id);
        read_cn_if!(r, m.dsgn_rev);
        read_cn_if!(r, m.eng_id);
        read_cn_if!(r, m.rom_cod);
        read_cn_if!(r, m.serl_num);
        read_cn_if!(r, m.supr_nam);
        Ok(m)
    }
}

#[derive(Debug, Clone)]
pub struct MrrRecord {
    pub finish_t: u32,
    pub disp_cod: Option<char>,
    pub usr_desc: Option<String>,
    pub exc_desc: Option<String>,
}
impl MrrRecord {
    pub const TYP: u8 = 1;
    pub const SUB: u8 = 20;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self {
            finish_t: r.read_u4()?,
            disp_cod: r.maybe_u1()?.map(|b| b as char),
            usr_desc: r.maybe_cn()?,
            exc_desc: r.maybe_cn()?,
        })
    }
}

/// Part Count Record.
#[derive(Debug, Clone)]
pub struct PcrRecord {
    pub head_num: u8,
    pub site_num: u8,
    pub part_cnt: u32,
    pub rtst_cnt: Option<u32>,
    pub abrt_cnt: Option<u32>,
    pub good_cnt: Option<u32>,
    pub func_cnt: Option<u32>,
}
impl PcrRecord {
    pub const TYP: u8 = 1;
    pub const SUB: u8 = 30;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self {
            head_num: r.read_u1()?,
            site_num: r.read_u1()?,
            part_cnt: r.read_u4()?,
            rtst_cnt: r.maybe_u4()?,
            abrt_cnt: r.maybe_u4()?,
            good_cnt: r.maybe_u4()?,
            func_cnt: r.maybe_u4()?,
        })
    }
}

/// Software Bin Record.
#[derive(Debug, Clone)]
pub struct SbrRecord {
    pub head_num: u8,
    pub site_num: u8,
    pub sbin_num: u16,
    pub sbin_cnt: u32,
    pub sbin_pf:  Option<char>,
    pub sbin_nam: Option<String>,
}
impl SbrRecord {
    pub const TYP: u8 = 1;
    pub const SUB: u8 = 50;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self {
            head_num: r.read_u1()?,
            site_num: r.read_u1()?,
            sbin_num: r.read_u2()?,
            sbin_cnt: r.read_u4()?,
            sbin_pf:  r.maybe_u1()?.map(|b| b as char),
            sbin_nam: r.maybe_cn()?,
        })
    }
}

/// Pin Map Record.
#[derive(Debug, Clone)]
pub struct PmrRecord {
    pub pmr_indx: u16,
    pub chan_typ: Option<u16>,
    pub chan_nam: Option<String>,
    pub phy_nam:  Option<String>,
    pub log_nam:  Option<String>,
    pub head_num: Option<u8>,
    pub site_num: Option<u8>,
}
impl PmrRecord {
    pub const TYP: u8 = 1;
    pub const SUB: u8 = 60;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self {
            pmr_indx: r.read_u2()?,
            chan_typ: r.maybe_u2()?,
            chan_nam: r.maybe_cn()?,
            phy_nam:  r.maybe_cn()?,
            log_nam:  r.maybe_cn()?,
            head_num: r.maybe_u1()?,
            site_num: r.maybe_u1()?,
        })
    }
}

/// Site Description Record.
#[derive(Debug, Clone, Default)]
pub struct SdrRecord {
    pub head_num: u8,
    pub site_grp: u8,
    pub site_cnt: u8,
    pub site_num: Vec<u8>,
    pub hand_typ: Option<String>,
    pub hand_id:  Option<String>,
    pub card_typ: Option<String>,
    pub card_id:  Option<String>,
    pub load_typ: Option<String>,
    pub load_id:  Option<String>,
    pub dib_typ:  Option<String>,
    pub dib_id:   Option<String>,
    pub cabl_typ: Option<String>,
    pub cabl_id:  Option<String>,
    pub cont_typ: Option<String>,
    pub cont_id:  Option<String>,
    pub lasr_typ: Option<String>,
    pub lasr_id:  Option<String>,
    pub extr_typ: Option<String>,
    pub extr_id:  Option<String>,
}
impl SdrRecord {
    pub const TYP: u8 = 1;
    pub const SUB: u8 = 80;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        let mut s = SdrRecord::default();
        s.head_num = r.read_u1()?;
        s.site_grp = r.read_u1()?;
        s.site_cnt = r.read_u1()?;
        s.site_num = r.read_kx_u1(s.site_cnt as usize)?;
        s.hand_typ = r.maybe_cn()?;
        s.hand_id  = r.maybe_cn()?;
        s.card_typ = r.maybe_cn()?;
        s.card_id  = r.maybe_cn()?;
        s.load_typ = r.maybe_cn()?;
        s.load_id  = r.maybe_cn()?;
        s.dib_typ  = r.maybe_cn()?;
        s.dib_id   = r.maybe_cn()?;
        s.cabl_typ = r.maybe_cn()?;
        s.cabl_id  = r.maybe_cn()?;
        s.cont_typ = r.maybe_cn()?;
        s.cont_id  = r.maybe_cn()?;
        s.lasr_typ = r.maybe_cn()?;
        s.lasr_id  = r.maybe_cn()?;
        s.extr_typ = r.maybe_cn()?;
        s.extr_id  = r.maybe_cn()?;
        Ok(s)
    }
}

/// Test Synopsis Record.
#[derive(Debug, Clone)]
pub struct TsrRecord {
    pub head_num: u8,
    pub site_num: u8,
    pub test_typ: Option<char>,
    pub test_num: u32,
    // Declared I4 (signed) by the STDF V4 spec, with -1 as the "not
    // counted" sentinel — but the reference ASCII dump prints that
    // sentinel as its raw unsigned bit pattern (4294967295), not `-1`, so
    // these are read/stored as u32 to match that behavior exactly.
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
impl TsrRecord {
    pub const TYP: u8 = 10;
    pub const SUB: u8 = 30;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self {
            head_num: r.read_u1()?,
            site_num: r.read_u1()?,
            test_typ: r.maybe_u1()?.map(|b| b as char),
            test_num: r.read_u4()?,
            exec_cnt: r.maybe_u4()?,
            fail_cnt: r.maybe_u4()?,
            alrm_cnt: r.maybe_u4()?,
            test_nam: r.maybe_cn()?,
            seq_name: r.maybe_cn()?,
            test_lbl: r.maybe_cn()?,
            opt_flag: r.maybe_u1()?,
            test_tim: r.maybe_r4()?,
            test_min: r.maybe_r4()?,
            test_max: r.maybe_r4()?,
            tst_sums: r.maybe_r4()?,
            tst_sqrs: r.maybe_r4()?,
        })
    }
}

/// Datalog Text Record.
#[derive(Debug, Clone)]
pub struct DtrRecord { pub text_dat: Option<String> }
impl DtrRecord {
    pub const TYP: u8 = 50;
    pub const SUB: u8 = 30;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self { text_dat: r.maybe_cn()? })
    }
}

/// One field of a Generic Data Record, tagged by its on-disk type code
/// (STDF V4 spec, GDR §V*n / Table C-1).
#[derive(Debug, Clone)]
pub enum GdrField {
    Pad,
    U1(u8),
    U2(u16),
    U4(u32),
    I1(i8),
    I2(i16),
    I4(i32),
    R4(f32),
    R8(f64),
    Cn(String),
    Bn(Vec<u8>),
    Dn(Vec<u8>),
    N1(u8),
    /// Type code not recognized — raw bytes could not be determined, so
    /// nothing further in the record can be parsed either.
    Unknown(u8),
}

/// Generic Data Record.
#[derive(Debug, Clone)]
pub struct GdrRecord { pub fields: Vec<GdrField> }
impl GdrRecord {
    pub const TYP: u8 = 50;
    pub const SUB: u8 = 10;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        let fld_cnt = r.read_u2()?;
        let mut fields = Vec::with_capacity(fld_cnt as usize);
        for _ in 0..fld_cnt {
            let typ = r.read_u1()?;
            let field = match typ {
                0  => GdrField::Pad,
                1  => GdrField::U1(r.read_u1()?),
                2  => GdrField::U2(r.read_u2()?),
                3  => GdrField::U4(r.read_u4()?),
                4  => GdrField::I1(r.read_i1()?),
                5  => GdrField::I2(r.read_i2()?),
                6  => GdrField::I4(r.read_i4()?),
                7  => GdrField::R4(r.read_r4()?),
                8  => GdrField::R8(r.read_r8()?),
                10 => GdrField::Cn(r.read_cn()?.unwrap_or_default()),
                11 => GdrField::Bn(r.read_bn()?),
                12 => GdrField::Dn(r.read_dn()?),
                13 => GdrField::N1(r.read_n1()?),
                other => {
                    // Unknown type code: we can't know its on-disk width,
                    // so stop decoding rather than mis-parse everything
                    // that follows.
                    fields.push(GdrField::Unknown(other));
                    break;
                }
            };
            fields.push(field);
        }
        Ok(Self { fields })
    }
}

/// Multiple-Result Parametric Record.
#[derive(Debug, Clone)]
pub struct MprRecord {
    pub test_num:  u32,
    pub head_num:  u8,
    pub site_num:  u8,
    pub test_flg:  u8,
    pub parm_flg:  u8,
    pub rtn_icnt:  Option<u16>,
    pub rslt_cnt:  Option<u16>,
    pub rtn_stat:  Vec<u8>,
    pub rtn_rslt:  Vec<f32>,
    pub test_txt:  Option<String>,
    pub alarm_id:  Option<String>,
    pub opt_flag:  Option<u8>,
    pub res_scal:  Option<i8>,
    pub llm_scal:  Option<i8>,
    pub hlm_scal:  Option<i8>,
    pub lo_limit:  Option<f32>,
    pub hi_limit:  Option<f32>,
    pub start_in:  Option<f32>,
    pub incr_in:   Option<f32>,
    pub rtn_indx:  Vec<u16>,
    pub units:     Option<String>,
    pub units_in:  Option<String>,
    pub lo_spec:   Option<f32>,
    pub hi_spec:   Option<f32>,
}
impl MprRecord {
    pub const TYP: u8 = 15;
    pub const SUB: u8 = 15;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        let test_num = r.read_u4()?;
        let head_num = r.read_u1()?;
        let site_num = r.read_u1()?;
        let test_flg = r.read_b1()?;
        let parm_flg = r.read_b1()?;
        let rtn_icnt = r.maybe_u2()?;
        let rslt_cnt = r.maybe_u2()?;
        let rtn_stat = match rtn_icnt {
            Some(n) if n > 0 => r.read_kx_n1(n as usize)?,
            _ => vec![],
        };
        let rtn_rslt = match rslt_cnt {
            Some(n) if n > 0 => r.read_kx_r4(n as usize)?,
            _ => vec![],
        };
        let test_txt = r.maybe_cn()?;
        let alarm_id = r.maybe_cn()?;

        let opt_flag = r.maybe_u1()?;
        // See the note in `PtrRecord::parse`: OPT_FLAG's "invalid" bits
        // don't hide bytes that are actually present on disk.
        let (res_scal, llm_scal, hlm_scal, lo_limit, hi_limit, start_in, incr_in,
             rtn_indx, units, units_in, lo_spec, hi_spec) =
            if opt_flag.is_some() {
                let rs   = r.maybe_i1()?;
                let lls  = r.maybe_i1()?;
                let hls  = r.maybe_i1()?;
                let lo   = r.maybe_r4()?;
                let hi   = r.maybe_r4()?;
                let si   = r.maybe_r4()?;
                let ii   = r.maybe_r4()?;
                let indx = match rtn_icnt {
                    Some(n) if n > 0 => r.read_kx_u2(n as usize)?,
                    _ => vec![],
                };
                let u    = r.maybe_cn()?;
                let ui   = r.maybe_cn()?;
                let _rf  = r.maybe_cn()?; // C_RESFMT
                let _lf  = r.maybe_cn()?; // C_LLMFMT
                let _hf  = r.maybe_cn()?; // C_HLMFMT
                let ls   = r.maybe_r4()?;
                let hs   = r.maybe_r4()?;
                (rs, lls, hls, lo, hi, si, ii, indx, u, ui, ls, hs)
            } else {
                (None, None, None, None, None, None, None, vec![], None, None, None, None)
            };

        Ok(Self { test_num, head_num, site_num, test_flg, parm_flg,
                  rtn_icnt, rslt_cnt, rtn_stat, rtn_rslt,
                  test_txt, alarm_id, opt_flag, res_scal, llm_scal, hlm_scal,
                  lo_limit, hi_limit, start_in, incr_in, rtn_indx,
                  units, units_in, lo_spec, hi_spec })
    }
}

/// Functional Test Record.
#[derive(Debug, Clone)]
pub struct FtrRecord {
    pub test_num:  u32,
    pub head_num:  u8,
    pub site_num:  u8,
    pub test_flg:  u8,
    pub opt_flag:  Option<u8>,
    pub cycl_cnt:  Option<u32>,
    pub rel_vadr:  Option<u32>,
    pub rept_cnt:  Option<u32>,
    pub num_fail:  Option<u32>,
    pub xfail_ad:  Option<i32>,
    pub yfail_ad:  Option<i32>,
    pub vect_off:  Option<i16>,
    pub rtn_icnt:  Option<u16>,
    pub pgm_icnt:  Option<u16>,
    pub rtn_indx:  Vec<u16>,
    pub rtn_stat:  Vec<u8>,
    pub pgm_indx:  Vec<u16>,
    pub pgm_stat:  Vec<u8>,
    pub fail_pin:  Option<Vec<u8>>,
    pub vect_nam:  Option<String>,
    pub time_set:  Option<String>,
    pub op_code:   Option<String>,
    pub test_txt:  Option<String>,
    pub alarm_id:  Option<String>,
    pub prog_txt:  Option<String>,
    pub rslt_txt:  Option<String>,
    pub patg_num:  Option<u8>,
    pub spin_map:  Option<Vec<u8>>,
}
impl FtrRecord {
    pub const TYP: u8 = 15;
    pub const SUB: u8 = 20;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        let test_num = r.read_u4()?;
        let head_num = r.read_u1()?;
        let site_num = r.read_u1()?;
        let test_flg = r.read_b1()?;
        let opt_flag = r.maybe_u1()?;
        let cycl_cnt = r.maybe_u4()?;
        let rel_vadr = r.maybe_u4()?;
        let rept_cnt = r.maybe_u4()?;
        let num_fail = r.maybe_u4()?;
        let xfail_ad = r.maybe_i4()?;
        let yfail_ad = r.maybe_i4()?;
        let vect_off = if r.remaining() >= 2 { Some(r.read_i2()?) } else { None };
        let rtn_icnt = r.maybe_u2()?;
        let pgm_icnt = r.maybe_u2()?;
        let rtn_indx = match rtn_icnt { Some(n) if n > 0 => r.read_kx_u2(n as usize)?, _ => vec![] };
        let rtn_stat = match rtn_icnt { Some(n) if n > 0 => r.read_kx_n1(n as usize)?, _ => vec![] };
        let pgm_indx = match pgm_icnt { Some(n) if n > 0 => r.read_kx_u2(n as usize)?, _ => vec![] };
        let pgm_stat = match pgm_icnt { Some(n) if n > 0 => r.read_kx_n1(n as usize)?, _ => vec![] };
        let fail_pin = r.maybe_dn()?;
        let vect_nam = r.maybe_cn()?;
        let time_set = r.maybe_cn()?;
        let op_code  = r.maybe_cn()?;
        let test_txt = r.maybe_cn()?;
        let alarm_id = r.maybe_cn()?;
        let prog_txt = r.maybe_cn()?;
        let rslt_txt = r.maybe_cn()?;
        let patg_num = r.maybe_u1()?;
        let spin_map = r.maybe_dn()?;

        Ok(Self { test_num, head_num, site_num, test_flg, opt_flag,
                  cycl_cnt, rel_vadr, rept_cnt, num_fail, xfail_ad, yfail_ad,
                  vect_off, rtn_icnt, pgm_icnt, rtn_indx, rtn_stat, pgm_indx, pgm_stat,
                  fail_pin, vect_nam, time_set, op_code, test_txt, alarm_id,
                  prog_txt, rslt_txt, patg_num, spin_map })
    }
}

#[derive(Debug, Clone)]
pub struct HbrRecord {
    pub head_num: u8,
    pub site_num: u8,
    pub hbin_num: u16,
    pub hbin_cnt: u32,
    pub hbin_pf:  Option<char>,
    pub hbin_nam: Option<String>,
}
impl HbrRecord {
    pub const TYP: u8 = 1;
    pub const SUB: u8 = 40;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self {
            head_num: r.read_u1()?,
            site_num: r.read_u1()?,
            hbin_num: r.read_u2()?,
            hbin_cnt: r.read_u4()?,
            hbin_pf:  r.maybe_u1()?.map(|b| b as char),
            hbin_nam: r.maybe_cn()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WirRecord { pub head_num: u8, pub site_grp: u8, pub start_t: u32 }
impl WirRecord {
    pub const TYP: u8 = 2;
    pub const SUB: u8 = 10;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self { head_num: r.read_u1()?, site_grp: r.read_u1()?, start_t: r.read_u4()? })
    }
}

#[derive(Debug, Clone)]
pub struct WrrRecord { pub head_num: u8, pub site_grp: u8, pub finish_t: u32, pub part_cnt: u32 }
impl WrrRecord {
    pub const TYP: u8 = 2;
    pub const SUB: u8 = 20;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self {
            head_num: r.read_u1()?,
            site_grp: r.read_u1()?,
            finish_t: r.read_u4()?,
            part_cnt: r.read_u4()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PirRecord { pub head_num: u8, pub site_num: u8 }
impl PirRecord {
    pub const TYP: u8 = 5;
    pub const SUB: u8 = 10;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self { head_num: r.read_u1()?, site_num: r.read_u1()? })
    }
}

#[derive(Debug, Clone)]
pub struct PrrRecord {
    pub head_num: u8,
    pub site_num: u8,
    pub part_flg: u8,
    pub num_test: u16,
    pub hard_bin: u16,
    pub soft_bin: Option<u16>,
    pub x_coord:  Option<i16>,
    pub y_coord:  Option<i16>,
    pub test_t:   Option<u32>,
    pub part_id:  Option<String>,
    pub part_txt: Option<String>,
    pub part_fix: Option<Vec<u8>>,
}
impl PrrRecord {
    pub const TYP: u8 = 5;
    pub const SUB: u8 = 20;
    pub fn is_pass(&self) -> bool { self.part_flg & 0x18 == 0 }
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        let head_num = r.read_u1()?;
        let site_num = r.read_u1()?;
        let part_flg = r.read_b1()?;
        let num_test = r.read_u2()?;
        let hard_bin = r.read_u2()?;
        let soft_bin = if r.remaining() >= 2 { Some(r.read_u2()?) } else { None };
        let x_coord  = if r.remaining() >= 2 { Some(r.read_i2()?) } else { None };
        let y_coord  = if r.remaining() >= 2 { Some(r.read_i2()?) } else { None };
        let test_t   = r.maybe_u4()?;
        let part_id  = r.maybe_cn()?;
        let part_txt = r.maybe_cn()?;
        let part_fix = r.maybe_bn()?;
        Ok(Self { head_num, site_num, part_flg, num_test, hard_bin, soft_bin,
                  x_coord, y_coord, test_t, part_id, part_txt, part_fix })
    }
}

#[derive(Debug, Clone)]
pub struct PtrRecord {
    pub test_num:  u32,
    pub head_num:  u8,
    pub site_num:  u8,
    pub test_flg:  u8,
    pub parm_flg:  u8,
    pub result:    Option<f32>,
    pub test_txt:  Option<String>,
    pub alarm_id:  Option<String>,
    pub opt_flag:  Option<u8>,
    pub res_scal:  Option<i8>,
    pub llm_scal:  Option<i8>,
    pub hlm_scal:  Option<i8>,
    pub lo_limit:  Option<f32>,
    pub hi_limit:  Option<f32>,
    pub units:     Option<String>,
    pub lo_spec:   Option<f32>,
    pub hi_spec:   Option<f32>,
}
impl PtrRecord {
    pub const TYP: u8 = 15;
    pub const SUB: u8 = 10;

    pub fn is_pass(&self)           -> bool { self.test_flg & 0x40 == 0 }
    pub fn result_invalid(&self)    -> bool { self.test_flg & 0x02 != 0 }
    pub fn lo_limit_invalid(&self)  -> bool {
        self.opt_flag.map(|f| f & 0x50 != 0).unwrap_or(true)
    }
    pub fn hi_limit_invalid(&self)  -> bool {
        self.opt_flag.map(|f| f & 0xA0 != 0).unwrap_or(true)
    }

    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        let test_num = r.read_u4()?;
        let head_num = r.read_u1()?;
        let site_num = r.read_u1()?;
        let test_flg = r.read_b1()?;
        let parm_flg = r.read_b1()?;

        let result = if r.remaining() >= 4 {
            let v = r.read_r4()?;
            if test_flg & 0x02 != 0 { None } else { Some(v) }
        } else { None };

        let test_txt = r.maybe_cn()?;
        let alarm_id = r.maybe_cn()?;

        let opt_flag = r.maybe_u1()?;
        // Note: OPT_FLAG's "invalid, use default" bits describe how a
        // *consuming* tool should interpret these values, not whether the
        // bytes are physically present on disk — the reference ASCII dump
        // prints whatever was actually read, regardless of those bits.
        let (res_scal, llm_scal, hlm_scal, lo_limit, hi_limit, units, lo_spec, hi_spec) =
            if opt_flag.is_some() {
                let rs  = r.maybe_i1()?;
                let lls = r.maybe_i1()?;
                let hls = r.maybe_i1()?;
                let lo  = r.maybe_r4()?;
                let hi  = r.maybe_r4()?;
                let u   = r.maybe_cn()?;
                let _rf = r.maybe_cn()?; // C_RESFMT
                let _lf = r.maybe_cn()?; // C_LLMFMT
                let _hf = r.maybe_cn()?; // C_HLMFMT
                let ls  = r.maybe_r4()?;
                let hs  = r.maybe_r4()?;
                (rs, lls, hls, lo, hi, u, ls, hs)
            } else {
                (None, None, None, None, None, None, None, None)
            };

        Ok(Self { test_num, head_num, site_num, test_flg, parm_flg,
                  result, test_txt, alarm_id, opt_flag,
                  res_scal, llm_scal, hlm_scal,
                  lo_limit, hi_limit, units, lo_spec, hi_spec })
    }
}

#[derive(Debug, Clone)]
pub struct BpsRecord { pub seq_name: Option<String> }
impl BpsRecord {
    pub const TYP: u8 = 20;
    pub const SUB: u8 = 10;
    pub fn parse(r: &mut RecordReader<'_>) -> Result<Self> {
        Ok(Self { seq_name: r.maybe_cn()? })
    }
}

#[derive(Debug, Clone)]
pub struct EpsRecord;
impl EpsRecord {
    pub const TYP: u8 = 20;
    pub const SUB: u8 = 20;
    pub fn parse(_r: &mut RecordReader<'_>) -> Result<Self> { Ok(Self) }
}
