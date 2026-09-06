# zstdf: project spec

Status: draft, v0.1 scope
Companion to: `README.md` (build instructions), the working `stdf-core` /
`stdf-py` proof-of-concept already in this repo

## 1. Goal

Build a Rust library that decodes STDF v4 binary semiconductor test data at
GB-to-TB scale, converts it to columnar Arrow/Parquet output for on premise DuckDB and plain text for comparison against Advantest proprietary stdf reader, and exposes a
clean Python interface via PyO3 - so it installs with a plain `pip install`
and works as a drop-in accelerator for existing pandas/Polars/PyArrow
workflows, on both Linux servers and Windows PCs.

## 2. Non-goals (v0.1 scope boundaries)

- **Not a full STDF writer.** v0.1 targets read/decode/convert only. Writing
  STDF (round-trip) is a candidate for a later phase, not v0.1.
- **Not a GUI or dashboard.** This is a library and CLI; visualization stays
  in Python/JMP/Spotfire as already discussed.
- **Not every vendor-specific record variant on day one.** Target the common,
  high-value record subset first (see Phase 1), add the long tail
  incrementally as real files surface gaps.

## 3. Requirements

### Functional

- Parse the core STDF v4 record set: FAR, MIR, MRR, WIR, WRR, PIR, PRR, PTR,
  MPR, FTR, PMR, TSR, HBR, SBR, PCR, DTR, GDR. Records outside this set decode as
  `Record::Unimplemented` (raw payload preserved) rather than failing the file.
- Strictly follow STDF v4 specification for the above record set, and report in plain text on any deviation including format, struct and data type
- For GDR record, export in structured text format that may be defined by the user.
- Support both endianness variants, detected once per file from FAR's
  CPU_TYPE field.
- Degrade gracefully on truncated/corrupt files - return whatever valid
  records were decoded up to the failure point, not just an all-or-nothing
  error, since a crashed test program can leave a valid-but-incomplete file.
- Emit long-format (EAV: part_id, test_num, test_name, value, ...) Arrow
  RecordBatches / Parquet as the primary output shape, matching how STDF
  test lists vary across lots and products.
- Partition Parquet output by lot_id / wafer_id / test date.
- Use memory and storage efficient data types and storage structures where STDF v4 specification 
  defines the valid data types
- Python API surface (see Section 5) with both a one-shot read and a
  streaming/batched iterator for files too large to hold in memory.

### Non-functional

- Cross-platform: Linux x86_64 is the primary target (where conversion
  actually runs, per our earlier discussion - close to the data on the
  remote server); Windows x86_64 is a required secondary target for local
  PC use.
- Memory: streaming/chunked - never require the whole file resident in
  memory at once.
- Safety: no `unsafe` in the decode path unless specifically justified and
  documented; the decoder is fuzz-tested against malformed/truncated input
  before v0.1 ships.
- Performance target: to be set from real benchmark numbers once Phase 1 is
  done (don't guess a number now - measure against a real reference parser
  and real sample files, then set the target).

## 4. Architecture

```
Source data (remote object storage or local disk)
        |
Parallel ingest (range reads / mmap, rayon workers)
        |
Streaming decoder      <-- stdf-core (done: framing, endianness, FAR)
        |
Columnar batch builder <-- stdf-arrow (not started)
        |
Partitioned parquet writer <-- stdf-parquet (not started)
        |
Data warehouse (Snowflake, BigQuery, ClickHouse, or plain Parquet + pandas/Polars)
```

Crate breakdown:

| Crate | Responsibility | Status |
|---|---|---|
| `stdf-core` | Pure binary decode, no I/O assumptions | FAR + framing done; other record types TODO |
| `stdf-io` | Local (`memmap2`) and remote (`object_store`) reading, record-boundary pre-scan | not started |
| `stdf-arrow` | Build typed Arrow `RecordBatch`es from decoded records | not started |
| `stdf-parquet` | Partitioned Parquet writing | not started |
| `stdf-py` | PyO3 bindings over the above | proof-of-concept done (`parse_far`) |
| `stdf-cli` (bin) | Command-line batch conversion tool | not started |

## 5. Python interface design

Package name: `stdf-rs` (verify availability on PyPI before publishing).

```python
import stdf_rs

# One-shot read of a single file
table = stdf_rs.read_stdf("lot123.stdf")          # -> pyarrow.Table

# Streaming for files too large to hold in memory
for batch in stdf_rs.iter_batches("lot123.stdf", batch_size=100_000):
    process(batch)                                 # -> pyarrow.RecordBatch

# Batch conversion of a directory (local or object storage URI) to
# partitioned parquet
stdf_rs.convert_to_parquet(
    input_dir="s3://bucket/stdf/",
    output_dir="s3://bucket/parquet/",
    partition_by=["lot_id", "wafer_id", "test_date"],
)
```

