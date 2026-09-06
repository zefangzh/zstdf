# zstdf phased execution spec

Status: execution plan for the current repo state

This document is the working implementation spec for the code in this
repository. It narrows the broader `ZSTDF_SPEC.md` and
`implementation_plan.md` into phases that can be implemented and validated
incrementally.

## Current progress

- Phase 1: complete in the active top-level `stdf-core` crate.
- Phase 2: complete in the active top-level `stdf-core` crate as of this pass.
- Phase 3: complete in the active top-level `stdf-core` crate as of this pass.
- Phase 4: complete in the active top-level `stdf-io` crate as of this pass.
- Phase 5: complete in the active top-level `stdf-arrow` crate as of this pass.
- Phase 6: complete in the active top-level `stdf-parquet` and `stdf-py` crates as of this pass.
- Phase 7A: complete as a hardening pass across `stdf-core`, `stdf-io`, `stdf-arrow`, `stdf-parquet`, and `stdf-py`.
- Phase 7B: complete with checked-in fixture corpus policy, deterministic fuzz smoke tooling, and performance baseline tooling.
- Phase 8: complete with CLI workflows, Python packaging metadata fix, CI workflow, and user docs.
- Phase 9A: complete with Parquet-specific performance attribution and profiler workflow.
- Phase 9B: complete with streaming Parquet conversion, atomic output commits, retry-safe manifests, orphan-temp cleanup, and no-overwrite idempotency.
- Phase 10A: complete with self-contained interactive HTML dashboard generation from EAV Parquet.
- Current validation: `cargo check --workspace` and `cargo test --workspace` pass with 158 unit/integration tests; targeted Phase 10A tests pass for `stdf-cli`.
- Next implementation phase: Phase 10B, partitioned output and multi-file conversion.

## Why this spec differs

The active workspace now contains top-level crates:

- `stdf-core`
- `stdf-io`
- `stdf-arrow`
- `stdf-parquet`
- `stdf-py`
- `stdf-cli`

So the practical order is:

1. finish a stable decode core
2. add whole-buffer and streaming APIs on top of that
3. introduce Arrow/Parquet crates only after the decoded record model is stable

This also tightens one correctness point from the existing docs:

- `FAR.CPU_TYPE=1` is treated as big-endian
- `FAR.CPU_TYPE=2` is treated as little-endian
- other values are rejected explicitly in v0.1 instead of being silently
  interpreted as big-endian

## Phase 1: decoder foundation

Goal: make `stdf-core` useful for real metadata and test-result parsing before
building more crates.

Milestone:

- reusable binary field reader
- strict FAR validation
- recoverable whole-buffer decode
- implemented record set:
  `FAR`, `MIR`, `MRR`, `WIR`, `WRR`, `PIR`, `PRR`, `PTR`, `PMR`, `TSR`,
  `HBR`, `SBR`
- unimplemented records preserved as raw payloads

Validation:

- `cargo test -p stdf-core`
- confirm:
  - little-endian and big-endian FAR decode
  - metadata records decode into typed structs
  - PTR optional fields and pass/fail extraction work
  - truncated trailing record returns prior decoded records plus a trailing error

## Phase 2: remaining high-value record coverage

Goal: finish the record family needed for text-dump parity and EAV conversion.

Milestone:

- implement `MPR`, `FTR`, `PCR`, `DTR`, `GDR`
- add shared helpers for nibble arrays, bit arrays, and generic data values
- add record display helpers for plain-text comparison output

Validation:

- `cargo test -p stdf-core`
- add golden byte fixtures for each new record type
- add one mixed-record sample covering `PIR -> PTR/MPR/FTR -> PRR`

## Phase 3: stable core APIs for downstream crates

Goal: expose a downstream-friendly API before introducing I/O and Arrow.

Milestone:

- add iterator-oriented decode entry points
- add decode summary/statistics
- stabilize record structs and error model
- keep graceful truncation behavior

Validation:

- `cargo test -p stdf-core`
- doc-tests or unit tests for:
  - one-shot decode
  - incremental decode
  - summary/error reporting

## Phase 4: I/O crate

Goal: introduce `stdf-io` only after the core binary contract is stable.

