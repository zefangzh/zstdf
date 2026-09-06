//! Batch STDF sanity checker.
//!
//! Recursively scans a directory tree (designed for TB-scale datasets with
//! thousands of files), runs the parser + validation engine on every
//! `*.stdf` file **in-process** (no per-file subprocess spawn — critical at
//! this scale), and writes one consolidated text report.
//!
//! Usage:
//!   batch_check <root_dir> [report_path] [threads]
//!
//! Example:
//!   batch_check "D:\fab_data\lot123" report.txt 8

use std::{
    collections::VecDeque,
    env,
    fmt::Write as FmtWrite,
    fs::{self, File},
    io::Write as IoWrite,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use stdf_parser::{
    sanity::{finding::Severity, full_engine},
    StdfError, StdfParser, StdfRecord,
};

// ═══════════════════════════════════════════════════════════════════════════
// ── Per-file result ──────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug)]
struct FileResult {
    path: PathBuf,
    size: u64,
    #[allow(dead_code)] // kept for future per-file timing diagnostics in the report
    duration_ms: u128,
    lot_id: String,
    part_typ: String,
    records: u64,
    parts: u64,
    tests: u64,
    parse_error: Option<String>,
    fatal: usize,
    error: usize,
    warning: usize,
    info: usize,
    aborted: bool,
    top_issues: Vec<String>,
}

impl FileResult {
    fn classification(&self) -> &'static str {
        if self.parse_error.is_some() {
            "UNREADABLE"
        } else if self.aborted || self.fatal > 0 || self.error > 0 {
            "ISSUES"
        } else if self.warning > 0 {
            "WARNING"
        } else {
            "CLEAN"
        }
    }
}

fn check_one(path: &Path) -> FileResult {
    let start = Instant::now();
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            return FileResult {
                path: path.to_path_buf(),
                size,
                duration_ms: start.elapsed().as_millis(),
                lot_id: String::new(),
                part_typ: String::new(),
                records: 0,
                parts: 0,
                tests: 0,
                parse_error: Some(format!("cannot open: {e}")),
                fatal: 0,
                error: 0,
                warning: 0,
                info: 0,
                aborted: false,
                top_issues: vec![],
            };
        }
    };

    let mut engine = full_engine();
    let mut records = 0u64;
    let mut parts = 0u64;
    let mut tests = 0u64;
    let mut lot_id = String::new();
    let mut part_typ = String::new();
    let mut parse_error: Option<String> = None;

    let mut parser = StdfParser::new(file);
    while let Some(item) = parser.next() {
        match item {
            Err(e) => {
                parse_error = Some(fmt_parse_error(&e));
                break;
            }
            Ok(rec) => {
                records += 1;
                match &rec {
                    StdfRecord::Mir(m) => {
                        lot_id = m.lot_id.clone().unwrap_or_default();
                        part_typ = m.part_typ.clone().unwrap_or_default();
                    }
                    StdfRecord::Prr(_) => parts += 1,
                    StdfRecord::Ptr(_) => tests += 1,
                    _ => {}
                }
                engine.feed(&rec, parser.last_record_offset());
            }
        }
    }

    let report = engine.finish();
    let fatal = report.findings.iter().filter(|f| f.severity == Severity::Fatal).count();
    let error = report.findings.iter().filter(|f| f.severity == Severity::Error).count();
    let warning = report.findings.iter().filter(|f| f.severity == Severity::Warning).count();
    let info = report.findings.iter().filter(|f| f.severity == Severity::Info).count();
    let aborted = report.likely_aborted();

    let top_issues = report
        .findings
        .iter()
        .filter(|f| matches!(f.severity, Severity::Fatal | Severity::Error))
        .take(8)
        .map(|f| format!("{} » {}", f.rule, f.message))
        .collect();

    FileResult {
        path: path.to_path_buf(),
        size,
        duration_ms: start.elapsed().as_millis(),
        lot_id,
        part_typ,
        records,
        parts,
        tests,
        parse_error,
        fatal,
        error,
        warning,
        info,
        aborted,
        top_issues,
    }
}

fn fmt_parse_error(e: &StdfError) -> String {
    e.to_string()
}

// ═══════════════════════════════════════════════════════════════════════════
// ── Directory walk ───────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════

