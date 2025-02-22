use pico::{Database, SourceId};
use pico_macros::{memo, Source};

#[test]
fn same_source_key() {
    let mut db = Database::default();

    let input_a = db.set(InputA {
        key: "source",
        value: "asdf".to_string(),
    });
    let input_b = db.set(InputB {
        key: "source",
        value: "this is b".to_string(),
    });

    assert_eq!(*memoized_a(&db, input_a), 'a');
    assert_eq!(*memoized_b(&db, input_b), 't');
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

#[memo]
fn memoized_a(db: &Database, input: SourceId<InputA>) -> char {
    db.get(input).value.chars().next().unwrap()
}

#[memo]
fn memoized_b(db: &Database, input: SourceId<InputB>) -> char {
    db.get(input).value.chars().next().unwrap()
}
