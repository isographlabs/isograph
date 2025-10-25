use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, memo};

static UNTRACKED_COUNTER: AtomicUsize = AtomicUsize::new(0);
static TRACKED_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Default)]
pub struct TestMap(HashMap<String, SourceId<Input>>);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
    #[tracked]
    map: TestMap,
}

#[test]
fn untracked_efficiency() {
    let mut db = TestDatabase::default();

    // Seed the database
    let input_id_1 = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });
    db.get_map_mut()
        .tracked()
        .0
        .insert("key".to_string(), input_id_1);

    // First time we run get_key_untracked, it will be run (i.e. it's not cached.)
    get_key_untracked(&db);
    assert_eq!(UNTRACKED_COUNTER.load(Ordering::SeqCst), 1);

    // Make unrelated changes
    let input_id_2 = db.set(Input {
        key: "key2",
        value: "asdf".to_string(),
    });
    db.get_map_mut()
        .tracked()
        .0
        .insert("key2".to_string(), input_id_2);

    // get_key_untracked is not invalidated, i.e. UNTRACKED_COUNTER is not
    // incremented
    get_key_untracked(&db);
    assert_eq!(UNTRACKED_COUNTER.load(Ordering::SeqCst), 1);
}

#[test]
fn tracked_inefficiency() {
    let mut db = TestDatabase::default();

    // Seed the database
    let input_id_1 = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });
    db.get_map_mut()
        .tracked()
        .0
        .insert("key".to_string(), input_id_1);

    // First time we run get_key_untracked, it will be run (i.e. it's not cached.)
    get_key_tracked(&db);
    assert_eq!(TRACKED_COUNTER.load(Ordering::SeqCst), 1);

    // Make unrelated changes
    let input_id_2 = db.set(Input {
        key: "key2",
        value: "asdf".to_string(),
    });
    db.get_map_mut()
        .tracked()
        .0
        .insert("key2".to_string(), input_id_2);

    // get_key_tracked is invalidated, because we used a tracked access to the
    // entire map.
    get_key_tracked(&db);
    assert_eq!(TRACKED_COUNTER.load(Ordering::SeqCst), 2);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[legacy_memo]
fn get_key_untracked(db: &TestDatabase) {
    UNTRACKED_COUNTER.fetch_add(1, Ordering::SeqCst);
    let map = db.get_map();
    let item = map
        .untracked()
        .0
        .get("key")
        .expect("Expected value to exist");

    let value = db.get(*item);

    assert_eq!(value.value, "asdf".to_string());
}

#[legacy_memo]
fn get_key_tracked(db: &TestDatabase) {
    TRACKED_COUNTER.fetch_add(1, Ordering::SeqCst);
    let map = db.get_map();
    let item = map.tracked().0.get("key").expect("Expected value to exist");

    let value = db.get(*item);

    assert_eq!(value.value, "asdf".to_string());
}
