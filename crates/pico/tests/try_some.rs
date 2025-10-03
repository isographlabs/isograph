use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, memo};

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn try_some_projects_some_value() {
    let mut db = TestDatabase::default();

    let id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    let result = maybe_first_letter(&db, id)
        .try_some()
        .expect("expected Some");
    assert_eq!(*result, 'a');
}

#[test]
fn try_some_handles_none() {
    let mut db = TestDatabase::default();

    let id = db.set(Input {
        key: "key",
        value: "".to_string(),
    });

    assert!(maybe_first_letter(&db, id).try_some().is_none());
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn maybe_first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> Option<char> {
    db.get(input_id).value.chars().next()
}

#[test]
fn try_some_never_clones_value() {
    let db = TestDatabase::default();

    let some_ref = some_value(&db);
    let inner = some_ref.try_some().expect("expected Some");
    let _ = &*inner;

    assert!(none_value(&db).try_some().is_none());
}

#[memo]
fn some_value(_db: &TestDatabase) -> Option<PanicOnClone> {
    Some(PanicOnClone)
}

#[memo]
fn none_value(_db: &TestDatabase) -> Option<PanicOnClone> {
    None
}

#[derive(Debug, PartialEq, Eq)]
struct PanicOnClone;

impl Clone for PanicOnClone {
    fn clone(&self) -> Self {
        panic!("PanicOnClone should not be cloned");
    }
}
