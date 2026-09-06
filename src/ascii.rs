//! ASCII text dump of a parsed STDF stream, matching the output format of
//! the reference `/opt/hp93000/soc/formatter/bin/STDFreader` tool byte-for-
//! byte (verified against the golden `samples/*.txt` files shipped with
//! this repo).
//!
//! Format notes reverse-engineered from the golden files:
//!   - Every field line is `  {NAME:<15}{value}` (2-space indent, field
//!     name + colon left-padded to 15 columns, then the value).
//!   - Fields that are structurally absent from the record on disk (the
//!     optional tail simply wasn't written) are omitted entirely — not
//!     printed blank. Fields that *are* present but hold an empty string
//!     (`Cn` with length 0) are printed with a blank value.
//!   - Single-byte flag fields (`TEST_FLG`, `PARM_FLG`, `OPT_FLAG`, ...)
//!     print as unpadded uppercase hex, e.g. `0xE`, `0xC0`.
//!   - `Cx` character fields print quoted, e.g. `'E'`, `' '`, and are
//!     considered present as long as the byte was read from the stream at
//!     all — including when that byte is a space.
//!   - Epoch (`U4`) timestamp fields print as `<secs> (dd-Mon-yyyy
//!     HH:MM:SS)`, always interpreted as plain UTC (no timezone shift).
//!   - Most `R4` result/limit fields print with 9 decimal places and no
//!     width padding. A few fields (`START_IN`, `INCR_IN`, `LO_SPEC`,
//!     `HI_SPEC`, `TEST_TIM`) print with 4 decimals left-justified in a
//!     10-column field instead.
//!   - `TEST_SUMS`/`TEST_SQRS` (TSR) print with 18 decimals.
//!   - `PTR.RESULT` is special-cased: when a given PTR omits its own
//!     `UNITS` field (because the tester only wrote that optional tail
//!     once per test number), the last non-empty `UNITS` seen for that
//!     `TEST_NUM` is appended directly after `RESULT`, with the numeric
//!     value left-justified in a 13-column field first.
//!   - Index/status arrays (`RTN_STAT`, `RTN_INDX`, ...) print as
//!     comma-separated unpadded hex.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::parser::StdfRecord;
use crate::records::GdrField;

/// Renders a stream of [`StdfRecord`]s into the reference tool's ASCII
/// text format, keeping the small bits of cross-record state (currently
/// just the per-`TEST_NUM` scale/units cache used by TSR) that the format
/// requires.
///
/// Scaling rules (reverse-engineered empirically from the golden files by
/// comparing raw on-disk bytes to the rendered text — see the doc comment
/// on `scale_result_f32`/`scale_f64` below for the details and why they
/// differ):
///   - `PTR.RESULT` and `TSR.TEST_MIN`/`TEST_MAX`: raw value × `10^scale`
///     computed in **single (f32) precision** (one rounding step), then
///     displayed with 9 decimals. `TSR` has no scale/units fields of its
///     own, so it borrows them from the *first* PTR/MPR seen for the same
///     `TEST_NUM` (never overwritten by later records for that test).
///   - `PTR`/`MPR` limit fields (`LO_LIMIT`, `HI_LIMIT`, `LO_SPEC`,
///     `HI_SPEC`) and `MPR.RTN_RSLT`: raw × `10^scale` computed in
///     **double (f64) precision**, no rounding back to f32.
///   - `TSR.TEST_SUMS`/`TEST_SQRS` and `TEST_TIM`/`START_IN`/`INCR_IN`:
///     never scaled, always the raw on-disk value.
///   - Displayed `UNITS` = an SI-prefix character (from the scale value,
///     see `si_prefix`) prepended to the raw on-disk units string. Each
///     `PTR`/`MPR` record always uses *its own* scale/units when present
///     (never a cross-record cache) — only `TSR` needs the cache.
#[derive(Default)]
pub struct AsciiDumper {
    /// TEST_NUM -> (RES_SCAL, raw UNITS) from the first PTR/MPR seen for
    /// that test number, used to render TSR's scale-less TEST_MIN/TEST_MAX.
    scale_cache: HashMap<u32, (i8, String)>,
}

