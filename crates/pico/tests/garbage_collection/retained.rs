use std::sync::atomic::{AtomicUsize, Ordering};

use pico::Database;
use pico_macros::memo;

static A_COUNTER: AtomicUsize = AtomicUsize::new(0);
static B_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn basic_retained() {
    // When we garbage collect, we will only keep the most recently called top-level field
    let mut db = Database::new_with_capacity(1.try_into().unwrap());

    let memo_ref_a = memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 1);

    let retain = db.retain(&memo_ref_a);
    retain.permanently_retain_query();

    // Run GC. both are retained — b, because of the LRU cache, and a, because of
    // the permanent retain.
    db.run_garbage_collection();

    memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 1);
}

#[memo]
fn memoized_a(_db: &Database) -> char {
    A_COUNTER.fetch_add(1, Ordering::SeqCst);
    'a'
}

#[memo]
fn memoized_b(_db: &Database) -> char {
    B_COUNTER.fetch_add(1, Ordering::SeqCst);
    'b'
}
