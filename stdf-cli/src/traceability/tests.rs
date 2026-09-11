use super::*;
use serde_json::{json, Value};
use std::sync::atomic::AtomicU64;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "zstdf-trace-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let p = self.path(name);
        fs::write(&p, bytes).unwrap();
        p
    }
    fn args(&self, inputs: Vec<PathBuf>) -> Arguments {
        Arguments {
            inputs,
            flow_config: self.write("flow.json", &serde_json::to_vec(&flow()).unwrap()),
            output: self.path("trace.html"),
            flow_closures: None,
            identity_map: None,
            memory_limit_mib: 16,
            disk_limit_mib: 8,
            max_report_mib: 4,
            max_devices: 100,
            max_sources: 100,
            temp_dir: Some(self.0.clone()),
            cancel_file: None,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn flow() -> Flow {
    serde_json::from_value(json!({"flow_id":"demo","version":"1","steps":[
    {"id":"A","required":true,"stop_on_fail":true,"matches":[{"job_nam":"A"},{"job_nam":"A2","job_rev":"v2"}]},
    {"id":"B","required":true,"matches":[{"job_nam":"B"}]},
    {"id":"C","required":true,"matches":[{"job_nam":"C"}]},
    {"id":"O","required":false,"matches":[{"job_nam":"O"}]}]})).unwrap()
}
fn rec(typ: u8, sub: u8, body: &[u8]) -> Vec<u8> {
    let mut r = (body.len() as u16).to_le_bytes().to_vec();
    r.extend([typ, sub]);
    r.extend(body);
    r
}
fn cn(s: &str) -> Vec<u8> {
    let mut b = vec![s.len() as u8];
    b.extend(s.as_bytes());
    b
}
fn start(job: &str, wafer: &str, time: u32) -> Vec<u8> {
    let mut b = rec(0, 10, &[2, 4]);
    let mut mir = vec![0; 15];
    mir[4..8].copy_from_slice(&time.to_le_bytes());
    for s in ["LOT", "DIE", "NODE", "TESTER", job, "v1"] {
        mir.extend(cn(s));
    }
    b.extend(rec(1, 10, &mir));
    let mut wir = vec![1, 255];
    wir.extend(time.to_le_bytes());
    wir.extend(cn(wafer));
    b.extend(rec(2, 10, &wir));
    b
}
fn pir(site: u8) -> Vec<u8> {
    rec(5, 10, &[1, site])
}
fn ptr(site: u8, name: &str, value: f32) -> Vec<u8> {
    let mut b = 1u32.to_le_bytes().to_vec();
    b.extend([1, site, 0, 0]);
    b.extend(value.to_le_bytes());
    b.extend(cn(name));
    rec(15, 10, &b)
}
fn prr(site: u8, x: i16, y: i16, flag: u8, hb: u16, sb: u16) -> Vec<u8> {
    let mut b = vec![1, site, flag];
    b.extend(0u16.to_le_bytes());
    b.extend(hb.to_le_bytes());
    b.extend(sb.to_le_bytes());
    b.extend(x.to_le_bytes());
    b.extend(y.to_le_bytes());
    b.extend(0u32.to_le_bytes());
    b.extend(cn("REPEATED"));
    rec(5, 20, &b)
}
fn finish(b: &mut Vec<u8>, time: u32) {
    b.extend(rec(1, 20, &time.to_le_bytes()));
}
fn simple(job: &str, time: u32, flags: &[u8]) -> Vec<u8> {
    let mut b = start(job, "W1", time);
    for &flag in flags {
        b.extend(pir(1));
        b.extend(prr(1, 1, 2, flag, 1, 1));
    }
    finish(&mut b, time + 10);
    b
}
fn report(args: &Arguments) -> Value {
    generate(args, &mut Vec::new(), Arc::new(AtomicBool::new(false))).unwrap();
    let html = fs::read_to_string(&args.output).unwrap();
    let s = html
        .split("id=\"trace-data\">")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    serde_json::from_str(s).unwrap()
}
fn device(v: &Value) -> &Value {
    &v["devices"][0]
}
fn step(v: &Value, i: usize) -> &Value {
    &device(v)["steps"][i]
}
fn closed(f: &Fixture, a: &mut Arguments) {
    a.flow_closures = Some(f.write(
        "closures.json",
        br#"[{"flow_id":"demo","version":"1","lot":"LOT"}]"#,
    ));
}

#[test]
fn normal_flow_and_pending_are_distinct_from_mrr_closure() {
    let f = Fixture::new();
    let a = f.args(vec![f.write("a.stdf", &simple("A", 100, &[0]))]);
    let v = report(&a);
    assert_eq!(step(&v, 0)["status"], "tested");
    assert_eq!(step(&v, 1)["status"], "pending");
    assert_eq!(step(&v, 3)["status"], "not_applicable");
    assert_eq!(device(&v)["closed"], false);
}
#[test]
fn pass_fail_pass_and_bin_only_changes_compare_all_attempts() {
    let f = Fixture::new();
    let mut b = simple("A", 100, &[0, 8, 0]);
    let a = f.args(vec![f.write("a.stdf", &b)]);
    let v = report(&a);
    assert_eq!(step(&v, 0)["tests"], 3);
    assert_eq!(step(&v, 0)["retests"], 2);
    assert_eq!(step(&v, 0)["changes"], json!(["verdict_changed"]));
    assert_eq!(step(&v, 0)["latest_passed"], true);
    b = start("A", "W1", 100);
    for (hb, sb) in [(1, 1), (2, 3)] {
        b.extend(pir(1));
        b.extend(prr(1, 1, 2, 0, hb, sb));
    }
    finish(&mut b, 110);
    f.write("a.stdf", &b);
    assert_eq!(
        step(&report(&a), 0)["changes"],
        json!(["hard_bin_changed", "soft_bin_changed"])
    );
}
#[test]
fn consistent_retest_unknown_result_and_invalid_bins() {
    let f = Fixture::new();
    let a = f.args(vec![f.write("a.stdf", &simple("A", 100, &[0, 0]))]);
    assert_eq!(step(&report(&a), 0)["status"], "retest_consistent");
    let mut b = start("A", "W1", 100);
    for flag in [0, 16, 4] {
        b.extend(prr(1, 1, 2, flag, 65535, 40000));
    }
    finish(&mut b, 110);
    f.write("a.stdf", &b);
    let v = report(&a);
    assert_eq!(step(&v, 0)["status"], "indeterminate");
    assert_eq!(step(&v, 0)["changes"], json!([]));
    assert_eq!(step(&v, 1)["status"], "indeterminate");
    assert!(step(&v, 0)["attempts"][1]["passed"].is_null());
    assert!(step(&v, 0)["attempts"][0]["hard_bin"].is_null());
}
#[test]
fn middle_missing_and_closed_tail_missing() {
    let f = Fixture::new();
    let mut a = f.args(vec![
        f.write("a.stdf", &simple("A", 100, &[0])),
        f.write("c.stdf", &simple("C", 200, &[0])),
    ]);
    let v = report(&a);
    assert_eq!(step(&v, 1)["status"], "missing");
    a.inputs.pop();
    closed(&f, &mut a);
    let v = report(&a);
    assert_eq!(step(&v, 2)["status"], "missing");
    assert_eq!(device(&v)["closed"], true);
}
#[test]
fn stop_and_retest_recovery_do_not_hide_existing_downstream() {
    let f = Fixture::new();
    let mut a = f.args(vec![f.write("a.stdf", &simple("A", 100, &[8]))]);
    closed(&f, &mut a);
    assert_eq!(step(&report(&a), 1)["status"], "not_applicable");
    f.write("a.stdf", &simple("A", 100, &[8, 0]));
    assert_eq!(step(&report(&a), 1)["status"], "missing");
    f.write("a.stdf", &simple("A", 100, &[8]));
    a.inputs.push(f.write("b.stdf", &simple("B", 200, &[0])));
    assert_eq!(step(&report(&a), 1)["status"], "tested");
}
#[test]
fn overlapping_or_missing_times_never_choose_final_or_stop() {
    for (t0, t1) in [(100, 105), (0, 200), (100, 100)] {
        let f = Fixture::new();
        let mut a = f.args(vec![
            f.write("a.stdf", &simple("A", t0, &[0])),
            f.write("b.stdf", &simple("A", t1, &[8])),
        ]);
        closed(&f, &mut a);
        let v = report(&a);
        assert_eq!(step(&v, 0)["order_unknown"], true);
        assert!(step(&v, 0)["latest_passed"].is_null());
        assert_eq!(step(&v, 0)["status"], "retest_different");
        assert_eq!(step(&v, 1)["status"], "indeterminate");
    }
}
#[test]
fn disjoint_runs_and_reversed_input_order_are_deterministic() {
    let f = Fixture::new();
    let mut a = f.args(vec![
        f.write("z.stdf", &simple("A", 100, &[8])),
        f.write("a.stdf", &simple("A", 200, &[0])),
    ]);
    let v = report(&a);
    let first = fs::read(&a.output).unwrap();
    a.inputs.reverse();
    assert_eq!(v, report(&a));
    assert_eq!(first, fs::read(&a.output).unwrap());
    assert_eq!(step(&v, 0)["latest_passed"], true);
    assert_eq!(step(&v, 0)["order_unknown"], false);
}
#[test]
fn duplicate_gzip_content_keeps_paths_without_retests() {
    let f = Fixture::new();
    let b = simple("A", 100, &[0]);
    let mut zip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    zip.write_all(&b).unwrap();
    f.write("copy.std.gz", &zip.finish().unwrap());
    f.write("original.stdf", &b);
    let a = f.args(vec![f.0.clone()]);
    let v = report(&a);
    assert_eq!(v["sources"].as_object().unwrap().len(), 1);
    assert_eq!(
        v["sources"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(step(&v, 0)["tests"], 1);
}
#[test]
fn interleaved_sites_and_repeated_part_ids_remain_independent() {
    let f = Fixture::new();
    let mut b = start("A", "bad!", 100);
    b.extend(pir(1));
    b.extend(pir(2));
    b.extend(ptr(2, "yCoordinate", 4.));
    b.extend(ptr(1, "X", 1.));
    b.extend(ptr(2, "coordinate_x", 3.));
    b.extend(ptr(1, "Y", 2.));
    b.extend(prr(2, 0, 0, 8, 1, 1));
    b.extend(prr(1, 0, 0, 0, 1, 1));
    b.extend(pir(1));
    b.extend(prr(1, 0, 0, 0, 1, 1));
    b.extend(pir(1));
    b.extend(prr(1, 0, 0, 0, 1, 1));
    finish(&mut b, 110);
    let a = f.args(vec![f.write("a.stdf", &b)]);
    let v = report(&a);
    let ds = v["devices"].as_array().unwrap();
    assert_eq!(ds.len(), 4);
    for (xy, pass) in [
        (json!(["lot-ptr", "LOT", 1, 2]), true),
        (json!(["lot-ptr", "LOT", 3, 4]), false),
    ] {
        let d = ds.iter().find(|d| d["identity"] == xy).unwrap();
        assert_eq!(d["steps"][0]["attempts"][0]["passed"], pass);
        assert_eq!(d["steps"][1]["status"], "indeterminate");
    }
}
#[test]
fn prr_without_ptr_and_mpr_ftr_only_are_preserved() {
    let f = Fixture::new();
    let mut b = start("A", "W1", 100);
    b.extend(prr(1, 1, 2, 0, 1, 1));
    for (sub, n) in [(15, 12), (20, 7)] {
        b.extend(pir(1));
        let mut body = vec![0; n];
        body[4] = 1;
        body[5] = 1;
        b.extend(rec(15, sub, &body));
        b.extend(prr(1, 1, 2, 0, 1, 1));
    }
    finish(&mut b, 110);
    let a = f.args(vec![f.write("a.stdf", &b)]);
    assert_eq!(step(&report(&a), 0)["tests"], 3);
}
#[test]
fn explicit_mapping_joins_namespaces_and_device_closure() {
    let f = Fixture::new();
    let mut b = start("B", "bad!", 200);
    b.extend(pir(1));
    b.extend(ptr(1, "X", 1.));
    b.extend(ptr(1, "Y", 2.));
    b.extend(prr(1, 0, 0, 0, 1, 1));
    finish(&mut b, 210);
    let mut a = f.args(vec![
        f.write("a.stdf", &simple("A", 100, &[0])),
        f.write("b.stdf", &b),
    ]);
    assert_eq!(report(&a)["devices"].as_array().unwrap().len(), 2);
    a.identity_map = Some(f.write(
        "identities.json",
        br#"[{"from":["lot-ptr","LOT",1,2],"to":["wafer-prr","W1",1,2]}]"#,
    ));
    a.flow_closures = Some(f.write(
        "closures.json",
        br#"[{"flow_id":"demo","version":"1","device":["lot-ptr","LOT",1,2]}]"#,
    ));
    let v = report(&a);
    assert_eq!(v["devices"].as_array().unwrap().len(), 1);
    assert_eq!(step(&v, 1)["status"], "tested");
    assert_eq!(step(&v, 2)["status"], "missing");
    assert!(device(&v)["issues"].as_array().unwrap().is_empty() == false);
}
#[test]
fn config_rejects_conflicting_maps_empty_selectors_and_wrong_closures() {
    let rows: Vec<Mapping> = serde_json::from_value(json!([
        {"from":["lot-ptr","LOT",1,2],"to":["wafer-prr","W1",1,2]},
        {"from":["lot-ptr","LOT",1,2],"to":["wafer-prr","W2",1,2]}]))
    .unwrap();
    assert!(config::mappings(&rows).is_err());
    let mut f = flow();
    f.steps[0].matches = vec![Selector::default()];
    assert!(f.validate().is_err());
    f = flow();
    f.steps[1].id = "A".into();
    assert!(f.validate().is_err());
    let c: Vec<Closure> =
        serde_json::from_value(json!([{"flow_id":"wrong","version":"1","lot":"LOT"}])).unwrap();
    assert!(config::validate_closures(&c, &flow()).is_err());
}
#[test]
fn ambiguous_and_unknown_step_prevent_missing_inference() {
    let f = Fixture::new();
    let mut a = f.args(vec![f.write("a.stdf", &simple("A", 100, &[0]))]);
    let mut config = flow();
    config.steps[1].matches = config.steps[0].matches.clone();
    fs::write(&a.flow_config, serde_json::to_vec(&config).unwrap()).unwrap();
    closed(&f, &mut a);
    let v = report(&a);
    assert_eq!(device(&v)["unidentified"].as_array().unwrap().len(), 1);
    assert_eq!(step(&v, 0)["status"], "indeterminate");
    fs::write(&a.flow_config, serde_json::to_vec(&flow()).unwrap()).unwrap();
    f.write("a.stdf", &simple("UNKNOWN", 100, &[0]));
    assert_eq!(step(&report(&a), 0)["status"], "indeterminate");
}
#[test]
fn selectors_are_exact_and_fields_anded_groups_ored() {
    let f = flow();
    let mut p = Selector {
        job_nam: Some("A2".into()),
        job_rev: Some("v2".into()),
        ..Default::default()
    };
    assert_eq!(f.match_step(&p), Some(0));
    p.job_rev = Some("v1".into());
    assert_eq!(f.match_step(&p), None);
    p.job_nam = Some("a".into());
    assert_eq!(f.match_step(&p), None);
}
#[test]
fn errors_limits_and_cancellation_preserve_existing_report() {
    let f = Fixture::new();
    let path = f.write("a.stdf", &simple("A", 100, &[0]));
    let mut a = f.args(vec![path.clone()]);
    f.write("trace.html", b"ORIGINAL");
    let check = |a: &Arguments| {
        assert!(generate(a, &mut Vec::new(), Arc::new(AtomicBool::new(false))).is_err());
        assert_eq!(fs::read(&a.output).unwrap(), b"ORIGINAL");
    };
    let mut b = start("A", "W1", 100);
    b.extend(pir(1));
    f.write("a.stdf", &b);
    check(&a);
    b = simple("A", 100, &[0]);
    b.push(0);
    f.write("a.stdf", &b);
    check(&a);
    b = start("A", "W1", 100);
    b.extend(rec(5, 20, &[1]));
    f.write("a.stdf", &b);
    check(&a);
    f.write("a.stdf", &simple("A", 100, &[0]));
    a.disk_limit_mib = 0;
    check(&a);
    a.disk_limit_mib = 8;
    a.cancel_file = Some(f.write("cancel", b""));
    check(&a);
    a.cancel_file = None;
    assert!(generate(&a, &mut Vec::new(), Arc::new(AtomicBool::new(true))).is_err());
    assert_eq!(fs::read(&a.output).unwrap(), b"ORIGINAL");
    a.max_devices = 1;
    let mut b = start("A", "W1", 100);
    b.extend(prr(1, 1, 2, 0, 1, 1));
    b.extend(prr(1, 2, 2, 0, 1, 1));
    finish(&mut b, 110);
    f.write("a.stdf", &b);
    check(&a);
    assert!(!fs::read_dir(&f.0).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".zstdf-analytics-")));
}
#[test]
fn device_group_limit_fails_without_truncating() {
    let f = Fixture::new();
    let b = simple("A", 100, &vec![0; 2000]);
    let a = f.args(vec![f.write("a.stdf", &b)]);
    f.write("trace.html", b"old");
    let e = generate(&a, &mut Vec::new(), Arc::new(AtomicBool::new(false))).unwrap_err();
    assert!(e.to_string().contains("group memory"), "{e}");
    assert_eq!(fs::read(&a.output).unwrap(), b"old");
}
#[test]
fn input_or_config_cannot_be_report_output() {
    let f = Fixture::new();
    let b = simple("A", 100, &[0]);
    let p = f.write("a.stdf", &b);
    let mut a = f.args(vec![p.clone()]);
    a.output = p.clone();
    assert!(generate(&a, &mut Vec::new(), Arc::new(AtomicBool::new(false))).is_err());
    assert_eq!(fs::read(p).unwrap(), b);
}
#[test]
fn html_escapes_script_injection_in_program_metadata() {
    let f = Fixture::new();
    let a = f.args(vec![f.write(
        "a.stdf",
        &simple("</script><script>alert(1)</script>", 100, &[0]),
    )]);
    let v = report(&a);
    assert_eq!(device(&v)["unidentified"].as_array().unwrap().len(), 1);
    let html = fs::read_to_string(&a.output).unwrap();
    assert!(!html.contains("<script>alert(1)"));
    assert!(html.contains("\\u003c/script\\u003e"));
}

#[test]
fn report_size_and_source_limits_preserve_old_output() {
    let f = Fixture::new();
    let mut b = start("A", "W1", 100);
    for x in 1..2500 {
        b.extend(prr(1, x, 2, 0, 1, 1));
    }
    finish(&mut b, 110);
    let mut a = f.args(vec![f.write("a.stdf", &b)]);
    a.max_devices = 3000;
    a.max_report_mib = 1;
    f.write("trace.html", b"old");
    let e = generate(&a, &mut Vec::new(), Arc::new(AtomicBool::new(false))).unwrap_err();
    assert!(e.to_string().contains("report size"), "{e}");
    assert_eq!(fs::read(&a.output).unwrap(), b"old");
    a.inputs.push(f.write("b.stdf", &simple("B", 200, &[0])));
    a.max_sources = 1;
    let e = generate(&a, &mut Vec::new(), Arc::new(AtomicBool::new(false))).unwrap_err();
    assert!(e.to_string().contains("source paths"), "{e}");
    assert_eq!(fs::read(&a.output).unwrap(), b"old");
}

#[test]
fn cancellation_after_staging_does_not_commit_or_leave_temp_output() {
    let f = Fixture::new();
    let p = f.write("trace.html", b"old");
    let result = stdf_parquet::catalog::atomic_write_checked(&p, b"new", || {
        Err(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "cancelled",
        ))
    });
    assert!(result.is_err());
    assert_eq!(fs::read(p).unwrap(), b"old");
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 1);
}

