//! Integration tests for the STDF parser and sanity-check engine.
//!
//! All tests are self-contained: they build binary STDF payloads in memory
//! using the fixture helpers below, then drive the parser and/or validation
//! engine and assert on the results.

use stdf_parser::{
    StdfParser, StdfRecord, StdfError,
    sanity::{full_engine, finding::Severity},
};

// ═══════════════════════════════════════════════════════════════════════════════
// ── Fixture builder helpers ──────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

fn make_record(typ: u8, sub: u8, body: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(4 + body.len());
    let len = body.len() as u16;
    v.extend_from_slice(&len.to_le_bytes());
    v.push(typ);
    v.push(sub);
    v.extend_from_slice(body);
    v
}

fn far_le() -> Vec<u8> {
    make_record(0, 10, &[2, 4]) // CPU_TYP=2 (LE), STDF_VER=4
}
fn far_be() -> Vec<u8> {
    // still uses LE for the length in FAR (parser bootstraps as LE)
    make_record(0, 10, &[1, 4])  // CPU_TYP=1 (BE)
}

fn mir_minimal() -> Vec<u8> {
    let mut b = vec![];
    b.extend_from_slice(&0u32.to_le_bytes()); // SETUP_T
    b.extend_from_slice(&1_000_000u32.to_le_bytes()); // START_T
    b.push(1);  // STAT_NUM
    // MODE_COD, RTST_COD, PROT_COD as spaces → None
    b.push(b' '); b.push(b' '); b.push(b' ');
    make_record(1, 10, &b)
}
fn mir_with_lot(lot: &str) -> Vec<u8> {
    let mut b = vec![];
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.push(1);
    b.push(b' '); b.push(b' '); b.push(b' ');
    b.extend_from_slice(&0u16.to_le_bytes()); // BURN_TIM
    b.push(b' '); // CMOD_COD
    // LOT_ID as Cn
    b.push(lot.len() as u8);
    b.extend_from_slice(lot.as_bytes());
    make_record(1, 10, &b)
}

fn mrr() -> Vec<u8> {
    let mut b = vec![];
    b.extend_from_slice(&2_000_000u32.to_le_bytes()); // FINISH_T
    make_record(1, 20, &b)
}

fn wir(head: u8) -> Vec<u8> {
    let mut b = vec![];
    b.push(head);
    b.push(255); // SITE_GRP
    b.extend_from_slice(&0u32.to_le_bytes()); // START_T
    make_record(2, 10, &b)
}
fn wrr(head: u8, parts: u32) -> Vec<u8> {
    let mut b = vec![];
    b.push(head);
    b.push(255);
    b.extend_from_slice(&0u32.to_le_bytes()); // FINISH_T
    b.extend_from_slice(&parts.to_le_bytes()); // PART_CNT
    make_record(2, 20, &b)
}

fn pir(head: u8, site: u8) -> Vec<u8> {
    make_record(5, 10, &[head, site])
}
fn prr(head: u8, site: u8, hbin: u16, pass: bool) -> Vec<u8> {
    let mut b = vec![];
    b.push(head);
    b.push(site);
    b.push(if pass { 0x00 } else { 0x18 }); // PART_FLG
    b.extend_from_slice(&1u16.to_le_bytes()); // NUM_TEST
    b.extend_from_slice(&hbin.to_le_bytes()); // HARD_BIN
    b.extend_from_slice(&hbin.to_le_bytes()); // SOFT_BIN
    make_record(5, 20, &b)
}

