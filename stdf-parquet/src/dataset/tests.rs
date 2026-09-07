use super::*;
use crate::catalog::{convert_dataset, load_catalog, verify_catalog, ErrorPolicy};
use crate::AtomicWriteOptions;
use crate::{files_to_partitioned_fragments, FragmentOptions};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "zstdf_dataset_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn input(&self, name: &str, lot: &str, wafer: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, input_bytes(lot, wafer)).unwrap();
        path
    }
    fn convert(
        &self,
        paths: &[PathBuf],
        keys: &[PartitionKey],
    ) -> crate::Result<MultiFileConversionSummary> {
        files_to_partitioned_parquet_dir(
            paths,
            &self.0.join("out"),
            1,
            &AtomicWriteOptions::default(),
            keys,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

fn record(buf: &mut Vec<u8>, typ: u8, sub: u8, body: &[u8]) {
    buf.extend_from_slice(&(body.len() as u16).to_le_bytes());
    buf.extend_from_slice(&[typ, sub]);
    buf.extend_from_slice(body);
}
fn input_bytes(lot: &str, wafer: &str) -> Vec<u8> {
    let mut buf = vec![];
    record(&mut buf, 0, 10, &[2, 4]);
    let mut mir = vec![0; 15];
    for value in [lot, "DEVICE", "NODE", "TESTER", "JOB"] {
        mir.push(value.len() as u8);
        mir.extend_from_slice(value.as_bytes());
    }
    record(&mut buf, 1, 10, &mir);
    add_wafer(&mut buf, wafer);
    add_part(&mut buf);
    buf
}
fn add_wafer(buf: &mut Vec<u8>, wafer: &str) {
    let mut wir = vec![1, 255, 0, 0, 0, 0, wafer.len() as u8];
    wir.extend_from_slice(wafer.as_bytes());
    record(buf, 2, 10, &wir);
}
fn add_part(buf: &mut Vec<u8>) {
    record(buf, 5, 10, &[1, 0]);
    let mut ptr = vec![1, 0, 0, 0, 1, 0, 0, 0];
    ptr.extend_from_slice(&3.25f32.to_le_bytes());
    record(buf, 15, 10, &ptr);
    record(buf, 5, 20, &[1, 0, 0, 1, 0, 1, 0, 1, 0]);
}

#[test]
fn multiple_inputs_have_readable_outputs_and_totals() {
    let f = Fixture::new();
    let summary = f
        .convert(
            &[f.input("a.stdf", "L1", "W1"), f.input("b.stdf", "L2", "W2")],
            &[],
        )
        .unwrap();
    assert_eq!((summary.files, summary.rows, summary.batches), (2, 2, 2));
    for output in summary.outputs {
        let rows: usize =
            ParquetRecordBatchReaderBuilder::try_new(File::open(&output.output_path).unwrap())
                .unwrap()
                .build()
                .unwrap()
                .map(|b| b.unwrap().num_rows())
                .sum();
        assert_eq!(rows, 1);
        assert!(crate::manifest_path(&output.output_path).exists());
    }
}
#[test]
fn partitions_match_lot_and_wafer() {
    let f = Fixture::new();
    let summary = f
        .convert(
            &[f.input("a.stdf", "LOT42", "W01")],
            &[PartitionKey::LotId, PartitionKey::WaferId],
        )
        .unwrap();
    assert!(summary.outputs[0]
        .output_path
        .starts_with(f.0.join("out/lot_id=LOT42/wafer_id=W01")));
}
#[test]
fn duplicate_basenames_are_stable_across_order_and_subsets() {
    let f = Fixture::new();
    let a = f.input("a/same.stdf", "L", "W");
    let b = f.input("b/same.stdf", "L", "W");
    let first = f.convert(&[a.clone(), b.clone()], &[]).unwrap();
    let second = f.convert(&[b, a.clone()], &[]).unwrap();
    assert_ne!(first.outputs[0].output_path, first.outputs[1].output_path);
    assert_eq!(first, second);
    assert_eq!(f.convert(&[a], &[]).unwrap().outputs[0], first.outputs[0]);
}
#[test]
fn duplicate_input_is_converted_once() {
    let f = Fixture::new();
    let a = f.input("a.stdf", "L", "W");
    assert_eq!(f.convert(&[a.clone(), a], &[]).unwrap().files, 1);
}
#[test]
fn no_overwrite_reuses_unchanged_output() {
    let f = Fixture::new();
    let paths = [f.input("a.stdf", "L", "W")];
    let first = f.convert(&paths, &[]).unwrap();
    let path = &first.outputs[0].output_path;
    let before = fs::metadata(path).unwrap().modified().unwrap();
    let result = files_to_partitioned_parquet_dir(
        &paths,
        &f.0.join("out"),
        1,
        &AtomicWriteOptions {
            overwrite: false,
            ..Default::default()
        },
        &[],
    )
    .unwrap();
    assert_eq!(first, result);
    assert_eq!(before, fs::metadata(path).unwrap().modified().unwrap());
}
#[test]
fn changed_input_gets_a_new_output_identity() {
    let f = Fixture::new();
    let path = f.input("a.stdf", "L", "W");
    let first = f.convert(&[path.clone()], &[]).unwrap();
    let mut bytes = input_bytes("L", "W");
    add_part(&mut bytes);
    fs::write(&path, bytes).unwrap();
    let second = f.convert(&[path], &[]).unwrap();
    assert_ne!(first.outputs[0].output_path, second.outputs[0].output_path);
    assert_eq!(second.rows, 2);
}
#[test]
fn missing_input_fails_before_any_output() {
    let f = Fixture::new();
    assert!(f
        .convert(
            &[f.input("a.stdf", "L", "W"), f.0.join("missing.stdf")],
            &[]
        )
        .is_err());
    assert!(!f.0.join("out").exists());
}
#[test]
fn truncated_input_is_rejected_before_output() {
    let f = Fixture::new();
    let path = f.input("a.stdf", "L", "W");
    let mut bytes = input_bytes("L", "W");
    bytes.pop();
    fs::write(&path, bytes).unwrap();
    assert!(f.convert(&[path], &[]).is_err());
    assert!(!f.0.join("out").exists());
}
#[test]
fn mixed_wafer_file_requires_input_file_partitioning() {
    let f = Fixture::new();
    let path = f.input("a.stdf", "L", "W1");
    let mut bytes = input_bytes("L", "W1");
    add_wafer(&mut bytes, "W2");
    add_part(&mut bytes);
    fs::write(&path, bytes).unwrap();
    assert!(f
        .convert(&[path.clone()], &[PartitionKey::WaferId])
        .is_err());
    assert!(!f.0.join("out").exists());
    assert_eq!(
        f.convert(&[path], &[PartitionKey::InputFile]).unwrap().rows,
        2
    );
}
#[test]
fn unsafe_partition_values_cannot_escape_output_directory() {
    let f = Fixture::new();
    let summary = f
        .convert(
            &[f.input("a.stdf", "../lot\\x", "W/1")],
            &[PartitionKey::LotId, PartitionKey::WaferId],
        )
        .unwrap();
    assert!(summary.outputs[0]
        .output_path
        .starts_with(f.0.join("out/lot_id=%2E%2E%2F%6C%6F%74%5C%78/wafer_id=W%2F1")));
}
#[test]
fn empty_inputs_and_duplicate_partition_keys_are_rejected() {
    let f = Fixture::new();
    assert!(f.convert(&[], &[]).is_err());
    assert!(f
        .convert(
            &[f.input("a.stdf", "L", "W")],
            &[PartitionKey::LotId, PartitionKey::LotId]
        )
        .is_err());
    assert!(!f.0.join("out").exists());
}
#[test]
fn gzip_is_streamed_and_detected_by_magic() {
    use std::io::Write;
    let f = Fixture::new();
    let path = f.0.join("compressed.stdf");
    let mut encoder =
        flate2::write::GzEncoder::new(File::create(&path).unwrap(), flate2::Compression::default());
    encoder.write_all(&input_bytes("L", "W")).unwrap();
    encoder.finish().unwrap();
    assert_eq!(f.convert(&[path], &[PartitionKey::LotId]).unwrap().rows, 1);
}
#[test]
fn far_only_input_writes_empty_parquet_in_null_partition() {
    let f = Fixture::new();
    let path = f.0.join("empty.stdf");
    fs::write(&path, [2, 0, 0, 10, 2, 4]).unwrap();
    let summary = f.convert(&[path], &[PartitionKey::WaferId]).unwrap();
    assert_eq!(summary.rows, 0);
    assert!(summary.outputs[0]
        .output_path
        .starts_with(f.0.join("out/wafer_id=__HIVE_DEFAULT_PARTITION__")));
}

#[test]
fn existing_dataset_lock_blocks_writes_without_removing_lock() {
    let f = Fixture::new();
    let path = f.input("a.stdf", "L", "W");
    fs::create_dir(f.0.join("out")).unwrap();
    let lock = f.0.join("out/.zstdf-convert.lock");
    fs::write(&lock, "pid=other").unwrap();
    assert!(f
        .convert(&[path], &[])
        .unwrap_err()
        .to_string()
        .contains("cannot acquire"));
    assert_eq!(fs::read_to_string(lock).unwrap(), "pid=other");
}

#[test]
fn truncated_gzip_trailer_is_rejected() {
    use std::io::Write;
    let f = Fixture::new();
    let path = f.0.join("broken.stdf.gz");
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&input_bytes("L", "W")).unwrap();
    let mut bytes = encoder.finish().unwrap();
    bytes.truncate(bytes.len() - 4);
    fs::write(&path, bytes).unwrap();
    assert!(f.convert(&[path], &[]).is_err());
    assert!(!f.0.join("out").exists());
}

#[test]
fn corrupt_existing_parquet_is_not_reported_as_a_successful_retry() {
    let f = Fixture::new();
    let paths = [f.input("a.stdf", "L", "W")];
    let first = f.convert(&paths, &[]).unwrap();
    fs::write(&first.outputs[0].output_path, "damaged").unwrap();
    assert!(files_to_partitioned_parquet_dir(
        &paths,
        &f.0.join("out"),
        1,
        &AtomicWriteOptions {
            overwrite: false,
            ..Default::default()
        },
        &[]
    )
    .is_err());
    assert!(!f.0.join("out/.zstdf-convert.lock").exists());
}

#[test]
fn reserved_values_and_case_variants_have_distinct_windows_paths() {
    let values = [
        None,
        Some(""),
        Some("__empty"),
        Some("__EMPTY"),
        Some("__HIVE_DEFAULT_PARTITION__"),
        Some("LOT"),
        Some("lot"),
        Some("Lot"),
        Some("a/b"),
        Some("a%2Fb"),
        Some(".."),
    ];
    let encoded: BTreeSet<_> = values
        .into_iter()
        .map(|v| partition_value(v).unwrap().to_ascii_lowercase())
        .collect();
    assert_eq!(encoded.len(), values.len());
}

#[test]
fn source_changed_after_preflight_is_not_committed() {
    let f = Fixture::new();
    let path = f.input("a.stdf", "L", "W");
    let (records, digest) = open_records(&path).unwrap();
    for record in records {
        record.unwrap();
    }
    let expected = digest.get();
    let mut changed = input_bytes("L", "W");
    add_part(&mut changed);
    fs::write(&path, changed).unwrap();
    let output = f.0.join("changed.parquet");
    let result = crate::records_to_parquet_path_atomic(
        checked_records(&path, expected).unwrap(),
        &output,
        1,
        &AtomicWriteOptions::default(),
    );
    assert!(result.unwrap_err().to_string().contains("input changed"));
    assert!(!output.exists());
    assert!(!crate::manifest_path(&output).exists());
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 1);
}

