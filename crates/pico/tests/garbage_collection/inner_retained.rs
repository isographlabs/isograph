use std::sync::atomic::{AtomicUsize, Ordering};

use pico::Database;
use pico_macros::memo;

static OUTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static INNER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn inner_retained() {
    // When we garbage collect, we will only keep the most recently called top-level field
    let mut db = Database::new_with_capacity(1.try_into().unwrap());

    outer(&db);
    assert_eq!(OUTER_COUNTER.load(Ordering::SeqCst), 1);

    inner(&db);
    assert_eq!(INNER_COUNTER.load(Ordering::SeqCst), 1);

    // Run GC. inner is retained, but not outer.
    db.run_garbage_collection();

    outer(&db);
    assert_eq!(OUTER_COUNTER.load(Ordering::SeqCst), 2);

    inner(&db);
    assert_eq!(INNER_COUNTER.load(Ordering::SeqCst), 1);
}

#[memo]
fn outer(db: &Database) -> &'static str {
    inner(db);
    OUTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    "outer"
}

#[memo]
fn inner(_db: &Database) -> &'static str {
    INNER_COUNTER.fetch_add(1, Ordering::SeqCst);
    "inner"
}