fn is_stdf_file(path: &Path) -> bool {
    path.extension()
        .map(|e| {
            let e = e.to_string_lossy().to_lowercase();
            e == "stdf" || e == "std"
        })
        .unwrap_or(false)
}

fn scan_dir(root: &Path, out: &mut Vec<(PathBuf, u64)>, scanned_dirs: &mut u64) {
    *scanned_dirs += 1;
    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[scan] cannot read {}: {e}", root.display());
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            scan_dir(&path, out, scanned_dirs);
        } else if file_type.is_file() && is_stdf_file(&path) {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            out.push((path, size));
            if out.len() % 5000 == 0 {
                println!("[scan] found {} STDF files so far...", out.len());
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ── Formatting helpers ───────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════

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
    let s = d.as_secs();
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

/// Minimal UTC "YYYY-MM-DD HH:MM:SS" formatter using only `std`
/// (avoids pulling in a chrono/time dependency for one timestamp).
fn format_now_utc() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
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

// ═══════════════════════════════════════════════════════════════════════════
// ── Main ─────────────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════════

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: batch_check <root_dir> [report_path] [threads]");
        std::process::exit(1);
    }
    let root = PathBuf::from(&args[1]);
    let report_path = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "stdf_batch_report.txt".to_string());
    // `.filter(|&n| n > 0)` rejects an explicit "0" (which would otherwise
    // spawn zero worker threads, silently skip every file, and still emit
    // a "successful" report claiming 0 files were scanned) and falls back
    // to the same sane default used when the argument is absent/invalid.
    let num_threads: usize = args
        .get(3)
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or_else(|| thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(16));

    if !root.is_dir() {
        eprintln!("Not a directory: {}", root.display());
        std::process::exit(1);
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  STDF Batch Sanity Check");
    println!("  Root    : {}", root.display());
    println!("  Report  : {report_path}");
    println!("  Threads : {num_threads}");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // ── Phase 1: scan ─────────────────────────────────────────────────────
    let scan_start = Instant::now();
    println!("\n[1/2] Scanning directory tree for *.stdf files...");
    let mut files = Vec::new();
    let mut dirs_scanned = 0u64;
    scan_dir(&root, &mut files, &mut dirs_scanned);
    let total_files = files.len();
    let total_bytes: u64 = files.iter().map(|(_, s)| *s).sum();
    println!(
        "      done in {} — {} files across {} directories, {} total",
        human_duration(scan_start.elapsed()),
        total_files,
        dirs_scanned,
        human_bytes(total_bytes)
    );

    if total_files == 0 {
        println!("\nNo .stdf files found under {}. Nothing to do.", root.display());
        return;
    }

    // ── Phase 2: parallel check ──────────────────────────────────────────
    println!("\n[2/2] Checking {total_files} files with {num_threads} worker threads...");

    let queue: Arc<Mutex<VecDeque<(PathBuf, u64)>>> = Arc::new(Mutex::new(files.into()));
    let (tx, rx) = mpsc::channel::<FileResult>();
    let bytes_done = Arc::new(AtomicU64::new(0));
    let files_done = Arc::new(AtomicU64::new(0));
    let check_start = Instant::now();

    let mut workers = Vec::with_capacity(num_threads);
    for _ in 0..num_threads {
        let queue = Arc::clone(&queue);
        let tx = tx.clone();
        let bytes_done = Arc::clone(&bytes_done);
        let files_done = Arc::clone(&files_done);
        workers.push(thread::spawn(move || loop {
            let next = { queue.lock().unwrap().pop_front() };
            let Some((path, size)) = next else { break };
            let result = std::panic::catch_unwind(|| check_one(&path)).unwrap_or_else(|_| FileResult {
                path: path.clone(),
                size,
                duration_ms: 0,
                lot_id: String::new(),
                part_typ: String::new(),
                records: 0,
                parts: 0,
                tests: 0,
                parse_error: Some("INTERNAL PANIC while parsing — file likely severely malformed".into()),
                fatal: 0,
                error: 0,
                warning: 0,
                info: 0,
                aborted: false,
                top_issues: vec![],
            });
            bytes_done.fetch_add(size, Ordering::Relaxed);
            files_done.fetch_add(1, Ordering::Relaxed);
            let _ = tx.send(result);
        }));
    }
    drop(tx); // let rx end once all workers' clones are dropped

    // Progress + collection on main thread
    let mut results: Vec<FileResult> = Vec::with_capacity(total_files);
    let mut last_report = Instant::now();
    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(res) => results.push(res),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if last_report.elapsed() >= Duration::from_secs(5) || results.len() == total_files {
            let done = files_done.load(Ordering::Relaxed);
            let bdone = bytes_done.load(Ordering::Relaxed);
            let elapsed = check_start.elapsed();
            let rate = bdone as f64 / elapsed.as_secs_f64().max(0.001);
            let eta = if rate > 0.0 {
                Duration::from_secs_f64(((total_bytes.saturating_sub(bdone)) as f64 / rate).max(0.0))
            } else {
                Duration::ZERO
            };
            println!(
                "      {done}/{total_files} files  |  {} / {}  |  elapsed {}  |  ETA {}",
                human_bytes(bdone),
                human_bytes(total_bytes),
                human_duration(elapsed),
                human_duration(eta)
            );
            last_report = Instant::now();
            if results.len() == total_files { break; }
        }
    }
    for w in workers {
        let _ = w.join();
    }

    let total_elapsed = check_start.elapsed();
    println!("      done in {}", human_duration(total_elapsed));

    // ── Build report ─────────────────────────────────────────────────────
    results.sort_by(|a, b| a.path.cmp(&b.path));

    let report_text = build_report(&root, &results, total_bytes, total_elapsed, num_threads);
    match File::create(&report_path).and_then(|mut f| f.write_all(report_text.as_bytes())) {
        Ok(()) => println!("\nReport written to: {report_path}"),
        Err(e) => eprintln!("\nFailed to write report to {report_path}: {e}"),
    }

    // ── Console summary ──────────────────────────────────────────────────
    print_console_summary(&results);
}

