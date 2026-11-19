use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage};
use pico_macros::{Db, Singleton, memo};

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[derive(Debug, Clone, PartialEq, Eq, Singleton)]
struct Config {
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Singleton)]
struct OtherConfig {
    name: String,
}

fn config(name: &str) -> Config {
    Config {
        name: name.to_string(),
    }
}

static MISSING_THEN_SET: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn missing_then_set_reader(db: &TestDatabase) -> Option<String> {
    MISSING_THEN_SET.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn missing_then_set_does_not_rerun_bug() {
    let mut db = TestDatabase::default();
    assert_eq!(*missing_then_set_reader(&db), None);
    assert_eq!(MISSING_THEN_SET.load(Ordering::SeqCst), 1);

    db.set(config("a"));

    // pico records no dependency on a missing key; the fix flips this assertion.
    assert_eq!(*missing_then_set_reader(&db), None);
    assert_eq!(MISSING_THEN_SET.load(Ordering::SeqCst), 1);
}

static ONLY_THAT_KEY_CONFIG: AtomicUsize = AtomicUsize::new(0);
static ONLY_THAT_KEY_OTHER: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn only_that_key_config_reader(db: &TestDatabase) -> Option<String> {
    ONLY_THAT_KEY_CONFIG.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[memo]
fn only_that_key_other_reader(db: &TestDatabase) -> Option<String> {
    ONLY_THAT_KEY_OTHER.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<OtherConfig>()
        .map(|config| config.name.clone())
}

#[test]
fn set_reruns_no_reader_of_that_key_bug() {
    let mut db = TestDatabase::default();
    assert_eq!(*only_that_key_config_reader(&db), None);
    assert_eq!(*only_that_key_other_reader(&db), None);

    db.set(config("a"));

    // pico records no dependency on a missing key; the fix flips this assertion.
    assert_eq!(*only_that_key_config_reader(&db), None);
    assert_eq!(ONLY_THAT_KEY_CONFIG.load(Ordering::SeqCst), 1);
    assert_eq!(*only_that_key_other_reader(&db), None);
    assert_eq!(ONLY_THAT_KEY_OTHER.load(Ordering::SeqCst), 1);
}

static TWICE: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn twice_reader(db: &TestDatabase) -> (Option<String>, Option<String>) {
    TWICE.fetch_add(1, Ordering::SeqCst);
    let first = db
        .get_singleton::<Config>()
        .map(|config| config.name.clone());
    let second = db
        .get_singleton::<Config>()
        .map(|config| config.name.clone());
    (first, second)
}

#[test]
fn same_missing_key_twice_in_one_memo_does_not_rerun_bug() {
    let mut db = TestDatabase::default();
    assert_eq!(*twice_reader(&db), (None, None));

    db.set(config("a"));

    // pico records no dependency on a missing key; the fix flips this assertion.
    assert_eq!(*twice_reader(&db), (None, None));
    assert_eq!(TWICE.load(Ordering::SeqCst), 1);
}

static PARENT: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn child_reader(db: &TestDatabase) -> Option<String> {
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[memo]
fn parent_reader(db: &TestDatabase) -> Option<String> {
    PARENT.fetch_add(1, Ordering::SeqCst);
    child_reader(db).clone()
}

#[test]
fn parent_of_missing_reader_does_not_rerun_bug() {
    let mut db = TestDatabase::default();
    assert_eq!(*parent_reader(&db), None);

    db.set(config("a"));

    // pico records no dependency on a missing key; the fix flips this assertion.
    assert_eq!(*parent_reader(&db), None);
    assert_eq!(PARENT.load(Ordering::SeqCst), 1);
}
