use pico::{Database, MemoRef, SourceId, Storage};
use pico_macros::{Db, Source, memo};
use std::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;

static TRACK_CLONE_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn try_ok_projects_ok_value() {
    let mut db = TestDatabase::default();

    let id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(*process_input(&db, id).unwrap(), 'a');
}

#[test]
fn projected_memo_correctly_restored() {
    let mut db = TestDatabase::default();

    let id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    let result = process_input(&db, id).unwrap();
    assert_eq!(*consume_result(&db, result), 'a');
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

#[memo]
fn first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> Result<char, FirstLetterError> {
    db.get(input_id)
        .value
        .chars()
        .next()
        .ok_or(FirstLetterError::EmptyString)
}

fn process_input(
    db: &TestDatabase,
    input_id: SourceId<Input>,
) -> Result<MemoRef<char>, ProcessInputError> {
    let result = first_letter(db, input_id).try_ok()?;
    Ok(result)
}

#[memo]
fn consume_result(db: &TestDatabase, first_letter: MemoRef<char>) -> char {
    let result = *first_letter;
    result
}

#[test]
fn try_ok_never_clones_ok_value() {
    let db = TestDatabase::default();

    let result = ok_value(&db).try_ok().expect("expected ok result");
    let _ = &*result;

    match err_value(&db).try_ok() {
        Ok(_) => panic!("expected error"),
        Err(_err) => {}
    }

    assert_eq!(TRACK_CLONE_COUNT.load(Ordering::SeqCst), 1);
}

#[memo]
fn ok_value(_db: &TestDatabase) -> Result<PanicOnClone, TrackCloneError> {
    Ok(PanicOnClone)
}

#[memo]
fn err_value(_db: &TestDatabase) -> Result<PanicOnClone, TrackCloneError> {
    Err(TrackCloneError::Count)
}

#[derive(Debug, PartialEq, Eq)]
struct PanicOnClone;

impl Clone for PanicOnClone {
    fn clone(&self) -> Self {
        panic!("PanicOnClone should not be cloned");
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
enum TrackCloneError {
    #[error("error cloned")]
    Count,
}

impl Clone for TrackCloneError {
    fn clone(&self) -> Self {
        TRACK_CLONE_COUNT.fetch_add(1, Ordering::SeqCst);
        TrackCloneError::Count
    }
}
