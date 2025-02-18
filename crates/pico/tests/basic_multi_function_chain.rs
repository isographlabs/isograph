use std::sync::{
    atomic::{AtomicUsize, Ordering},
    LazyLock, Mutex,
};

use pico::{Database, SourceId};
use pico_macros::{memo, Source};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static CAPITALIZED_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

static RUN_SERIALLY: LazyLock<Mutex<()>> = LazyLock::new(Mutex::default);

#[test]
fn multi_function_chain() {
    let _serial_lock = RUN_SERIALLY.lock();
    FIRST_LETTER_COUNTER.store(0, Ordering::SeqCst);
    CAPITALIZED_LETTER_COUNTER.store(0, Ordering::SeqCst);

    let mut db = Database::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(*capitalized_first_letter(&db, input_id), 'A');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(CAPITALIZED_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "qwer".to_string(),
    });

    assert_eq!(*capitalized_first_letter(&db, input_id), 'Q');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
    assert_eq!(CAPITALIZED_LETTER_COUNTER.load(Ordering::SeqCst), 2);
}

#[test]
fn multi_function_chain_with_irrelevant_change() {
    let _serial_lock = RUN_SERIALLY.lock();
    FIRST_LETTER_COUNTER.store(0, Ordering::SeqCst);
    CAPITALIZED_LETTER_COUNTER.store(0, Ordering::SeqCst);

    let mut db = Database::default();

    let id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(*capitalized_first_letter(&db, id), 'A');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(CAPITALIZED_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "alto".to_string(),
    });

    assert_eq!(*capitalized_first_letter(&db, id), 'A');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
    assert_eq!(CAPITALIZED_LETTER_COUNTER.load(Ordering::SeqCst), 1);
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
fn capitalized_first_letter(db: &Database, input_id: SourceId<Input>) -> char {
    CAPITALIZED_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let first = first_letter(db, input_id);
    first.to_ascii_uppercase()
}
