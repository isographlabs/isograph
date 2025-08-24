use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId, Storage};
use pico_macros::{memo, Db, Source};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    pub storage: Storage<Self>,
}

impl TestDatabase {
    #[memo]
    fn first_letter(&self, input_id: SourceId<Input>) -> char {
        FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
        let input = self.get(input_id);
        input.value.chars().next().unwrap()
    }
}

#[test]
fn memo_on_db_method() {
    let mut db = TestDatabase::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(*db.first_letter(input_id), 'a');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "qwer".to_string(),
    });

    assert_eq!(*db.first_letter(input_id), 'q');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}