impl AsciiDumper {
    pub fn new() -> Self { Self::default() }

    pub fn header(file_name: &str) -> String {
        format!(
            "Starting /opt/hp93000/soc/formatter/bin/STDFreader:\n\n\
             \u{20}\u{20}\u{20}STDF is a trademark of Teradyne, Inc.\n\n\
             Data from file '{file_name}'\n"
        )
    }

    pub fn footer() -> &'static str {
        "End of file. Done!\n"
    }

    /// Renders one record, e.g. `"MIR Record\n  SETUP_T:       ...\n"`.
    pub fn render(&mut self, record: &StdfRecord) -> String {
        let mut out = String::new();
        match record {
            StdfRecord::Far(r) => {
                out.push_str("FAR Record\n");
                field_u(&mut out, "CPU_TYPE", r.cpu_typ);
                field_u(&mut out, "STDF_VER", r.stdf_ver);
            }
            StdfRecord::Atr(r) => {
                out.push_str("ATR Record\n");
                field_time(&mut out, "MOD_TIME", r.mod_tim);
                field_opt_str(&mut out, "CMD_LINE", &r.cmd_line);
            }
            StdfRecord::Mir(r) => {
                out.push_str("MIR Record\n");
                field_time(&mut out, "SETUP_T", r.setup_t);
                field_time(&mut out, "START_T", r.start_t);
                field_u(&mut out, "STAT_NUM", r.stat_num);
                field_opt_char(&mut out, "MODE_COD", r.mode_cod);
                field_opt_char(&mut out, "RTST_COD", r.rtst_cod);
                field_opt_char(&mut out, "PROT_COD", r.prot_cod);
                field_opt_u(&mut out, "BURN_TIM", r.burn_tim);
                field_opt_char(&mut out, "CMOD_COD", r.cmod_cod);
                field_opt_str(&mut out, "LOT_ID", &r.lot_id);
                field_opt_str(&mut out, "PART_TYP", &r.part_typ);
                field_opt_str(&mut out, "NODE_NAM", &r.node_nam);
                field_opt_str(&mut out, "TSTR_TYP", &r.tstr_typ);
                field_opt_str(&mut out, "JOB_NAM", &r.job_nam);
                field_opt_str(&mut out, "JOB_REV", &r.job_rev);
                field_opt_str(&mut out, "SBLOT_ID", &r.sblot_id);
                field_opt_str(&mut out, "OPER_NAM", &r.oper_nam);
                field_opt_str(&mut out, "EXEC_TYP", &r.exec_typ);
                field_opt_str(&mut out, "EXEC_VER", &r.exec_ver);
                field_opt_str(&mut out, "TEST_COD", &r.test_cod);
                field_opt_str(&mut out, "TST_TEMP", &r.tst_temp);
                field_opt_str(&mut out, "USER_TXT", &r.user_txt);
                field_opt_str(&mut out, "AUX_FILE", &r.aux_file);
                field_opt_str(&mut out, "PKG_TYP", &r.pkg_typ);
                field_opt_str(&mut out, "FAMLY_ID", &r.famly_id);
                field_opt_str(&mut out, "DATE_COD", &r.date_cod);
                field_opt_str(&mut out, "FACIL_ID", &r.facil_id);
                field_opt_str(&mut out, "FLOOR_ID", &r.floor_id);
                field_opt_str(&mut out, "PROC_ID", &r.proc_id);
                field_opt_str(&mut out, "OPER_FRQ", &r.oper_frq);
                field_opt_str(&mut out, "SPEC_NAM", &r.spec_nam);
                field_opt_str(&mut out, "SPEC_VER", &r.spec_ver);
                field_opt_str(&mut out, "FLOW_ID", &r.flow_id);
                field_opt_str(&mut out, "SETUP_ID", &r.setup_id);
                field_opt_str(&mut out, "DSGN_REV", &r.dsgn_rev);
                field_opt_str(&mut out, "ENG_ID", &r.eng_id);
                field_opt_str(&mut out, "ROM_COD", &r.rom_cod);
                field_opt_str(&mut out, "SERL_NUM", &r.serl_num);
                field_opt_str(&mut out, "SUPR_NAM", &r.supr_nam);
            }
            StdfRecord::Mrr(r) => {
                out.push_str("MRR Record\n");
                field_time(&mut out, "FINISH_T", r.finish_t);
                field_opt_char(&mut out, "DISP_COD", r.disp_cod);
                field_opt_str(&mut out, "USR_DESC", &r.usr_desc);
                field_opt_str(&mut out, "EXC_DESC", &r.exc_desc);
            }
            StdfRecord::Pcr(r) => {
                out.push_str("PCR Record\n");
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_NUM", r.site_num);
                field_u(&mut out, "PART_CNT", r.part_cnt);
                field_opt_u(&mut out, "RTST_CNT", r.rtst_cnt);
                field_opt_u(&mut out, "ABRT_CNT", r.abrt_cnt);
                field_opt_u(&mut out, "GOOD_CNT", r.good_cnt);
                field_opt_u(&mut out, "FUNC_CNT", r.func_cnt);
            }
            StdfRecord::Hbr(r) => {
                out.push_str("HBR Record\n");
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_NUM", r.site_num);
                field_u(&mut out, "HBIN_NUM", r.hbin_num);
                field_u(&mut out, "HBIN_CNT", r.hbin_cnt);
                field_opt_char(&mut out, "HBIN_PF", r.hbin_pf);
                field_opt_str(&mut out, "HBIN_NAM", &r.hbin_nam);
            }
            StdfRecord::Sbr(r) => {
                out.push_str("SBR Record\n");
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_NUM", r.site_num);
                field_u(&mut out, "SBIN_NUM", r.sbin_num);
                field_u(&mut out, "SBIN_CNT", r.sbin_cnt);
                field_opt_char(&mut out, "SBIN_PF", r.sbin_pf);
                field_opt_str(&mut out, "SBIN_NAM", &r.sbin_nam);
            }
            StdfRecord::Pmr(r) => {
                out.push_str("PMR Record\n");
                field_u(&mut out, "PMR_INDX", r.pmr_indx);
                field_opt_u(&mut out, "CHAN_TYP", r.chan_typ);
                field_opt_str(&mut out, "CHAN_NAM", &r.chan_nam);
                field_opt_str(&mut out, "PHY_NAM", &r.phy_nam);
                field_opt_str(&mut out, "LOG_NAM", &r.log_nam);
                field_opt_u(&mut out, "HEAD_NUM", r.head_num);
                field_opt_u(&mut out, "SITE_NUM", r.site_num);
            }
            StdfRecord::Sdr(r) => {
                out.push_str("SDR Record\n");
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_GRP", r.site_grp);
                field_u(&mut out, "SITE_CNT", r.site_cnt);
                field_str(&mut out, "SITE_NUM", &join_dec(&r.site_num));
                field_opt_str(&mut out, "HAND_TYP", &r.hand_typ);
                field_opt_str(&mut out, "HAND_ID", &r.hand_id);
                field_opt_str(&mut out, "CARD_TYP", &r.card_typ);
                field_opt_str(&mut out, "CARD_ID", &r.card_id);
                field_opt_str(&mut out, "LOAD_TYP", &r.load_typ);
                field_opt_str(&mut out, "LOAD_ID", &r.load_id);
                field_opt_str(&mut out, "DIB_TYP", &r.dib_typ);
                field_opt_str(&mut out, "DIB_ID", &r.dib_id);
                field_opt_str(&mut out, "CABL_TYP", &r.cabl_typ);
                field_opt_str(&mut out, "CABL_ID", &r.cabl_id);
                field_opt_str(&mut out, "CONT_TYP", &r.cont_typ);
                field_opt_str(&mut out, "CONT_ID", &r.cont_id);
                field_opt_str(&mut out, "LASR_TYP", &r.lasr_typ);
                field_opt_str(&mut out, "LASR_ID", &r.lasr_id);
                field_opt_str(&mut out, "EXTR_TYP", &r.extr_typ);
                field_opt_str(&mut out, "EXTR_ID", &r.extr_id);
            }
            StdfRecord::Wir(r) => {
                out.push_str("WIR Record\n");
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_GRP", r.site_grp);
                field_time(&mut out, "START_T", r.start_t);
            }
            StdfRecord::Wrr(r) => {
                out.push_str("WRR Record\n");
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_GRP", r.site_grp);
                field_time(&mut out, "FINISH_T", r.finish_t);
                field_u(&mut out, "PART_CNT", r.part_cnt);
            }
            StdfRecord::Pir(r) => {
                out.push_str("PIR Record\n");
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_NUM", r.site_num);
            }
            StdfRecord::Prr(r) => {
                out.push_str("PRR Record\n");
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_NUM", r.site_num);
                field_hex(&mut out, "PART_FLG", r.part_flg);
                field_u(&mut out, "NUM_TEST", r.num_test);
                field_u(&mut out, "HARD_BIN", r.hard_bin);
                field_opt_u(&mut out, "SOFT_BIN", r.soft_bin);
                field_opt_i(&mut out, "X_COORD", r.x_coord);
                field_opt_i(&mut out, "Y_COORD", r.y_coord);
                field_opt_u(&mut out, "TEST_T", r.test_t);
                field_opt_str(&mut out, "PART_ID", &r.part_id);
                field_opt_str(&mut out, "PART_TXT", &r.part_txt);
                field_opt_bn(&mut out, "PART_FIX", &r.part_fix);
            }
            StdfRecord::Tsr(r) => {
                out.push_str("TSR Record\n");
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_NUM", r.site_num);
                field_opt_char(&mut out, "TEST_TYP", r.test_typ);
                field_u(&mut out, "TEST_NUM", r.test_num);
                field_opt_i(&mut out, "EXEC_CNT", r.exec_cnt);
                field_opt_i(&mut out, "FAIL_CNT", r.fail_cnt);
                field_opt_i(&mut out, "ALRM_CNT", r.alrm_cnt);
                field_opt_str(&mut out, "TEST_NAM", &r.test_nam);
                field_opt_str(&mut out, "SEQ_NAME", &r.seq_name);
                field_opt_str(&mut out, "TEST_LBL", &r.test_lbl);
                field_opt_hex(&mut out, "OPT_FLAG", r.opt_flag);
                if let Some(v) = r.test_tim { field_str(&mut out, "TEST_TIM", &fmt_pad4(v as f64)); }
                let cached = self.scale_cache.get(&r.test_num);
                if let Some(v) = r.test_min {
                    field_str(&mut out, "TEST_MIN", &fmt_tsr_result(v, cached));
                }
                if let Some(v) = r.test_max {
                    field_str(&mut out, "TEST_MAX", &fmt_tsr_result(v, cached));
                }
                if let Some(v) = r.tst_sums { field_str(&mut out, "TEST_SUMS", &fmt_r4_18(v)); }
                if let Some(v) = r.tst_sqrs { field_str(&mut out, "TEST_SQRS", &fmt_r4_18(v)); }
            }
            StdfRecord::Ptr(r) => {
                out.push_str("PTR Record\n");
                field_u(&mut out, "TEST_NUM", r.test_num);
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_NUM", r.site_num);
                field_hex(&mut out, "TEST_FLG", r.test_flg);
                field_hex(&mut out, "PARM_FLG", r.parm_flg);
                if let Some(rs) = r.res_scal {
                    self.scale_cache.entry(r.test_num)
                        .or_insert_with(|| (rs, r.units.clone().unwrap_or_default()));
                }
                let res_scal = r.res_scal.unwrap_or(0);
                let llm_scal = r.llm_scal.unwrap_or(0);
                let hlm_scal = r.hlm_scal.unwrap_or(0);
                if let Some(v) = r.result {
                    field_str(&mut out, "RESULT", &fmt_9(scale_result_f32(v, res_scal)));
                }
                field_opt_str(&mut out, "TEST_TXT", &r.test_txt);
                field_opt_str(&mut out, "ALARM_ID", &r.alarm_id);
                field_opt_hex(&mut out, "OPT_FLAG", r.opt_flag);
                if let Some(v) = r.lo_limit { field_str(&mut out, "LO_LIMIT", &fmt_9(scale_f64(v, llm_scal))); }
                if let Some(v) = r.hi_limit { field_str(&mut out, "HI_LIMIT", &fmt_9(scale_f64(v, hlm_scal))); }
                if let Some(units) = &r.units {
                    field_str(&mut out, "UNITS", &units_with_prefix(res_scal as i32, units));
                }
                if let Some(v) = r.lo_spec { field_str(&mut out, "LO_SPEC", &fmt_pad4(scale_f64(v, res_scal))); }
                if let Some(v) = r.hi_spec { field_str(&mut out, "HI_SPEC", &fmt_pad4(scale_f64(v, res_scal))); }
            }
            StdfRecord::Mpr(r) => {
                out.push_str("MPR Record\n");
                field_u(&mut out, "TEST_NUM", r.test_num);
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_NUM", r.site_num);
                field_hex(&mut out, "TEST_FLG", r.test_flg);
                field_hex(&mut out, "PARM_FLG", r.parm_flg);
                field_opt_u(&mut out, "RTN_ICNT", r.rtn_icnt);
                field_opt_u(&mut out, "RSLT_CNT", r.rslt_cnt);
                if !r.rtn_stat.is_empty() { field_str(&mut out, "RTN_STAT", &join_hex(&r.rtn_stat)); }
                if let Some(rs) = r.res_scal {
                    self.scale_cache.entry(r.test_num)
                        .or_insert_with(|| (rs, r.units.clone().unwrap_or_default()));
                }
                let res_scal = r.res_scal.unwrap_or(0);
                let llm_scal = r.llm_scal.unwrap_or(0);
                let hlm_scal = r.hlm_scal.unwrap_or(0);
                if !r.rtn_rslt.is_empty() {
                    let joined = r.rtn_rslt.iter()
                        .map(|v| fmt_9(scale_f64(*v, res_scal)))
                        .collect::<Vec<_>>().join(", ");
                    field_str(&mut out, "RTN_RSLT", &joined);
                }
                field_opt_str(&mut out, "TEST_TXT", &r.test_txt);
                field_opt_str(&mut out, "ALARM_ID", &r.alarm_id);
                field_opt_hex(&mut out, "OPT_FLAG", r.opt_flag);
                if let Some(v) = r.lo_limit { field_str(&mut out, "LO_LIMIT", &fmt_9(scale_f64(v, llm_scal))); }
                if let Some(v) = r.hi_limit { field_str(&mut out, "HI_LIMIT", &fmt_9(scale_f64(v, hlm_scal))); }
                if let Some(v) = r.start_in { field_str(&mut out, "START_IN", &fmt_pad4(v as f64)); }
                if let Some(v) = r.incr_in { field_str(&mut out, "INCR_IN", &fmt_pad4(v as f64)); }
                if !r.rtn_indx.is_empty() { field_str(&mut out, "RTN_INDX", &join_hex(&r.rtn_indx)); }
                if let Some(units) = &r.units {
                    field_str(&mut out, "UNITS", &units_with_prefix(res_scal as i32, units));
                }
                field_opt_str(&mut out, "UNITS_IN", &r.units_in);
                if let Some(v) = r.lo_spec { field_str(&mut out, "LO_SPEC", &fmt_pad4(scale_f64(v, res_scal))); }
                if let Some(v) = r.hi_spec { field_str(&mut out, "HI_SPEC", &fmt_pad4(scale_f64(v, res_scal))); }
            }
            StdfRecord::Ftr(r) => {
                out.push_str("FTR Record\n");
                field_u(&mut out, "TEST_NUM", r.test_num);
                field_u(&mut out, "HEAD_NUM", r.head_num);
                field_u(&mut out, "SITE_NUM", r.site_num);
                field_hex(&mut out, "TEST_FLG", r.test_flg);
                field_opt_hex(&mut out, "OPT_FLAG", r.opt_flag);
                field_opt_u(&mut out, "CYCL_CNT", r.cycl_cnt);
                field_opt_u(&mut out, "REL_VADR", r.rel_vadr);
                field_opt_u(&mut out, "REPT_CNT", r.rept_cnt);
                field_opt_u(&mut out, "NUM_FAIL", r.num_fail);
                field_opt_i(&mut out, "XFAIL_AD", r.xfail_ad);
                field_opt_i(&mut out, "YFAIL_AD", r.yfail_ad);
                field_opt_i(&mut out, "VECT_OFF", r.vect_off);
                field_opt_u(&mut out, "RTN_ICNT", r.rtn_icnt);
                field_opt_u(&mut out, "PGM_ICNT", r.pgm_icnt);
                if r.rtn_icnt.is_some() { field_str(&mut out, "RTN_INDX", &join_hex(&r.rtn_indx)); }
                if r.rtn_icnt.is_some() { field_str(&mut out, "RTN_STAT", &join_hex(&r.rtn_stat)); }
                if r.pgm_icnt.is_some() { field_str(&mut out, "PGM_INDX", &join_hex(&r.pgm_indx)); }
                if r.pgm_icnt.is_some() { field_str(&mut out, "PGM_STAT", &join_hex(&r.pgm_stat)); }
                field_opt_dn(&mut out, "FAIL_PIN", &r.fail_pin);
                field_opt_str(&mut out, "VECT_NAM", &r.vect_nam);
                field_opt_str(&mut out, "TIME_SET", &r.time_set);
                field_opt_str(&mut out, "OP_CODE", &r.op_code);
                field_opt_str(&mut out, "TEST_TXT", &r.test_txt);
                field_opt_str(&mut out, "ALARM_ID", &r.alarm_id);
                field_opt_str(&mut out, "PROG_TXT", &r.prog_txt);
                field_opt_str(&mut out, "RSLT_TXT", &r.rslt_txt);
                field_opt_u(&mut out, "PATG_NUM", r.patg_num);
                field_opt_dn(&mut out, "SPIN_MAP", &r.spin_map);
            }
            StdfRecord::Bps(r) => {
                out.push_str("BPS Record\n");
                field_opt_str(&mut out, "SEQ_NAME", &r.seq_name);
            }
            StdfRecord::Eps(_) => {
                out.push_str("EPS Record\n");
            }
            StdfRecord::Gdr(r) => {
                out.push_str("GDR Record\n");
                field_u(&mut out, "FLD_CNT", r.fields.len() as u16);
                for (i, f) in r.fields.iter().enumerate() {
                    let _ = writeln!(out, "  Field # {:>2}:    {}", i + 1, fmt_gdr_field(f));
                }
            }
            StdfRecord::Dtr(r) => {
                out.push_str("DTR Record\n");
                field_opt_str(&mut out, "TEXT_DAT", &r.text_dat);
            }
            StdfRecord::Unknown { typ, sub, .. } => {
                let _ = writeln!(out, "UNK Record (typ={typ}, sub={sub})");
            }
        }
        out
    }
}