fn ptr_passing(test_num: u32, result: f32, lo: f32, hi: f32, name: &str) -> Vec<u8> {
    let mut b = vec![];
    b.extend_from_slice(&test_num.to_le_bytes());
    b.push(1); b.push(0); // HEAD, SITE
    b.push(0x00); // TEST_FLG: pass, result valid
    b.push(0x00); // PARM_FLG
    b.extend_from_slice(&result.to_le_bytes()); // RESULT
    // TEST_TXT
    b.push(name.len() as u8);
    b.extend_from_slice(name.as_bytes());
    b.push(0); // ALARM_ID empty
    b.push(0x00); // OPT_FLAG: all limits valid
    b.push(0); b.push(0); b.push(0); // RES_SCAL, LLM_SCAL, HLM_SCAL
    b.extend_from_slice(&lo.to_le_bytes());
    b.extend_from_slice(&hi.to_le_bytes());
    b.push(0); // UNITS empty
    b.push(0); b.push(0); b.push(0); // C_RESFMT, C_LLMFMT, C_HLMFMT
    make_record(15, 10, &b)
}
fn ptr_no_result(test_num: u32) -> Vec<u8> {
    let mut b = vec![];
    b.extend_from_slice(&test_num.to_le_bytes());
    b.push(1); b.push(0);
    b.push(0x02); // TEST_FLG: result invalid
    b.push(0x00);
    // No RESULT field at all
    make_record(15, 10, &b)
}
fn ptr_inverted_limits(test_num: u32) -> Vec<u8> {
    ptr_passing(test_num, 1.0, 5.0, 1.0, "INV_LIMIT") // lo > hi
}

fn hbr(hbin: u16, cnt: u32) -> Vec<u8> {
    hbr_for_site(255, 255, hbin, cnt) // HEAD=ALL, SITE=ALL summary record
}

fn hbr_for_site(head: u8, site: u8, hbin: u16, cnt: u32) -> Vec<u8> {
    let mut b = vec![];
    b.push(head); b.push(site);
    b.extend_from_slice(&hbin.to_le_bytes());
    b.extend_from_slice(&cnt.to_le_bytes());
    b.push(b'P'); // HBIN_PF
    b.push(0);    // HBIN_NAM empty
    make_record(1, 40, &b)
}

fn bps() -> Vec<u8> { make_record(20, 10, &[0]) }
fn eps() -> Vec<u8> { make_record(20, 20, &[]) }

// Build a clean minimal file: FAR MIR WIR PIR PTR PRR WRR HBR MRR
fn build_minimal_clean_file() -> Vec<u8> {
    let mut v = vec![];
    v.extend(far_le());
    v.extend(mir_with_lot("LOT001"));
    v.extend(wir(1));
    v.extend(pir(1, 1));
    v.extend(ptr_passing(1000, 1.5, 0.0, 3.3, "VOLT_SUPPLY"));
    v.extend(prr(1, 1, 1, true));
    v.extend(wrr(1, 1));
    v.extend(hbr(1, 1));
    v.extend(mrr());
    v
}

