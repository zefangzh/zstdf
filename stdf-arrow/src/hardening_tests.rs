use arrow::array::{Array, BooleanArray, StringArray, UInt32Array};
use stdf_core::records::{Mir, Pir, Prr, Ptr, Wir};
use stdf_core::{StdfError, StdfRecord};

use crate::schema::{PART_ID, PART_PASS, TEST_NUM, TEST_PASS, WAFER_ID};
use crate::{records_to_batches, BatchBuilder};

#[test]
fn interleaved_multisite_parts_keep_tests_with_correct_sites() {
    let records = vec![
        Ok(StdfRecord::Mir(mir("LOT_MULTI"))),
        Ok(StdfRecord::Pir(pir(1, 0))),
        Ok(StdfRecord::Pir(pir(1, 1))),
        Ok(StdfRecord::Ptr(ptr(100, 1, 0, 1.0, 0))),
        Ok(StdfRecord::Ptr(ptr(200, 1, 1, 2.0, 0))),
        Ok(StdfRecord::Prr(prr(1, 1, Some("SITE1"), 0))),
        Ok(StdfRecord::Prr(prr(1, 0, Some("SITE0"), 0))),
    ];

    let batches = records_to_batches(records, 10).unwrap();
    let batch = &batches[0];

    assert_eq!(batch.num_rows(), 2);
    assert_eq!(as_string(batch, PART_ID).value(0), "SITE1");
    assert_eq!(as_u32(batch, TEST_NUM).value(0), 200);
    assert_eq!(as_string(batch, PART_ID).value(1), "SITE0");
    assert_eq!(as_u32(batch, TEST_NUM).value(1), 100);
}

#[test]
fn ptr_without_pir_synthesizes_stable_part_id() {
    let records = vec![
        Ok(StdfRecord::Mir(mir("LOT_SYNTH"))),
        Ok(StdfRecord::Ptr(ptr(300, 1, 0, 3.0, 0))),
        Ok(StdfRecord::Prr(prr(1, 0, None, 0))),
    ];

    let batches = records_to_batches(records, 10).unwrap();

    assert_eq!(as_string(&batches[0], PART_ID).value(0), "H1_S0_P1");
}

#[test]
fn prr_without_tests_finishes_zero_row_batch() {
    let mut builder = BatchBuilder::new(10);
    builder.push_record(&StdfRecord::Mir(mir("LOT_EMPTY_PART")));
    builder.push_record(&StdfRecord::Prr(prr(1, 0, Some("P0"), 0)));

    let batch = builder.finish().unwrap();

    assert_eq!(batch.num_rows(), 0);
    assert_eq!(batch.num_columns(), 19);
}

#[test]
fn wafer_id_updates_between_completed_parts() {
    let records = vec![
        Ok(StdfRecord::Mir(mir("LOT_WAFER"))),
        Ok(StdfRecord::Wir(wir("W1"))),
        Ok(StdfRecord::Pir(pir(1, 0))),
        Ok(StdfRecord::Ptr(ptr(401, 1, 0, 4.0, 0))),
        Ok(StdfRecord::Prr(prr(1, 0, Some("P1"), 0))),
        Ok(StdfRecord::Wir(wir("W2"))),
        Ok(StdfRecord::Pir(pir(1, 0))),
        Ok(StdfRecord::Ptr(ptr(402, 1, 0, 5.0, 0))),
        Ok(StdfRecord::Prr(prr(1, 0, Some("P2"), 0))),
    ];

    let batches = records_to_batches(records, 10).unwrap();
    let wafer_id = as_string(&batches[0], WAFER_ID);

    assert_eq!(wafer_id.value(0), "W1");
    assert_eq!(wafer_id.value(1), "W2");
}

#[test]
fn failing_part_flag_propagates_to_part_pass_column() {
    let records = vec![
        Ok(StdfRecord::Mir(mir("LOT_FAIL"))),
        Ok(StdfRecord::Pir(pir(1, 0))),
        Ok(StdfRecord::Ptr(ptr(501, 1, 0, 5.0, 0))),
        Ok(StdfRecord::Prr(prr(1, 0, Some("PF"), 0x08))),
    ];

    let batches = records_to_batches(records, 10).unwrap();

    assert!(!as_bool(&batches[0], PART_PASS).value(0));
}

#[test]
fn zero_batch_size_is_treated_as_one() {
    let records = vec![
        Ok(StdfRecord::Mir(mir("LOT_ZERO_BATCH"))),
        Ok(StdfRecord::Pir(pir(1, 0))),
        Ok(StdfRecord::Ptr(ptr(601, 1, 0, 6.0, 0))),
        Ok(StdfRecord::Prr(prr(1, 0, Some("P1"), 0))),
        Ok(StdfRecord::Pir(pir(1, 0))),
        Ok(StdfRecord::Ptr(ptr(602, 1, 0, 7.0, 0))),
        Ok(StdfRecord::Prr(prr(1, 0, Some("P2"), 0))),
    ];

    let batches = records_to_batches(records, 0).unwrap();

    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].num_rows(), 1);
    assert_eq!(batches[1].num_rows(), 1);
}

