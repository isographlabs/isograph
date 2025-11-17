use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, memo};

static RETURN_VALUE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn arg_reference() {
    let mut db = TestDatabase::default();

    let input_id = db.set(Input {
        key: "input",
        value: "asdf".to_string(),
    });

    first_letter(&db, input_id);

    // The returned value is a MemoRef<'_, ReturnValue>. The underlying
    // ReturnValue is not cloned.
    assert_eq!(RETURN_VALUE_COUNTER.load(Ordering::SeqCst), 0);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct ReturnValue(char);

impl Clone for ReturnValue {
    fn clone(&self) -> Self {
        RETURN_VALUE_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self(self.0)
    }
}

#[memo]
fn first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> ReturnValue {
    let input = db.get(input_id);
    ReturnValue(input.value.chars().next().unwrap())
}
