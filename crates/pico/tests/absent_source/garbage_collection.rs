use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage};
use pico_macros::{Db, Singleton, memo};
use prelude::Postfix;

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

static COLLECTED: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn collected_reader(db: &TestDatabase) -> Option<String> {
    COLLECTED.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[memo]
fn unrelated(_db: &TestDatabase) -> &'static str {
    "unrelated"
}

#[test]
fn collected_reader_of_missing_key_sees_set() {
    let mut db = TestDatabase::new();
    assert_eq!(*collected_reader(&db), None);
    assert_eq!(COLLECTED.load(Ordering::SeqCst), 1);

    // The LRU cache has capacity 1, so it holds only `unrelated`,
    // and `collected_reader` is collected.
    unrelated(&db);
    db.run_garbage_collection();

    db.set(config("a"));

    assert_eq!(*collected_reader(&db), "a".to_string().wrap_some());
    assert_eq!(COLLECTED.load(Ordering::SeqCst), 2);
}

static RETAINED: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn retained_reader(db: &TestDatabase) -> Option<String> {
    RETAINED.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn retained_reader_of_missing_key_sees_set() {
    let mut db = TestDatabase::new();
    assert_eq!(*retained_reader(&db), None);

    db.run_garbage_collection();

    assert_eq!(*retained_reader(&db), None);
    assert_eq!(RETAINED.load(Ordering::SeqCst), 1);

    db.set(config("a"));

    assert_eq!(*retained_reader(&db), "a".to_string().wrap_some());
    assert_eq!(RETAINED.load(Ordering::SeqCst), 2);
}

static REMOVE_AFTER_COLLECTION: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn remove_after_collection_reader(db: &TestDatabase) -> Option<String> {
    REMOVE_AFTER_COLLECTION.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn remove_after_collection() {
    let mut db = TestDatabase::new();
    db.set(config("a"));
    assert_eq!(
        *remove_after_collection_reader(&db),
        "a".to_string().wrap_some()
    );

    db.run_garbage_collection();
    db.remove_singleton::<Config>();

    assert_eq!(*remove_after_collection_reader(&db), None);
    assert_eq!(REMOVE_AFTER_COLLECTION.load(Ordering::SeqCst), 2);
}