Milestone:

- local file reader
- buffered streaming parser
- large-file iteration without whole-file buffering

Validation:

- `cargo test -p stdf-io`
- integration test reading a temp STDF file from disk
- verify output parity against `stdf-core::decode_all`

## Phase 5: Arrow crate

Goal: produce long-format EAV batches from decoded records.

Milestone:

- `stdf-arrow` crate
- context tracking across MIR/WIR/PIR/PRR
- `PTR` EAV rows with room for later `MPR` and `FTR` expansion
- batch flush by configurable row count

Validation:

- `cargo test -p stdf-arrow`
- synthetic flow:
  `FAR -> MIR -> WIR -> PIR -> PTR* -> PRR`
- verify row counts, lot/wafer propagation, and nullable optional fields

Implemented:

- long-format EAV schema where each row is one test result with part context
- `StdfContext` state machine for `MIR`, `WIR`, `PIR`, `PTR`, and `PRR`
- configurable `BatchBuilder` row flushing
- `records_to_batches` convenience API for decoded record iterators

## Phase 6: Parquet + Python

Goal: surface the first end-to-end user workflow.

Milestone:

- `stdf-parquet` crate
- `stdf-py` one-shot file read as PyArrow `RecordBatch` list
- `stdf-py` file-to-Parquet conversion

Validation:

- `cargo test --workspace`
- Rust-level Python wrapper smoke test through `cargo test -p stdf-py`
- verify a converted dataset loads through the Rust Parquet Arrow reader

Implemented:

- `stdf_parquet::write_record_batches`
- `stdf_parquet::write_record_batches_to_path`
- `stdf_parquet::records_to_parquet_path`
- `_zstdf.read_batches(path, batch_size=None)` returning PyArrow `RecordBatch` objects
- `_zstdf.write_parquet(input_path, output_path, batch_size=None, overwrite=None)` returning written row count

## Phase 7: hardening

Goal: make the decoder trustworthy on damaged files and vendor variance.

Milestone:

- fuzz target for `stdf-core`
- golden tests with real vendor files
- performance baseline

Validation:

- `cargo test --workspace`
- fuzz corpus runs without panics
- benchmark results checked into docs

Phase 7A implemented:

- strict `FAR.CPU_TYPE` validation instead of silently guessing byte order
- strict `StdfReader` initialization through `detect_byte_order_result`
- streaming reader error reporting for partial trailing headers
- deterministic malformed-input/noise tests for decode APIs
- Arrow context hardening for interleaved sites, missing `PIR`, empty parts, wafer updates, pass/fail nullability, and batch-size normalization
- Parquet hardening for in-memory output, empty schema files, invalid output paths, decode-error propagation, and multi-batch row preservation
- Python wrapper hardening for invalid paths, empty FAR-only input, zero batch size, and module registration

Phase 7B implemented:

- checked-in synthetic golden corpus under `fixtures/stdf/golden`
- documented fixture and private vendor corpus policy in `fixtures/stdf/README.md`
- golden corpus integration tests, including valid LE/BE, PTR/PRR flow, unknown vendor record, and truncation cases
- deterministic 1,500-case fuzz regression test seeded by checked-in fixtures
- runnable fuzz smoke:
  `cargo run -p stdf-core --bin fuzz_decode -- --iterations 1500 --max-len 1024`
- runnable performance baseline:
  `cargo run -p stdf-core --bin perf_baseline -- --parts 1000 --tests-per-part 5`
- performance baseline documentation and 15% regression policy in `docs/performance_baseline.md`

## Phase 8: packaging, CLI workflows, CI, and docs

Goal: make the current implementation usable through stable commands and
repeatable automation.

Milestone:

- useful `zstdf-cli` commands for inspect/dump/convert workflows
- active `pyproject.toml` points to the top-level Python crate
- CI workflow captures the same quality gates used locally
- user-facing docs explain CLI and Python entry points

Validation:

- `cargo test -p stdf-cli`
- `cargo check --workspace`
- `cargo test --workspace`
- fuzz smoke command from CI
- performance smoke command from CI

Implemented:

