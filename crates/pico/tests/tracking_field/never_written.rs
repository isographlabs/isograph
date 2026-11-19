use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, memo};

#[derive(Default)]
pub struct TestMap(HashMap<String, SourceId<Input>>);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
    #[tracked]
    map: TestMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

fn insert(db: &mut TestDatabase, key: &'static str, value: &str) {
    let input_id = db.set(Input {
        key,
        value: value.to_string(),
    });
    db.get_map_mut()
        .tracked()
        .0
        .insert(key.to_string(), input_id);
}

static ITERATION: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn values_tracked(db: &TestDatabase) -> Vec<String> {
    ITERATION.fetch_add(1, Ordering::SeqCst);
    db.get_map()
        .tracked()
        .0
        .values()
        .map(|input_id| db.get(*input_id).value.clone())
        .collect()
}

/// Before the first write, the map's counter singleton does not exist.
/// Reading it inside a memo still records a dependency.
#[test]
fn iteration_before_first_write_then_insert_reruns() {
    let mut db = TestDatabase::default();
    assert_eq!(*values_tracked(&db), Vec::<String>::new());

    insert(&mut db, "a", "asdf");

    assert_eq!(*values_tracked(&db), vec!["asdf".to_string()]);
    assert_eq!(ITERATION.load(Ordering::SeqCst), 2);
}

static LOOKUP: AtomicUsize = AtomicUsize::new(0);

/// Reads the key "a" with `untracked()`, and on a miss depends on the whole map
/// with `tracked()`.
#[memo]
fn lookup_a(db: &TestDatabase) -> Option<String> {
    LOOKUP.fetch_add(1, Ordering::SeqCst);
    let input_id = match db.get_map().untracked().0.get("a").copied() {
        Some(input_id) => input_id,
        None => db.get_map().tracked().0.get("a").copied()?,
    };
    Some(db.get(input_id).value.clone())
}

#[test]
fn untracked_miss_then_tracked_then_insert_reruns() {
    let mut db = TestDatabase::default();
    assert_eq!(*lookup_a(&db), None);

    insert(&mut db, "a", "asdf");

    assert_eq!(*lookup_a(&db), Some("asdf".to_string()));
    assert_eq!(LOOKUP.load(Ordering::SeqCst), 2);
}
