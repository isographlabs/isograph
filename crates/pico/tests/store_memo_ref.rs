use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, MemoRef, SourceId, Storage};
use pico_macros::{Db, Source, memo};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static FIRST_LETTER_AS_MEMO_REF_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn store_memo_ref() {
    let mut db = TestDatabase::default();

    let id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(**first_letter_as_memo_ref(&db, id), 'a');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(FIRST_LETTER_AS_MEMO_REF_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "alto".to_string(),
    });

    assert_eq!(**first_letter_as_memo_ref(&db, id), 'a');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
    assert_eq!(FIRST_LETTER_AS_MEMO_REF_COUNTER.load(Ordering::SeqCst), 1);
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
fn first_letter_as_memo_ref(db: &TestDatabase, input_id: SourceId<Input>) -> MemoRef<char> {
    FIRST_LETTER_AS_MEMO_REF_COUNTER.fetch_add(1, Ordering::SeqCst);
    first_letter(db, input_id)
}