#[test]
fn manifest_row_mismatch_is_not_accepted_on_retry() {
    let f = Fixture::new();
    let paths = [f.input("a.stdf", "L", "W")];
    let first = f.convert(&paths, &[]).unwrap();
    let manifest = crate::manifest_path(&first.outputs[0].output_path);
    let text = fs::read_to_string(&manifest)
        .unwrap()
        .replace("\"rows\": 1", "\"rows\": 999");
    fs::write(manifest, text).unwrap();
    assert!(files_to_partitioned_parquet_dir(
        &paths,
        &f.0.join("out"),
        1,
        &AtomicWriteOptions {
            overwrite: false,
            ..Default::default()
        },
        &[]
    )
    .is_err());
}

fn fragment_options() -> FragmentOptions {
    FragmentOptions {
        max_memory_bytes: 16 * 1024 * 1024,
        max_pending_tests: 1000,
        max_open_writers: 2,
        row_group_rows: 2,
        max_output_files: 100,
    }
}

#[test]
fn row_partitions_split_mixed_wafers_and_revisit_evicted_partitions() {
    let f = Fixture::new();
    let path = f.input("mixed.stdf", "L", "W0");
    let mut bytes = input_bytes("L", "W0");
    for wafer in ["W1", "W2", "W0", "W1", "W2"] {
        add_wafer(&mut bytes, wafer);
        add_part(&mut bytes);
    }
    fs::write(&path, bytes).unwrap();
    let summary = files_to_partitioned_fragments(
        &[path],
        &f.0.join("out"),
        &[PartitionKey::WaferId],
        &fragment_options(),
    )
    .unwrap();
    assert_eq!(summary.conversion.rows, 6);
    assert!(summary.peak_open_writers <= 2);
    assert_eq!(summary.conversion.outputs.len(), 6);
    for output in summary.conversion.outputs {
        let mut reader =
            ParquetRecordBatchReaderBuilder::try_new(File::open(&output.output_path).unwrap())
                .unwrap()
                .build()
                .unwrap();
        let batch = reader.next().unwrap().unwrap();
        let wafers = batch
            .column(1)
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap();
        assert!(output
            .output_path
            .to_string_lossy()
            .contains(&format!("wafer_id={}", wafers.value(0))));
    }
}

