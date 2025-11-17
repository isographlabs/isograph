use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{MemoRef, Storage};
use pico_macros::{Db, memo};

static OUTPUT_CLONE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn memo_ref_never_cloned() {
    let db = TestDatabase::default();

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

#[memo(raw)]
fn get_output(_db: &TestDatabase) -> Output {
    Output {}
}

#[memo]
fn consume_output(_db: &TestDatabase, _output: MemoRef<Output>) {}
