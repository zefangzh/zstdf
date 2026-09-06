use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use stdf_core::{records, StdfRecord};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args(env::args().skip(1))?;
    let metrics = run_pipeline(&config)?;
    metrics.print();
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    parts: usize,
    tests_per_part: usize,
    batch_size: usize,
    output: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            parts: 10_000,
            tests_per_part: 5,
            batch_size: 65_536,
            output: None,
        }
    }
}

impl Config {
    fn from_args(args: impl Iterator<Item = String>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Self::default();
        let mut args = args.peekable();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--parts" => config.parts = parse_next(&mut args, "--parts")?,
                "--tests-per-part" => {
                    config.tests_per_part = parse_next(&mut args, "--tests-per-part")?;
                }
                "--batch-size" => config.batch_size = parse_next(&mut args, "--batch-size")?,
                "--output" => {
                    config.output = Some(PathBuf::from(next_string(&mut args, "--output")?))
                }
                "--help" | "-h" => print_help_and_exit(),
                unknown => return Err(format!("unknown argument: {unknown}").into()),
            }
        }

        config.batch_size = config.batch_size.max(1);
        Ok(config)
    }
}

#[derive(Debug)]
struct Metrics {
    parts: usize,
    tests_per_part: usize,
    input_bytes: usize,
    parquet_bytes: Option<u64>,
    records: usize,
    rows: usize,
    batches: usize,
    generate_elapsed: Duration,
    parse_elapsed: Duration,
    arrow_elapsed: Duration,
    parquet_write_elapsed: Duration,
    parquet_close_elapsed: Duration,
}

impl Metrics {
    fn print(&self) {
        let total_elapsed = self.generate_elapsed
            + self.parse_elapsed
            + self.arrow_elapsed
            + self.parquet_write_elapsed
            + self.parquet_close_elapsed;
        println!("zstdf parquet performance");
        println!("parts={}", self.parts);
        println!("tests_per_part={}", self.tests_per_part);
        println!("input_bytes={}", self.input_bytes);
        if let Some(parquet_bytes) = self.parquet_bytes {
            println!("parquet_bytes={parquet_bytes}");
        }
        println!("records={}", self.records);
        println!("rows={}", self.rows);
        println!("batches={}", self.batches);
        println!(
            "generate_elapsed_ms={:.3}",
            self.generate_elapsed.as_secs_f64() * 1000.0
        );
        println!(
            "parse_elapsed_ms={:.3}",
            self.parse_elapsed.as_secs_f64() * 1000.0
        );
        println!(
            "arrow_elapsed_ms={:.3}",
            self.arrow_elapsed.as_secs_f64() * 1000.0
        );
        println!(
            "parquet_write_elapsed_ms={:.3}",
            self.parquet_write_elapsed.as_secs_f64() * 1000.0
        );
        println!(
            "parquet_close_elapsed_ms={:.3}",
            self.parquet_close_elapsed.as_secs_f64() * 1000.0
        );
        println!(
            "total_elapsed_ms={:.3}",
            total_elapsed.as_secs_f64() * 1000.0
        );
        println!(
            "rows_per_sec={:.0}",
            self.rows as f64 / total_elapsed.as_secs_f64().max(1e-9)
        );
    }
}

fn run_pipeline(config: &Config) -> Result<Metrics, Box<dyn std::error::Error>> {
    let generate_start = Instant::now();
    let data = build_synthetic_stdf(config.parts, config.tests_per_part);
    let generate_elapsed = generate_start.elapsed();

    let parse_start = Instant::now();
    let parsed = records(&data)?.collect::<stdf_core::Result<Vec<StdfRecord>>>()?;
    let parse_elapsed = parse_start.elapsed();
    let record_count = parsed.len();

    let arrow_start = Instant::now();
    let batches = stdf_arrow::records_to_batches(parsed.into_iter().map(Ok), config.batch_size)?;
    let arrow_elapsed = arrow_start.elapsed();
    let rows = batches.iter().map(RecordBatch::num_rows).sum();

    let (parquet_bytes, parquet_write_elapsed, parquet_close_elapsed) =
        if let Some(output) = &config.output {
            let file = File::create(output)?;
            let (write_elapsed, close_elapsed) = time_parquet_write(file, &batches)?;
            let parquet_bytes = std::fs::metadata(output)?.len();
            (Some(parquet_bytes), write_elapsed, close_elapsed)
        } else {
            let mut buffer = Vec::new();
            let (write_elapsed, close_elapsed) = time_parquet_write(&mut buffer, &batches)?;
            (Some(buffer.len() as u64), write_elapsed, close_elapsed)
        };

    Ok(Metrics {
        parts: config.parts,
        tests_per_part: config.tests_per_part,
        input_bytes: data.len(),
        parquet_bytes,
        records: record_count,
        rows,
        batches: batches.len(),
        generate_elapsed,
        parse_elapsed,
        arrow_elapsed,
        parquet_write_elapsed,
        parquet_close_elapsed,
    })
}