fn collect_records(bytes: &[u8]) -> (Vec<StdfRecord>, Option<StdfError>) {
    let mut records = vec![];
    let mut err     = None;
    for item in StdfParser::new(std::io::Cursor::new(bytes)) {
        match item {
            Ok(r)  => records.push(r),
            Err(e) => { err = Some(e); break; }
        }
    }
    (records, err)
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Parser tests ─────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn parser_minimal_clean_file_yields_all_record_types() {
    let bytes = build_minimal_clean_file();
    let (records, err) = collect_records(&bytes);
    assert!(err.is_none(), "unexpected parse error: {err:?}");

    let types: Vec<_> = records.iter().map(|r| r.record_type()).collect();
    assert_eq!(types, ["FAR","MIR","WIR","PIR","PTR","PRR","WRR","HBR","MRR"]);
}

#[test]
fn parser_detects_missing_far() {
    // File starts with MIR instead of FAR
    let bytes = mir_minimal();
    let (_, err) = collect_records(&bytes);
    assert!(matches!(err, Some(StdfError::MissingFar { .. })),
        "expected MissingFar, got {err:?}");
}

#[test]
fn parser_rejects_unsupported_stdf_version() {
    // FAR with STDF_VER=3 (unsupported)
    let bad_far = make_record(0, 10, &[2, 3]);
    let (_, err) = collect_records(&bad_far);
    assert!(matches!(err, Some(StdfError::UnsupportedVersion(3))),
        "expected UnsupportedVersion(3), got {err:?}");
}

#[test]
fn parser_rejects_invalid_cpu_type() {
    let bad_far = make_record(0, 10, &[99, 4]);
    let (_, err) = collect_records(&bad_far);
    assert!(matches!(err, Some(StdfError::InvalidCpuType(99))));
}

#[test]
fn parser_handles_empty_file_without_panic() {
    let (records, err) = collect_records(&[]);
    assert!(records.is_empty());
    assert!(err.is_none(), "empty file should yield Ok(None), got {err:?}");
}

#[test]
fn parser_handles_truncated_record_header() {
    // A partial (3-byte) header at EOF: read_exact treats it as UnexpectedEof
    // which the parser maps to clean Ok(None).  The key invariant is no panic.
    let mut bytes = far_le();
    bytes.extend([0x05, 0x00, 0x01]); // only 3 bytes of a header (needs 4)
    let (records, err) = collect_records(&bytes);
    // FAR parsed, then the 3-byte stub causes the parser to stop (either None or Err)
    assert!(!records.is_empty(), "FAR should be parsed before the truncated header");
    // No panic — err may be Some or None depending on platform IO semantics
    let _ = err;
}

#[test]
fn parser_handles_truncated_record_body() {
    let mut bytes = far_le();
    bytes.extend(mir_minimal());
    // Add a WIR that's declared to be 6 bytes but only 2 provided
    bytes.extend_from_slice(&6u16.to_le_bytes());
    bytes.push(2); bytes.push(10); // WIR typ/sub
    bytes.extend([0x01, 0xFF]); // only 2 of 6 body bytes
    let (records, err) = collect_records(&bytes);
    // FAR and MIR should parse ok; WIR body read causes IO error
    let types: Vec<_> = records.iter().map(|r| r.record_type()).collect();
    assert!(types.contains(&"FAR"));
    assert!(err.is_some());
}

#[test]
fn parser_decodes_big_endian_file() {
    // FAR says CPU_TYP=1 (big-endian), then MIR with BE fields
    let mut bytes = far_be();
    // MIR with big-endian multi-byte fields
    let mut mir_body = vec![];
    mir_body.extend_from_slice(&0u32.to_be_bytes()); // SETUP_T
    mir_body.extend_from_slice(&12345u32.to_be_bytes()); // START_T
    mir_body.push(1);
    mir_body.push(b' '); mir_body.push(b' '); mir_body.push(b' ');
    // Encode MIR header also in LE (FAR length was read as LE by bootstrap)
    // but MIR record length header must be in the *detected* endian (BE after FAR)
    let mir_len = mir_body.len() as u16;
    bytes.extend_from_slice(&mir_len.to_be_bytes()); // BE length
    bytes.push(1); bytes.push(10); // MIR typ/sub
    bytes.extend(mir_body);
    let (records, err) = collect_records(&bytes);
    assert!(err.is_none(), "unexpected error: {err:?}");
    let types: Vec<_> = records.iter().map(|r| r.record_type()).collect();
    assert!(types.contains(&"FAR"));
    assert!(types.contains(&"MIR"));
    if let StdfRecord::Mir(m) = &records[1] {
        assert_eq!(m.start_t, 12345);
    }
}

#[test]
fn parser_passes_unknown_record_through() {
    let mut bytes = far_le();
    bytes.extend(make_record(99, 99, &[0xDE, 0xAD, 0xBE, 0xEF]));
    let (records, err) = collect_records(&bytes);
    assert!(err.is_none());
    if let Some(StdfRecord::Unknown { typ, sub, data }) = records.get(1) {
        assert_eq!(*typ, 99);
        assert_eq!(*sub, 99);
        assert_eq!(data, &[0xDE, 0xAD, 0xBE, 0xEF]);
    } else {
        panic!("expected Unknown record, got {:?}", records.get(1));
    }
}

#[test]
fn parser_ptr_decodes_limits_correctly() {
    let mut bytes = far_le();
    bytes.extend(ptr_passing(5001, 2.5, 0.0, 5.0, "PMON_VOLT"));
    let (records, err) = collect_records(&bytes);
    assert!(err.is_none());
    if let Some(StdfRecord::Ptr(p)) = records.get(1) {
        assert_eq!(p.test_num, 5001);
        assert!((p.result.unwrap() - 2.5).abs() < 1e-5);
        assert!((p.lo_limit.unwrap() - 0.0).abs() < 1e-5);
        assert!((p.hi_limit.unwrap() - 5.0).abs() < 1e-5);
        assert_eq!(p.test_txt.as_deref(), Some("PMON_VOLT"));
        assert!(!p.result_invalid());
        assert!(!p.lo_limit_invalid());
        assert!(!p.hi_limit_invalid());
    } else {
        panic!("expected PTR, got {:?}", records.get(1));
    }
}

#[test]
fn parser_ptr_result_invalid_flag_suppresses_result() {
    let mut bytes = far_le();
    bytes.extend(ptr_no_result(7777));
    let (records, _) = collect_records(&bytes);
    if let Some(StdfRecord::Ptr(p)) = records.get(1) {
        assert!(p.result_invalid(), "result_invalid bit should be set");
        assert!(p.result.is_none(), "result should be None when invalid");
    } else {
        panic!("expected PTR");
    }
}

#[test]
fn parser_hbr_decodes_bin_count() {
    let mut bytes = far_le();
    bytes.extend(hbr(1, 42));
    let (records, err) = collect_records(&bytes);
    assert!(err.is_none());
    if let Some(StdfRecord::Hbr(h)) = records.get(1) {
        assert_eq!(h.hbin_num, 1);
        assert_eq!(h.hbin_cnt, 42);
        assert_eq!(h.hbin_pf, Some('P'));
    }
}

#[test]
fn parser_bps_eps_roundtrip() {
    let mut bytes = far_le();
    bytes.extend(bps());
    bytes.extend(eps());
    let (records, err) = collect_records(&bytes);
    assert!(err.is_none());
    assert!(matches!(records[1], StdfRecord::Bps(_)));
    assert!(matches!(records[2], StdfRecord::Eps(_)));
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Sanity-check / validation tests ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

fn run_engine(bytes: &[u8]) -> stdf_parser::sanity::finding::ValidationReport {
    let mut engine = full_engine();
    let mut parser = StdfParser::new(std::io::Cursor::new(bytes));
    while let Some(item) = parser.next() {
        match item {
            Ok(rec) => engine.feed(&rec, parser.last_record_offset()),
            Err(_)  => break,
        }
    }
    engine.finish()
}

// ── Clean file ───────────────────────────────────────────────────────────────

#[test]
fn sanity_clean_file_produces_no_findings() {
    let bytes  = build_minimal_clean_file();
    let report = run_engine(&bytes);
    assert!(report.is_clean(),
        "expected no findings, got:\n{:#?}", report.findings);
}

// ── PIR/PRR pairing ──────────────────────────────────────────────────────────

#[test]
fn sanity_detects_pir_without_prr() {
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("LOT002"));
    bytes.extend(wir(1));
    bytes.extend(pir(1, 1));
    // NO PRR
    bytes.extend(wrr(1, 1));
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    let found: Vec<_> = report.by_rule("STRUCT/PIR_PRR_PAIR").collect();
    assert!(!found.is_empty(), "expected PIR_PRR_PAIR finding");
}

#[test]
fn sanity_detects_prr_without_pir() {
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("LOT003"));
    bytes.extend(wir(1));
    bytes.extend(prr(1, 1, 1, true)); // PRR without PIR
    bytes.extend(wrr(1, 1));
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    assert!(report.by_rule("STRUCT/PIR_PRR_PAIR").count() > 0);
}

