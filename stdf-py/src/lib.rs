use arrow::pyarrow::ToPyArrow;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::path::Path;
use stdf_core::{StdfError, StdfRecord};
use stdf_io::StdfReader;

const DEFAULT_BATCH_SIZE: usize = 65_536;

#[pyfunction]
fn read_batches(py: Python<'_>, path: &str, batch_size: Option<usize>) -> PyResult<Vec<PyObject>> {
    let reader = StdfReader::open(path).map_err(to_py_err)?;
    let batches = stdf_arrow::records_to_batches(
        reader.records(),
        batch_size.unwrap_or(DEFAULT_BATCH_SIZE).max(1),
    )
    .map_err(to_py_err)?;

    batches
        .iter()
        .map(|batch| batch.to_pyarrow(py))
        .collect::<PyResult<Vec<_>>>()
}

#[pyfunction]
#[pyo3(signature = (input_path, output_path, batch_size=None, overwrite=None))]
fn write_parquet(
    input_path: &str,
    output_path: &str,
    batch_size: Option<usize>,
    overwrite: Option<bool>,
) -> PyResult<usize> {
    let batch_size = batch_size.unwrap_or(DEFAULT_BATCH_SIZE).max(1);
    let options = stdf_parquet::AtomicWriteOptions {
        overwrite: overwrite.unwrap_or(true),
        write_manifest: true,
        cleanup_orphaned_temp: true,
    };
    if !options.overwrite && Path::new(output_path).exists() {
        let summary = stdf_parquet::records_to_parquet_path_atomic(
            std::iter::empty::<std::result::Result<StdfRecord, StdfError>>(),
            output_path,
            batch_size,
            &options,
        )
        .map_err(to_py_err)?;
        return Ok(summary.rows);
    }

    let reader = StdfReader::open(input_path).map_err(to_py_err)?;
    let summary = stdf_parquet::records_to_parquet_path_atomic(
        reader.records(),
        output_path,
        batch_size,
        &options,
    )
    .map_err(to_py_err)?;
    Ok(summary.rows)
}

#[pymodule]
fn _zstdf(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", "0.1.0")?;
    m.add_function(wrap_pyfunction!(read_batches, m)?)?;
    m.add_function(wrap_pyfunction!(write_parquet, m)?)?;
    Ok(())
}

