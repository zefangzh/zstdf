# zstdf phased execution spec

Status: execution plan for the current repo state

This document is the working implementation spec for the code in this
repository. It narrows the broader `ZSTDF_SPEC.md` and
`implementation_plan.md` into phases that can be implemented and validated
incrementally.

## Current progress

- Coordinate identity update: `eav-v2` adds `part_sequence` and nullable
  `part_merge_key`. `coordinate-v1` supersedes the legacy identity contract for
  both dashboard commands. See `docs/coordinate_identity.md`; original wafer,
  PRR coordinates and PART_ID remain unchanged. Existing eav-v1 data requires
  reconversion into a new output/dataset.

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
- Phase 10B: implemented with file-granularity partitioned output, multi-file CLI/Python conversion, streaming gzip, and validated retries. See the scope boundary below.
- Phase 10C.1: implemented for PTR EAV rows with row-level partitioning, bounded state/writers, per-source staged publication, and validated retries.
- Phase 10C.2: implemented with SHA-256 catalog snapshots, explicit recovery, per-file failure reporting, and lot-selectable dataset dashboards.
- Phase 10C.2 validation baseline: 242 workspace tests, desktop/mobile browser smoke, and Windows RSS stress checks. Phase 10C.2 adds 24 regression tests.
- Phase 10D.1: implemented as the `stdf-analytics` bounded external-sort foundation; not yet connected to dashboard commands.
- Next implementation milestone: Phase 10D.2, disk-backed dashboard reducers and small-data analytics parity. Full Phase 10D remains incomplete.

## Why this spec differs

The active workspace now contains top-level crates:

- `stdf-core`
- `stdf-io`
- `stdf-arrow`
- `stdf-parquet`
- `stdf-py`
- `stdf-cli`
- `stdf-validate`
- `stdf-ascii`
- `stdf-analytics` (Phase 10D scratch-storage foundation)

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

## Phase 10B: multi-file conversion and file-granularity partitions

Goal: convert collections of local STDF inputs into predictable Parquet outputs
without buffering all inputs or opening one Parquet writer per source at once.

Milestone implemented:

- Rust `files_to_partitioned_parquet_dir` with per-file and total summaries.
- CLI `convert-many --output-dir DIR [--partition-by lot-id,wafer-id] INPUT...`.
- Python `write_parquet_many(...)` returning `(files, rows)` and releasing the GIL.
- Recursive CLI discovery of `.std`, `.stdf`, `.std.gz`, and `.stdf.gz`; explicit
  files can have other extensions. Canonical paths are sorted and deduplicated;
  directory traversal skips symbolic links to avoid cycles.
- Sequential streaming of both plain and gzip inputs, including gzip magic detection.
- Preflight reads every input before output creation and checks partition values
  against emitted EAV rows. Missing or truncated input prevents the run from writing.
- One output per source. Stable names contain fingerprints of canonical source
  path and raw file contents; basename collisions and reordered/subset retries
  do not reassign outputs. Source changes detected before commit abort the write.
- `input-file`, `lot-id`, and `wafer-id` keys, percent-encoded directory values,
  null/empty sentinels, and Windows case-collision protection.
- Atomic per-file writing and manifests from Phase 9B. Multi-file no-overwrite
  also validates the existing footer, EAV schema, and source/manifest row counts.
- An exclusive dataset lock prevents cooperating multi-file jobs from sharing
  writers or cleaning each other's temporary files. Recovery is explicit after
  a process crash; a lock is never automatically assumed stale.

Scope and tradeoffs:

- This is file-granularity partitioning. A source with more than one selected
  lot/wafer value is rejected with guidance to use `input-file`. Automatic row
  splitting is the proposed Phase 10C milestone, not silently approximated here.
- Conversion reads each source twice (preflight and write). Retry still needs
  the source for validation. This trades throughput for predictable destinations
  and validation before writes.