// ── WIR/WRR pairing ──────────────────────────────────────────────────────────

#[test]
fn sanity_detects_wir_without_wrr() {
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("LOT004"));
    bytes.extend(wir(1));
    // No WRR
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    assert!(report.by_rule("STRUCT/WIR_WRR_PAIR").count() > 0 ||
            report.by_rule("ABORT").count() > 0,
        "expected WIR_WRR or ABORT finding");
}

#[test]
fn regression_wrr_cannot_close_wir_opened_on_a_different_head() {
    // Two heads interleave: head 1 and head 2 both open a WIR, but only
    // head 1's WRR ever arrives. A LIFO-stack-based pairing rule would pop
    // *head 2's* WIR offset when it sees head 1's WRR (since it only
    // tracks stack order, not head identity), incorrectly marking head 1's
    // session as the one left open while masking the true problem with
    // head 2. The fix must key on (head_num, site_grp) instead.
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("LOT010"));
    bytes.extend(wir(1));
    bytes.extend(wir(2));
    bytes.extend(wrr(1, 0)); // closes head 1 only
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    let findings: Vec<_> = report.by_rule("STRUCT/WIR_WRR_PAIR").collect();
    assert_eq!(
        findings.len(),
        1,
        "expected exactly one unclosed-WIR finding (for head 2), got: {:?}",
        findings.iter().map(|f| &f.message).collect::<Vec<_>>()
    );
    assert!(
        findings[0].message.contains("head=2"),
        "the unclosed WIR must be correctly attributed to head=2 (not head=1, which was explicitly closed): {}",
        findings[0].message
    );
}