#[test]
fn missing_mrr_and_double_pir_are_rejected() {
    let f = Fixture::new();
    let mut b = start("A", "W1", 100);
    b.extend(prr(1, 1, 2, 0, 1, 1));
    let a = f.args(vec![f.write("a.stdf", &b)]);
    assert!(
        generate(&a, &mut Vec::new(), Arc::new(AtomicBool::new(false)))
            .unwrap_err()
            .to_string()
            .contains("missing MRR")
    );
    b.extend(pir(1));
    b.extend(pir(1));
    f.write("a.stdf", &b);
    assert!(
        generate(&a, &mut Vec::new(), Arc::new(AtomicBool::new(false)))
            .unwrap_err()
            .to_string()
            .contains("unclosed")
    );
}

#[test]
fn invalid_ptr_coordinates_conflicts_and_prr_priority_share_converter_rules() {
    for value in [0., -1., 1.5, f32::NAN, f32::INFINITY] {
        let f = Fixture::new();
        let mut b = start("A", "bad!", 100);
        b.extend(pir(1));
        b.extend(ptr(1, "X", value));
        b.extend(ptr(1, "Y", 2.));
        b.extend(prr(1, 0, 0, 0, 1, 1));
        finish(&mut b, 110);
        let a = f.args(vec![f.write("a.stdf", &b)]);
        assert!(device(&report(&a))["identity"].is_null());
    }
    for (wafer, x, y, expected) in [
        ("W1", 1, 2, json!(["wafer-prr", "W1", 1, 2])),
        ("bad!", 1, 2, Value::Null),
        ("W1", 0, 2, Value::Null),
        ("W1", 1, -1, Value::Null),
    ] {
        let f = Fixture::new();
        let mut b = start("A", wafer, 100);
        b.extend(pir(1));
        b.extend(ptr(1, "X", 3.));
        b.extend(ptr(1, "X", 4.));
        b.extend(ptr(1, "Y", 5.));
        b.extend(prr(1, x, y, 0, 1, 1));
        finish(&mut b, 110);
        let a = f.args(vec![f.write("a.stdf", &b)]);
        assert_eq!(device(&report(&a))["identity"], expected);
    }
}

