use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId};
use pico_macros::{memo, Source};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn basic() {
    let mut db = Database::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(*first_letter(&db, input_id), 'a');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "qwer".to_string(),
    });

    assert_eq!(*first_letter(&db, input_id), 'q');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn first_letter(db: &Database, input_id: SourceId<Input>) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let input = db.get(input_id);
    input.value.chars().next().unwrap()
}
