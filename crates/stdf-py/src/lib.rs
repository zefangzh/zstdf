use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use stdf_core::{decode_record, detect_endianness, Record};

/// Parse the FAR record at the start of an STDF byte buffer and return
/// (cpu_type, stdf_version) as a Python tuple.
///
/// This is deliberately minimal - it exists to prove the PyO3 binding
/// pipeline works end to end (Rust decode -> Python-callable function ->
/// pip-installable wheel via maturin) before building out the full
/// streaming/Arrow/Parquet API described in STDF_RS_SPEC.md Phase 5.
#[pyfunction]
fn parse_far(data: &[u8]) -> PyResult<(u8, u8)> {
    let endianness = detect_endianness(data).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let (record, _consumed) =
        decode_record(data, endianness).map_err(|e| PyValueError::new_err(e.to_string()))?;
    match record {
        Record::Far(far) => Ok((far.cpu_type, far.stdf_ver)),
        _ => Err(PyValueError::new_err(
            "expected a FAR record at the start of the buffer",
        )),
    }
}

#[pymodule]
fn stdf_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_far, m)?)?;
    Ok(())
}
