use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, legacy_memo};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn basic() {
    let mut db = TestDatabase::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(*first_letter(&db, input_id).lookup(), 'a');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "qwer".to_string(),
    });

    assert_eq!(*first_letter(&db, input_id).lookup(), 'q');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
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
