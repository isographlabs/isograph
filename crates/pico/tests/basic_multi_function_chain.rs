use std::sync::{
    LazyLock, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use pico::{Database, MemoRef, SourceId, Storage};
use pico_macros::{Db, Source, memo};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static CAPITALIZED_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static MEMO_REF_PARAM_COUNTER: AtomicUsize = AtomicUsize::new(0);

static RUN_SERIALLY: LazyLock<Mutex<()>> = LazyLock::new(Mutex::default);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn multi_function_chain() {
    let _serial_lock = RUN_SERIALLY.lock();
    FIRST_LETTER_COUNTER.store(0, Ordering::SeqCst);
    CAPITALIZED_LETTER_COUNTER.store(0, Ordering::SeqCst);

    let mut db = TestDatabase::default();

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

    let mut db = TestDatabase::default();

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

#[test]
fn sequential_functions_with_memoref_param() {
    let _serial_lock = RUN_SERIALLY.lock();
    FIRST_LETTER_COUNTER.store(0, Ordering::SeqCst);
    MEMO_REF_PARAM_COUNTER.store(0, Ordering::SeqCst);

    let mut db = TestDatabase::default();

    let id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    assert_eq!(
        *capitalized_first_letter_from_memoref(&db, first_letter(&db, id)),
        'A',
    );
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);
    assert_eq!(MEMO_REF_PARAM_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "bsdf".to_string(),
    });

    assert_eq!(*first_letter(&db, id).lookup(&db), 'b');
    assert_eq!(
        *capitalized_first_letter_from_memoref(&db, first_letter(&db, id)),
        'B',
    );
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);
    assert_eq!(MEMO_REF_PARAM_COUNTER.load(Ordering::SeqCst), 2);

    db.set(Input {
        key: "key",
        value: "balt".to_string(),
    });

    assert_eq!(
        *capitalized_first_letter_from_memoref(&db, first_letter(&db, id)),
        'B',
    );
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 3);
    assert_eq!(MEMO_REF_PARAM_COUNTER.load(Ordering::SeqCst), 2);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo(raw)]
fn first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let input = db.get(input_id);
    input.value.chars().next().unwrap()
}

#[memo]
fn capitalized_first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> char {
    CAPITALIZED_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let first = first_letter(db, input_id);
    first.lookup(db).to_ascii_uppercase()
}

#[memo]
fn capitalized_first_letter_from_memoref(db: &TestDatabase, first: MemoRef<char>) -> char {
    MEMO_REF_PARAM_COUNTER.fetch_add(1, Ordering::SeqCst);
    first.lookup_tracked(db).to_ascii_uppercase()
}
