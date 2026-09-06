//! Core STDF (Standard Test Data Format) v4 binary decoding.
//!
//! Pure decode logic with no I/O assumptions - operates on byte slices so
//! it's reusable from a CLI, a long-running service, or Python bindings
//! (see the `stdf-py` crate). See ZSTDF_SPEC.md for the overall project
//! plan and phased roadmap.

mod error;
mod fields;
mod header;
mod records;

pub use error::StdfError;
pub use fields::FieldReader;
pub use header::{detect_endianness, Endianness, RecordHeader};
pub use records::{
    decode_all, decode_record, DecodeSummary, FarRecord, HbrRecord, MirRecord, MrrRecord,
    PirRecord, PmrRecord, PrrRecord, PtrRecord, Record, SbrRecord, TsrRecord, WirRecord,
    WrrRecord,
};
