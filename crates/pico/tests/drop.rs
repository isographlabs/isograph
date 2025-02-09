use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId};
use pico_macros::{memo, Source};

static SECOND_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn drop() {
    // When we garbage collect, we will keep only the results from first_word
    // (which is called after first_letter).
    let mut db = Database::new_with_capacity(1.try_into().unwrap());

    let input = db.set(Input {
        key: "theword",
        value: "pico is awesome".to_string(),
    });

    let result = first_letter(&mut db, input);
    assert_eq!(result, 'p');
    let result = second_letter(&mut db, input);
    assert_eq!(result, 'i');

    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(SECOND_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    // Complete reuse
    let result = first_letter(&mut db, input);
    assert_eq!(result, 'p');
    let result = second_letter(&mut db, input);
    assert_eq!(result, 'i');

    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(SECOND_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    // GC will get rid of the cached value for first_letter
    db.garbage_collect();

    let result = first_letter(&mut db, input);
    assert_eq!(result, 'p');
    let result = second_letter(&mut db, input);
    assert_eq!(result, 'i');

    // First is re-calculated; second is not
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
    assert_eq!(SECOND_LETTER_COUNTER.load(Ordering::SeqCst), 1);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn first_letter(db: &Database, input: SourceId<Input>) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);

    let text = db.get(input).value;

    text.chars().next().expect("Should exist")
}

#[memo]
fn second_letter(db: &Database, input: SourceId<Input>) -> char {
    SECOND_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);

    let text = db.get(input).value;

    let mut chars = text.chars();
    chars.next();
    chars.next().expect("Should exist")
}
