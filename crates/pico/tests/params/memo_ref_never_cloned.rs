use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, MemoRef};
use pico_macros::memo;

static OUTPUT_CLONE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn memo_ref_never_cloned() {
    let db = Database::default();

    let output = get_output(&db);

    consume_output(&db, output);
    assert_eq!(OUTPUT_CLONE_COUNTER.load(Ordering::SeqCst), 0);

    // We cannot mutate the db while the MemoRef exists, since it
    // contains a reference to 'db.
}

#[derive(PartialEq, Eq)]
struct Output {}

impl Clone for Output {
    fn clone(&self) -> Self {
        OUTPUT_CLONE_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {}
    }
}

#[memo]
fn get_output(_db: &Database) -> Output {
    Output {}
}

#[memo]
fn consume_output<'db1>(db: &'db1 Database, _output: MemoRef<'_, Output>) {}
