use std::sync::atomic::{AtomicUsize, Ordering};

use pico::Database;
use pico_macros::memo;

static A_COUNTER: AtomicUsize = AtomicUsize::new(0);
static B_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn basic() {
    // When we garbage collect, we will only keep the most recently called top-level field
    let mut db = Database::new_with_capacity(1.try_into().unwrap());

    memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    memoized_b(&db);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 1);

    // Run GC. memoized_b is retained, but not memoized_a.
    db.run_garbage_collection();

    memoized_a(&db);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 2);

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