- `zstdf-cli info <input>`
- `zstdf-cli dump <input> [--limit N]`
- `zstdf-cli convert <input> <output.parquet> [--batch-size N] [--no-overwrite]`
- `zstdf-cli dashboard <input.parquet> <output.html> [--title TITLE] [--max-correlation-tests N]`
- CLI tests for summary output, dump limit, Parquet conversion, and invalid input
- `pyproject.toml` updated to `stdf-py/Cargo.toml` and `_zstdf`
- GitHub Actions workflow in `.github/workflows/ci.yml`
- CLI/Python usage docs in `docs/cli.md`

## Phase 9A: Parquet performance attribution and profiling workflow

Goal: identify STDF-to-Parquet bottlenecks with a repeatable runner and make
industry-standard profiler workflows straightforward.

Milestone:

- Parquet-specific macro benchmark runner
- stage timing for parse, Arrow build, Parquet write, and Parquet close
- profiler workflow docs for Windows and Linux/WSL
- CI smoke command for the Parquet runner

Validation:

- `cargo test -p stdf-parquet`
- `cargo run -p stdf-parquet --bin parquet_perf -- --parts 1000 --tests-per-part 5 --batch-size 4096`
- `cargo check --workspace`
- `cargo test --workspace`

Implemented:

- `cargo run -p stdf-parquet --bin parquet_perf -- --parts N --tests-per-part N --batch-size N [--output path.parquet]`
- stage metrics: generation, parse, Arrow EAV construction, Parquet write, Parquet close, total rows/sec
- PowerShell helper in `scripts/profile_parquet.ps1`
- profiler guide in `docs/parquet_profiling.md`
- CI Parquet performance smoke step

## Phase 9B: robustness for large files and failed jobs

Goal: make STDF -> Parquet conversion safe under memory pressure, process
crashes, and repeated server jobs.

Milestone:

- stream decoded STDF records into Arrow batches instead of collecting all
  batches before Parquet writing
- write Parquet through a same-directory temporary file and atomically rename
  only after successful writer close/fsync
- write a compact JSON manifest with rows, batches, batch size, and schema
  version after successful commit
- support retry-safe no-overwrite/idempotent conversion paths in CLI and Python
- remove orphaned temporary files from prior crashed attempts before retry

Validation:

- `cargo test -p stdf-arrow`
- `cargo test -p stdf-parquet`
- `cargo test -p stdf-cli`
- `cargo test -p stdf-py`
- `cargo check --workspace`
- `cargo test --workspace`

Implemented:

- `stdf_arrow::record_batches(...)` streaming batch iterator
- `stdf_parquet::write_record_batch_iter(...)` streaming writer
- `stdf_parquet::records_to_parquet_path_atomic(...)`
- `stdf_parquet::AtomicWriteOptions`
- manifest helper `stdf_parquet::manifest_path(...)`
- CLI `--no-overwrite` manifest fast path
- Python `write_parquet(..., overwrite=False)` manifest fast path
- tests for manifest creation, retry idempotency, decode-error cleanup, orphan
  temp cleanup, and no-overwrite behavior with missing original input

## Phase 10A: interactive HTML data dashboard

Goal: turn generated EAV Parquet into an offline, interactive dashboard for
rapid lot/test debug without requiring a Python notebook or web service.

Milestone:

- add CLI dashboard generation from EAV Parquet
- read Parquet batch-by-batch and compute aggregate-only payloads so large raw
  Parquet rows are not retained by the CLI or embedded into the browser
- include yield KPIs, site/wafer yield, hard/soft bin distribution, failure
  Pareto, failure commonality, test correlation, XY failure map, process
  windows, and data-completeness checks
- render a self-contained HTML file with interactive tabs, search, minimum
  failure threshold, commonality dimension filtering, charts, and drilldown

Validation:

- `cargo test -p stdf-cli`
- `cargo check --workspace`
- `cargo test --workspace`

Implemented:

- `stdf-cli/src/dashboard.rs`
- CLI command:
  `zstdf-cli dashboard <input.parquet> <output.html> [--title TITLE] [--max-correlation-tests N]`
- tests for dashboard analytics, correlation detection, safe HTML/JSON
  rendering, and end-to-end CLI generation from a converted Parquet file