#[test]
fn fragment_row_groups_are_capped_and_retry_is_idempotent() {
    let f = Fixture::new();
    let path = f.input("many.stdf", "L", "W");
    let mut bytes = input_bytes("L", "W");
    for _ in 0..8 {
        add_part(&mut bytes);
    }
    fs::write(&path, bytes).unwrap();
    let first =
        files_to_partitioned_fragments(&[path.clone()], &f.0.join("out"), &[], &fragment_options())
            .unwrap();
    assert_eq!(first.conversion.rows, 9);
    assert_eq!(first.conversion.outputs.len(), 5);
    let second =
        files_to_partitioned_fragments(&[path], &f.0.join("out"), &[], &fragment_options())
            .unwrap();
    assert_eq!(first.conversion, second.conversion);
    for file in first.conversion.outputs {
        let builder =
            ParquetRecordBatchReaderBuilder::try_new(File::open(file.output_path).unwrap())
                .unwrap();
        assert!(builder
            .metadata()
            .row_groups()
            .iter()
            .all(|g| g.num_rows() <= 2));
    }
}

#[test]
fn fragment_limit_failure_leaves_no_partial_source() {
    let f = Fixture::new();
    let path = f.input("many.stdf", "L", "W");
    let mut bytes = input_bytes("L", "W");
    add_part(&mut bytes);
    add_part(&mut bytes);
    fs::write(&path, bytes).unwrap();
    let options = FragmentOptions {
        max_output_files: 1,
        row_group_rows: 1,
        ..fragment_options()
    };
    let error =
        files_to_partitioned_fragments(&[path], &f.0.join("out"), &[], &options).unwrap_err();
    assert!(error.to_string().contains("output files"));
    assert_eq!(fs::read_dir(f.0.join("out")).unwrap().count(), 0);
}

