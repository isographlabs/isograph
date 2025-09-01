use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage};
use pico_macros::{Db, memo};

static OUTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static INNER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    pub storage: Storage<Self>,
}

impl TestDatabase {
    pub fn new() -> Self {
        Self {
            storage: Storage::new_with_capacity(1.try_into().unwrap()),
        }
    }
}

#[test]
fn outer_retained() {
    // When we garbage collect, we will only keep the most recently called top-level field
    let mut db = TestDatabase::new();

    inner(&db);
    assert_eq!(INNER_COUNTER.load(Ordering::SeqCst), 1);

    outer(&db);
    assert_eq!(OUTER_COUNTER.load(Ordering::SeqCst), 1);

    // Run GC. outer is retained directly, and inner is retained because it is reachable from outer.
    db.run_garbage_collection();

    outer(&db);
    assert_eq!(OUTER_COUNTER.load(Ordering::SeqCst), 1);

    inner(&db);
    assert_eq!(INNER_COUNTER.load(Ordering::SeqCst), 1);
}

#[memo]
fn outer(db: &TestDatabase) -> &'static str {
    inner(db);
    OUTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    "outer"
}

#[memo]
fn inner(_db: &TestDatabase) -> &'static str {
    INNER_COUNTER.fetch_add(1, Ordering::SeqCst);
    "inner"
}
