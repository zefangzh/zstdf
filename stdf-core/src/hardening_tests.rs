use crate::{
    decode_all, decode_with_summary, detect_byte_order, detect_byte_order_result, raw_records,
    records, summarize, ByteOrder, RawRecordIter, RecordIter, StdfError, StdfRecord,
};

fn push_record(buf: &mut Vec<u8>, typ: u8, sub: u8, body: &[u8]) {
    buf.extend_from_slice(&(body.len() as u16).to_le_bytes());
    buf.push(typ);
    buf.push(sub);
    buf.extend_from_slice(body);
}

fn far_le() -> Vec<u8> {
    let mut data = Vec::new();
    push_record(&mut data, 0, 10, &[2, 4]);
    data
}

fn ptr_body(test_num: u32, head: u8, site: u8, result: f32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&test_num.to_le_bytes());
    body.push(head);
    body.push(site);
    body.push(0);
    body.push(0);
    body.extend_from_slice(&result.to_le_bytes());
    body
}

fn prr_body(head: u8, site: u8) -> Vec<u8> {
    let mut body = Vec::new();
    body.push(head);
    body.push(site);
    body.push(0);
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    body
}

#[test]
fn rejects_little_endian_far_with_big_endian_cpu_type() {
    let mut data = Vec::new();
    push_record(&mut data, 0, 10, &[1, 4]);

    assert_eq!(detect_byte_order(&data), None);
    assert!(matches!(
        detect_byte_order_result(&data),
        Err(StdfError::InvalidField {
            record: "FAR",
            field: "CPU_TYPE",
            ..
        })
    ));
}

#[test]
fn rejects_big_endian_far_with_little_endian_cpu_type() {
    let data = [0, 2, 0, 10, 2, 4];

    assert_eq!(detect_byte_order(&data), None);
    assert!(matches!(
        detect_byte_order_result(&data),
        Err(StdfError::InvalidField {
            record: "FAR",
            field: "CPU_TYPE",
            ..
        })
    ));
}

#[test]
fn rejects_zero_cpu_type() {
    let mut data = Vec::new();
    push_record(&mut data, 0, 10, &[0, 4]);

    assert!(detect_byte_order_result(&data).is_err());
}

#[test]
fn rejects_vendor_cpu_type_without_guessing_byte_order() {
    let mut data = Vec::new();
    push_record(&mut data, 0, 10, &[9, 4]);

    assert_eq!(detect_byte_order(&data), None);
}

#[test]
fn decode_all_rejects_unsupported_version_without_panic() {
    let mut data = Vec::new();
    push_record(&mut data, 0, 10, &[2, 3]);

    let decoded = decode_with_summary(&data);

    assert!(matches!(
        decoded.error,
        Some(StdfError::UnsupportedVersion(3))
    ));
    assert!(decoded.records.is_empty());
}

#[test]
fn raw_records_rejects_short_prefix_lengths() {
    for len in 0..6 {
        let data = vec![0xAA; len];
        assert!(raw_records(&data).is_err(), "len {len} should be rejected");
    }
}

#[test]
fn records_api_rejects_invalid_far() {
    let data = [2, 0, 1, 10, 2, 4];

    assert!(records(&data).is_err());
}

#[test]
fn malformed_ptr_body_becomes_unknown_and_iteration_continues() {
    let mut data = far_le();
    push_record(&mut data, 15, 10, &[1, 2, 3]);
    push_record(&mut data, 1, 20, &1234u32.to_le_bytes());

    let records = RecordIter::new(&data, ByteOrder::LittleEndian)
        .collect::<crate::Result<Vec<_>>>()
        .unwrap();

    assert_eq!(records.len(), 3);
    assert!(matches!(
        records[1],
        StdfRecord::Unknown {
            typ: 15,
            sub: 10,
            ..
        }
    ));
    assert!(matches!(records[2], StdfRecord::Mrr(_)));
}

#[test]
fn raw_iter_large_declared_len_reports_eof() {
    let data = [0xFF, 0x7F, 99, 99, 1, 2, 3];
    let mut iter = RawRecordIter::new(&data, ByteOrder::LittleEndian);

    assert!(matches!(
        iter.next(),
        Some(Err(StdfError::UnexpectedEof {
            position: 4,
            expected: 32767,
        }))
    ));
}

#[test]
fn summary_bytes_consumed_stops_before_truncated_record() {
    let mut data = far_le();
    push_record(&mut data, 5, 10, &[1, 0]);
    data.extend_from_slice(&10u16.to_le_bytes());
    data.push(15);
    data.push(10);
    data.extend_from_slice(&[1, 2]);

    let summary = summarize(&data);

    assert_eq!(summary.total_records, 2);
    assert_eq!(summary.bytes_consumed, 12);
    assert!(summary.truncated);
}