#[test]
fn fragment_preflight_rejects_incomplete_part_without_output() {
    let f = Fixture::new();
    let path = f.input("bad.stdf", "L", "W");
    let mut bytes = input_bytes("L", "W");
    record(&mut bytes, 5, 10, &[1, 0]);
    fs::write(&path, bytes).unwrap();
    assert!(
        files_to_partitioned_fragments(&[path], &f.0.join("out"), &[], &fragment_options())
            .unwrap_err()
            .to_string()
            .contains("incomplete")
    );
    assert!(!f.0.join("out").exists());
}

#[test]
fn fragment_options_reject_zero_and_unaffordable_limits() {
    let f = Fixture::new();
    let paths = [f.input("one.stdf", "L", "W")];
    for options in [
        FragmentOptions {
            max_open_writers: 0,
            ..fragment_options()
        },
        FragmentOptions {
            max_memory_bytes: 1,
            ..fragment_options()
        },
        FragmentOptions {
            row_group_rows: 0,
            ..fragment_options()
        },
    ] {
        assert!(files_to_partitioned_fragments(&paths, &f.0.join("out"), &[], &options).is_err());
    }
    assert!(!f.0.join("out").exists());
}

#[test]
fn fragment_retry_rejects_corruption() {
    let f = Fixture::new();
    let paths = [f.input("one.stdf", "L", "W")];
    let first =
        files_to_partitioned_fragments(&paths, &f.0.join("out"), &[], &fragment_options()).unwrap();
    fs::write(&first.conversion.outputs[0].output_path, b"broken").unwrap();
    assert!(
        files_to_partitioned_fragments(&paths, &f.0.join("out"), &[], &fragment_options()).is_err()
    );
}

