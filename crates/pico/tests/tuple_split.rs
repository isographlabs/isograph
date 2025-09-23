use pico::Storage;
use pico_macros::{Db, memo};

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn split_projects_each_tuple_value() {
    let db = TestDatabase::default();

    let pair = pair_value(&db);
    let (left, right) = pair.split();

    let _ = &*left;
    let _ = &*right;
}

#[memo]
fn pair_value(_db: &TestDatabase) -> (PanicOnClone, PanicOnClone) {
    (PanicOnClone, PanicOnClone)
}

#[derive(Debug, PartialEq, Eq)]
struct PanicOnClone;

impl Clone for PanicOnClone {
    fn clone(&self) -> Self {
        panic!("PanicOnClone should not be cloned");
    }
}
