use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage};
use pico_macros::{Db, Singleton, memo};

#[derive(Db)]
struct TestDatabase {
    storage: Storage<Self>,
}

impl TestDatabase {
    pub fn new() -> Self {
        Self {
            storage: Storage::new_with_capacity(1.try_into().unwrap()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Singleton)]
struct Config {
    name: String,
}

fn config(name: &str) -> Config {
    Config {
        name: name.to_string(),
    }
}

static RETAINED: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn retained_reader(db: &TestDatabase) -> Option<String> {
    RETAINED.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn retained_reader_of_missing_key_does_not_see_set_bug() {
    let mut db = TestDatabase::new();
    assert_eq!(*retained_reader(&db), None);

    db.run_garbage_collection();

    assert_eq!(*retained_reader(&db), None);
    assert_eq!(RETAINED.load(Ordering::SeqCst), 1);

    db.set(config("a"));

    // pico records no dependency on a missing key; the fix flips this assertion.
    assert_eq!(*retained_reader(&db), None);
    assert_eq!(RETAINED.load(Ordering::SeqCst), 1);
}
