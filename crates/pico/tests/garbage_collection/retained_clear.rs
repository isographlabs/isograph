use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage, clear_retain, retain};
use pico_macros::{Db, legacy_memo};

static A_COUNTER: AtomicUsize = AtomicUsize::new(0);
static B_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

impl TestDatabase {
    pub fn new() -> Self {
        Self {
            storage: Storage::new_with_capacity(1.try_into().unwrap()),
        }
    }
}

/// When a result is temporarily retained and then cleared, that does not
/// cause it to be retained unless it was actually most recently used
/// or the `RetainedQuery` was still existing at the time of garbage collection 
#[test]
fn clear_retained() {
    let mut db = TestDatabase::new();

    let memo_ref_a = memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 1);

    let retain_a = retain(&db, memo_ref_a);

    // Run GC. both are retained — b, because of the LRU cache, and a, because of
    // the temporary retain.
    db.run_garbage_collection();
    clear_retain(&db, retain_a);

    memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    let memo_ref_b = memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 1);
    let retain_b = retain(&db, memo_ref_b);
    clear_retain(&db, retain_b);

    memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    // Run GC. only a is retained — because of the LRU cache
    // The `retain_b` was already cleared so does not cause b to be retained
    db.run_garbage_collection();

    memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    let memo_ref_b = memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 2);
    let retain_b = retain(&db, memo_ref_b);
    clear_retain(&db, retain_b);

    // Run GC. only b is retained — because of the LRU cache
    // The `retain_b` was cleared but it was the most recently used
    db.run_garbage_collection();

    memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 2);

    memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 2);

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