fn time_parquet_write<W: Write + Send>(
    writer: W,
    batches: &[RecordBatch],
) -> Result<(Duration, Duration), Box<dyn std::error::Error>> {
    let schema = batches
        .first()
        .map(RecordBatch::schema)
        .unwrap_or_else(stdf_arrow::eav_schema);
    let mut writer = ArrowWriter::try_new(writer, schema, None)?;
    let mut write_elapsed = Duration::ZERO;

    for batch in batches {
        let write_start = Instant::now();
        writer.write(batch)?;
        write_elapsed += write_start.elapsed();
    }

    let close_start = Instant::now();
    writer.close()?;
    let close_elapsed = close_start.elapsed();

    Ok((write_elapsed, close_elapsed))
}

fn build_synthetic_stdf(parts: usize, tests_per_part: usize) -> Vec<u8> {
    let mut data = Vec::new();
    push_record(&mut data, 0, 10, &[2, 4]);
    push_record(&mut data, 1, 10, &mir_body("PERF_LOT"));

    for part in 0..parts {
        let site = (part % 8) as u8;
        push_record(&mut data, 5, 10, &[1, site]);
        for test in 0..tests_per_part {
            let test_num = (part * tests_per_part + test) as u32;
            push_record(
                &mut data,
                15,
                10,
                &ptr_body(test_num, site, test as f32 + 0.25),
            );
        }
        push_record(
            &mut data,
            5,
            20,
            &prr_body(site, tests_per_part as u16, part),
        );
    }

    data
}

fn mir_body(lot_id: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.push(1);
    body.push(b'P');
    body.push(b' ');
    body.push(b' ');
    body.extend_from_slice(&0u16.to_le_bytes());
    body.push(b' ');
    push_cn(&mut body, lot_id);
    push_cn(&mut body, "DEVICE");
    push_cn(&mut body, "NODE");
    push_cn(&mut body, "T");
    push_cn(&mut body, "JOB");
    body
}

fn ptr_body(test_num: u32, site: u8, result: f32) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&test_num.to_le_bytes());
    body.push(1);
    body.push(site);
    body.push(0);
    body.push(0);
    body.extend_from_slice(&result.to_le_bytes());
    body
}

fn prr_body(site: u8, num_tests: u16, part: usize) -> Vec<u8> {
    let mut body = Vec::new();
    body.push(1);
    body.push(site);
    body.push(0);
    body.extend_from_slice(&num_tests.to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&((part % i16::MAX as usize) as i16).to_le_bytes());
    body.extend_from_slice(&((site as i16) * 10).to_le_bytes());
    body.extend_from_slice(&(num_tests as u32).to_le_bytes());
    push_cn(&mut body, &format!("P{part}"));
    body
}

fn push_cn(buf: &mut Vec<u8>, value: &str) {
    assert!(value.len() <= u8::MAX as usize);
    buf.push(value.len() as u8);
    buf.extend_from_slice(value.as_bytes());
}

fn push_record(buf: &mut Vec<u8>, typ: u8, sub: u8, body: &[u8]) {
    buf.extend_from_slice(&(body.len() as u16).to_le_bytes());
    buf.push(typ);
    buf.push(sub);
    buf.extend_from_slice(body);
}

fn parse_next<T: std::str::FromStr>(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    name: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    next_string(args, name)?
        .parse()
        .map_err(|_| format!("{name} value is invalid").into())
}

fn next_string(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    args.next()
        .ok_or_else(|| format!("{name} requires a value").into())
}

fn print_help_and_exit() -> ! {
    println!(
        "Usage: cargo run -p stdf-parquet --bin parquet_perf -- [--parts N] [--tests-per-part N] [--batch-size N] [--output path.parquet]"
    );
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config_arguments() {
        let config = Config::from_args(
            [
                "--parts",
                "12",
                "--tests-per-part",
                "3",
                "--batch-size",
                "0",
                "--output",
                "out.parquet",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .unwrap();

        assert_eq!(config.parts, 12);
        assert_eq!(config.tests_per_part, 3);
        assert_eq!(config.batch_size, 1);
        assert_eq!(config.output, Some(PathBuf::from("out.parquet")));
    }

    #[test]
    fn synthetic_stdf_produces_expected_record_count() {
        let data = build_synthetic_stdf(4, 2);
        let records = records(&data)
            .unwrap()
            .collect::<stdf_core::Result<Vec<_>>>()
            .unwrap();

        assert_eq!(records.len(), 2 + 4 * 4);
    }

    #[test]
    fn pipeline_reports_expected_rows_and_batches() {
        let metrics = run_pipeline(&Config {
            parts: 3,
            tests_per_part: 2,
            batch_size: 2,
            output: None,
        })
        .unwrap();

        assert_eq!(metrics.rows, 6);
        assert_eq!(metrics.batches, 3);
        assert_eq!(metrics.records, 14);
        assert!(metrics.parquet_bytes.unwrap() > 0);
    }
}
