use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage};
use pico_macros::{Db, Singleton, Source, memo};

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
/// The first set is a vacant insert and does not advance the epoch; the second does.
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

static MISSING_THEN_SET: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn missing_then_set_reader(db: &TestDatabase) -> Option<String> {
    MISSING_THEN_SET.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn missing_then_set_reruns() {
    let mut db = TestDatabase::default();
    assert_eq!(*missing_then_set_reader(&db), None);
    assert_eq!(MISSING_THEN_SET.load(Ordering::SeqCst), 1);

    db.set(config("a"));

    assert_eq!(*missing_then_set_reader(&db), Some("a".to_string()));
    assert_eq!(MISSING_THEN_SET.load(Ordering::SeqCst), 2);
}

static STILL_MISSING: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn still_missing_reader(db: &TestDatabase) -> Option<String> {
    STILL_MISSING.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn missing_and_still_missing_across_epochs_does_not_rerun() {
    let mut db = TestDatabase::default();
    assert_eq!(*still_missing_reader(&db), None);

    for _ in 0..3 {
        advance_epoch(&mut db);
        assert_eq!(*still_missing_reader(&db), None);
    }

    assert_eq!(STILL_MISSING.load(Ordering::SeqCst), 1);
}

static UNRELATED_SET: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn unrelated_set_reader(db: &TestDatabase) -> Option<String> {
    UNRELATED_SET.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn unrelated_set_does_not_rerun_reader_of_missing_key() {
    let mut db = TestDatabase::default();
    assert_eq!(*unrelated_set_reader(&db), None);

    db.set(OtherConfig {
        name: "first".to_string(),
    });
    db.set(OtherConfig {
        name: "second".to_string(),
    });

    assert_eq!(*unrelated_set_reader(&db), None);
    assert_eq!(UNRELATED_SET.load(Ordering::SeqCst), 1);
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
fn set_reruns_only_readers_of_that_key() {
    let mut db = TestDatabase::default();
    assert_eq!(*only_that_key_config_reader(&db), None);
    assert_eq!(*only_that_key_other_reader(&db), None);

    db.set(config("a"));

    assert_eq!(*only_that_key_config_reader(&db), Some("a".to_string()));
    assert_eq!(ONLY_THAT_KEY_CONFIG.load(Ordering::SeqCst), 2);
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
fn same_missing_key_twice_in_one_memo() {
    let mut db = TestDatabase::default();
    assert_eq!(*twice_reader(&db), (None, None));

    db.set(config("a"));

    assert_eq!(
        *twice_reader(&db),
        (Some("a".to_string()), Some("a".to_string()))
    );
    assert_eq!(TWICE.load(Ordering::SeqCst), 2);
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
fn parent_of_missing_reader_reruns() {
    let mut db = TestDatabase::default();
    assert_eq!(*parent_reader(&db), None);

    db.set(config("a"));

    assert_eq!(*parent_reader(&db), Some("a".to_string()));
    assert_eq!(PARENT.load(Ordering::SeqCst), 2);
}

static OUTSIDE_THEN_SET: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn outside_then_set_reader(db: &TestDatabase) -> Option<String> {
    OUTSIDE_THEN_SET.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn read_outside_a_memo_then_set() {
    let mut db = TestDatabase::default();
    assert_eq!(db.get_singleton::<Config>(), None);

    db.set(config("a"));

    assert_eq!(*outside_then_set_reader(&db), Some("a".to_string()));
    assert_eq!(OUTSIDE_THEN_SET.load(Ordering::SeqCst), 1);
    assert_eq!(db.get_singleton::<Config>(), Some(&config("a")));
}

#[memo]
fn absent_node_reader(db: &TestDatabase) -> Option<String> {
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn read_outside_a_memo_after_absent_node_exists() {
    let db = TestDatabase::default();
    assert_eq!(*absent_node_reader(&db), None);

    assert_eq!(db.get_singleton::<Config>(), None);
}