Design notes:

- Errors surface as normal Python exceptions (`ValueError` for now, a custom
  `StdfError` exception type once the API stabilizes), not error codes.
- Returned tables should be built via Arrow's C Data Interface so crossing
  the Rust/Python boundary is zero-copy, not a serialize/deserialize step.
- `parse_far` (already implemented) stays as a low-level primitive; the
  three functions above are the actual user-facing API, built on top of it.

## 6. Phased task breakdown

### Phase 0 - Setup (done)
- [x] Cargo workspace scaffold (`stdf-core`, `stdf-py`)
- [x] `stdf-core` compiles and passes tests
- [x] `stdf-py` compiles against Python 3.12 and the compiled extension is
      importable and callable from plain Python
- [ ] CI matrix (GitHub Actions: linux-x86_64, windows-x86_64)
- [ ] Collect real sample STDF files from at least 2 ATE vendors (Teradyne,
      Advantest are the most common) for test fixtures - vendor quirks in
      spec interpretation are a known issue, so real files matter more than
      hand-built ones beyond the unit-test level

### Phase 1 - Core decode (`stdf-core`)
- [ ] Decode MIR, MRR, WIR, WRR (lot/wafer metadata)
- [ ] Decode PIR, PRR (part start/stop)
- [ ] Decode PTR, MPR, FTR (the actual test results - highest value target)
- [ ] Decode PMR, TSR (test/pin metadata needed to make PTR/MPR meaningful)
- [ ] Decode HBR, SBR (bin summaries)
- [ ] Golden-file parity tests against real vendor files, not just hand-built
      byte arrays

### Phase 2 - Streaming I/O (`stdf-io`)
- [ ] Local file via `memmap2` (cross-platform Linux/Windows)
- [ ] Remote reads via `object_store` (S3 first; GCS/Azure if needed - see
      open questions)
- [ ] Record-boundary pre-scan (walk length prefixes only) to split a large
      file into chunks for parallel decode

### Phase 3 - Arrow integration (`stdf-arrow`)
- [ ] Finalize long-format schema (part_id, test_num, test_name, value,
      unit, lo_limit, hi_limit, pass_fail, lot_id, wafer_id, ...)
- [ ] Columnar builder consuming decoded records into `RecordBatch`

### Phase 4 - Parquet writer (`stdf-parquet`)
- [ ] Partitioned write logic (lot_id / wafer_id / test_date)
- [ ] Compression choice - benchmark zstd vs snappy on real data before
      picking a default

### Phase 5 - Python bindings (`stdf-py`)
- [ ] Implement `read_stdf`, `iter_batches`, `convert_to_parquet` per Section 5
- [ ] `maturin build` producing wheels for linux and windows targets
- [ ] Round-trip test: convert a real file, load the result in pandas and
      in Polars, confirm values match a known-good reference

### Phase 6 - Parallelism and performance
- [ ] `rayon` across files and within-file chunks
- [ ] Benchmark harness: `criterion` (Rust side) + `pytest-benchmark`
      (Python side), with before/after numbers against a pure-Python
      reference parser on the same real files

### Phase 7 - Hardening and release
- [ ] `cargo-fuzz` targeting the decoder against corrupted/truncated input
- [ ] Docs + usage examples
- [ ] Publish to crates.io and PyPI

## 7. Testing strategy

- **Unit tests** (in place for Phase 0): hand-built byte sequences covering
  the happy path, truncation, and unimplemented-type handling.
- **Golden-file tests** (Phase 1+): real STDF files with independently
  verified expected output, ideally from more than one ATE vendor since
  vendors are known to interpret the spec slightly differently.
- **Fuzzing** (Phase 7): `cargo-fuzz` against the decoder entry point,
  targeting the truncation and malformed-header paths specifically.
- **Benchmarks** (Phase 6): tracked in CI to catch performance regressions,
  not just correctness regressions.

## 8. Open questions to resolve before Phase 1 goes further

1. Which record types beyond the Phase-1 list are actually needed for your
   first real use case? (Easier to answer once you have real sample files.)
2. Is long-format the *only* output shape needed, or should wide-format
   (pivoted) be a configurable option in `stdf-arrow` rather than a
   downstream transform?
3. Object storage backends: is S3 sufficient, or do you also need GCS/Azure
   support in `stdf-io`?
4. Minimum supported Rust version and Python version - affects which PyO3
   version and which `edition` to commit to.

## 9. Definition of done for v0.1

- Correctly parses the Phase-1 record set against real sample files from at
  least two ATE vendors.
- Produces valid partitioned Parquet output, confirmed importable by
  pandas, Polars, and (via PyArrow) JMP.
- Ships installable Linux and Windows wheels via `pip install`.
- Has a benchmark showing a measured speedup over a pure-Python reference
  parser on the same files - not an assumed number.
