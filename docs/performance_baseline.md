# Performance Baseline

Phase 7B adds a repeatable synthetic benchmark runner that does not require
nightly Rust or external tooling.

Run:

```powershell
cargo run -p stdf-core --bin perf_baseline -- --parts 10000 --tests-per-part 5
```

The runner reports:

- synthetic byte size
- decoded record count
- whole-buffer decode elapsed time
- iterator decode elapsed time
- records per second
- deterministic input checksum

## Regression Policy

Use the same machine, compiler, and command-line parameters when comparing
results. Treat a sustained records/sec drop above 15% as a regression candidate
unless the change intentionally adds validation or more complete decoding.

## Fuzz Smoke

Run:

```powershell
cargo run -p stdf-core --bin fuzz_decode -- --iterations 10000 --max-len 4096
```

This deterministic fuzz smoke mutates checked-in golden fixtures and exercises:

- byte-order detection
- `decode_all`
- `decode_with_summary`
- `summarize`

It is intended as a fast local/CI guard. It is not a replacement for coverage-
guided fuzzing with a tool such as `cargo-fuzz` once the project adds that
dependency explicitly.

## Parquet Pipeline Timing

For STDF -> Parquet attribution, use:

```powershell
cargo run -p stdf-parquet --bin parquet_perf -- --parts 10000 --tests-per-part 5 --batch-size 65536
```

See `docs/parquet_profiling.md` for profiler-specific workflows.

The production conversion path is streaming: decoded STDF records are flushed
into Arrow batches and written to Parquet without collecting all batches in a
`Vec`. Keep `--batch-size` large enough for writer efficiency but small enough
for worst-case server memory limits. For crash/retry robustness, CLI and Python
conversion write through same-directory temporary files, commit the final
Parquet atomically, and write a JSON manifest beside the output.
