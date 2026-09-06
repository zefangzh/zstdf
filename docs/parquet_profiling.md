# Parquet Profiling

The Parquet-specific performance runner reports coarse timings for the full
STDF -> Arrow -> Parquet path.

Run a baseline:

```powershell
cargo run -p stdf-parquet --bin parquet_perf -- --parts 10000 --tests-per-part 5 --batch-size 65536
```

The runner reports:

- synthetic STDF generation time
- STDF parse time
- Arrow EAV batch construction time
- Parquet `write(batch)` time
- Parquet `close()` time
- rows/sec and output bytes

## Industry Profiling Workflow

Use the runner as the stable workload, then profile that process with the
platform profiler.

On Windows:

- Use Visual Studio Performance Profiler for CPU sampling.
- Use Windows Performance Recorder/Analyzer for system-level CPU and I/O.
- Use Intel VTune if available for deeper CPU attribution.

On Linux or WSL:

```bash
cargo install flamegraph
cargo flamegraph -p stdf-parquet --bin parquet_perf -- --parts 10000 --tests-per-part 5
```

Interpretation:

- If `parse_elapsed_ms` dominates, inspect `stdf-core` record parsing.
- If `arrow_elapsed_ms` dominates, inspect `stdf-arrow` EAV builders and string cloning.
- If `parquet_write_elapsed_ms` dominates, inspect Parquet encoding/compression.
- If `parquet_close_elapsed_ms` dominates, inspect row group finalization and metadata.

The runner is a macro benchmark. Use a profiler for line/function attribution
and use this runner in CI or release checks for regression tracking.