#[test]
fn row_partition_values_match_unpartitioned_rows_for_mixed_lots() {
    let f = Fixture::new();
    let path = f.input("mixed.stdf", "L1", "W1");
    let mut bytes = input_bytes("L1", "W1");
    bytes.extend_from_slice(&input_bytes("L2", "W2")[6..]);
    fs::write(&path, &bytes).unwrap();
    fn values(batch: &arrow::record_batch::RecordBatch) -> Vec<(String, String, String, u32, u32)> {
        let string = |index, row| {
            batch
                .column(index)
                .as_any()
                .downcast_ref::<arrow::array::StringArray>()
                .unwrap()
                .value(row)
                .to_string()
        };
        (0..batch.num_rows())
            .map(|row| {
                (
                    string(0, row),
                    string(1, row),
                    string(2, row),
                    batch
                        .column(10)
                        .as_any()
                        .downcast_ref::<arrow::array::UInt32Array>()
                        .unwrap()
                        .value(row),
                    batch
                        .column(13)
                        .as_any()
                        .downcast_ref::<arrow::array::Float32Array>()
                        .unwrap()
                        .value(row)
                        .to_bits(),
                )
            })
            .collect()
    }
    let mut expected: Vec<_> =
        stdf_arrow::records_to_batches(stdf_core::records(&bytes).unwrap(), 10)
            .unwrap()
            .iter()
            .flat_map(values)
            .collect();
    let summary = files_to_partitioned_fragments(
        &[path],
        &f.0.join("out"),
        &[PartitionKey::LotId, PartitionKey::WaferId],
        &fragment_options(),
    )
    .unwrap();
    let mut actual = Vec::new();
    for output in summary.conversion.outputs {
        for batch in
            ParquetRecordBatchReaderBuilder::try_new(File::open(output.output_path).unwrap())
                .unwrap()
                .build()
                .unwrap()
        {
            actual.extend(values(&batch.unwrap()));
        }
    }
    expected.sort();
    actual.sort();
    assert_eq!(actual, expected);
}

#[test]
fn huge_mpr_hits_pending_limit_and_malformed_ptr_is_rejected() {
    let f = Fixture::new();
    let path = f.0.join("mpr.stdf");
    let mut bytes = vec![2, 0, 0, 10, 2, 4];
    let mut body = vec![1, 0, 0, 0, 1, 0, 0, 0, 0, 0];
    body.extend_from_slice(&2000u16.to_le_bytes());
    body.resize(body.len() + 2000 * 4, 0);
    record(&mut bytes, 15, 15, &body);
    fs::write(&path, bytes).unwrap();
    let error =
        files_to_partitioned_fragments(&[path.clone()], &f.0.join("out"), &[], &fragment_options())
            .unwrap_err();
    assert!(matches!(
        error,
        crate::StdfParquetError::Stdf(StdfError::ResourceLimit {
            resource: "pending tests",
            ..
        })
    ));
    let mut bytes = vec![2, 0, 0, 10, 2, 4];
    record(&mut bytes, 15, 10, &[0]);
    fs::write(&path, bytes).unwrap();
    assert!(
        files_to_partitioned_fragments(&[path], &f.0.join("out"), &[], &fragment_options())
            .unwrap_err()
            .to_string()
            .contains("malformed")
    );
    assert!(!f.0.join("out").exists());
}

