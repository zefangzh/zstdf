use crate::error::StdfError;
use crate::fields::FieldReader;
use crate::header::{detect_endianness, read_header, Endianness, RecordHeader};

/// File Attributes Record - always the first record in a valid STDF file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FarRecord {
    pub cpu_type: u8,
    pub stdf_ver: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirRecord {
    pub setup_t: u32,
    pub start_t: u32,
    pub stat_num: u8,
    pub mode_cod: char,
    pub rtst_cod: char,
    pub prot_cod: char,
    pub burn_tim: u16,
    pub cmod_cod: char,
    pub lot_id: String,
    pub part_typ: String,
    pub node_nam: String,
    pub tstr_typ: String,
    pub job_nam: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MrrRecord {
    pub finish_t: u32,
    pub disp_cod: Option<char>,
    pub usr_desc: Option<String>,
    pub exc_desc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WirRecord {
    pub head_num: u8,
    pub site_grp: u8,
    pub start_t: u32,
    pub wafer_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrrRecord {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PirRecord {
    pub head_num: u8,
    pub site_num: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrrRecord {
    pub head_num: u8,
    pub site_num: u8,
    pub part_flg: u8,
    pub num_test: u16,
    pub hard_bin: u16,
    pub soft_bin: u16,
    pub x_coord: Option<i16>,
    pub y_coord: Option<i16>,
    pub test_t: Option<u32>,
    pub part_id: Option<String>,
    pub part_txt: Option<String>,
    pub part_fix: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PtrRecord {
    pub test_num: u32,
    pub head_num: u8,
    pub site_num: u8,
    pub test_flg: u8,
    pub parm_flg: u8,
    pub result: f32,
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

impl PtrRecord {
    pub fn test_pass(&self) -> Option<bool> {
        if self.test_flg & 0x40 == 0 {
            return None;
        }
        Some(self.test_flg & 0x80 == 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbrRecord {
    pub head_num: u8,
    pub site_num: u8,
    pub hbin_num: u16,
    pub hbin_cnt: u32,
    pub hbin_pf: Option<char>,
    pub hbin_nam: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SbrRecord {
    pub head_num: u8,
    pub site_num: u8,
    pub sbin_num: u16,
    pub sbin_cnt: u32,
    pub sbin_pf: Option<char>,
    pub sbin_nam: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmrRecord {
    pub pmr_indx: u16,
    pub chan_typ: Option<u16>,
    pub chan_nam: Option<String>,
    pub phy_nam: Option<String>,
    pub log_nam: Option<String>,
    pub head_num: Option<u8>,
    pub site_num: Option<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TsrRecord {
    pub head_num: u8,
    pub site_num: u8,
    pub test_typ: char,
    pub test_num: u32,
    pub exec_cnt: u32,
    pub fail_cnt: u32,
    pub alrm_cnt: u32,
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

/// A decoded STDF record.
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    Far(FarRecord),
    Mir(MirRecord),
    Mrr(MrrRecord),
    Wir(WirRecord),
    Wrr(WrrRecord),
    Pir(PirRecord),
    Prr(PrrRecord),
    Ptr(PtrRecord),
    Hbr(HbrRecord),
    Sbr(SbrRecord),
    Pmr(PmrRecord),
    Tsr(TsrRecord),
    /// Any record type not yet implemented. Carries the raw payload so
    /// callers can skip it, log it, or inspect it manually rather than
    /// failing the whole file on an unimplemented type.
    Unimplemented {
        rec_typ: u8,
        rec_sub: u8,
        payload: Vec<u8>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecodeSummary {
    pub endianness: Endianness,
    pub records: Vec<Record>,
    pub trailing_error: Option<StdfError>,
}

/// Decode one record starting at offset 0 of `bytes`. Returns the parsed
/// record and how many bytes it consumed.
pub fn decode_record(bytes: &[u8], endianness: Endianness) -> Result<(Record, usize), StdfError> {
    let header: RecordHeader = read_header(bytes, endianness)?;
    let total_len = 4 + header.len as usize;
    if bytes.len() < total_len {
        return Err(StdfError::Truncated);
    }
    let payload = &bytes[4..total_len];

    let record = match (header.rec_typ, header.rec_sub) {
        (0, 10) => decode_far(payload)?,
        (1, 10) => decode_mir(payload, endianness)?,
        (1, 20) => decode_mrr(payload, endianness)?,
        (1, 40) => decode_hbr(payload, endianness)?,
        (1, 50) => decode_sbr(payload, endianness)?,
        (1, 60) => decode_pmr(payload, endianness)?,
        (2, 10) => decode_wir(payload, endianness)?,
        (2, 20) => decode_wrr(payload, endianness)?,
        (5, 10) => decode_pir(payload, endianness)?,
        (5, 20) => decode_prr(payload, endianness)?,
        (10, 30) => decode_tsr(payload, endianness)?,
        (15, 10) => decode_ptr(payload, endianness)?,
        (rec_typ, rec_sub) => Record::Unimplemented {
            rec_typ,
            rec_sub,
            payload: payload.to_vec(),
        },
    };

    Ok((record, total_len))
}

/// Decode an entire STDF buffer. Valid records before a truncation are
/// returned in `records`; a trailing partial record is surfaced separately.
pub fn decode_all(bytes: &[u8]) -> Result<DecodeSummary, StdfError> {
    let endianness = detect_endianness(bytes)?;
    let mut offset = 0;
    let mut records = Vec::new();

    while offset < bytes.len() {
        match decode_record(&bytes[offset..], endianness) {
            Ok((record, consumed)) => {
                records.push(record);
                offset += consumed;
            }
            Err(StdfError::Truncated | StdfError::UnexpectedEof { .. }) => {
                return Ok(DecodeSummary {
                    endianness,
                    records,
                    trailing_error: Some(StdfError::Truncated),
                });
            }
            Err(err) => return Err(err),
        }
    }

    Ok(DecodeSummary {
        endianness,
        records,
        trailing_error: None,
    })
}

fn decode_far(payload: &[u8]) -> Result<Record, StdfError> {
    if payload.len() < 2 {
        return Err(StdfError::InvalidFar);
    }
    Ok(Record::Far(FarRecord {
        cpu_type: payload[0],
        stdf_ver: payload[1],
    }))
}

fn decode_mir(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Mir(MirRecord {
        setup_t: r.read_u4()?,
        start_t: r.read_u4()?,
        stat_num: r.read_u1()?,
        mode_cod: r.read_c1()?,
        rtst_cod: r.read_c1()?,
        prot_cod: r.read_c1()?,
        burn_tim: r.read_u2()?,
        cmod_cod: r.read_c1()?,
        lot_id: r.read_cn()?,
        part_typ: r.read_cn()?,
        node_nam: r.read_cn()?,
        tstr_typ: r.read_cn()?,
        job_nam: r.read_cn()?,
        job_rev: r.read_optional_cn()?,
        sblot_id: r.read_optional_cn()?,
        oper_nam: r.read_optional_cn()?,
        exec_typ: r.read_optional_cn()?,
        exec_ver: r.read_optional_cn()?,
        test_cod: r.read_optional_cn()?,
        tst_temp: r.read_optional_cn()?,
        user_txt: r.read_optional_cn()?,
        aux_file: r.read_optional_cn()?,
        pkg_typ: r.read_optional_cn()?,
        famly_id: r.read_optional_cn()?,
        date_cod: r.read_optional_cn()?,
        facil_id: r.read_optional_cn()?,
        floor_id: r.read_optional_cn()?,
        proc_id: r.read_optional_cn()?,
        oper_frq: r.read_optional_cn()?,
        spec_nam: r.read_optional_cn()?,
        spec_ver: r.read_optional_cn()?,
        flow_id: r.read_optional_cn()?,
        setup_id: r.read_optional_cn()?,
        dsgn_rev: r.read_optional_cn()?,
        eng_id: r.read_optional_cn()?,
        rom_cod: r.read_optional_cn()?,
        serl_num: r.read_optional_cn()?,
        supr_nam: r.read_optional_cn()?,
    }))
}

fn decode_mrr(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Mrr(MrrRecord {
        finish_t: r.read_u4()?,
        disp_cod: r.read_optional_c1()?,
        usr_desc: r.read_optional_cn()?,
        exc_desc: r.read_optional_cn()?,
    }))
}

fn decode_hbr(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Hbr(HbrRecord {
        head_num: r.read_u1()?,
        site_num: r.read_u1()?,
        hbin_num: r.read_u2()?,
        hbin_cnt: r.read_u4()?,
        hbin_pf: r.read_optional_c1()?,
        hbin_nam: r.read_optional_cn()?,
    }))
}

fn decode_sbr(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Sbr(SbrRecord {
        head_num: r.read_u1()?,
        site_num: r.read_u1()?,
        sbin_num: r.read_u2()?,
        sbin_cnt: r.read_u4()?,
        sbin_pf: r.read_optional_c1()?,
        sbin_nam: r.read_optional_cn()?,
    }))
}

fn decode_pmr(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Pmr(PmrRecord {
        pmr_indx: r.read_u2()?,
        chan_typ: r.read_optional_u2()?,
        chan_nam: r.read_optional_cn()?,
        phy_nam: r.read_optional_cn()?,
        log_nam: r.read_optional_cn()?,
        head_num: r.read_optional_u1()?,
        site_num: r.read_optional_u1()?,
    }))
}

fn decode_wir(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Wir(WirRecord {
        head_num: r.read_u1()?,
        site_grp: r.read_u1()?,
        start_t: r.read_u4()?,
        wafer_id: r.read_optional_cn()?,
    }))
}

fn decode_wrr(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Wrr(WrrRecord {
        head_num: r.read_u1()?,
        site_grp: r.read_u1()?,
        finish_t: r.read_u4()?,
        part_cnt: r.read_u4()?,
        rtst_cnt: r.read_optional_u4()?,
        abrt_cnt: r.read_optional_u4()?,
        good_cnt: r.read_optional_u4()?,
        func_cnt: r.read_optional_u4()?,
        wafer_id: r.read_optional_cn()?,
        fabwf_id: r.read_optional_cn()?,
        frame_id: r.read_optional_cn()?,
        mask_id: r.read_optional_cn()?,
        usr_desc: r.read_optional_cn()?,
        exc_desc: r.read_optional_cn()?,
    }))
}

fn decode_pir(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Pir(PirRecord {
        head_num: r.read_u1()?,
        site_num: r.read_u1()?,
    }))
}

fn decode_prr(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Prr(PrrRecord {
        head_num: r.read_u1()?,
        site_num: r.read_u1()?,
        part_flg: r.read_u1()?,
        num_test: r.read_u2()?,
        hard_bin: r.read_u2()?,
        soft_bin: r.read_u2()?,
        x_coord: r.read_optional_i2()?,
        y_coord: r.read_optional_i2()?,
        test_t: r.read_optional_u4()?,
        part_id: r.read_optional_cn()?,
        part_txt: r.read_optional_cn()?,
        part_fix: r.read_optional_bn()?,
    }))
}

fn decode_tsr(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Tsr(TsrRecord {
        head_num: r.read_u1()?,
        site_num: r.read_u1()?,
        test_typ: r.read_c1()?,
        test_num: r.read_u4()?,
        exec_cnt: r.read_u4()?,
        fail_cnt: r.read_u4()?,
        alrm_cnt: r.read_u4()?,
        test_nam: r.read_optional_cn()?,
        seq_name: r.read_optional_cn()?,
        test_lbl: r.read_optional_cn()?,
        opt_flag: r.read_optional_u1()?,
        test_tim: r.read_optional_r4()?,
        test_min: r.read_optional_r4()?,
        test_max: r.read_optional_r4()?,
        tst_sums: r.read_optional_r4()?,
        tst_sqrs: r.read_optional_r4()?,
    }))
}

fn decode_ptr(payload: &[u8], endianness: Endianness) -> Result<Record, StdfError> {
    let mut r = FieldReader::new(payload, endianness);
    Ok(Record::Ptr(PtrRecord {
        test_num: r.read_u4()?,
        head_num: r.read_u1()?,
        site_num: r.read_u1()?,
        test_flg: r.read_u1()?,
        parm_flg: r.read_u1()?,
        result: r.read_r4()?,
        test_txt: r.read_optional_cn()?,
        alarm_id: r.read_optional_cn()?,
        opt_flag: r.read_optional_u1()?,
        res_scal: r.read_optional_i1()?,
        llm_scal: r.read_optional_i1()?,
        hlm_scal: r.read_optional_i1()?,
        lo_limit: r.read_optional_r4()?,
        hi_limit: r.read_optional_r4()?,
        units: r.read_optional_cn()?,
        c_resfmt: r.read_optional_cn()?,
        c_llmfmt: r.read_optional_cn()?,
        c_hlmfmt: r.read_optional_cn()?,
        lo_spec: r.read_optional_r4()?,
        hi_spec: r.read_optional_r4()?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(len: u16, rec_typ: u8, rec_sub: u8, endianness: Endianness) -> Vec<u8> {
        let mut out = Vec::with_capacity(4);
        match endianness {
            Endianness::Little => out.extend_from_slice(&len.to_le_bytes()),
            Endianness::Big => out.extend_from_slice(&len.to_be_bytes()),
        }
        out.push(rec_typ);
        out.push(rec_sub);
        out
    }

    fn push_u2(buf: &mut Vec<u8>, value: u16, endianness: Endianness) {
        match endianness {
            Endianness::Little => buf.extend_from_slice(&value.to_le_bytes()),
            Endianness::Big => buf.extend_from_slice(&value.to_be_bytes()),
        }
    }

    fn push_u4(buf: &mut Vec<u8>, value: u32, endianness: Endianness) {
        match endianness {
            Endianness::Little => buf.extend_from_slice(&value.to_le_bytes()),
            Endianness::Big => buf.extend_from_slice(&value.to_be_bytes()),
        }
    }

    fn push_i2(buf: &mut Vec<u8>, value: i16, endianness: Endianness) {
        match endianness {
            Endianness::Little => buf.extend_from_slice(&value.to_le_bytes()),
            Endianness::Big => buf.extend_from_slice(&value.to_be_bytes()),
        }
    }

    fn push_f32(buf: &mut Vec<u8>, value: f32, endianness: Endianness) {
        push_u4(buf, value.to_bits(), endianness);
    }

    fn push_cn(buf: &mut Vec<u8>, value: &str) {
        buf.push(value.len() as u8);
        buf.extend_from_slice(value.as_bytes());
    }

    fn far_record(endianness: Endianness) -> Vec<u8> {
        let cpu_type = match endianness {
            Endianness::Little => 2,
            Endianness::Big => 1,
        };
        vec![0x02, 0x00, 0x00, 0x0A, cpu_type, 0x04]
    }

    #[test]
    fn detects_little_endian_far() {
        let bytes = [0x02, 0x00, 0x00, 0x0A, 0x02, 0x04];
        assert_eq!(detect_endianness(&bytes).unwrap(), Endianness::Little);
    }

    #[test]
    fn detects_big_endian_far() {
        let bytes = [0x00, 0x02, 0x00, 0x0A, 0x01, 0x04];
        assert_eq!(detect_endianness(&bytes).unwrap(), Endianness::Big);
    }

    #[test]
    fn rejects_unsupported_far_cpu_type() {
        let bytes = [0x02, 0x00, 0x00, 0x0A, 0x00, 0x04];
        assert_eq!(
            detect_endianness(&bytes).unwrap_err(),
            StdfError::UnsupportedCpuType(0)
        );
    }

    #[test]
    fn decodes_far_record() {
        let bytes = [0x02, 0x00, 0x00, 0x0A, 0x02, 0x04];
        let (record, consumed) = decode_record(&bytes, Endianness::Little).unwrap();
        assert_eq!(consumed, 6);
        match record {
            Record::Far(far) => {
                assert_eq!(far.cpu_type, 2);
                assert_eq!(far.stdf_ver, 4);
            }
            _ => panic!("expected FAR record"),
        }
    }

    #[test]
    fn decodes_mir_record() {
        let endianness = Endianness::Little;
        let mut payload = Vec::new();
        push_u4(&mut payload, 1, endianness);
        push_u4(&mut payload, 2, endianness);
        payload.push(3);
        payload.push(b'P');
        payload.push(b'R');
        payload.push(b' ');
        push_u2(&mut payload, 15, endianness);
        payload.push(b'C');
        push_cn(&mut payload, "LOT001");
        push_cn(&mut payload, "ASIC");
        push_cn(&mut payload, "NODE");
        push_cn(&mut payload, "TSTR");
        push_cn(&mut payload, "JOB");
        push_cn(&mut payload, "REV1");

        let mut bytes = header(payload.len() as u16, 1, 10, endianness);
        bytes.extend_from_slice(&payload);

        let (record, _) = decode_record(&bytes, endianness).unwrap();
        match record {
            Record::Mir(mir) => {
                assert_eq!(mir.lot_id, "LOT001");
                assert_eq!(mir.part_typ, "ASIC");
                assert_eq!(mir.job_rev.as_deref(), Some("REV1"));
                assert_eq!(mir.exec_typ, None);
            }
            _ => panic!("expected MIR record"),
        }
    }

    #[test]
    fn decodes_ptr_record_and_derives_pass_fail() {
        let endianness = Endianness::Little;
        let mut payload = Vec::new();
        push_u4(&mut payload, 1234, endianness);
        payload.push(1);
        payload.push(2);
        payload.push(0x40);
        payload.push(0x00);
        push_f32(&mut payload, 1.25, endianness);
        push_cn(&mut payload, "IDDQ");
        push_cn(&mut payload, "ALARM");
        payload.push(0x00);
        payload.push(0);
        payload.push(0);
        payload.push(0);
        push_f32(&mut payload, 0.5, endianness);
        push_f32(&mut payload, 1.5, endianness);
        push_cn(&mut payload, "mA");

        let mut bytes = header(payload.len() as u16, 15, 10, endianness);
        bytes.extend_from_slice(&payload);

        let (record, _) = decode_record(&bytes, endianness).unwrap();
        match record {
            Record::Ptr(ptr) => {
                assert_eq!(ptr.test_num, 1234);
                assert_eq!(ptr.result, 1.25);
                assert_eq!(ptr.test_txt.as_deref(), Some("IDDQ"));
                assert_eq!(ptr.units.as_deref(), Some("mA"));
                assert_eq!(ptr.test_pass(), Some(true));
            }
            _ => panic!("expected PTR record"),
        }
    }

    #[test]
    fn decodes_prr_record_with_optional_fields() {
        let endianness = Endianness::Big;
        let mut payload = Vec::new();
        payload.push(1);
        payload.push(4);
        payload.push(0x08);
        push_u2(&mut payload, 12, endianness);
        push_u2(&mut payload, 100, endianness);
        push_u2(&mut payload, 200, endianness);
        push_i2(&mut payload, 11, endianness);
        push_i2(&mut payload, -7, endianness);
        push_u4(&mut payload, 99, endianness);
        push_cn(&mut payload, "PART-7");
        push_cn(&mut payload, "bin fail");
        payload.push(2);
        payload.extend_from_slice(&[0xAA, 0x55]);

        let mut bytes = header(payload.len() as u16, 5, 20, endianness);
        bytes.extend_from_slice(&payload);

        let (record, _) = decode_record(&bytes, endianness).unwrap();
        match record {
            Record::Prr(prr) => {
                assert_eq!(prr.x_coord, Some(11));
                assert_eq!(prr.y_coord, Some(-7));
                assert_eq!(prr.test_t, Some(99));
                assert_eq!(prr.part_id.as_deref(), Some("PART-7"));
                assert_eq!(prr.part_fix.as_deref(), Some(&[0xAA, 0x55][..]));
            }
            _ => panic!("expected PRR record"),
        }
    }

    #[test]
    fn errors_on_truncated_record() {
        let bytes = [0x10, 0x00, 0x00, 0x0A, 0x02];
        let err = decode_record(&bytes, Endianness::Little).unwrap_err();
        assert_eq!(err, StdfError::Truncated);
    }

    #[test]
    fn decode_all_returns_records_before_truncation() {
        let mut bytes = far_record(Endianness::Little);
        let mut pir = header(2, 5, 10, Endianness::Little);
        pir.extend_from_slice(&[1, 1]);
        bytes.extend_from_slice(&pir);
        bytes.extend_from_slice(&[0x08, 0x00, 15, 10, 0x01, 0x02]);

        let summary = decode_all(&bytes).unwrap();
        assert_eq!(summary.records.len(), 2);
        assert_eq!(summary.trailing_error, Some(StdfError::Truncated));
        assert!(matches!(summary.records[1], Record::Pir(_)));
    }

    #[test]
    fn unimplemented_type_is_skippable_not_fatal() {
        let bytes = [0x02, 0x00, 15, 15, 0xAB, 0xCD];
        let (record, consumed) = decode_record(&bytes, Endianness::Little).unwrap();
        assert_eq!(consumed, 6);
        match record {
            Record::Unimplemented {
                rec_typ,
                rec_sub,
                payload,
            } => {
                assert_eq!(rec_typ, 15);
                assert_eq!(rec_sub, 15);
                assert_eq!(payload, vec![0xAB, 0xCD]);
            }
            _ => panic!("expected Unimplemented record"),
        }
    }
}