#[test]
fn sdr_wafer_groups_and_separate_wafers_never_join_by_coordinates() {
    let f = Fixture::new();
    let mut b = start("A", "W1", 100);
    b.extend(rec(1, 80, &[1, 1, 1, 1]));
    b.extend(rec(1, 80, &[1, 2, 1, 2]));
    let mut wir = vec![1, 2];
    wir.extend(100u32.to_le_bytes());
    wir.extend(cn("W2"));
    b.extend(rec(2, 10, &wir));
    b.extend(pir(1));
    b.extend(pir(2));
    b.extend(prr(2, 1, 2, 0, 1, 1));
    b.extend(prr(1, 1, 2, 0, 1, 1));
    finish(&mut b, 110);
    let a = f.args(vec![f.write("a.stdf", &b)]);
    let v = report(&a);
    let ids: BTreeSet<_> = v["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["identity"][1].as_str().unwrap())
        .collect();
    assert_eq!(ids, BTreeSet::from(["W1", "W2"]));
}

#[test]
fn multiple_runs_within_source_keep_sequence_and_interleave_with_other_sources() {
    let f = Fixture::new();
    let mut b = simple("A", 100, &[8]);
    let second = simple("A", 300, &[0]);
    b.extend_from_slice(&second[6..]);
    let a = f.args(vec![
        f.write("a.stdf", &b),
        f.write("b.stdf", &simple("A", 200, &[8])),
    ]);
    let v = report(&a);
    assert_eq!(step(&v, 0)["order_unknown"], false);
    assert_eq!(step(&v, 0)["latest_passed"], true);
    let seqs: Vec<_> = step(&v, 0)["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["sequence"].as_u64().unwrap())
        .collect();
    assert_eq!(seqs, vec![1, 1, 2]);
}

#[test]
fn a_long_earlier_run_cannot_hide_cross_source_overlap() {
    let f = Fixture::new();
    let mut b = start("A", "W1", 100);
    b.extend(prr(1, 1, 2, 8, 1, 1));
    finish(&mut b, 300);
    let second = simple("A", 200, &[0]);
    b.extend_from_slice(&second[6..]);
    let a = f.args(vec![
        f.write("a.stdf", &b),
        f.write("b.stdf", &simple("A", 220, &[8])),
    ]);
    let v = report(&a);
    assert_eq!(step(&v, 0)["order_unknown"], true);
    assert!(step(&v, 0)["latest_passed"].is_null());
}