// ── Field-line helpers ───────────────────────────────────────────────────────

fn field_line(out: &mut String, name: &str, value: &str) {
    let _ = writeln!(out, "  {:<15}{value}", format!("{name}:"));
}
fn field_str(out: &mut String, name: &str, value: &str) { field_line(out, name, value); }
fn field_u(out: &mut String, name: &str, value: impl std::fmt::Display) {
    field_line(out, name, &value.to_string());
}
fn field_opt_u(out: &mut String, name: &str, value: Option<impl std::fmt::Display>) {
    if let Some(v) = value { field_line(out, name, &v.to_string()); }
}
fn field_opt_i(out: &mut String, name: &str, value: Option<impl std::fmt::Display>) {
    if let Some(v) = value { field_line(out, name, &v.to_string()); }
}
fn field_hex(out: &mut String, name: &str, value: u8) {
    field_line(out, name, &format!("0x{value:X}"));
}
fn field_opt_hex(out: &mut String, name: &str, value: Option<u8>) {
    if let Some(v) = value { field_hex(out, name, v); }
}
fn field_opt_char(out: &mut String, name: &str, value: Option<char>) {
    if let Some(c) = value { field_line(out, name, &format!("'{c}'")); }
}
fn field_opt_str(out: &mut String, name: &str, value: &Option<String>) {
    if let Some(s) = value { field_line(out, name, s); }
}
fn field_opt_bn(out: &mut String, name: &str, value: &Option<Vec<u8>>) {
    if let Some(bytes) = value { field_line(out, name, &join_hex(bytes)); }
}
/// `Dn` (bit-encoded) fields — `FAIL_PIN`/`SPIN_MAP` — print each byte as
/// 8-digit binary (MSB-first) with a trailing space after every byte
/// (including the last one), e.g. `"00000100 00000000 "`.
fn field_opt_dn(out: &mut String, name: &str, value: &Option<Vec<u8>>) {
    if let Some(bytes) = value { field_line(out, name, &join_bin(bytes)); }
}