fn to_py_err(error: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Once;
    use std::time::{SystemTime, UNIX_EPOCH};

    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    use super::*;

    #[test]
    fn write_parquet_creates_readable_eav_file() {
        let input = temp_path("input.stdf");
        let output = temp_path("output.parquet");
        std::fs::write(&input, build_test_stdf()).unwrap();

        let rows = write_parquet(
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            Some(1),
            None,
        )
        .expect("Python wrapper should convert STDF records to parquet");

        assert_eq!(rows, 1);
        let reader =
            ParquetRecordBatchReaderBuilder::try_new(std::fs::File::open(&output).unwrap())
                .unwrap()
                .build()
                .unwrap();
        let batches = reader.collect::<std::result::Result<Vec<_>, _>>().unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 1);

        std::fs::remove_file(input).ok();
        std::fs::remove_file(stdf_parquet::manifest_path(&output)).ok();
        std::fs::remove_file(output).ok();
    }

    #[test]
    fn write_parquet_invalid_input_returns_error() {
        let output = temp_path("missing_output.parquet");

        let result = write_parquet(
            "does-not-exist.stdf",
            output.to_str().unwrap(),
            Some(1),
            None,
        );

        assert!(result.is_err());
        assert!(!output.exists());
    }

    #[test]
    fn write_parquet_zero_batch_size_is_treated_as_one() {
        let input = temp_path("zero_batch_input.stdf");
        let output = temp_path("zero_batch_output.parquet");
        std::fs::write(&input, build_test_stdf()).unwrap();

        let rows = write_parquet(
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            Some(0),
            None,
        )
        .expect("zero batch size should be normalized");

        assert_eq!(rows, 1);
        std::fs::remove_file(input).ok();
        std::fs::remove_file(stdf_parquet::manifest_path(&output)).ok();
        std::fs::remove_file(output).ok();
    }

    #[test]
    fn write_parquet_far_only_file_returns_zero_rows() {
        let input = temp_path("far_only.stdf");
        let output = temp_path("far_only.parquet");
        std::fs::write(&input, far_only()).unwrap();

        let rows = write_parquet(
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            Some(10),
            None,
        )
        .expect("FAR-only STDF should create an empty EAV parquet file");

        assert_eq!(rows, 0);
        let reader =
            ParquetRecordBatchReaderBuilder::try_new(std::fs::File::open(&output).unwrap())
                .unwrap()
                .build()
                .unwrap();
        assert_eq!(
            reader
                .collect::<std::result::Result<Vec<_>, _>>()
                .unwrap()
                .len(),
            0
        );
        std::fs::remove_file(input).ok();
        std::fs::remove_file(stdf_parquet::manifest_path(&output)).ok();
        std::fs::remove_file(output).ok();
    }

    #[test]
    fn write_parquet_no_overwrite_returns_existing_manifest_summary() {
        let input = temp_path("idempotent_input.stdf");
        let output = temp_path("idempotent_output.parquet");
        std::fs::write(&input, build_test_stdf()).unwrap();

        let first_rows = write_parquet(
            input.to_str().unwrap(),
            output.to_str().unwrap(),
            Some(1),
            None,
        )
        .unwrap();
        let repeat_rows = write_parquet(
            "does-not-exist.stdf",
            output.to_str().unwrap(),
            Some(1),
            Some(false),
        )
        .unwrap();

        assert_eq!(first_rows, 1);
        assert_eq!(repeat_rows, first_rows);
        std::fs::remove_file(input).ok();
        std::fs::remove_file(stdf_parquet::manifest_path(&output)).ok();
        std::fs::remove_file(output).ok();
    }

    #[test]
    fn read_batches_invalid_input_returns_error() {
        prepare_python();
        Python::with_gil(|py| {
            let result = read_batches(py, "does-not-exist.stdf", Some(1));
            assert!(result.is_err());
        });
    }

    #[test]
    fn read_batches_far_only_returns_empty_without_pyarrow_conversion() {
        let input = temp_path("read_empty.stdf");
        std::fs::write(&input, far_only()).unwrap();

        prepare_python();
        Python::with_gil(|py| {
            let batches = read_batches(py, input.to_str().unwrap(), Some(1)).unwrap();
            assert!(batches.is_empty());
        });

        std::fs::remove_file(input).ok();
    }

    #[test]
    fn module_registers_expected_functions() {
        prepare_python();
        Python::with_gil(|py| {
            let module = PyModule::new(py, "_zstdf").unwrap();
            super::_zstdf(&module).unwrap();

            assert!(module.hasattr("__version__").unwrap());
            assert!(module.hasattr("read_batches").unwrap());
            assert!(module.hasattr("write_parquet").unwrap());
        });
    }

    fn prepare_python() {
        static INIT: Once = Once::new();
        INIT.call_once(pyo3::prepare_freethreaded_python);
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("zstdf_py_{nanos}_{name}"))
    }

    fn build_test_stdf() -> Vec<u8> {
        let mut buf = Vec::new();

        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.push(0);
        buf.push(10);
        buf.push(2);
        buf.push(4);

        let mut mir = Vec::new();
        mir.extend_from_slice(&1000u32.to_le_bytes());
        mir.extend_from_slice(&2000u32.to_le_bytes());
        mir.push(1);
        mir.push(b'P');
        mir.push(b' ');
        mir.push(b' ');
        mir.extend_from_slice(&0u16.to_le_bytes());
        mir.push(b' ');
        mir.push(6);
        mir.extend_from_slice(b"PY_LOT");
        mir.push(4);
        mir.extend_from_slice(b"IC_A");
        mir.push(4);
        mir.extend_from_slice(b"ND01");
        mir.push(2);
        mir.extend_from_slice(b"J7");
        mir.push(4);
        mir.extend_from_slice(b"JOB1");
        buf.extend_from_slice(&(mir.len() as u16).to_le_bytes());
        buf.push(1);
        buf.push(10);
        buf.extend_from_slice(&mir);

        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.push(5);
        buf.push(10);
        buf.push(1);
        buf.push(0);

        let mut ptr = Vec::new();
        ptr.extend_from_slice(&9001u32.to_le_bytes());
        ptr.push(1);
        ptr.push(0);
        ptr.push(0x00);
        ptr.push(0x00);
        ptr.extend_from_slice(&4.25f32.to_le_bytes());
        buf.extend_from_slice(&(ptr.len() as u16).to_le_bytes());
        buf.push(15);
        buf.push(10);
        buf.extend_from_slice(&ptr);

        let mut prr = Vec::new();
        prr.push(1);
        prr.push(0);
        prr.push(0x00);
        prr.extend_from_slice(&1u16.to_le_bytes());
        prr.extend_from_slice(&1u16.to_le_bytes());
        prr.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&(prr.len() as u16).to_le_bytes());
        buf.push(5);
        buf.push(20);
        buf.extend_from_slice(&prr);

        buf
    }

    fn far_only() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&2u16.to_le_bytes());
        buf.push(0);
        buf.push(10);
        buf.push(2);
        buf.push(4);
        buf
    }
}
