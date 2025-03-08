use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId};
use pico_macros::{memo, Source};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn overriding_same_source() {
    let mut db = Database::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(*first_letter(&db, input_id), 'a');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(*first_letter(&db, input_id), 'a');
    // Since the source (which first_letter depends on) was updated,
    // we would expect that first_letter would need to be re-evaluated.
    // But, since we changed it to the same value, we don't re-evaluate.
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
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