- Input fingerprints use fixed FNV-1a 128-bit for local identity, not cryptographic
  integrity. Footer validation does not verify every Parquet data page.
- Changed source bytes create a new output identity and retain the previous
  version. Moving a source also changes its identity. There is no automatic
  version selection when globbing every Parquet file in the output directory.
- Writes are atomic per file, not a transaction over the whole dataset. If an
  I/O failure occurs during writing, earlier completed outputs remain reusable.
- Memory still includes active-part test state, Arrow batches, and Parquet row
  groups. `batch_size` is a target row count, not a hard process memory limit.
  Phase 10B does not claim protection from every malformed-file memory exhaustion.

Validation:

- `cargo fmt --all --check`
- `cargo check --workspace --offline`
- `cargo test --workspace --offline`
- 19 dataset tests cover readable totals, partitions, duplicate paths and names,
  reorder/subset stability, retries, changed sources, source mutation before
  commit, invalid/missing/truncated inputs, mixed wafers, escaped/reserved/case
  variants, gzip and damaged trailers, empty data, locks, damaged Parquet, and
  mismatched manifests.
- Three CLI tests cover recursive discovery/deduplication, argument validation,
  and empty directories. Two Python tests cover conversion/retry and invalid
  requests; the existing module registration test checks the new function.
- Smoke workflow: convert two fixture files, rerun with `--no-overwrite`, and
  generate the existing interactive dashboard from an emitted Parquet file.

## Phase 10C: row partitions, memory limits, and dataset versions

Status: 10C.1 and 10C.2 implemented for PTR EAV conversion.

Goal: support multi-wafer/multi-lot sources under a configured resource budget,
with resumable dataset versions that downstream dashboards can consume safely.

Milestone 10C.1: bounded row-level partition writer (implemented)

- Route EAV rows by their actual lot/wafer context, including interleaved sites.
- Add configurable memory, pending-test, open-writer, row-group, and output-file
  limits. Check limits before retaining data; either spill to bounded disk
  storage or return a specific resource-limit error.
- Use a bounded writer cache and numbered immutable fragments. Reopened
  partitions start a new fragment; do not attempt to append to closed Parquet.
- Preserve every completed part's rows and prevent a missing PRR from retaining
  unlimited test results. Report incomplete parts explicitly.

Implementation and scope:

- `bounded_record_batches` applies pending-test and conservative byte reservations
  before retaining tests. It emits one completed part per batch, avoiding an
  unbounded list of completed zero-row parts. Missing PRR, duplicate PIR, malformed
  metadata/test records, and exceeded limits return explicit errors.
- Active parts snapshot lot/wafer metadata at PIR (or their first PTR for an
  implicit part). SDR mappings select the wafer group for each head/site;
  interleaved parts are not reassigned when another wafer starts or finishes.
- The bounded converter currently supports PTR EAV output. MPR and FTR return
  an unsupported-expansion error instead of silently losing measurements; large
  MPR result counts are checked against the pending-test limit first.
- CLI: `convert-partitioned --output-dir DIR [resource limits] INPUT...`.
  Python: `write_parquet_partitioned(...)` returns `(files, rows, fragments)`.
  The older `convert`/`convert-many` entry points retain their existing behavior.
- The writer cache is limited by both open-file count and measured Parquet
  buffer size. Eviction closes an immutable fragment; a later visit creates a
  new numbered fragment. Each fragment contains at most one bounded row group.
- Preflight validates all inputs using bounded conversion. Each source is then
  written under a fresh staging directory, synced, and published by directory
  rename after all fragments and `_SUCCESS.json` close. Handled failures remove
  only the staging directory owned by that invocation.
- Source generation IDs include path, source fingerprint, partition keys, and
  resource options. Repeated invocations validate receipts, safe relative paths,
  fragment schemas, and row counts before reusing a generation. These local
  fingerprints and footer checks are not cryptographic data-page verification.
