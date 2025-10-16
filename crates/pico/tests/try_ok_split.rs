use pico::Storage;
use pico_macros::{Db, memo};

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn try_ok_then_split_projects_each_tuple_value() {
    let db = TestDatabase::default();

    let (left, right) = ok_pair_value(&db)
        .try_ok()
        .expect("expected Ok result before split")
        .split();

    let _ = &*left;
    let _ = &*right;
}

#[memo]
fn ok_pair_value(_db: &TestDatabase) -> Result<(PanicOnClone, PanicOnClone), ()> {
    Ok((PanicOnClone, PanicOnClone))
}

#[derive(Debug, PartialEq, Eq)]
struct PanicOnClone;

impl Clone for PanicOnClone {
    fn clone(&self) -> Self {
        panic!("PanicOnClone should not be cloned");
    }
}