// ── BPS/EPS balance ──────────────────────────────────────────────────────────

#[test]
fn sanity_detects_unbalanced_bps() {
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("LOT005"));
    bytes.extend(bps()); bytes.extend(bps()); // two open
    bytes.extend(eps()); // only one close
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    assert!(report.by_rule("STRUCT/BPS_EPS_BALANCE").count() > 0);
}

#[test]
fn sanity_detects_eps_without_bps() {
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("LOT006"));
    bytes.extend(eps()); // EPS before BPS
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    assert!(report.by_rule("STRUCT/BPS_EPS_BALANCE").count() > 0);
}

// ── MIR singleton ────────────────────────────────────────────────────────────

#[test]
fn sanity_detects_duplicate_mir() {
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("LOT007"));
    bytes.extend(mir_with_lot("LOT007B")); // second MIR
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    assert!(report.by_rule("STRUCT/MIR_SINGLETON").count() > 0);
}

// ── Abort detection ──────────────────────────────────────────────────────────

#[test]
fn sanity_abort_no_mrr_is_warning() {
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("LOT008"));
    // no MRR
    let report = run_engine(&bytes);
    assert!(report.likely_aborted(), "should be flagged as likely aborted");
    assert!(report.by_rule("ABORT/NO_MRR").count() > 0);
}

#[test]
fn sanity_abort_mid_wafer_detected() {
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("LOT009"));
    bytes.extend(wir(1));
    // truncated: no WRR, no MRR
    let report = run_engine(&bytes);
    assert!(report.has_severity(&Severity::Error) || report.has_severity(&Severity::Warning));
}

// ── Field validation ─────────────────────────────────────────────────────────

#[test]
fn sanity_ptr_inverted_limits_flagged() {
    let mut bytes = far_le();
    bytes.extend(ptr_inverted_limits(100));
    let report = run_engine(&bytes);
    let found: Vec<_> = report.by_rule("FIELD/PTR_LIMIT_ORDER").collect();
    assert!(!found.is_empty(), "expected PTR_LIMIT_ORDER finding for lo>hi");
    assert!(found[0].message.contains("100"), "message should include test_num");
}

#[test]
fn sanity_ptr_result_absent_when_flag_clear() {
    // Build a PTR that says result is valid (flag clear) but has no RESULT bytes
    let mut b = vec![];
    b.extend_from_slice(&99u32.to_le_bytes()); // test_num
    b.push(1); b.push(0);
    b.push(0x00); // TEST_FLG: result should be valid
    b.push(0x00); // PARM_FLG
    // RESULT field is missing (only 8 bytes of body)
    let bad_ptr = make_record(15, 10, &b);

    let mut bytes = far_le();
    bytes.extend(bad_ptr);
    let report = run_engine(&bytes);
    assert!(report.by_rule("FIELD/PTR_RESULT_PRESENT").count() > 0,
        "should flag missing result when result_invalid=0");
}