fn print_console_summary(results: &[FileResult]) {
    let clean = results.iter().filter(|r| r.classification() == "CLEAN").count();
    let warn = results.iter().filter(|r| r.classification() == "WARNING").count();
    let issues = results.iter().filter(|r| r.classification() == "ISSUES").count();
    let unreadable = results.iter().filter(|r| r.classification() == "UNREADABLE").count();

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  SUMMARY  ({} files)", results.len());
    println!("  ✓ Clean        : {clean}");
    println!("  ⚠ Warnings only: {warn}");
    println!("  ✗ Issues       : {issues}   (fatal/error findings or likely-aborted)");
    println!("  ☒ Unreadable   : {unreadable}   (parse failed outright)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if issues + unreadable > 0 {
        println!("\n  Files needing attention:");
        for r in results.iter().filter(|r| matches!(r.classification(), "ISSUES" | "UNREADABLE")) {
            println!("    [{}] {}", r.classification(), r.path.display());
        }
    }
}

fn build_report(
    root: &Path,
    results: &[FileResult],
    total_bytes: u64,
    elapsed: Duration,
    num_threads: usize,
) -> String {
    let mut out = String::new();
    let now = format_now_utc();

    let clean: Vec<_> = results.iter().filter(|r| r.classification() == "CLEAN").collect();
    let warn: Vec<_> = results.iter().filter(|r| r.classification() == "WARNING").collect();
    let issues: Vec<_> = results.iter().filter(|r| r.classification() == "ISSUES").collect();
    let unreadable: Vec<_> = results.iter().filter(|r| r.classification() == "UNREADABLE").collect();

    let total_records: u64 = results.iter().map(|r| r.records).sum();
    let total_parts: u64 = results.iter().map(|r| r.parts).sum();
    let total_tests: u64 = results.iter().map(|r| r.tests).sum();

    let _ = writeln!(out, "════════════════════════════════════════════════════════════════════");
    let _ = writeln!(out, "  STDF BATCH SANITY REPORT");
    let _ = writeln!(out, "════════════════════════════════════════════════════════════════════");
    let _ = writeln!(out, "  Generated : {now}");
    let _ = writeln!(out, "  Root path : {}", root.display());
    let _ = writeln!(out, "  Threads   : {num_threads}");
    let _ = writeln!(out, "  Duration  : {}", human_duration(elapsed));
    let _ = writeln!(out);
    let _ = writeln!(out, "── AGGREGATE STATISTICS ─────────────────────────────────────────────");
    let _ = writeln!(out, "  Files scanned      : {}", results.len());
    let _ = writeln!(out, "  Total size         : {}", human_bytes(total_bytes));
    let _ = writeln!(out, "  Total records      : {total_records}");
    let _ = writeln!(out, "  Total parts (PRR)  : {total_parts}");
    let _ = writeln!(out, "  Total PTR tests    : {total_tests}");
    let _ = writeln!(out);
    let _ = writeln!(out, "  ✓ Clean            : {}", clean.len());
    let _ = writeln!(out, "  ⚠ Warnings only    : {}", warn.len());
    let _ = writeln!(out, "  ✗ Issues (err/fatal): {}", issues.len());
    let _ = writeln!(out, "  ☒ Unreadable       : {}", unreadable.len());
    let _ = writeln!(out);

    // ── Aggregate finding-rule histogram ─────────────────────────────────
    let mut rule_hist: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for r in results {
        for issue in &r.top_issues {
            if let Some(rule) = issue.split(" » ").next() {
                *rule_hist.entry(rule.to_string()).or_default() += 1;
            }
        }
    }
    if !rule_hist.is_empty() {
        let mut hist: Vec<_> = rule_hist.into_iter().collect();
        hist.sort_by(|a, b| b.1.cmp(&a.1));
        let _ = writeln!(out, "── TOP FINDING RULES (fatal/error, across all files) ────────────────");
        for (rule, count) in hist.iter().take(15) {
            let _ = writeln!(out, "  {count:>6}  {rule}");
        }
        let _ = writeln!(out);
    }

    // ── Unreadable files (highest priority) ──────────────────────────────
    if !unreadable.is_empty() {
        let _ = writeln!(out, "══ UNREADABLE FILES ({}) — parser could not fully read these ═══════", unreadable.len());
        for r in &unreadable {
            let _ = writeln!(out, "\n  FILE: {}", r.path.display());
            let _ = writeln!(out, "    Size    : {}", human_bytes(r.size));
            let _ = writeln!(out, "    Records read before failure: {}", r.records);
            let _ = writeln!(out, "    Error   : {}", r.parse_error.as_deref().unwrap_or("unknown"));
            // Validation findings accumulated from the records that *did*
            // parse successfully before the failure are still meaningful
            // (e.g. they can show the file was already mid-abort even
            // before the hard parse error struck) — surface them here
            // rather than silently discarding them.
            if r.fatal + r.error + r.warning + r.info > 0 {
                let _ = writeln!(
                    out,
                    "    Findings before failure: {} fatal, {} error, {} warning, {} info",
                    r.fatal, r.error, r.warning, r.info
                );
                for issue in &r.top_issues {
                    let _ = writeln!(out, "      - {issue}");
                }
            }
        }
        let _ = writeln!(out);
    }

    // ── Issues (fatal/error/aborted) ──────────────────────────────────────
    if !issues.is_empty() {
        let _ = writeln!(out, "══ FILES WITH ISSUES ({}) — fatal/error findings or likely aborted ══", issues.len());
        for r in &issues {
            let _ = writeln!(out, "\n  FILE: {}", r.path.display());
            let _ = writeln!(
                out,
                "    Lot={}  PartType={}  Size={}  Records={}  Parts={}  Tests={}",
                if r.lot_id.is_empty() { "-" } else { &r.lot_id },
                if r.part_typ.is_empty() { "-" } else { &r.part_typ },
                human_bytes(r.size),
                r.records,
                r.parts,
                r.tests
            );
            let _ = writeln!(
                out,
                "    Findings: {} fatal, {} error, {} warning, {} info  |  aborted={}",
                r.fatal, r.error, r.warning, r.info, r.aborted
            );
            for issue in &r.top_issues {
                let _ = writeln!(out, "      - {issue}");
            }
        }
        let _ = writeln!(out);
    }

    // ── Warnings-only (compact one-liners) ────────────────────────────────
    if !warn.is_empty() {
        let _ = writeln!(out, "── FILES WITH WARNINGS ONLY ({}) ─────────────────────────────────────", warn.len());
        for r in &warn {
            let _ = writeln!(
                out,
                "  [{} warn] {}  (lot={}, records={}, parts={})",
                r.warning,
                r.path.display(),
                if r.lot_id.is_empty() { "-" } else { &r.lot_id },
                r.records,
                r.parts
            );
        }
        let _ = writeln!(out);
    }

    // ── Clean files (count + compact list) ────────────────────────────────
    let _ = writeln!(out, "── CLEAN FILES ({}) ──────────────────────────────────────────────────", clean.len());
    for r in &clean {
        let _ = writeln!(
            out,
            "  {}  (lot={}, records={}, parts={})",
            r.path.display(),
            if r.lot_id.is_empty() { "-" } else { &r.lot_id },
            r.records,
            r.parts
        );
    }

    let _ = writeln!(out, "\n════════════════════════════════════════════════════════════════════");
    let _ = writeln!(out, "  END OF REPORT");
    let _ = writeln!(out, "════════════════════════════════════════════════════════════════════");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(typ: u8, sub: u8, body: &[u8]) -> Vec<u8> {
        let mut v = Vec::with_capacity(4 + body.len());
        let len = body.len() as u16;
        v.extend_from_slice(&len.to_le_bytes());
        v.push(typ);
        v.push(sub);
        v.extend_from_slice(body);
        v
    }
    fn far_le() -> Vec<u8> { make_record(0, 10, &[2, 4]) } // CPU_TYP=2 (LE), STDF_VER=4
    fn mir_minimal() -> Vec<u8> {
        let mut b = vec![];
        b.extend_from_slice(&0u32.to_le_bytes()); // SETUP_T
        b.extend_from_slice(&1_000_000u32.to_le_bytes()); // START_T
        b.push(1); // STAT_NUM
        b.push(b' '); b.push(b' '); b.push(b' ');
        make_record(1, 10, &b)
    }
    fn wir(head: u8) -> Vec<u8> {
        let mut b = vec![];
        b.push(head);
        b.push(255); // SITE_GRP
        b.extend_from_slice(&0u32.to_le_bytes()); // START_T
        make_record(2, 10, &b)
    }

    /// Bug 7 regression: a file with a genuine parse error partway through
    /// must still surface the validation findings gathered from the
    /// records that *did* parse successfully before the failure — both on
    /// `FileResult` and in the rendered report text — instead of the report
    /// showing only the raw parse-error string.
    #[test]
    fn unreadable_file_report_includes_findings_gathered_before_the_parse_error() {
        let mut bytes = far_le();
        bytes.extend(mir_minimal());
        bytes.extend(wir(1)); // opened but deliberately never closed by a WRR
        // A record header that declares a 10-byte body but is immediately
        // followed by end-of-file — read_exact() on the body will fail
        // with UnexpectedEof, which the parser surfaces as a genuine parse
        // error (not just "file ended cleanly").
        bytes.extend_from_slice(&10u16.to_le_bytes());
        bytes.push(5); // PIR-ish TYP
        bytes.push(10); // SUB
        // no body bytes follow -> truncated

        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "stdf_batch_check_bug7_test_{}.stdf",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, &bytes).expect("failed to write temp test file");

        let result = check_one(&path);
        let _ = fs::remove_file(&path);

        assert!(result.parse_error.is_some(), "expected a genuine parse error to be recorded");
        assert_eq!(result.classification(), "UNREADABLE");
        assert!(
            result.fatal + result.error + result.warning + result.info > 0,
            "expected findings accumulated before the parse error (e.g. unclosed WIR) to be preserved on FileResult"
        );

        let report_text = build_report(
            Path::new("."),
            std::slice::from_ref(&result),
            result.size,
            Duration::from_secs(1),
            1,
        );
        assert!(
            report_text.contains("Findings before failure"),
            "expected the UNREADABLE section of the report to surface pre-failure findings, got:\n{report_text}"
        );
    }

    /// Bug 9 regression: an explicit thread-count argument of "0" must not
    /// be taken literally (which would spawn zero worker threads and
    /// silently skip every file); it should fall back to the same sane
    /// default used when no thread-count argument is given at all.
    #[test]
    fn zero_thread_count_argument_falls_back_to_a_positive_default() {
        let parse_threads = |arg: Option<&str>| -> usize {
            arg.and_then(|s| s.parse::<usize>().ok())
                .filter(|&n| n > 0)
                .unwrap_or_else(|| thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(16))
        };
        assert!(parse_threads(Some("0")) > 0, "an explicit 0 must not result in zero worker threads");
        assert_eq!(parse_threads(Some("4")), 4, "a valid explicit value must still be respected");
        assert!(parse_threads(None) > 0, "the absent-argument default must also be positive");
    }
}
