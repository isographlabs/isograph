use std::sync::atomic::{AtomicUsize, Ordering};

use pico::Database;
use pico_macros::{memo, Singleton};

static FIRST_LETTER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn singleton() {
    let mut db = Database::default();

    assert_eq!(db.get_singleton::<Input>(), None);

    let id_1 = db.set(Input {
        value: "asdf".to_string(),
    });

    assert!(db.get_singleton::<Input>().is_some());

    assert_eq!(*first_letter(&db), 'a');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 1);

    let id_2 = db.set(Input {
        value: "qwer".to_string(),
    });

    assert_eq!(*first_letter(&db), 'q');
    assert_eq!(FIRST_LETTER_COUNTER.load(Ordering::SeqCst), 2);

    assert_eq!(id_1, id_2);
    let val = db.get(id_1);
    assert_eq!(val.value, "qwer");
}

#[derive(Debug, Clone, PartialEq, Eq, Singleton)]
struct Input {
    pub value: String,
}

#[memo]
fn first_letter(db: &Database) -> char {
    FIRST_LETTER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let input = db
        .get_singleton::<Input>()
        .expect("Expected input to have been set");
    input.value.chars().next().unwrap()
}