#[test]
fn gzip_without_prr_fails_at_the_pending_limit() {
    use std::io::Write;
    let f = Fixture::new();
    let path = f.0.join("pending.stdf.gz");
    let mut bytes = vec![2, 0, 0, 10, 2, 4];
    let mut ptr = vec![1, 0, 0, 0, 1, 0, 0, 0];
    ptr.extend_from_slice(&1.0f32.to_le_bytes());
    for _ in 0..10_000 {
        record(&mut bytes, 15, 10, &ptr);
    }
    let mut encoder =
        flate2::write::GzEncoder::new(File::create(&path).unwrap(), flate2::Compression::default());
    encoder.write_all(&bytes).unwrap();
    encoder.finish().unwrap();
    let options = FragmentOptions {
        max_pending_tests: 32,
        ..fragment_options()
    };
    assert!(
        files_to_partitioned_fragments(&[path], &f.0.join("out"), &[], &options)
            .unwrap_err()
            .to_string()
            .contains("pending tests")
    );
    assert!(!f.0.join("out").exists());
}

#[test]
fn receipt_rejects_path_traversal_and_empty_source_retries() {
    let f = Fixture::new();
    let path = f.0.join("empty.stdf");
    fs::write(&path, [2, 0, 0, 10, 2, 4]).unwrap();
    for _ in 0..2 {
        assert_eq!(
            files_to_partitioned_fragments(
                &[path.clone()],
                &f.0.join("empty-out"),
                &[],
                &fragment_options()
            )
            .unwrap()
            .conversion
            .rows,
            0
        );
    }
    let path = f.input("one.stdf", "L", "W");
    let summary =
        files_to_partitioned_fragments(&[path.clone()], &f.0.join("out"), &[], &fragment_options())
            .unwrap();
    let receipt = summary.conversion.outputs[0]
        .output_path
        .parent()
        .unwrap()
        .join("_SUCCESS.json");
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
    json["fragments"][0]["path"] = serde_json::json!("../outside.parquet");
    fs::write(&receipt, serde_json::to_vec(&json).unwrap()).unwrap();
    assert!(
        files_to_partitioned_fragments(&[path], &f.0.join("out"), &[], &fragment_options())
            .unwrap_err()
            .to_string()
            .contains("unsafe")
    );
}

#[test]
fn concurrent_site_groups_on_one_head_route_to_their_own_wafers() {
    let f = Fixture::new();
    let path = f.0.join("groups.stdf");
    let mut bytes = vec![2, 0, 0, 10, 2, 4];
    record(&mut bytes, 1, 80, &[1, 1, 1, 0]);
    record(&mut bytes, 1, 80, &[1, 2, 1, 1]);
    record(&mut bytes, 2, 10, &[1, 1, 0, 0, 0, 0, 2, b'W', b'1']);
    record(&mut bytes, 2, 10, &[1, 2, 0, 0, 0, 0, 2, b'W', b'2']);
    for site in [0, 1] {
        record(&mut bytes, 5, 10, &[1, site]);
        let mut ptr = vec![1, 0, 0, 0, 1, site, 0, 0];
        ptr.extend_from_slice(&1.0f32.to_le_bytes());
        record(&mut bytes, 15, 10, &ptr);
    }
    for site in [1, 0] {
        record(&mut bytes, 5, 20, &[1, site, 0, 1, 0, 1, 0, 1, 0]);
    }
    fs::write(&path, bytes).unwrap();
    let result = files_to_partitioned_fragments(
        &[path],
        &f.0.join("out"),
        &[PartitionKey::WaferId],
        &fragment_options(),
    )
    .unwrap();
    assert_eq!(result.conversion.rows, 2);
    for output in result.conversion.outputs {
        let batch =
            ParquetRecordBatchReaderBuilder::try_new(File::open(output.output_path).unwrap())
                .unwrap()
                .build()
                .unwrap()
                .next()
                .unwrap()
                .unwrap();
        let site = batch
            .column(4)
            .as_any()
            .downcast_ref::<arrow::array::UInt8Array>()
            .unwrap()
            .value(0);
        let wafer = batch
            .column(1)
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .unwrap()
            .value(0);
        assert_eq!(wafer, if site == 0 { "W1" } else { "W2" });
    }
}

