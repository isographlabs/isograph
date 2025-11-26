use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, MemoRef, Storage};
use pico_macros::{Db, Singleton, memo};

static BOOK_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[test]
fn intern_ref_chain() {
    let mut db = TestDatabase::default();
    db.set(EconomistTable {
        economists: vec![
            Economist {
                name: "John Maynard Keynes".to_string(),
                best_book: "General theory".to_string(),
            },
            Economist {
                name: "Milton Friedman".to_string(),
                best_book: "A monetary history of the United States".to_string(),
            },
        ],
    });

    assert_eq!(
        get_economist_best_book(&db, "John Maynard Keynes".to_string()),
        &"General theory"
    );
    assert_eq!(BOOK_COUNTER.load(Ordering::SeqCst), 1);

    // Make changes that don't affect John Maynard Keynes
    db.set(EconomistTable {
        economists: vec![
            Economist {
                name: "John Maynard Keynes".to_string(),
                best_book: "General theory".to_string(),
            },
            Economist {
                name: "Adam Smith".to_string(),
                best_book: "The wealth of nations".to_string(),
            },
        ],
    });

    assert_eq!(
        get_economist_best_book(&db, "John Maynard Keynes".to_string()),
        &"General theory"
    );

    // Unfortunately, we do not short circuit, but we should find a way to!
    assert_eq!(BOOK_COUNTER.load(Ordering::SeqCst), 2);
}

#[memo]
fn get_economists(db: &TestDatabase) -> Vec<Economist> {
    let economists_table = db
        .get_singleton::<EconomistTable>()
        .expect("Expected users table to be set");
    economists_table.economists.clone()
}

#[memo]
fn get_economist_by_name(db: &TestDatabase, name: String) -> MemoRef<Economist> {
    let economist = get_economists(db)
        .iter()
        .find(|user| user.name == name)
        .expect("Expected user to be found");
    db.intern_ref(economist)
}

#[memo]
fn get_economist_best_book(db: &TestDatabase, name: String) -> String {
    BOOK_COUNTER.fetch_add(1, Ordering::SeqCst);
    let user = get_economist_by_name(db, name).lookup(db);
    user.best_book.clone()
}

#[derive(Debug, Clone, PartialEq, Eq, Singleton)]
struct EconomistTable {
    pub economists: Vec<Economist>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Economist {
    name: String,
    best_book: String,
}
