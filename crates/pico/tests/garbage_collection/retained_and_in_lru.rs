use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{retain, Database, Storage};
use pico_macros::{memo, Db};

static A_COUNTER: AtomicUsize = AtomicUsize::new(0);
static B_COUNTER: AtomicUsize = AtomicUsize::new(0);

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
fn basic_retained() {
    // When we garbage collect, we will only keep the most recently called top-level field
    let mut db = TestDatabase::new();

    let memo_ref_a = memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 1);

    let retain = retain(&db, memo_ref_a);
    retain.never_garbage_collect();

    memoized_a(&db);

    // Run GC. Only a is retained. It's present in the LRU cache and is permanently retained.
    db.run_garbage_collection();

    memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 2);
}

#[memo]
fn memoized_a(_db: &TestDatabase) -> char {
    A_COUNTER.fetch_add(1, Ordering::SeqCst);
    'a'
}

#[memo]
fn memoized_b(_db: &TestDatabase) -> char {
    B_COUNTER.fetch_add(1, Ordering::SeqCst);
    'b'
}