#[test]
fn sanity_mir_missing_lot_id_flagged() {
    let mut bytes = far_le();
    bytes.extend(mir_minimal()); // no lot_id
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    assert!(report.by_rule("FIELD/MIR_LOT_ID").count() > 0);
}

// ── Semantic validation ───────────────────────────────────────────────────────

#[test]
fn sanity_test_name_consistency_flags_mismatch() {
    let mut bytes = far_le();
    bytes.extend(ptr_passing(5000, 1.0, 0.0, 3.3, "VOLT_A"));
    bytes.extend(ptr_passing(5000, 1.1, 0.0, 3.3, "VOLT_B")); // same num, different name
    let report = run_engine(&bytes);
    assert!(report.by_rule("SEMANTIC/TEST_NAME_CONSISTENCY").count() > 0);
}

#[test]
fn sanity_hbr_bin_count_mismatch_detected() {
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("LOT010"));
    bytes.extend(wir(1));
    bytes.extend(pir(1, 1));
    bytes.extend(prr(1, 1, 1, true)); // 1 PRR for bin 1
    bytes.extend(wrr(1, 1));
    bytes.extend(hbr(1, 99)); // HBR says 99 — mismatch
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    assert!(report.by_rule("SEMANTIC/HBR_BIN_COUNT").count() > 0);
}

#[test]
fn sanity_hbr_bin_count_matches_passes() {
    let bytes = build_minimal_clean_file();
    let report = run_engine(&bytes);
    assert_eq!(report.by_rule("SEMANTIC/HBR_BIN_COUNT").count(), 0,
        "clean file should have no HBR mismatch");
}

// ═══════════════════════════════════════════════════════════════════════════════
// ── Edge-case / regression tests ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn regression_mir_lot_id_missing_is_warning_not_error() {
    let mut bytes = far_le();
    bytes.extend(mir_minimal());
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    // The MIR_LOT_ID rule fires with Severity::Warning
    for f in report.by_rule("FIELD/MIR_LOT_ID") {
        assert_eq!(f.severity, Severity::Warning, "LOT_ID absence should be a Warning");
    }
}

#[test]
fn regression_ptr_with_nan_limits_does_not_panic() {
    let mut b = vec![];
    b.extend_from_slice(&1u32.to_le_bytes()); // test_num
    b.push(1); b.push(0);
    b.push(0x00); b.push(0x00);
    b.extend_from_slice(&1.0f32.to_le_bytes()); // RESULT
    b.push(0); b.push(0); // TEST_TXT, ALARM_ID
    b.push(0x00); // OPT_FLAG
    b.push(0); b.push(0); b.push(0); // scalings
    b.extend_from_slice(&f32::NAN.to_le_bytes()); // lo NaN
    b.extend_from_slice(&f32::NAN.to_le_bytes()); // hi NaN
    b.push(0); // UNITS
    b.push(0); b.push(0); b.push(0); // fmt strings
    let mut bytes = far_le();
    bytes.extend(make_record(15, 10, &b));
    let (records, err) = collect_records(&bytes);
    assert!(err.is_none(), "NaN in limits must not cause parse error");
    if let Some(StdfRecord::Ptr(p)) = records.get(1) {
        // NaN limits are present but comparisons in sanity check should not panic
        assert!(p.lo_limit.map(|v| v.is_nan()).unwrap_or(false));
    }
    // Run sanity — should not panic
    let _ = run_engine(&bytes);
}

#[test]
fn regression_multi_site_pir_prr_pairs() {
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("MULTISITE"));
    bytes.extend(wir(1));
    // site 1 and site 2 in parallel: both PIRs open before either PRR closes
    bytes.extend(pir(1, 1));
    bytes.extend(pir(1, 2));
    bytes.extend(prr(1, 1, 1, true));
    bytes.extend(prr(1, 2, 1, true));
    bytes.extend(wrr(1, 2));
    bytes.extend(hbr(1, 2));
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    // Multi-site PIR/PRR interleaving (PIR/PIR then PRR/PRR) is normal
    // production behaviour and must not be flagged as an error.
    assert_eq!(report.by_rule("STRUCT/PIR_PRR_PAIR").count(), 0,
        "multi-site PIR/PIR PRR/PRR interleave should not be flagged: {:#?}",
        report.by_rule("STRUCT/PIR_PRR_PAIR").collect::<Vec<_>>());
}

