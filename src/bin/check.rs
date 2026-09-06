use std::{env, fs::File, path::Path};
use stdf_parser::{StdfParser, StdfRecord, sanity::{full_engine, finding::Severity}};

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: check <file.stdf>");
        std::process::exit(1);
    });

    let file = File::open(&path).unwrap_or_else(|e| {
        eprintln!("Cannot open {path}: {e}");
        std::process::exit(1);
    });

    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  STDF Sanity Report");
    println!("  File : {}", Path::new(&path).file_name().unwrap_or_default().to_string_lossy());
    println!("  Size : {} bytes ({:.1} MB)", file_size, file_size as f64 / 1_048_576.0);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut engine = full_engine();
    let mut record_count     = 0u64;
    let mut parse_errors     = 0u64;
    let mut record_histogram = std::collections::HashMap::<String, u64>::new();
    let mut part_count       = 0u64;
    let mut test_count       = 0u64;
    let mut lot_id           = String::from("(not found)");
    let mut part_typ         = String::from("(not found)");
    let mut node_nam         = String::from("(not found)");

    let mut parser = StdfParser::new(file);
    while let Some(item) = parser.next() {
        match item {
            Err(e) => {
                eprintln!("[PARSE ERROR at +{:#010x}] {e}", parser.last_record_offset());
                parse_errors += 1;
                break;
            }
            Ok(rec) => {
                *record_histogram.entry(rec.record_type().to_string()).or_default() += 1;
                record_count += 1;

                // Collect high-level summary info
                if let StdfRecord::Mir(m) = &rec {
                    lot_id   = m.lot_id.clone().unwrap_or_default();
                    part_typ = m.part_typ.clone().unwrap_or_default();
                    node_nam = m.node_nam.clone().unwrap_or_default();
                }
                if matches!(&rec, StdfRecord::Prr(_)) { part_count += 1; }
                if matches!(&rec, StdfRecord::Ptr(_)) { test_count += 1; }

                engine.feed(&rec, parser.last_record_offset());
            }
        }
    }

    let report = engine.finish();

    // ── File summary ─────────────────────────────────────────────────────────
    println!("\n[ File Summary ]");
    println!("  Lot ID     : {lot_id}");
    println!("  Part type  : {part_typ}");
    println!("  Tester node: {node_nam}");
    println!("  Records    : {record_count}");
    println!("  Parts      : {part_count}");
    println!("  PTR tests  : {test_count}");
    if parse_errors > 0 {
        println!("  PARSE ERRORS: {parse_errors}  ← file may be truncated");
    }

    // ── Record type histogram ─────────────────────────────────────────────────
    println!("\n[ Record Histogram ]");
    let mut hist: Vec<_> = record_histogram.iter().collect();
    hist.sort_by(|a, b| b.1.cmp(a.1));
    for (typ, cnt) in &hist {
        println!("  {typ:<6} {cnt:>8}");
    }

    // ── Validation findings ───────────────────────────────────────────────────
    println!("\n[ Validation Findings ]  {}", report.summary());

    if report.is_clean() && parse_errors == 0 {
        println!("  ✓  No issues detected — file looks healthy.");
    } else {
        let order = [Severity::Fatal, Severity::Error, Severity::Warning, Severity::Info];
        for sev in &order {
            let findings: Vec<_> = report.findings.iter().filter(|f| &f.severity == sev).collect();
            if findings.is_empty() { continue; }
            let tag = match sev {
                Severity::Fatal   => "[FATAL  ]",
                Severity::Error   => "[ERROR  ]",
                Severity::Warning => "[WARNING]",
                Severity::Info    => "[INFO   ]",
            };
            for f in findings {
                let loc = match f.file_offset {
                    Some(o) => format!("+{o:#010x}"),
                    None    => "           ".to_string(),
                };
                let rec = f.record_type.unwrap_or("---");
                println!("  {tag} {rec:<6} {loc}  {}  »  {}", f.rule, f.message);
            }
        }
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if report.likely_aborted() || parse_errors > 0 {
        std::process::exit(2);
    }
}
