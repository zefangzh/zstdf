use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use flate2::{write::GzEncoder, Compression};
use stdf_core::{RecordType, StdfError};

use crate::{IoError, StdfReader, StreamingRecordReader};

fn push_record(buf: &mut Vec<u8>, typ: u8, sub: u8, body: &[u8]) {
    buf.extend_from_slice(&(body.len() as u16).to_le_bytes());
    buf.push(typ);
    buf.push(sub);
    buf.extend_from_slice(body);
}

fn minimal_stdf() -> Vec<u8> {
    let mut data = Vec::new();
    push_record(&mut data, 0, 10, &[2, 4]);
    push_record(&mut data, 5, 10, &[1, 0]);
    data
}

fn temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("zstdf_io_hardening_{nanos}_{name}"))
}

#[test]
fn open_detects_gzip_by_magic_without_gz_extension() {
    let path = temp_path("magic_only.stdf");
    let file = std::fs::File::create(&path).unwrap();
    let mut encoder = GzEncoder::new(file, Compression::default());
    encoder.write_all(&minimal_stdf()).unwrap();
    encoder.finish().unwrap();

    let reader = StdfReader::open(&path).unwrap();

    assert_eq!(reader.records().filter_map(|record| record.ok()).count(), 2);
    std::fs::remove_file(path).ok();
}

#[test]
fn open_rejects_tiny_file() {
    let path = temp_path("tiny.stdf");
    std::fs::write(&path, [1, 2, 3]).unwrap();

    assert!(StdfReader::open(&path).is_err());
    std::fs::remove_file(path).ok();
}

#[test]
fn from_bytes_rejects_unsupported_version() {
    let mut data = Vec::new();
    push_record(&mut data, 0, 10, &[2, 3]);

    assert!(StdfReader::from_bytes(data).is_err());
}

#[test]
fn from_bytes_rejects_invalid_cpu_type() {
    let mut data = Vec::new();
    push_record(&mut data, 0, 10, &[9, 4]);

    assert!(StdfReader::from_bytes(data).is_err());
}

#[test]
fn reader_summarize_reports_trailing_junk() {
    let mut data = minimal_stdf();
    data.extend_from_slice(&[0xAA, 0xBB]);
    let reader = StdfReader::from_bytes(data).unwrap();

    let summary = reader.summarize();

    assert_eq!(summary.total_records, 2);
    assert!(summary.truncated);
}

#[test]
fn raw_records_returns_error_for_trailing_junk() {
    let mut data = minimal_stdf();
    data.push(0xCC);
    let reader = StdfReader::from_bytes(data).unwrap();
    let results = reader.raw_records().collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|item| item.is_ok()).count(), 2);
    assert_eq!(results.iter().filter(|item| item.is_err()).count(), 1);
}

#[test]
fn streaming_reader_rejects_invalid_cpu_type() {
    let mut data = Vec::new();
    push_record(&mut data, 0, 10, &[0, 4]);

    assert!(StreamingRecordReader::new(std::io::Cursor::new(data)).is_err());
}

#[test]
fn streaming_reader_reports_partial_trailing_header() {
    let mut data = minimal_stdf();
    data.extend_from_slice(&[1, 2]);
    let mut reader = StreamingRecordReader::new(std::io::Cursor::new(data)).unwrap();

    assert_eq!(
        reader.next().unwrap().unwrap().record_type(),
        RecordType::Far
    );
    assert_eq!(
        reader.next().unwrap().unwrap().record_type(),
        RecordType::Pir
    );
    assert!(matches!(
        reader.next(),
        Some(Err(IoError::Decode(StdfError::UnexpectedEof {
            position: 12,
            expected: 4,
        })))
    ));
}