#[test]
fn regression_multi_site_unclosed_pir_detected_at_eof() {
    // Site 1 closes normally; site 2's PIR is left open (aborted mid-part).
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("MULTISITE2"));
    bytes.extend(wir(1));
    bytes.extend(pir(1, 1));
    bytes.extend(pir(1, 2));
    bytes.extend(prr(1, 1, 1, true));
    // site 2 never gets its PRR — simulated abort
    let report = run_engine(&bytes);

    let struct_findings: Vec<_> = report.by_rule("STRUCT/PIR_PRR_PAIR").collect();
    assert_eq!(struct_findings.len(), 1, "exactly one open PIR (site 2) should remain: {struct_findings:#?}");
    assert!(struct_findings[0].message.contains("site=2"), "should identify site 2: {}", struct_findings[0].message);

    let abort_findings: Vec<_> = report.by_rule("ABORT/OPEN_PIR").collect();
    assert_eq!(abort_findings.len(), 1, "exactly one ABORT/OPEN_PIR finding for site 2: {abort_findings:#?}");
    assert!(abort_findings[0].message.contains("site=2"), "abort finding should name site 2: {}", abort_findings[0].message);
    // Site 1's closed PIR/PRR must not also be reported as aborted.
    assert!(!abort_findings.iter().any(|f| f.message.contains("site=1")));
}

#[test]
fn regression_hbr_per_site_counts_are_summed_not_overwritten() {
    // Two sites each report their own HBR for bin 1 (1 part each); a naive
    // "last HBR wins" implementation would compare only 1 against the PRR
    // total of 2 and falsely flag a mismatch.
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("HBRSUM"));
    bytes.extend(wir(1));
    bytes.extend(pir(1, 1));
    bytes.extend(pir(1, 2));
    bytes.extend(prr(1, 1, 1, true));
    bytes.extend(prr(1, 2, 1, true));
    bytes.extend(wrr(1, 2));
    bytes.extend(hbr_for_site(1, 1, 1, 1)); // head=1 site=1 bin=1 count=1
    bytes.extend(hbr_for_site(1, 2, 1, 1)); // head=1 site=2 bin=1 count=1
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    assert_eq!(report.by_rule("SEMANTIC/HBR_BIN_COUNT").count(), 0,
        "per-site HBR counts (1+1) should sum to match PRR total (2): {:#?}",
        report.by_rule("SEMANTIC/HBR_BIN_COUNT").collect::<Vec<_>>());
}

#[test]
fn regression_hbr_all_heads_rollup_alongside_per_site_is_not_double_counted() {
    // Some testers emit a genuine per-site HBR *and* a HEAD_NUM=255 "ALL
    // HEADS" rollup record for the same bin, where the rollup restates
    // the same parts rather than adding new ones (observed in real
    // production STDF files). Naively summing every HBR record
    // regardless of head/site would double the true count.
    let mut bytes = far_le();
    bytes.extend(mir_with_lot("HBRROLLUP"));
    bytes.extend(wir(1));
    bytes.extend(pir(1, 1));
    bytes.extend(prr(1, 1, 1, true)); // 1 part, bin 1
    bytes.extend(wrr(1, 1));
    bytes.extend(hbr_for_site(1, 1, 1, 1));   // true per-site record: head=1 site=1 bin=1 cnt=1
    bytes.extend(hbr_for_site(255, 0, 1, 1)); // ALL_HEADS rollup restating the same 1 part
    bytes.extend(mrr());
    let report = run_engine(&bytes);
    assert_eq!(report.by_rule("SEMANTIC/HBR_BIN_COUNT").count(), 0,
        "granular (1) + rollup (1) of the same parts must not sum to 2 vs PRR's 1: {:#?}",
        report.by_rule("SEMANTIC/HBR_BIN_COUNT").collect::<Vec<_>>());
}
