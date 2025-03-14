use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId};
use pico_macros::{memo, Source};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static FIRST_LETTER_AND_EXCLAMATION_POINT_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn single_chain_reuse() {
    FIRST_LETTER_COUNTER.store(0, Ordering::SeqCst);
    FIRST_LETTER_AND_EXCLAMATION_POINT_COUNTER.store(0, Ordering::SeqCst);

    let mut db = Database::default();

    let id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(*first_letter(&db, id), 'a');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    assert_eq!(
        *first_letter_and_exclamation_point(&db, id),
        "a!".to_string()
    );
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(
        FIRST_LETTER_AND_EXCLAMATION_POINT_COUNTER.load(Ordering::SeqCst),
        1
    );
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

#[memo]
fn first_letter_and_exclamation_point(db: &Database, input_id: SourceId<Input>) -> String {
    FIRST_LETTER_AND_EXCLAMATION_POINT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let capitalized_first_letter = *first_letter(db, input_id);
    format!("{capitalized_first_letter}!")
}
