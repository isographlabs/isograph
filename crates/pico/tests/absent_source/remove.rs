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

fn config(name: &str) -> Config {
    Config {
        name: name.to_string(),
    }
}

static REMOVED_THEN_SET: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn removed_then_set_reader(db: &TestDatabase) -> Option<String> {
    REMOVED_THEN_SET.fetch_add(1, Ordering::SeqCst);
    db.get_singleton::<Config>()
        .map(|config| config.name.clone())
}

#[test]
fn removed_then_set_does_not_rerun_bug() {
    let mut db = TestDatabase::default();
    db.set(config("a"));
    assert_eq!(*removed_then_set_reader(&db), Some("a".to_string()));

    db.remove_singleton::<Config>();
    assert_eq!(*removed_then_set_reader(&db), None);

    // Equal to the value before the remove.
    db.set(config("a"));

    // pico records no dependency on a missing key; the fix flips this assertion.
    assert_eq!(*removed_then_set_reader(&db), None);
    assert_eq!(REMOVED_THEN_SET.load(Ordering::SeqCst), 2);
}
