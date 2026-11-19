use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage};
use pico_macros::{Db, Singleton, Source, memo};
use prelude::Postfix;

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[derive(Debug, Clone, PartialEq, Eq, Singleton)]
struct Config {
    name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Unrelated {
    #[key]
    key: &'static str,
    value: String,
}

fn config(name: &str) -> Config {
    Config {
        name: name.to_string(),
    }
}

/// Advances the epoch by setting an unrelated source twice with different values.
fn advance_epoch(db: &mut TestDatabase) {
    db.set(Unrelated {
        key: "unrelated",
        value: "first".to_string(),
    });
    db.set(Unrelated {
        key: "unrelated",
        value: "second".to_string(),
    });
}

static PRESENT_THEN_REMOVED: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn present_then_removed_reader(db: &TestDatabase) -> Option<String> {
    PRESENT_THEN_REMOVED.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn present_then_removed_reruns_and_returns_none() {
    let mut db = TestDatabase::default();
    db.set(config("a"));
    assert_eq!(
        *present_then_removed_reader(&db),
        "a".to_string().wrap_some()
    );
    assert_eq!(PRESENT_THEN_REMOVED.load(Ordering::SeqCst), 1);

    db.remove_singleton::<Config>();

    assert_eq!(*present_then_removed_reader(&db), None);
    assert_eq!(PRESENT_THEN_REMOVED.load(Ordering::SeqCst), 2);
}

static REMOVED_THEN_SET: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn removed_then_set_reader(db: &TestDatabase) -> Option<String> {
    REMOVED_THEN_SET.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn removed_then_set_reruns() {
    let mut db = TestDatabase::default();
    db.set(config("a"));
    assert_eq!(*removed_then_set_reader(&db), "a".to_string().wrap_some());

    db.remove_singleton::<Config>();
    assert_eq!(*removed_then_set_reader(&db), None);

    // Equal to the value before the remove.
    db.set(config("a"));

    assert_eq!(*removed_then_set_reader(&db), "a".to_string().wrap_some());
    assert_eq!(REMOVED_THEN_SET.load(Ordering::SeqCst), 3);
}

static STILL_REMOVED: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn still_removed_reader(db: &TestDatabase) -> Option<String> {
    STILL_REMOVED.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn removed_and_still_removed_does_not_rerun() {
    let mut db = TestDatabase::default();
    db.set(config("a"));
    assert_eq!(*still_removed_reader(&db), "a".to_string().wrap_some());

    db.remove_singleton::<Config>();
    assert_eq!(*still_removed_reader(&db), None);
    assert_eq!(STILL_REMOVED.load(Ordering::SeqCst), 2);

    db.remove_singleton::<Config>();
    advance_epoch(&mut db);

    assert_eq!(*still_removed_reader(&db), None);
    assert_eq!(STILL_REMOVED.load(Ordering::SeqCst), 2);
}

static REMOVE_OF_MISSING: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn remove_of_missing_reader(db: &TestDatabase) -> Option<String> {
    REMOVE_OF_MISSING.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn remove_of_missing_key_does_not_rerun() {
    let mut db = TestDatabase::default();
    assert_eq!(*remove_of_missing_reader(&db), None);

    db.remove_singleton::<Config>();

    assert_eq!(*remove_of_missing_reader(&db), None);
    assert_eq!(REMOVE_OF_MISSING.load(Ordering::SeqCst), 1);
}

#[memo]
fn never_read_reader(db: &TestDatabase) -> Option<String> {
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn remove_of_never_read_key_then_read() {
    let mut db = TestDatabase::default();
    db.remove_singleton::<Config>();

    assert_eq!(*never_read_reader(&db), None);
}