#[test]
fn catalog_selects_only_latest_successful_source_version() {
    let f = Fixture::new();
    let path = f.input("one.stdf", "L", "W");
    let root = f.0.join("catalog");
    convert_dataset(
        &[path.clone()],
        &root,
        &[],
        &fragment_options(),
        ErrorPolicy::FailFast,
    )
    .unwrap();
    let first = load_catalog(&root).unwrap();
    let old = first.sources.values().next().unwrap().fragments[0]
        .path
        .clone();
    let mut bytes = input_bytes("L", "W");
    add_part(&mut bytes);
    fs::write(&path, bytes).unwrap();
    convert_dataset(
        &[path],
        &root,
        &[],
        &fragment_options(),
        ErrorPolicy::FailFast,
    )
    .unwrap();
    let current = verify_catalog(&root).unwrap();
    assert_eq!(current.sources.len(), 1);
    assert_eq!(current.sources.values().next().unwrap().rows, 2);
    assert!(root.join(old).exists());
    assert!(current.revision > first.revision);
}

#[test]
fn catalog_records_partial_failure_and_preserves_previous_success() {
    let f = Fixture::new();
    let path = f.input("one.stdf", "L", "W");
    let root = f.0.join("out");
    convert_dataset(
        &[path.clone()],
        &root,
        &[],
        &fragment_options(),
        ErrorPolicy::FailFast,
    )
    .unwrap();
    fs::write(&path, b"broken").unwrap();
    let next = f.input("two.stdf", "L", "W");
    let run = convert_dataset(
        &[path, next],
        &root,
        &[],
        &fragment_options(),
        ErrorPolicy::Continue,
    )
    .unwrap();
    assert_eq!(run.failures.len(), 1);
    let catalog = verify_catalog(&root).unwrap();
    assert_eq!(catalog.sources.len(), 2);
    assert_eq!(catalog.run.status, "partial");
}

#[test]
fn catalog_detects_data_page_corruption_not_only_footer_damage() {
    let f = Fixture::new();
    let path = f.input("one.stdf", "L", "W");
    let root = f.0.join("out");
    convert_dataset(
        &[path],
        &root,
        &[],
        &fragment_options(),
        ErrorPolicy::FailFast,
    )
    .unwrap();
    let catalog = load_catalog(&root).unwrap();
    let output = root.join(&catalog.sources.values().next().unwrap().fragments[0].path);
    let mut bytes = fs::read(&output).unwrap();
    bytes[8] ^= 1;
    fs::write(output, bytes).unwrap();
    assert!(verify_catalog(&root)
        .unwrap_err()
        .to_string()
        .contains("SHA-256"));
}

#[test]
fn catalog_changed_options_select_a_new_generation_without_double_counting() {
    let f = Fixture::new();
    let path = f.input("one.stdf", "L", "W");
    let root = f.0.join("out");
    convert_dataset(
        &[path.clone()],
        &root,
        &[],
        &fragment_options(),
        ErrorPolicy::FailFast,
    )
    .unwrap();
    let before = load_catalog(&root)
        .unwrap()
        .sources
        .values()
        .next()
        .unwrap()
        .options_sha256
        .clone();
    let options = FragmentOptions {
        row_group_rows: 1,
        ..fragment_options()
    };
    convert_dataset(&[path], &root, &[], &options, ErrorPolicy::FailFast).unwrap();
    let catalog = verify_catalog(&root).unwrap();
    assert_eq!(catalog.sources.len(), 1);
    assert_ne!(
        before,
        catalog.sources.values().next().unwrap().options_sha256
    );
}