fn join_dec(vals: &[u8]) -> String {
    vals.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", ")
}
fn join_hex<T: Into<u32> + Copy>(vals: &[T]) -> String {
    vals.iter().map(|v| format!("0x{:X}", (*v).into())).collect::<Vec<_>>().join(", ")
}
fn join_bin(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:08b} ")).collect()
}

fn fmt_9(v: f64) -> String { format!("{v:.9}") }
fn fmt_r4_18(v: f32) -> String { format!("{v:.18}") }
fn fmt_pad4(v: f64) -> String { format!("{v:<10.4}") }

/// `RES_SCAL`/`LLM_SCAL`/`HLM_SCAL`-style scaling for limit fields and
/// `MPR.RTN_RSLT`: `raw × 10^scale` computed in double precision, with no
/// rounding back to `f32`. Verified byte-for-byte against the golden
/// files' `LO_LIMIT`/`HI_LIMIT`/`RTN_RSLT` values (e.g. raw `-0.8999999…`
/// × `10^3` = `-899.999976158`, matching the golden `LO_LIMIT` exactly —
/// whereas rounding back to `f32` first would give `-900.000000000`,
/// which does *not* match).
fn scale_f64(raw: f32, scale: i8) -> f64 {
    raw as f64 * 10f64.powi(scale as i32)
}

