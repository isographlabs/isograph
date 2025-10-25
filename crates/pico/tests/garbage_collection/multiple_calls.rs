use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage};
use pico_macros::{Db, memo};

static A_COUNTER: AtomicUsize = AtomicUsize::new(0);
static B_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

impl TestDatabase {
    pub fn new() -> Self {
        Self {
            storage: Storage::new_with_capacity(2.try_into().unwrap()),
        }
    }
}

#[test]
fn multiple_calls() {
    // When we garbage collect, we will only keep the most recently called top-level field
    let mut db = TestDatabase::new();

    memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 1);
    memoized_b(&db);

    // Run GC. Even though memoized_b has been called twice, memoized_a is still retained,
    // because the internal LRU cache does not double-count the call to memoized_b
    db.run_garbage_collection();

    memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 1);
}

#[legacy_memo]
fn memoized_a(_db: &TestDatabase) -> char {
    A_COUNTER.fetch_add(1, Ordering::SeqCst);
    'a'
}

#[legacy_memo]
fn memoized_b(_db: &TestDatabase) -> char {
    B_COUNTER.fetch_add(1, Ordering::SeqCst);
    'b'
}
