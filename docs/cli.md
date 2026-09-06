# zstdf CLI

`zstdf-cli` provides the first packaged command-line workflow for inspecting
and converting STDF files.

## Commands

Print decode summary:

```powershell
cargo run -p stdf-cli --bin zstdf-cli -- info path\to\input.stdf
```

Dump records as stable text lines:

```powershell
cargo run -p stdf-cli --bin zstdf-cli -- dump path\to\input.stdf --limit 20
```

Convert STDF to long-format EAV Parquet:

```powershell
cargo run -p stdf-cli --bin zstdf-cli -- convert path\to\input.stdf path\to\output.parquet --batch-size 65536
```

Conversion streams decoded records into Arrow batches and then into Parquet, so
memory is bounded by the configured batch size rather than the whole input
file. Successful conversions are committed atomically through a same-directory
temporary file and write a manifest at `output.parquet.json`.

Use `--no-overwrite` for retry-safe jobs. If the output Parquet file and
manifest already exist, the command returns the committed row summary instead
of rewriting the file:

```powershell
cargo run -p stdf-cli --bin zstdf-cli -- convert path\to\input.stdf path\to\output.parquet --no-overwrite
```

Gzip input is auto-detected by `.gz` extension or gzip magic bytes.

Generate a self-contained interactive HTML dashboard from EAV Parquet:

```powershell
cargo run -p stdf-cli --bin zstdf-cli -- dashboard path\to\output.parquet path\to\dashboard.html --title "Lot DataView"
```

The dashboard reads Parquet batch-by-batch and embeds aggregate analysis rather
than raw rows, avoiding both CLI-side and browser-side memory blowups on large
files. It includes yield KPIs, failure Pareto, failure commonality, test
correlation, wafer/XY maps, bin distributions, process window statistics, and
data-completeness checks.

## Python Packaging

The active Python extension crate is `stdf-py`, exposed as module `_zstdf`.

```powershell
maturin develop
```

Then in Python:

```python
import _zstdf

batches = _zstdf.read_batches("input.stdf")
rows = _zstdf.write_parquet("input.stdf", "output.parquet")
rows = _zstdf.write_parquet("input.stdf", "output.parquet", overwrite=False)
```