/// `PTR.RESULT`/`TSR.TEST_MIN`/`TEST_MAX`-style scaling: `raw × 10^scale`
/// computed in **single precision** (matching the reference tool's
/// internal `float` arithmetic), then widened to `f64` only for display.
/// Verified against the golden files: raw `0.016442839…` × `10^6`
/// golden-displays as `16442.839843750` — exactly the nearest-`f32`
/// result of that multiplication, not the double-precision result
/// (`16442.839056253…`, which does *not* match).
///
/// For a *negative* scale the reference tool divides by `10^|scale|`
/// rather than multiplying by the reciprocal — also verified
/// empirically: raw `9332799488.0` with scale `-9` golden-displays as
/// `9.332799911`, matching `raw / 1e9` computed in `f32` (`1e9` is
/// exactly representable in `f32`, so this is a single rounding step);
/// multiplying by `f32`'s `10^-9` (whose reciprocal isn't exact) instead
/// gives `9.332798958`, which does *not* match.
fn scale_result_f32(raw: f32, scale: i8) -> f64 {
    let scaled = if scale >= 0 {
        raw * 10f32.powi(scale as i32)
    } else {
        raw / 10f32.powi(-(scale as i32))
    };
    scaled as f64
}

/// SI-prefix character for a `RES_SCAL`-style scale value, prepended to
/// the raw on-disk `UNITS` string to get the displayed one (e.g. raw `"A"`
/// with scale `6` displays as `"uA"`). Table taken from the STDF V4
/// spec's suggested `RES_SCAL` prefixes; verified against the golden
/// files (scale `3` -> `mV`, scale `6` -> `uA`, scale `0` -> unprefixed).
fn si_prefix(scale: i32) -> &'static str {
    match scale {
        -12 => "T",
        -9 => "G",
        -6 => "M",
        -3 => "K",
        -2 => "H",
        -1 => "D",
        0 => "",
        1 => "d",
        2 => "c",
        3 => "m",
        6 => "u",
        9 => "n",
        12 => "p",
        15 => "f",
        _ => "",
    }
}

