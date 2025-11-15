use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage};
use pico_macros::{Db, Singleton, legacy_memo};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn singleton() {
    let mut db = TestDatabase::default();

    assert_eq!(db.get_singleton::<Input>(), None);

    let id_1 = db.set(Input {
        value: "asdf".to_string(),
    });

    assert!(db.get_singleton::<Input>().is_some());

    assert_eq!(*first_letter(&db).lookup(), 'a');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    let id_2 = db.set(Input {
        value: "qwer".to_string(),
    });

    assert_eq!(*first_letter(&db).lookup(), 'q');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);

    assert_eq!(id_1, id_2);
    let val = db.get(id_1);
    assert_eq!(val.value, "qwer");
}

#[derive(Debug, Clone, PartialEq, Eq, Singleton)]
struct Input {
    pub value: String,
}

#[legacy_memo]
fn first_letter(db: &TestDatabase) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let input = db
        .get_singleton::<Input>()
        .expect("Expected input to have been set");
    input.value.chars().next().unwrap()
}
