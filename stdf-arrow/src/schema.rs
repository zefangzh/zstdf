use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

pub const LOT_ID: usize = 0;
pub const WAFER_ID: usize = 1;
pub const PART_ID: usize = 2;
pub const HEAD_NUM: usize = 3;
pub const SITE_NUM: usize = 4;
pub const X_COORD: usize = 5;
pub const Y_COORD: usize = 6;
pub const HARD_BIN: usize = 7;
pub const SOFT_BIN: usize = 8;
pub const PART_PASS: usize = 9;
pub const TEST_NUM: usize = 10;
pub const TEST_TXT: usize = 11;
pub const TEST_TYPE: usize = 12;
pub const RESULT: usize = 13;
pub const TEST_PASS: usize = 14;
pub const LO_LIMIT: usize = 15;
pub const HI_LIMIT: usize = 16;
pub const UNITS: usize = 17;
pub const TEST_TIME_MS: usize = 18;

/// Long-format Arrow schema: one row per test result with part context repeated.
pub fn eav_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("lot_id", DataType::Utf8, false),
        Field::new("wafer_id", DataType::Utf8, true),
        Field::new("part_id", DataType::Utf8, false),
        Field::new("head_num", DataType::UInt8, false),
        Field::new("site_num", DataType::UInt8, false),
        Field::new("x_coord", DataType::Int16, true),
        Field::new("y_coord", DataType::Int16, true),
        Field::new("hard_bin", DataType::UInt16, false),
        Field::new("soft_bin", DataType::UInt16, false),
        Field::new("part_pass", DataType::Boolean, false),
        Field::new("test_num", DataType::UInt32, false),
        Field::new("test_txt", DataType::Utf8, true),
        Field::new("test_type", DataType::Utf8, false),
        Field::new("result", DataType::Float32, true),
        Field::new("test_pass", DataType::Boolean, true),
        Field::new("lo_limit", DataType::Float32, true),
        Field::new("hi_limit", DataType::Float32, true),
        Field::new("units", DataType::Utf8, true),
        Field::new("test_time_ms", DataType::UInt32, true),
    ]))
}
