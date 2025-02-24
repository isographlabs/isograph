use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId};
use pico_macros::{memo, Source};

static INPUT_CLONE_COUNTER: AtomicUsize = AtomicUsize::new(0);
static ASSERT_INPUT_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn source_id_never_cloned() {
    let mut db = Database::default();
    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });
    assert_eq!(INPUT_CLONE_COUNTER.load(Ordering::SeqCst), 0);

    assert_input_cloned(&db, input_id);
    assert_eq!(ASSERT_INPUT_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "something else".to_string(),
    });
    assert_input_cloned(&db, input_id);
    assert_eq!(ASSERT_INPUT_COUNTER.load(Ordering::SeqCst), 2);

    db.set(Input {
        key: "key",
        value: "even more different".to_string(),
    });
    assert_input_cloned(&db, input_id);
    assert_eq!(ASSERT_INPUT_COUNTER.load(Ordering::SeqCst), 3);
}

#[derive(Debug, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

impl Clone for Input {
    fn clone(&self) -> Self {
        INPUT_CLONE_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            key: self.key,
            value: self.value.clone(),
        }
    }
}

#[memo]
fn assert_input_cloned(db: &Database, input_id: SourceId<Input>) {
    ASSERT_INPUT_COUNTER.fetch_add(1, Ordering::SeqCst);
    db.get(input_id);
    assert_eq!(INPUT_CLONE_COUNTER.load(Ordering::SeqCst), 0);
}