- Metadata, pending parts, Arrow copies, writer buffers, and encoding reservations
  have budget allocations. Limits govern accounted state, not an OS-enforced RSS
  ceiling. Decoder scratch space and allocator/runtime overhead still exist.
  Oversized parts fail; disk spilling is not implemented. Input/root paths are
  capped at 1024 encoded bytes to bound retained path metadata.
- Interrupted processes may leave a root lock and unpublished `.staging-*`
  directories. Do not glob those fragments into a dataset; explicit recovery
  and a catalog selecting current generations belong to 10C.2.

Validation for 10C.1:

- Mixed lots/wafers, interleaved sites, and repeated partition visits must have
  exact row/value parity with the unpartitioned EAV output.
- Generate many more partitions than the writer limit; verify open handles and
  buffered bytes stay within configured limits and all fragments remain readable.
- Stress a single oversized part, missing PRR, huge MPR arrays, tiny budgets,
  and highly compressed gzip. Measure peak RSS against a documented allowance
  for runtime/allocator overhead; require bounded failure or spill, never OOM.

Validation executed:

- `cargo test --workspace --offline` (218 tests, including 18 added in 10C.1).
- New cases cover pending tests across sites, tiny budgets, duplicate PIR,
  incomplete parts, lot/wafer snapshots, same-head concurrent wafer groups,
  row/value parity, writer eviction/revisits, capped row groups, fragment-limit
  cleanup, safe retry receipts, gzip missing PRR, malformed PTR, oversized MPR,
  empty sources, and CLI/Python conversion and retries.
- `scripts/measure_partition_memory.ps1` generates gzip inputs incrementally and
  measures the actual CLI process. On the local Windows debug build, a 16 MiB
  accounted budget used peak RSS of 11.62 MiB for 2,000 completed parts and
  11.37 MiB for 20,000. A 1,000,000-PTR input without PRR failed at 129 pending
  tests with a limit of 128, used 7.33 MiB peak RSS, and created no output.
- The repeatable stress gate allows baseline RSS plus 16 MiB budget and 64 MiB
  runtime/allocator variation. These synthetic results are a regression baseline,
  not proof of a hard RSS ceiling on all vendor inputs.

Milestone 10C.2: versioned catalog and recoverable runs (implemented)

- Atomically publish a dataset catalog with cryptographic source/output hashes,
  schema/options version, source identity, fragment paths, rows, and run status.
- Record which output version is current per source. Keep older files available
  but exclude them from the catalog's current snapshot to prevent double counting.
- Add explicit resume/retry and stale-lock recovery with owner/process checks.
  Track per-file errors and define fail-fast versus continue-on-error behavior.
- Let dashboards consume the catalog snapshot, followed by a `dashboard-dir`
  workflow with lot selection. Part keys now follow `coordinate-v1`; only
  unresolved attempts retain source scoping.

Validation for 10C.2:

- Inject failures before close, after Parquet commit, during catalog publication,
  and during retry. Resume must produce one current version without lost rows.
- Test concurrent jobs, stale versus active locks, changed inputs/options/schema,
  corrupt output pages, disk-full errors, and missing fragments.
- Dashboard totals from the catalog must equal the sum of current source versions;
  identical part IDs across sources and superseded outputs must not merge or
  double-count parts.

Implementation and scope for 10C.2:

- CLI `convert-partitioned` and Python `write_parquet_partitioned` now write
  `_catalog.json` above immutable `objects/<generation>/source-...` fragments.
  Python signatures and tuple results remain unchanged. The lower-level Rust
  `files_to_partitioned_fragments` API remains available without a catalog.
- Catalog version 1 records EAV schema version, options hash, canonical source
  identity, SHA-256 source/output hashes, current fragments, rows, revision, and
  latest run status. Source updates replace one catalog entry, not old files.
- Per-source publication is atomic, not a whole-run transaction. Fail-fast is
  default. `--continue-on-error` records failures, converts remaining inputs,
  and still exits nonzero. Failed updates retain the last successful version;
  the dashboard explicitly warns when the latest run is incomplete.
