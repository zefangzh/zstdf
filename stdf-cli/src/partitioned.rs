use super::{find_stdf_files, CliResult};
use std::io::Write;
use std::path::PathBuf;
use stdf_parquet::{
    catalog::{convert_dataset, ErrorPolicy},
    FragmentOptions, PartitionKey,
};

#[derive(Debug, clap::Args)]
pub struct Arguments {
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, value_delimiter = ',', default_value = "lot-id,wafer-id")]
    partition_by: Vec<PartitionKey>,
    /// Accounted conversion memory budget, excluding runtime/allocator overhead.
    #[arg(long, default_value_t = 256)]
    memory_limit_mib: usize,
    #[arg(long, default_value_t = 100_000)]
    max_pending_tests: usize,
    #[arg(long, default_value_t = 4)]
    max_open_writers: usize,
    /// Maximum rows per row group and fragment.
    #[arg(long, default_value_t = 65_536)]
    row_group_rows: usize,
    /// Total fragments per invocation; reserves memory for their metadata.
    #[arg(long, default_value_t = 10_000)]
    max_output_files: usize,
    /// Convert remaining files after a failure; still exits nonzero if any fail.
    #[arg(long)]
    continue_on_error: bool,
}

pub fn execute(args: Arguments, out: &mut impl Write) -> CliResult<()> {
    let mut inputs = Vec::new();
    for input in args.inputs {
        if input.is_dir() {
            inputs.extend(find_stdf_files(&input)?);
        } else {
            inputs.push(input);
        }
    }
    let options = FragmentOptions {
        max_memory_bytes: args
            .memory_limit_mib
            .checked_mul(1024 * 1024)
            .ok_or("memory budget overflow")?,
        max_pending_tests: args.max_pending_tests,
        max_open_writers: args.max_open_writers,
        row_group_rows: args.row_group_rows,
        max_output_files: args.max_output_files,
    };
    let run = convert_dataset(
        &inputs,
        &args.output_dir,
        &args.partition_by,
        &options,
        if args.continue_on_error {
            ErrorPolicy::Continue
        } else {
            ErrorPolicy::FailFast
        },
    )?;
    let summary = run.summary;
    writeln!(out, "files={}", summary.conversion.files)?;
    writeln!(out, "rows={}", summary.conversion.rows)?;
    writeln!(out, "fragments={}", summary.conversion.outputs.len())?;
    writeln!(out, "peak_open_writers={}", summary.peak_open_writers)?;
    writeln!(out, "peak_writer_bytes={}", summary.peak_writer_bytes)?;
    writeln!(out, "peak_pending_bytes={}", summary.peak_pending_bytes)?;
    for file in summary.conversion.outputs {
        writeln!(
            out,
            "output={} rows={}",
            file.output_path.display(),
            file.rows
        )?;
    }
    for failure in &run.failures {
        writeln!(out, "failed={} error={}", failure.path, failure.error)?;
    }
    if !run.failures.is_empty() {
        return Err(format!(
            "{} input(s) failed; see _catalog.json for details",
            run.failures.len()
        )
        .into());
    }
    Ok(())
}
