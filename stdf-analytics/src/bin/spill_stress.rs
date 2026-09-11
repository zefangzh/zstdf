//! Repeatable external-sort stress smoke; no STDF input or dashboard is changed.
use std::sync::{atomic::AtomicBool, Arc};
use stdf_analytics::{Limits, SpillStore};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let rows: u64 = args.next().as_deref().unwrap_or("100000").parse()?;
    if args.next().is_some() {
        return Err("usage: spill_stress [rows]".into());
    }
    let started = std::time::Instant::now();
    let mut store = SpillStore::new(
        &std::env::temp_dir(),
        Limits {
            memory_bytes: 1024 * 1024,
            disk_bytes: 256 * 1024 * 1024,
            max_record_bytes: 1024,
            merge_fan_in: 8,
        },
        Arc::new(AtomicBool::new(false)),
    )?;
    for n in (0..rows).rev() {
        store.push(&(n % 1009).to_be_bytes(), &n.to_le_bytes())?;
    }
    let mut sorted = store.finish()?;
    let mut previous: Option<(u64, u64)> = None;
    let mut actual = 0;
    while let Some(record) = sorted.next_record()? {
        let key = u64::from_be_bytes(record.key.try_into().map_err(|_| "invalid key")?);
        let value = u64::from_le_bytes(record.value.try_into().map_err(|_| "invalid value")?);
        if let Some((k, v)) = previous {
            if key < k || (key == k && value >= v) {
                return Err("unstable sort order".into());
            }
        }
        previous = Some((key, value));
        actual += 1;
    }
    if actual != rows {
        return Err("row count mismatch".into());
    }
    println!(
        "rows={actual} peak_scratch_bytes={} accounted_memory_bytes=1048576 elapsed_ms={}",
        sorted.peak_disk_bytes(),
        started.elapsed().as_millis()
    );
    let path = sorted.scratch_path().to_owned();
    drop(sorted);
    if path.exists() {
        return Err("owned scratch cleanup failed".into());
    }
    Ok(())
}