- Repeating conversion resumes valid immutable generations. Version-2 fragment
  receipts verify entire output files, not just footers. Older raw generations
  are not automatically imported: rerun conversion to build a managed catalog.
- `.catalog.guard` uses a kernel file lock that releases on process exit. Keep
  the persistent guard file; deleting it can break locking. `recover-dataset`
  holds this lock, refuses active/unknown legacy owners, removes abandoned
  staging/catalog temp files, and marks a running catalog as interrupted.
  Published generations are never removed by recovery.
- `verify-dataset` validates SHA-256, paths, unique fragments, EAV schemas, and
  row totals. It reports run status separately from snapshot integrity.
- `dashboard-dir` verifies the snapshot, scopes part IDs by source, aggregates
  across fragments, and embeds All-lot and per-lot views. Switching lots refreshes
  every chart. Memory/lot/part limits fail before replacing existing HTML.
- Conversion reserves one quarter of its accounted budget for catalog work;
  catalog JSON is capped at min(memory/64, 16 MiB), with capped serialization.
  Dashboard analysis uses conservative cumulative row/string reservations.
  Neither is an OS-enforced RSS ceiling. Parquet reader scratch/allocator
  overhead requires headroom; dashboards reject oversized workloads rather
  than spilling. A 256 MiB dashboard budget fits roughly 30,000 short rows.
- This targets cooperating local writers on ordinary local filesystems. Network
  filesystem lock/durability guarantees and physical power-loss behavior are
  not certified. Hashes detect accidental corruption; an attacker able to rewrite
  both data and catalog is outside this integrity model. Retained generations
  require disk capacity; automatic garbage collection is not implemented.
- Existing dashboard analytics still group tests by test number and aggregate XY
  across the selected lots/wafers. Part merging now uses `coordinate-v1`.
  They are not yet a retest-aware or test-program-version-aware analysis model.
- Tested with Rust 1.97.1 on Windows; native file locks require a sufficiently
  recent Rust toolchain (the workspace does not yet advertise a tested MSRV).

Validation executed for 10C.2:

- 242 workspace tests pass, including publication failure injection, failure
  before fragment close and after source commit, retry, active/dead/unknown
  owners, lock exclusion, metadata bounds, source/options changes, corruption,
  missing files, partial runs, scoped identities, and script-safe lot labels.
- `scripts/smoke_dataset_dashboard.cjs` passes in headless Edge at 1440x900 and
  390x844: lot yields, tab switching, search, no horizontal overflow, and no JS
  runtime errors. Requires Playwright on NODE_PATH and installed Edge.
- Updated `scripts/measure_partition_memory.ps1` passes with catalog conversion:
  2,000 parts used 11.41 MiB peak RSS; 20,000 used 11.77 MiB. A million PTRs
  without PRR failed at the pending-test bound, used 8.02 MiB, and published no
  source/fragments. Its diagnostic catalog correctly records failure.
- Fault injection models selected I/O/crash boundaries; it is not a physical
  disk-full or host-power-loss test.

## Phase 10D: scalable dataset analytics (in progress)

Milestone: replace cumulative in-memory part/result retention with a bounded
disk-backed aggregation stage, while preserving catalog snapshot semantics and
the self-contained HTML interface. Add an explicit disk budget and cancellation
cleanup; do not simply raise the current dashboard memory defaults.

Validation before implementation:

- Golden parity for yield, Pareto, commonality, correlations, and lot selection
  against the current small-data implementation.
- Million-row and high-cardinality fixtures must stay within measured memory
  allowances; disk-full, cancellation, and process-kill tests preserve the prior
  HTML/catalog and clean only owned temporary state.
- Define retest/part-instance and test-program identity in a versioned schema
  before changing counting semantics. Include repeated PART_ID, reused test
  numbers with different units/limits, and per-wafer XY selection regressions.
