use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, legacy_memo};

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn same_source_key() {
    let mut db = TestDatabase::default();

    let input_a = db.set(InputA {
        key: "source",
        value: "asdf".to_string(),
    });
    let input_b = db.set(InputB {
        key: "source",
        value: "this is b".to_string(),
    });

    assert_eq!(*memoized_a(&db, input_a).lookup(), 'a');
    assert_eq!(*memoized_b(&db, input_b).lookup(), 't');
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct InputA {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct InputB {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[legacy_memo]
fn memoized_a(db: &TestDatabase, input: SourceId<InputA>) -> char {
    db.get(input).value.chars().next().unwrap()
}

#[legacy_memo]
fn memoized_b(db: &TestDatabase, input: SourceId<InputB>) -> char {
    db.get(input).value.chars().next().unwrap()
}
