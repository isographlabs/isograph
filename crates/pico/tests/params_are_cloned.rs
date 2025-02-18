use std::sync::{
    atomic::{AtomicUsize, Ordering},
    LazyLock, Mutex,
};

use pico::{Database, SourceId};
use pico_macros::{memo, Source};

static INPUT_PARAM_CLONE_COUNTER: AtomicUsize = AtomicUsize::new(0);
static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

static RUN_SERIALLY: LazyLock<Mutex<()>> = LazyLock::new(Mutex::default);

#[test]
fn params_are_cloned() {
    let _serial_lock = RUN_SERIALLY.lock();
    FIRST_LETTER_COUNTER.store(0, Ordering::SeqCst);
    INPUT_PARAM_CLONE_COUNTER.store(0, Ordering::SeqCst);

    let mut db = Database::default();

    let input_id = db.set(Input {
        key: "input",
        value: "asdf".to_string(),
    });

    // The param is moved into the database when we call db.set,
    // and not cloned.
    assert_eq!(INPUT_PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 0);

    first_letter(&db, input_id, None);

    // The parameter is cloned when we call first_letter
    assert_eq!(INPUT_PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    first_letter(&db, input_id, None);
    // The parameter is not cloned when we call first_letter again and
    // reuse the cached value
    assert_eq!(INPUT_PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
}

#[test]
fn params_are_cloned_when_re_executing() {
    let _serial_lock = RUN_SERIALLY.lock();
    FIRST_LETTER_COUNTER.store(0, Ordering::SeqCst);
    INPUT_PARAM_CLONE_COUNTER.store(0, Ordering::SeqCst);

    let mut db = Database::default();

    let input_id = db.set(Input {
        key: "input",
        value: "asdf".to_string(),
    });
    let alt_input_id = db.set(AltInput {
        key: "alt_input",
        value: "qwerty".to_string(),
    });

    first_letter(&db, input_id, Some(alt_input_id));

    assert_eq!(INPUT_PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    db.set(AltInput {
        key: "alt_input",
        value: "isograph".to_string(),
    });

    first_letter(&db, input_id, Some(alt_input_id));

    // A different input has changed, so we re-executed first_letter,
    // so the param was cloned again.
    assert_eq!(INPUT_PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 2);
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
}

#[derive(Debug, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

impl Clone for Input {
    fn clone(&self) -> Self {
        INPUT_PARAM_CLONE_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {
            key: self.key,
            value: self.value.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Source, Clone)]
struct AltInput {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn first_letter(
    db: &Database,
    input_id: SourceId<Input>,
    alt_input_id: Option<SourceId<AltInput>>,
) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    if let Some(alt_input_id) = alt_input_id {
        db.get(alt_input_id);
    }
    let input = db.get(input_id);
    input.value.chars().next().unwrap()
}