- Retain explicit PTR-only behavior in bounded conversion until a separately
  validated MPR/FTR expansion milestone is implemented.

### 10D.1: bounded scratch-store foundation (implemented)

The new `stdf-analytics` crate provides stable external sorting of binary
key/value records without introducing a database runtime. Memory limits bound
accounted chunk/front state, disk limits include live merge inputs and output,
and fan-in limits open input files. Run metadata uses numeric ranges rather than
an unbounded vector of paths. Identical keys retain ingestion order.

Cancellation is cooperative through `Arc<AtomicBool>` during ingestion, merge,
and replay. Handled failures and normal drop clean only the invocation's owned
scratch directory. Corrupt lengths are rejected before allocation, truncated
records fail rather than silently ending replay, and an errored store cannot
be reused. There is no publication or mutation of dataset/HTML paths.

Scope boundary: this is a storage API, not a dashboard engine. Existing
`dashboard-dir` memory/part limits and counting behavior are unchanged. Hard
process termination can leave scratch directories; automatic stale-job recovery,
OS signal wiring, real disk-full testing, and measured RSS gates remain pending.
Scratch bytes are not filesystem allocation/quota bytes, and memory accounting
is not a process RSS ceiling. Callers must stream replay rather than collect it.

Validation:

- `cargo test -p stdf-analytics --offline`: 19 tests, including multi-pass stable
  sorting, duplicate/binary keys, quotas during merge, cancellation, invalid
  configuration, corrupt/truncated records, missing runs, and isolated cleanup.
- `cargo run -p stdf-analytics --bin spill_stress -- 100000`: bounded scratch
  stress with row/order validation and cleanup verification.
- `scripts/measure_analytics_memory.ps1`: repeatable Windows stress/RSS sampling
  at 100,000 and 1,000,000 records, with configurable 64 MiB absolute/16 MiB
  growth regression allowances. These are runtime headroom gates, not memory
  guarantees for a future dashboard or arbitrary record distributions.
- Existing CLI tests remain the compatibility baseline; no analytics result
  parity claim is made until integration in 10D.2.

### 10D.2: disk-backed dashboard reducers (next)

Milestone: use the scratch store to group complete part identities across
catalog fragments, reduce part yield and correlation observations incrementally,
and retain only explicitly bounded aggregate/output state. Add CLI scratch-path
and disk-budget options only when the dashboard actually consumes them.

Current contract `coordinate-v1` uses eav-v2 `part_merge_key`: a valid uppercase
alphanumeric wafer and positive PRR X/Y, otherwise lot and unambiguous positive
integer PTR X/Y. Resolved identities merge across source/lot (for wafer keys),
PART_ID and head/site. Null merge keys use `(source, part_sequence)` and remain
separate. The prior `analytics-identity-v1` tuple contract is superseded by this
explicit schema/identity version; do not reproduce it in the new reducers.
Part pass remains a conjunction, metadata comes from the first row, correlation
uses the first finite result per test number, and tests remain grouped by number.
These policies are not first/final-retest or test-program-aware analytics.

Validation: compare every All-lot/per-lot payload field (excluding timestamps)
against the existing implementation for yield, Pareto, commonality, correlations,
quality, bins, process windows, and XY maps. Include fragment splits, repeated
PART_ID, identical IDs across sources, reused test numbers with different
units/limits, null/empty wafer IDs, missing/nonfinite results, and changed catalog
versions. Preserve existing HTML on every handled failure.

### 10D.3: operational qualification (pending)

Milestone: explicit stale scratch recovery with owner/lock checks, CLI cancellation,
disk exhaustion handling, and process-kill recovery. Validate million-row and
high-cardinality dashboards with measured RSS and disk peaks. Add per-wafer XY
selection with separate parity/UI tests. Do not remove the existing safeguards
or advertise unbounded-size dashboard support until these gates pass.
