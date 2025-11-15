use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, legacy_memo};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static CAPITALIZED_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static UNCHANGED_SUBTREE: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn side_chain() {
    FIRST_LETTER_COUNTER.store(0, Ordering::SeqCst);
    CAPITALIZED_LETTER_COUNTER.store(0, Ordering::SeqCst);

    let mut db = TestDatabase::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(
        *capitalized_first_letter(&db, input_id.clone()).lookup(),
        'A'
    );
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(CAPITALIZED_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(UNCHANGED_SUBTREE.load(Ordering::SeqCst), 1);

    // capitalized_first_letter is not re-executed
    // Every part was re-used so nothing has changed
    assert_eq!(*capitalized_first_letter(&db, input_id).lookup(), 'A');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(CAPITALIZED_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(UNCHANGED_SUBTREE.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "qwer".to_string(),
    });

    assert_eq!(*capitalized_first_letter(&db, input_id).lookup(), 'Q');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
    assert_eq!(CAPITALIZED_LETTER_COUNTER.load(Ordering::SeqCst), 2);

    // Even though capitalized_first_letter was re-executed, we re-used
    // the previous value returned from unchanged_subtree()
    assert_eq!(UNCHANGED_SUBTREE.load(Ordering::SeqCst), 1);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[legacy_memo]
fn first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let input = db.get(input_id);
    input.value.chars().next().unwrap()
}

#[legacy_memo]
fn capitalized_first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> char {
    CAPITALIZED_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    unchanged_subtree(db);
    let first = first_letter(db, input_id);
    first.lookup().to_ascii_uppercase()
}

#[legacy_memo]
fn unchanged_subtree(_db: &TestDatabase) -> &'static str {
    UNCHANGED_SUBTREE.fetch_add(1, Ordering::SeqCst);
    "this function should not be re-executed, \
    even if a parent has been re-executed"
}
