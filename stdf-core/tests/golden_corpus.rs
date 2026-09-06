use stdf_core::{
    decode_all, decode_with_summary, detect_byte_order_result, summarize, ByteOrder, RecordType,
    StdfRecord,
};

struct GoldenFixture {
    name: &'static str,
    hex: &'static str,
    expected_records: usize,
    expected_error: bool,
    expected_byte_order: ByteOrder,
}

const FIXTURES: &[GoldenFixture] = &[
    GoldenFixture {
        name: "minimal_le",
        hex: include_str!("../../fixtures/stdf/golden/minimal_le.hex"),
        expected_records: 1,
        expected_error: false,
        expected_byte_order: ByteOrder::LittleEndian,
    },
    GoldenFixture {
        name: "minimal_be",
        hex: include_str!("../../fixtures/stdf/golden/minimal_be.hex"),
        expected_records: 1,
        expected_error: false,
        expected_byte_order: ByteOrder::BigEndian,
    },
    GoldenFixture {
        name: "ptr_prr_flow",
        hex: include_str!("../../fixtures/stdf/golden/ptr_prr_flow.hex"),
        expected_records: 4,
        expected_error: false,
        expected_byte_order: ByteOrder::LittleEndian,
    },
    GoldenFixture {
        name: "malformed_truncated",
        hex: include_str!("../../fixtures/stdf/golden/malformed_truncated.hex"),
        expected_records: 1,
        expected_error: true,
        expected_byte_order: ByteOrder::LittleEndian,
    },
    GoldenFixture {
        name: "unknown_vendor",
        hex: include_str!("../../fixtures/stdf/golden/unknown_vendor.hex"),
        expected_records: 2,
        expected_error: false,
        expected_byte_order: ByteOrder::LittleEndian,
    },
];

#[test]
fn golden_fixtures_match_expected_decode_contract() {
    for fixture in FIXTURES {
        let data = parse_hex_fixture(fixture.hex);
        let decoded = decode_with_summary(&data);

        assert_eq!(
            decoded.summary.byte_order,
            Some(fixture.expected_byte_order),
            "{} byte order",
            fixture.name
        );
        assert_eq!(
            decoded.records.len(),
            fixture.expected_records,
            "{} record count",
            fixture.name
        );
        assert_eq!(
            decoded.error.is_some(),
            fixture.expected_error,
            "{} error state",
            fixture.name
        );
    }
}

#[test]
fn ptr_prr_flow_fixture_decodes_expected_record_types() {
    let data = parse_hex_fixture(include_str!("../../fixtures/stdf/golden/ptr_prr_flow.hex"));
    let (records, err) = decode_all(&data);

    assert!(err.is_none());
    assert!(matches!(records[0], StdfRecord::Far(_)));
    assert!(matches!(records[1], StdfRecord::Pir(_)));
    assert!(matches!(records[2], StdfRecord::Ptr(_)));
    assert!(matches!(records[3], StdfRecord::Prr(_)));
}

#[test]
fn unknown_vendor_fixture_preserves_raw_payload() {
    let data = parse_hex_fixture(include_str!(
        "../../fixtures/stdf/golden/unknown_vendor.hex"
    ));
    let (records, err) = decode_all(&data);

    assert!(err.is_none());
    assert!(matches!(
        &records[1],
        StdfRecord::Unknown {
            typ: 99,
            sub: 77,
            data,
        } if data == b"ABC"
    ));
}

#[test]
fn malformed_fixture_reports_truncation_without_losing_prior_records() {
    let data = parse_hex_fixture(include_str!(
        "../../fixtures/stdf/golden/malformed_truncated.hex"
    ));
    let decoded = decode_with_summary(&data);

    assert_eq!(decoded.records.len(), 1);
    assert!(decoded.error.is_some());
    assert!(decoded.summary.truncated);
    assert_eq!(decoded.summary.record_counts[&RecordType::Far], 1);
}

#[test]
fn deterministic_fuzz_corpus_runs_at_least_1500_cases_without_panics() {
    let seeds: Vec<Vec<u8>> = FIXTURES
        .iter()
        .map(|fixture| parse_hex_fixture(fixture.hex))
        .collect();
    let mut state = 0xA5A5_1234u32;

    for case_id in 0..1500 {
        let seed = &seeds[case_id % seeds.len()];
        let mut data = seed.clone();
        state = mutate_case(&mut data, state, case_id);

        let _ = detect_byte_order_result(&data);
        let _ = decode_all(&data);
        let _ = decode_with_summary(&data);
        let _ = summarize(&data);
    }
}

fn mutate_case(data: &mut Vec<u8>, mut state: u32, case_id: usize) -> u32 {
    state = state
        .wrapping_mul(1_664_525)
        .wrapping_add(1_013_904_223)
        .wrapping_add(case_id as u32);
    match case_id % 8 {
        0 if !data.is_empty() => data.truncate((state as usize) % data.len()),
        1 => data.extend_from_slice(&state.to_le_bytes()),
        2 if !data.is_empty() => {
            let index = (state as usize) % data.len();
            data[index] ^= (state >> 24) as u8;
        }
        3 if data.len() >= 2 => {
            data[0] = (state >> 8) as u8;
            data[1] = state as u8;
        }
        4 => data.clear(),
        5 if data.len() > 4 => data[4] = (state % 255) as u8,
        6 => {
            data.splice(0..0, [0xFF, 0x00, 0xAA, 0x55]);
        }
        _ => data.reverse(),
    }
    state
}

fn parse_hex_fixture(input: &str) -> Vec<u8> {
    input
        .lines()
        .flat_map(|line| line.split('#').next().unwrap_or("").split_whitespace())
        .map(|token| u8::from_str_radix(token, 16).expect("fixture contains valid hex byte"))
        .collect()
}
