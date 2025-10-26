use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId, Storage, WithSerialize};
use pico_macros::{Db, Source, legacy_memo};
use serde::Serialize;

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static PARAM_CLONE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn with_serialize() {
    let mut db = TestDatabase::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    accepts_param_with_serialize(&db, input_id, WithSerialize(Param {}));
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "qwer".to_string(),
    });

    accepts_param_with_serialize(&db, input_id, WithSerialize(Param {}));
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
    assert_eq!(PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 2);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[derive(Serialize)]
struct Param {}

impl Clone for Param {
    fn clone(&self) -> Self {
        PARAM_CLONE_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {}
    }
}

#[legacy_memo]
fn accepts_param_with_serialize(
    db: &TestDatabase,
    input_id: SourceId<Input>,
    _param: WithSerialize<Param>,
) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let input = db.get(input_id);
    input.value.chars().next().unwrap()
}