#[test]
fn invalid_test_pass_flag_maps_to_null() {
    let records = vec![
        Ok(StdfRecord::Mir(mir("LOT_FLAG"))),
        Ok(StdfRecord::Pir(pir(1, 0))),
        Ok(StdfRecord::Ptr(ptr(701, 1, 0, 7.0, 0x40))),
        Ok(StdfRecord::Prr(prr(1, 0, Some("PFLAG"), 0))),
    ];

    let batches = records_to_batches(records, 10).unwrap();

    assert!(as_bool(&batches[0], TEST_PASS).is_null(0));
}

#[test]
fn records_to_batches_propagates_decode_errors() {
    let result = records_to_batches(vec![Err(StdfError::UnsupportedVersion(3))], 10);

    assert!(matches!(result, Err(StdfError::UnsupportedVersion(3))));
}

#[test]
fn many_ptrs_for_one_part_remain_in_order() {
    let mut records = vec![
        Ok(StdfRecord::Mir(mir("LOT_MANY"))),
        Ok(StdfRecord::Pir(pir(1, 0))),
    ];
    for test_num in 800..850 {
        records.push(Ok(StdfRecord::Ptr(ptr(test_num, 1, 0, test_num as f32, 0))));
    }
    records.push(Ok(StdfRecord::Prr(prr(1, 0, Some("PMANY"), 0))));

    let batches = records_to_batches(records, 100).unwrap();
    let test_num = as_u32(&batches[0], TEST_NUM);

    assert_eq!(batches[0].num_rows(), 50);
    assert_eq!(test_num.value(0), 800);
    assert_eq!(test_num.value(49), 849);
}

fn as_string(batch: &arrow::record_batch::RecordBatch, index: usize) -> &StringArray {
    batch
        .column(index)
        .as_any()
        .downcast_ref::<StringArray>()
        .unwrap()
}

fn as_u32(batch: &arrow::record_batch::RecordBatch, index: usize) -> &UInt32Array {
    batch
        .column(index)
        .as_any()
        .downcast_ref::<UInt32Array>()
        .unwrap()
}

fn as_bool(batch: &arrow::record_batch::RecordBatch, index: usize) -> &BooleanArray {
    batch
        .column(index)
        .as_any()
        .downcast_ref::<BooleanArray>()
        .unwrap()
}

fn mir(lot_id: &str) -> Mir {
    Mir {
        setup_t: 0,
        start_t: 0,
        stat_num: 0,
        mode_cod: b'P',
        rtst_cod: b' ',
        prot_cod: b' ',
        burn_tim: 0,
        cmod_cod: b' ',
        lot_id: lot_id.to_string(),
        part_typ: "DEVICE".to_string(),
        node_nam: "NODE".to_string(),
        tstr_typ: "T".to_string(),
        job_nam: "JOB".to_string(),
        job_rev: None,
        sblot_id: None,
        oper_nam: None,
        exec_typ: None,
        exec_ver: None,
        test_cod: None,
        tst_temp: None,
        user_txt: None,
        aux_file: None,
        pkg_typ: None,
        famly_id: None,
        date_cod: None,
        facil_id: None,
        floor_id: None,
        proc_id: None,
        oper_frq: None,
        spec_nam: None,
        spec_ver: None,
        flow_id: None,
        setup_id: None,
        dsgn_rev: None,
        eng_id: None,
        rom_cod: None,
        serl_num: None,
        supr_nam: None,
    }
}

fn wir(wafer_id: &str) -> Wir {
    Wir {
        head_num: 1,
        site_grp: None,
        start_t: None,
        wafer_id: Some(wafer_id.to_string()),
    }
}

fn pir(head_num: u8, site_num: u8) -> Pir {
    Pir { head_num, site_num }
}

fn ptr(test_num: u32, head_num: u8, site_num: u8, result: f32, test_flg: u8) -> Ptr {
    Ptr {
        test_num,
        head_num,
        site_num,
        test_flg,
        parm_flg: 0,
        result,
        test_txt: Some(format!("T{test_num}")),
        alarm_id: None,
        opt_flag: None,
        res_scal: None,
        llm_scal: None,
        hlm_scal: None,
        lo_limit: None,
        hi_limit: None,
        units: None,
        c_resfmt: None,
        c_llmfmt: None,
        c_hlmfmt: None,
        lo_spec: None,
        hi_spec: None,
    }
}

fn prr(head_num: u8, site_num: u8, part_id: Option<&str>, part_flg: u8) -> Prr {
    Prr {
        head_num,
        site_num,
        part_flg,
        num_test: 1,
        hard_bin: 1,
        soft_bin: 1,
        x_coord: None,
        y_coord: None,
        test_t: None,
        part_id: part_id.map(str::to_string),
        part_txt: None,
        part_fix: None,
    }
}
