use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, memo};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn equal_reset_after_unrelated_epoch_reruns_bug() {
    let mut db = TestDatabase::default();

    let input_id = db.set(Input {
        key: "input",
        value: "asdf".to_string(),
    });
    db.set(Input {
        key: "unrelated",
        value: "first".to_string(),
    });

    assert_eq!(*first_letter(&db, input_id), 'a');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    // An unequal set of an existing source advances the epoch.
    db.set(Input {
        key: "unrelated",
        value: "second".to_string(),
    });
    db.set(Input {
        key: "input",
        value: "asdf".to_string(),
    });

    assert_eq!(*first_letter(&db, input_id), 'a');
    // set_source's equal branch sets time_updated = current_epoch (database.rs); the fix flips this assertion.
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let input = db.get(input_id);
    input.value.chars().next().unwrap()
}
