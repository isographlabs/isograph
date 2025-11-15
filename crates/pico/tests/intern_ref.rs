use pico::{Database, MemoRef, SourceId, Storage};
use pico_macros::{Db, Source, legacy_memo};
use thiserror::Error;

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn intern_ref() {
    let mut db = TestDatabase::default();

    let id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    let memoized_result = process_input(&db, id);
    let memo_ref = *memoized_result.lookup(&db).as_ref().unwrap();
    assert_eq!(*memo_ref.lookup(&db), 'a');
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
enum FirstLetterError {
    #[error("empty string")]
    EmptyString,
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
enum ProcessInputError {
    #[error("cannot process input")]
    ReadError(#[from] FirstLetterError),
}

#[legacy_memo]
fn first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> Result<char, FirstLetterError> {
    db.get(input_id)
        .value
        .chars()
        .next()
        .ok_or(FirstLetterError::EmptyString)
}

#[legacy_memo]
fn process_input(
    db: &TestDatabase,
    input_id: SourceId<Input>,
) -> Result<MemoRef<char>, ProcessInputError> {
    let result = first_letter(db, input_id).try_lookup(db)?;
    Ok(pico::intern_ref(db, result))
}
