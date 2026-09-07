use super::*;

#[test]
fn catalog_cli_verifies_and_dashboard_keeps_source_identity() {
    let dir = tests::temp_path("catalog_dashboard");
    fs::create_dir(&dir).unwrap();
    let first = dir.join("one.stdf");
    let second = dir.join("two.stdf");
    let root = dir.join("out");
    let html = dir.join("dashboard.html");
    fs::write(&first, tests::build_test_stdf()).unwrap();
    fs::write(&second, tests::build_test_stdf()).unwrap();
    execute(
        Cli::try_parse_from([
            "zstdf-cli",
            "convert-partitioned",
            "--output-dir",
            root.to_str().unwrap(),
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ])
        .unwrap(),
        &mut Vec::new(),
    )
    .unwrap();
    execute(
        Cli::try_parse_from(["zstdf-cli", "verify-dataset", root.to_str().unwrap()]).unwrap(),
        &mut Vec::new(),
    )
    .unwrap();
    let mut out = Vec::new();
    execute(
        Cli::try_parse_from([
            "zstdf-cli",
            "dashboard-dir",
            root.to_str().unwrap(),
            html.to_str().unwrap(),
        ])
        .unwrap(),
        &mut out,
    )
    .unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("parts=2"));
    assert!(text.contains("rows=2"));
    assert!(fs::read_to_string(&html).unwrap().contains("lot-select"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn catalog_cli_partial_run_returns_failure_but_retains_good_sources() {
    let dir = tests::temp_path("catalog_partial");
    fs::create_dir(&dir).unwrap();
    let first = dir.join("one.stdf");
    let second = dir.join("missing.stdf");
    let root = dir.join("out");
    fs::write(&first, tests::build_test_stdf()).unwrap();
    let args = [
        "zstdf-cli",
        "convert-partitioned",
        "--continue-on-error",
        "--output-dir",
        root.to_str().unwrap(),
        second.to_str().unwrap(),
        first.to_str().unwrap(),
    ];
    assert!(execute(Cli::try_parse_from(args).unwrap(), &mut Vec::new()).is_err());
    let catalog = stdf_parquet::catalog::verify_catalog(&root).unwrap();
    assert_eq!(catalog.sources.len(), 1);
    assert_eq!(catalog.run.status, "partial");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn partitioned_cli_converts_and_reports_resource_metrics() {
    let dir = tests::temp_path("fragment_cli");
    fs::create_dir(&dir).unwrap();
    let input = dir.join("one.stdf");
    let output = dir.join("out");
    fs::write(&input, tests::build_test_stdf()).unwrap();
    let args = [
        "zstdf-cli",
        "convert-partitioned",
        "--output-dir",
        output.to_str().unwrap(),
        "--max-open-writers",
        "1",
        "--row-group-rows",
        "1",
        input.to_str().unwrap(),
    ];
    for _ in 0..2 {
        let mut out = Vec::new();
        execute(Cli::try_parse_from(args).unwrap(), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("rows=1"));
        assert!(text.contains("fragments=1"));
        assert!(text.contains("peak_pending_bytes="));
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn partitioned_cli_rejects_invalid_budget_before_output() {
    let dir = tests::temp_path("fragment_cli_budget");
    fs::create_dir(&dir).unwrap();
    let input = dir.join("one.stdf");
    let output = dir.join("out");
    fs::write(&input, tests::build_test_stdf()).unwrap();
    let cli = Cli::try_parse_from([
        "zstdf-cli",
        "convert-partitioned",
        "--output-dir",
        output.to_str().unwrap(),
        "--memory-limit-mib",
        "0",
        input.to_str().unwrap(),
    ])
    .unwrap();
    assert!(execute(cli, &mut Vec::new()).is_err());
    assert!(!output.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn convert_many_expands_directories_and_deduplicates_inputs() {
    let dir = tests::temp_path("multi_cli");
    fs::create_dir_all(dir.join("nested")).unwrap();
    let input = dir.join("one.stdf");
    fs::write(&input, tests::build_test_stdf()).unwrap();
    fs::write(dir.join("nested/two.STD"), tests::build_test_stdf()).unwrap();
    fs::write(dir.join("ignore.txt"), b"not STDF").unwrap();
    let output = dir.join("dataset");
    let mut out = Vec::new();
    execute(
        Cli::try_parse_from([
            "zstdf-cli",
            "convert-many",
            "--output-dir",
            output.to_str().unwrap(),
            "--partition-by",
            "lot-id,wafer-id",
            dir.to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .unwrap(),
        &mut out,
    )
    .unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("files=2"));
    assert!(text.contains("rows=2"));
    assert!(output
        .join("lot_id=__empty/wafer_id=__HIVE_DEFAULT_PARTITION__")
        .is_dir());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn convert_many_rejects_unknown_keys_and_missing_inputs() {
    assert!(Cli::try_parse_from([
        "zstdf-cli",
        "convert-many",
        "--output-dir",
        "out",
        "--partition-by",
        "invalid",
        "in.stdf"
    ])
    .is_err());
    assert!(Cli::try_parse_from(["zstdf-cli", "convert-many", "--output-dir", "out"]).is_err());
}

#[test]
fn convert_many_empty_directory_produces_no_output() {
    let dir = tests::temp_path("multi_empty");
    fs::create_dir(&dir).unwrap();
    let output = dir.join("out");
    let cli = Cli::try_parse_from([
        "zstdf-cli",
        "convert-many",
        "--output-dir",
        output.to_str().unwrap(),
        dir.to_str().unwrap(),
    ])
    .unwrap();
    assert!(execute(cli, &mut Vec::new()).is_err());
    assert!(!output.exists());
    fs::remove_dir(dir).unwrap();
}
