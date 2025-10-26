use std::sync::{
    LazyLock, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, legacy_memo};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);
static PARAM_CLONE_COUNTER: AtomicUsize = AtomicUsize::new(0);
static RUN_SERIALLY: LazyLock<Mutex<()>> = LazyLock::new(Mutex::default);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn owned_param() {
    let _serial_lock = RUN_SERIALLY.lock();
    FIRST_LETTER_COUNTER.store(0, Ordering::SeqCst);
    PARAM_CLONE_COUNTER.store(0, Ordering::SeqCst);

    let mut db = TestDatabase::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    accepts_owned_param(&db, input_id, Param {});
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    // When the parameter is stored, we do not clone it. We only clone it when
    // calling the memoized function.
    assert_eq!(PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "qwer".to_string(),
    });

    accepts_owned_param(&db, input_id, Param {});
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);

    // We clone the param when executing the function again.
    assert_eq!(PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 2);
}

#[test]
fn borrowed_param() {
    let _serial_lock = RUN_SERIALLY.lock();
    FIRST_LETTER_COUNTER.store(0, Ordering::SeqCst);
    PARAM_CLONE_COUNTER.store(0, Ordering::SeqCst);

    let mut db = TestDatabase::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    let param = Param {};
    accepts_borrowed_param(&db, input_id, &param);
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    // We clone the param when storing it, but not when executing the memoized
    // function.
    assert_eq!(PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 1);

    db.set(Input {
        key: "key",
        value: "qwer".to_string(),
    });

    accepts_borrowed_param(&db, input_id, &param);
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);

    // This time around, we don't need to clone it.
    assert_eq!(PARAM_CLONE_COUNTER.load(Ordering::SeqCst), 1);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[derive(Hash)]
struct Param {}

impl Clone for Param {
    fn clone(&self) -> Self {
        PARAM_CLONE_COUNTER.fetch_add(1, Ordering::SeqCst);
        Self {}
    }
}

#[legacy_memo]
fn accepts_owned_param(db: &TestDatabase, input_id: SourceId<Input>, _param: Param) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let input = db.get(input_id);
    input.value.chars().next().unwrap()
}

#[legacy_memo]
fn accepts_borrowed_param(db: &TestDatabase, input_id: SourceId<Input>, _param: &Param) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let input = db.get(input_id);
    input.value.chars().next().unwrap()
}