#[test]
fn big_endian_ptr_flow_decodes() {
    let mut data = Vec::new();
    data.extend_from_slice(&2u16.to_be_bytes());
    data.push(0);
    data.push(10);
    data.push(1);
    data.push(4);

    data.extend_from_slice(&2u16.to_be_bytes());
    data.push(5);
    data.push(10);
    data.push(1);
    data.push(0);

    let mut ptr = Vec::new();
    ptr.extend_from_slice(&42u32.to_be_bytes());
    ptr.push(1);
    ptr.push(0);
    ptr.push(0);
    ptr.push(0);
    ptr.extend_from_slice(&6.5f32.to_be_bytes());
    data.extend_from_slice(&(ptr.len() as u16).to_be_bytes());
    data.push(15);
    data.push(10);
    data.extend_from_slice(&ptr);

    let (records, err) = decode_all(&data);

    assert!(err.is_none());
    assert!(matches!(records[2], StdfRecord::Ptr(_)));
}

#[test]
fn unknown_zero_length_record_is_preserved() {
    let mut data = far_le();
    push_record(&mut data, 99, 1, &[]);

    let (records, err) = decode_all(&data);

    assert!(err.is_none());
    assert!(matches!(
        &records[1],
        StdfRecord::Unknown { typ: 99, sub: 1, data } if data.is_empty()
    ));
}

#[test]
fn cn_length_overrun_record_becomes_unknown() {
    let mut data = far_le();
    push_record(&mut data, 50, 30, &[10, b'a', b'b']);

    let (records, err) = decode_all(&data);

    assert!(err.is_none());
    assert!(matches!(
        records[1],
        StdfRecord::Unknown {
            typ: 50,
            sub: 30,
            ..
        }
    ));
}

#[test]
fn second_far_with_empty_body_becomes_unknown() {
    let mut data = far_le();
    push_record(&mut data, 0, 10, &[]);

    let (records, err) = decode_all(&data);

    assert!(err.is_none());
    assert!(matches!(
        records[1],
        StdfRecord::Unknown {
            typ: 0,
            sub: 10,
            ..
        }
    ));
}

#[test]
fn decode_with_summary_invalid_far_has_error_but_not_truncated() {
    let decoded = decode_with_summary(&[0, 0, 99, 99, 2, 4]);

    assert!(decoded.error.is_some());
    assert!(!decoded.summary.truncated);
    assert_eq!(decoded.summary.total_records, 0);
}

#[test]
fn deterministic_noise_inputs_never_panic() {
    let mut state = 0x1234_5678u32;
    for len in 0..128 {
        let mut data = Vec::with_capacity(len);
        for _ in 0..len {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            data.push((state >> 24) as u8);
        }

        let _ = decode_all(&data);
        let _ = summarize(&data);
        let _ = decode_with_summary(&data);
    }
}

#[test]
fn raw_iterator_consumes_to_end_after_truncated_body_error() {
    let data = [8, 0, 15, 10, 1, 2, 3];
    let mut iter = RawRecordIter::new(&data, ByteOrder::LittleEndian);

    assert!(iter.next().unwrap().is_err());
    assert!(iter.next().is_none());
    assert_eq!(iter.position(), data.len());
}

#[test]
fn decode_summary_counts_raw_type_even_when_decode_falls_back_unknown() {
    let mut data = far_le();
    push_record(&mut data, 15, 10, &[1, 2, 3]);

    let decoded = decode_with_summary(&data);

    assert_eq!(decoded.records.len(), 2);
    assert_eq!(decoded.summary.total_records, 2);
    assert_eq!(decoded.summary.record_counts[&crate::RecordType::Ptr], 1);
    assert!(matches!(decoded.records[1], StdfRecord::Unknown { .. }));
}

#[test]
fn valid_flow_with_missing_prr_still_decodes_prior_ptr() {
    let mut data = far_le();
    push_record(&mut data, 5, 10, &[1, 0]);
    push_record(&mut data, 15, 10, &ptr_body(101, 1, 0, 9.0));

    let (records, err) = decode_all(&data);

    assert!(err.is_none());
    assert_eq!(records.len(), 3);
    assert!(matches!(records[2], StdfRecord::Ptr(_)));
}

#[test]
fn prr_with_extra_optional_coordinates_decodes() {
    let mut data = far_le();
    let mut prr = prr_body(1, 0);
    prr.extend_from_slice(&(-12i16).to_le_bytes());
    prr.extend_from_slice(&(34i16).to_le_bytes());
    push_record(&mut data, 5, 20, &prr);

    let (records, err) = decode_all(&data);

    assert!(err.is_none());
    match &records[1] {
        StdfRecord::Prr(prr) => {
            assert_eq!(prr.x_coord, Some(-12));
            assert_eq!(prr.y_coord, Some(34));
        }
        other => panic!("expected PRR, got {:?}", other.record_type()),
    }
}
