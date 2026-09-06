//! Parses an STDF file and exports it as a human-readable ASCII text dump,
//! matching the output format of the reference
//! `/opt/hp93000/soc/formatter/bin/STDFreader` tool (see `src/ascii.rs` for
//! the format notes, and `samples/*.txt` for golden-truth reference output).
//!
//! Usage:
//!   to_ascii [--debug|--helper] <file.stdf> [output.txt]
//!
//! If `output.txt` is omitted, the input file's extension is replaced with
//! `.txt` (e.g. `foo.stdf` -> `foo.txt`).
//!
//! `--debug` / `--helper` (`-d`) prints conversion timestamp, STDF size,
//! and duration to stderr. The ASCII dump itself is unchanged.

use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use stdf_parser::{ascii::AsciiDumper, StdfParser};

fn main() {
    let (debug, positional) = parse_args(env::args().skip(1));
    if positional.is_empty() {
        eprintln!("Usage: to_ascii [--debug|--helper] <file.stdf> [output.txt]");
        std::process::exit(1);
    }
    let in_path = PathBuf::from(&positional[0]);
    let out_path = positional.get(1).map(PathBuf::from).unwrap_or_else(|| in_path.with_extension("txt"));

    let file = File::open(&in_path).unwrap_or_else(|e| {
        eprintln!("Cannot open {}: {e}", in_path.display());
        std::process::exit(1);
    });
    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);

    let out_file = File::create(&out_path).unwrap_or_else(|e| {
        eprintln!("Cannot create {}: {e}", out_path.display());
        std::process::exit(1);
    });
    let mut out = BufWriter::new(out_file);

    let file_name = Path::new(&in_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| in_path.display().to_string());

    let converted_at = format_now_utc();
    let started = Instant::now();

    write!(out, "{}", AsciiDumper::header(&file_name)).expect("write failed");

    let mut dumper = AsciiDumper::new();
    let mut parser = StdfParser::new(file);
    let mut record_count = 0u64;
    let mut had_error = false;

    while let Some(item) = parser.next() {
        match item {
            Ok(rec) => {
                out.write_all(dumper.render(&rec).as_bytes()).expect("write failed");
                record_count += 1;
            }
            Err(e) => {
                eprintln!(
                    "[PARSE ERROR at +{:#010x}] {e} — {record_count} record(s) written before failure",
                    parser.last_record_offset()
                );
                had_error = true;
                break;
            }
        }
    }

    write!(out, "{}", AsciiDumper::footer()).expect("write failed");
    out.flush().expect("flush failed");

    println!(
        "{} record(s) written to {}",
        record_count,
        out_path.display()
    );

    if debug {
        let elapsed = started.elapsed();
        eprintln!("  Converted at : {converted_at}");
        eprintln!("  STDF size    : {} ({} bytes)", human_bytes(file_size), file_size);
        eprintln!("  Duration     : {}", human_duration(elapsed));
    }

    if had_error {
        std::process::exit(2);
    }
}

fn parse_args<I>(args: I) -> (bool, Vec<String>)
where
    I: IntoIterator<Item = String>,
{
    let mut debug = false;
    let mut positional = Vec::new();
    let mut end_of_flags = false;
    for arg in args {
        if !end_of_flags {
            match arg.as_str() {
                "--" => {
                    end_of_flags = true;
                    continue;
                }
                "--debug" | "--helper" | "-d" => {
                    debug = true;
                    continue;
                }
                "-h" | "--help" => {
                    eprintln!(
                        "Usage: to_ascii [--debug|--helper] <file.stdf> [output.txt]\n\
                         \n\
                         --debug, --helper, -d\n\
                             Print conversion timestamp, STDF size, and duration to stderr."
                    );
                    std::process::exit(0);
                }
                s if s.starts_with('-') => {
                    eprintln!("Unknown flag: {s}");
                    eprintln!("Usage: to_ascii [--debug|--helper] <file.stdf> [output.txt]");
                    std::process::exit(1);
                }
                _ => {}
            }
        }
        positional.push(arg);
    }
    (debug, positional)
}

fn human_bytes(b: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = b as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    format!("{v:.2} {}", UNITS[u])
}

fn human_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{ms} ms")
    } else {
        format!("{:.3} s", d.as_secs_f64())
    }
}

/// Minimal UTC "YYYY-MM-DD HH:MM:SS UTC" formatter using only `std`
/// (avoids pulling in a chrono/time dependency for one timestamp).
fn format_now_utc() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let days = secs / 86_400;
    let time_of_day = secs % 86_400;
    let (hh, mm, ss) = (time_of_day / 3600, (time_of_day % 3600) / 60, time_of_day % 60);

    // Civil-from-days algorithm (Howard Hinnant), proleptic Gregorian, UTC.
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02} UTC")
}