/// `PTR`/`MPR`'s standalone `UNITS:` line value: the SI-prefix character
/// is rendered in a fixed 1-column field — a literal space when there's
/// no prefix, never omitted — immediately followed by the raw units
/// string. Verified against the golden files: an unprefixed 1-char unit
/// like `"V"` displays as `" V"` (leading space), and an empty/no-units
/// record displays as a single space (not an empty string) — both of
/// which a plain `si_prefix(scale) + units` concatenation would get
/// wrong (it would omit the space entirely instead of reserving that
/// column for the absent prefix).
fn units_with_prefix(scale: i32, raw_units: &str) -> String {
    match si_prefix(scale) {
        "" => format!(" {raw_units}"),
        p  => format!("{p}{raw_units}"),
    }
}

/// `TSR.TEST_MIN`/`TEST_MAX`: scaled via `scale_result_f32` using the
/// `(RES_SCAL, raw UNITS)` cached from the first PTR/MPR seen for this
/// `TEST_NUM` (TSR itself carries no scale/units fields). When a non-empty
/// units string is available, the value is left-padded to a minimum width
/// of 12 columns, followed by a literal space and the SI-prefixed units
/// (e.g. `"0.000000000  N"`, `"16442.839843750 uA"` — note the field is
/// *not* truncated when the value overflows the 12-column minimum, but
/// the single separating space is always present).
fn fmt_tsr_result(v: f32, cached: Option<&(i8, String)>) -> String {
    let (scale, units) = match cached {
        Some((s, u)) => (*s, u.as_str()),
        None => (0, ""),
    };
    let scaled = fmt_9(scale_result_f32(v, scale));
    if units.is_empty() {
        scaled
    } else {
        format!("{:<12} {}{units}", scaled, si_prefix(scale as i32))
    }
}

