use pico::{Database, Storage};
use pico_macros::Db;

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn intern_then_intern_ref() {
    let db = TestDatabase::default();

    let memo_ref_1 = db.intern_value("foo".to_string());
    let val_ref = memo_ref_1.lookup(&db);
    let memo_ref_2 = db.intern_ref(val_ref);

    // This test ensures that this call does not panic.
    memo_ref_2.lookup(&db);
}
