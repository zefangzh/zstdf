use std::fs;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use stdf_analytics::{Limits, SpillStore};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(std::path::PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "zstdf-spill-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn store(&self, limits: Limits) -> SpillStore {
        SpillStore::new(&self.0, limits, Arc::new(AtomicBool::new(false))).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn limits() -> Limits {
    Limits {
        memory_bytes: 4096,
        disk_bytes: 1_000_000,
        max_record_bytes: 256,
        merge_fan_in: 2,
    }
}

#[test]
fn empty_store_is_readable_and_cleans_up() {
    let f = Fixture::new();
    let mut sorted = f.store(limits()).finish().unwrap();
    assert_eq!(sorted.next_record().unwrap(), None);
    drop(sorted);
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 0);
}

#[test]
fn multiple_merge_passes_preserve_order_and_duplicate_arrival_order() {
    let f = Fixture::new();
    let mut store = f.store(limits());
    let mut expected = Vec::new();
    for n in (0u32..2000).rev() {
        let key = format!("{:03}", n % 17).into_bytes();
        let value = n.to_le_bytes().to_vec();
        expected.push((key.clone(), value.clone()));
        store.push(&key, &value).unwrap();
    }
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    let mut sorted = store.finish().unwrap();
    for (key, value) in expected {
        let row = sorted.next_record().unwrap().unwrap();
        assert_eq!((row.key, row.value), (key, value));
    }
    assert!(sorted.next_record().unwrap().is_none());
    assert!(sorted.peak_disk_bytes() <= limits().disk_bytes);
}

#[test]
fn arbitrary_binary_keys_and_values_round_trip() {
    let f = Fixture::new();
    let mut store = f.store(limits());
    for key in [vec![255, 0], vec![], vec![0], vec![0, 0]] {
        store.push(&key, &[0, 255, 10]).unwrap();
    }
    let mut sorted = store.finish().unwrap();
    for key in [vec![], vec![0], vec![0, 0], vec![255, 0]] {
        let row = sorted.next_record().unwrap().unwrap();
        assert_eq!(row.key, key);
        assert_eq!(row.value, [0, 255, 10]);
    }
}

#[test]
fn record_limit_rejects_before_retention() {
    let f = Fixture::new();
    let mut store = f.store(limits());
    assert!(store
        .push(&[0; 257], &[])
        .unwrap_err()
        .to_string()
        .contains("record limit"));
}

#[test]
fn disk_limit_failure_cleans_only_owned_directory() {
    let f = Fixture::new();
    fs::write(f.0.join("keep.html"), "previous dashboard").unwrap();
    let mut l = limits();
    l.disk_bytes = 10;
    let mut store = f.store(l);
    store.push(b"key", b"value").unwrap();
    assert!(store.finish().is_err());
    assert_eq!(
        fs::read_to_string(f.0.join("keep.html")).unwrap(),
        "previous dashboard"
    );
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 1);
}

#[test]
fn cancelled_ingestion_rejects_and_cleans_up() {
    let f = Fixture::new();
    let cancel = Arc::new(AtomicBool::new(false));
    let mut store = SpillStore::new(&f.0, limits(), cancel.clone()).unwrap();
    store.push(b"a", b"b").unwrap();
    cancel.store(true, Ordering::Relaxed);
    assert_eq!(
        store.push(b"c", b"d").unwrap_err().kind(),
        std::io::ErrorKind::Interrupted
    );
    drop(store);
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 0);
}

#[test]
fn cancelled_finish_preserves_neighbor_job() {
    let f = Fixture::new();
    let neighbor = f.store(limits());
    let cancel = Arc::new(AtomicBool::new(true));
    let store = SpillStore::new(&f.0, limits(), cancel).unwrap();
    assert!(store.finish().is_err());
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 1);
    drop(neighbor);
}

#[test]
fn replay_observes_cancellation() {
    let f = Fixture::new();
    let cancel = Arc::new(AtomicBool::new(false));
    let mut store = SpillStore::new(&f.0, limits(), cancel.clone()).unwrap();
    store.push(b"a", b"b").unwrap();
    let mut sorted = store.finish().unwrap();
    cancel.store(true, Ordering::Relaxed);
    assert_eq!(
        sorted.next_record().unwrap_err().kind(),
        std::io::ErrorKind::Interrupted
    );
}

#[test]
fn invalid_limits_create_no_scratch() {
    let f = Fixture::new();
    for l in [
        Limits {
            memory_bytes: 0,
            ..limits()
        },
        Limits {
            disk_bytes: 0,
            ..limits()
        },
        Limits {
            merge_fan_in: 1,
            ..limits()
        },
        Limits {
            max_record_bytes: usize::MAX,
            ..limits()
        },
        Limits {
            merge_fan_in: 100,
            ..limits()
        },
    ] {
        assert!(SpillStore::new(&f.0, l, Arc::new(AtomicBool::new(false))).is_err());
    }
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 0);
}

#[test]
fn missing_scratch_parent_is_not_created() {
    let f = Fixture::new();
    assert!(SpillStore::new(
        &f.0.join("missing"),
        limits(),
        Arc::new(AtomicBool::new(false))
    )
    .is_err());
}

#[test]
fn dropping_unfinished_store_cleans_flushed_runs() {
    let f = Fixture::new();
    let mut store = f.store(limits());
    for _ in 0..100 {
        store.push(b"a", &[1; 200]).unwrap();
    }
    drop(store);
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 0);
}