fn fmt_gdr_field(f: &GdrField) -> String {
    match f {
        GdrField::Pad       => "PAD Field".to_string(),
        GdrField::U1(v)     => format!("U1: 0x{v:X}"),
        GdrField::U2(v)     => format!("U2: 0x{v:X}"),
        GdrField::U4(v)     => format!("U4: 0x{v:X}"),
        GdrField::I1(v)     => format!("I1: {v}"),
        GdrField::I2(v)     => format!("I2: {v}"),
        GdrField::I4(v)     => format!("I4: {v}"),
        GdrField::R4(v)     => format!("R4: {}", fmt_9(*v as f64)),
        GdrField::R8(v)     => format!("R8: {v:.9}"),
        GdrField::Cn(s)     => format!("Cn: {s}"),
        GdrField::Bn(b)     => format!("Bn: {}", join_hex(b)),
        GdrField::Dn(b)     => format!("Dn: {}", join_hex(b)),
        GdrField::N1(v)     => format!("N1: 0x{v:X}"),
        GdrField::Unknown(t) => format!("Unknown Field Type: {t}"),
    }
}

// ── Time formatting ───────────────────────────────────────────────────────────

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `<epoch> (dd-Mon-yyyy HH:MM:SS)`, interpreting `epoch` as plain UTC
/// seconds-since-1970 with no timezone adjustment (matches the reference
/// tool exactly — verified against the golden files' MIR/ATR/MRR times).
fn field_time(out: &mut String, name: &str, epoch: u32) {
    field_line(out, name, &format!("{epoch} ({})", format_epoch(epoch)));
}

/// Civil-from-days conversion (Howard Hinnant's algorithm), proleptic
/// Gregorian, UTC — kept dependency-free rather than pulling in chrono.
fn format_epoch(epoch: u32) -> String {
    let secs = epoch as i64;
    let days = secs.div_euclid(86_400);
    let time_of_day = secs.rem_euclid(86_400);
    let (hh, mm, ss) = (time_of_day / 3600, (time_of_day % 3600) / 60, time_of_day % 60);

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{d:02}-{}-{y:04} {hh:02}:{mm:02}:{ss:02}", MONTHS[(m - 1) as usize])
}
