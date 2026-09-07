use std::sync::Arc;

use arrow::array::{
    ArrayRef, BooleanBuilder, Float32Builder, Int16Builder, StringBuilder, UInt16Builder,
    UInt32Builder, UInt8Builder,
};
use arrow::record_batch::RecordBatch;

use crate::context::{PartResult, StdfContext};
use crate::schema::eav_schema;
use stdf_core::StdfRecord;

#[derive(Debug)]
pub struct BatchBuilder {
    context: StdfContext,
    completed_parts: Vec<PartResult>,
    batch_size: usize,
}

impl BatchBuilder {
    pub fn new(batch_size: usize) -> Self {
        Self {
            context: StdfContext::new(),
            completed_parts: Vec::new(),
            batch_size: batch_size.max(1),
        }
    }

    pub fn push_record(&mut self, record: &StdfRecord) -> Option<RecordBatch> {
        if let Some(part) = self.context.push_record(record) {
            self.completed_parts.push(part);
        }

        (self.completed_rows() >= self.batch_size).then(|| self.drain_completed_parts())
    }

    pub fn finish(mut self) -> Option<RecordBatch> {
        (!self.completed_parts.is_empty()).then(|| self.drain_completed_parts())
    }

    fn completed_rows(&self) -> usize {
        self.completed_parts
            .iter()
            .map(|part| part.tests.len())
            .sum()
    }

    fn drain_completed_parts(&mut self) -> RecordBatch {
        let parts = std::mem::take(&mut self.completed_parts);
        build_batch(&parts)
    }
}

pub(crate) fn build_batch(parts: &[PartResult]) -> RecordBatch {
    let rows = parts.iter().map(|part| part.tests.len()).sum();
    let mut lot_id = StringBuilder::with_capacity(rows, rows * 8);
    let mut wafer_id = StringBuilder::with_capacity(rows, rows * 8);
    let mut part_id = StringBuilder::with_capacity(rows, rows * 8);
    let mut head_num = UInt8Builder::with_capacity(rows);
    let mut site_num = UInt8Builder::with_capacity(rows);
    let mut x_coord = Int16Builder::with_capacity(rows);
    let mut y_coord = Int16Builder::with_capacity(rows);
    let mut hard_bin = UInt16Builder::with_capacity(rows);
    let mut soft_bin = UInt16Builder::with_capacity(rows);
    let mut part_pass = BooleanBuilder::with_capacity(rows);
    let mut test_num = UInt32Builder::with_capacity(rows);
    let mut test_txt = StringBuilder::with_capacity(rows, rows * 12);
    let mut test_type = StringBuilder::with_capacity(rows, rows * 3);
    let mut result = Float32Builder::with_capacity(rows);
    let mut test_pass = BooleanBuilder::with_capacity(rows);
    let mut lo_limit = Float32Builder::with_capacity(rows);
    let mut hi_limit = Float32Builder::with_capacity(rows);
    let mut units = StringBuilder::with_capacity(rows, rows * 4);
    let mut test_time_ms = UInt32Builder::with_capacity(rows);

    for part in parts {
        for test in &part.tests {
            lot_id.append_value(&part.lot_id);
            append_opt_str(&mut wafer_id, part.wafer_id.as_deref());
            part_id.append_value(&part.part_id);
            head_num.append_value(part.head_num);
            site_num.append_value(part.site_num);
            append_opt_i16(&mut x_coord, part.x_coord);
            append_opt_i16(&mut y_coord, part.y_coord);
            hard_bin.append_value(part.hard_bin);
            soft_bin.append_value(part.soft_bin);
            part_pass.append_value(part.part_pass);
            test_num.append_value(test.test_num);
            append_opt_str(&mut test_txt, test.test_txt.as_deref());
            test_type.append_value(&test.test_type);
            append_opt_f32(&mut result, test.result);
            append_opt_bool(&mut test_pass, test.test_pass);
            append_opt_f32(&mut lo_limit, test.lo_limit);
            append_opt_f32(&mut hi_limit, test.hi_limit);
            append_opt_str(&mut units, test.units.as_deref());
            append_opt_u32(&mut test_time_ms, part.test_time_ms);
        }
    }

    RecordBatch::try_new(
        eav_schema(),
        vec![
            Arc::new(lot_id.finish()) as ArrayRef,
            Arc::new(wafer_id.finish()) as ArrayRef,
            Arc::new(part_id.finish()) as ArrayRef,
            Arc::new(head_num.finish()) as ArrayRef,
            Arc::new(site_num.finish()) as ArrayRef,
            Arc::new(x_coord.finish()) as ArrayRef,
            Arc::new(y_coord.finish()) as ArrayRef,
            Arc::new(hard_bin.finish()) as ArrayRef,
            Arc::new(soft_bin.finish()) as ArrayRef,
            Arc::new(part_pass.finish()) as ArrayRef,
            Arc::new(test_num.finish()) as ArrayRef,
            Arc::new(test_txt.finish()) as ArrayRef,
            Arc::new(test_type.finish()) as ArrayRef,
            Arc::new(result.finish()) as ArrayRef,
            Arc::new(test_pass.finish()) as ArrayRef,
            Arc::new(lo_limit.finish()) as ArrayRef,
            Arc::new(hi_limit.finish()) as ArrayRef,
            Arc::new(units.finish()) as ArrayRef,
            Arc::new(test_time_ms.finish()) as ArrayRef,
        ],
    )
    .expect("EAV builders must match the declared schema")
}

fn append_opt_str(builder: &mut StringBuilder, value: Option<&str>) {
    if let Some(value) = value {
        builder.append_value(value);
    } else {
        builder.append_null();
    }
}

fn append_opt_bool(builder: &mut BooleanBuilder, value: Option<bool>) {
    if let Some(value) = value {
        builder.append_value(value);
    } else {
        builder.append_null();
    }
}

fn append_opt_f32(builder: &mut Float32Builder, value: Option<f32>) {
    if let Some(value) = value {
        builder.append_value(value);
    } else {
        builder.append_null();
    }
}

fn append_opt_i16(builder: &mut Int16Builder, value: Option<i16>) {
    if let Some(value) = value {
        builder.append_value(value);
    } else {
        builder.append_null();
    }
}

fn append_opt_u32(builder: &mut UInt32Builder, value: Option<u32>) {
    if let Some(value) = value {
        builder.append_value(value);
    } else {
        builder.append_null();
    }
}
