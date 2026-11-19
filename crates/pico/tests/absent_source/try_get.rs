use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, Storage};
use pico_macros::{Db, Source, memo};
use prelude::Postfix;

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct File {
    #[key]
    path: &'static str,
    contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Unrelated {
    #[key]
    key: &'static str,
    value: String,
}

fn file(path: &'static str, contents: &str) -> File {
    File {
        path,
        contents: contents.to_string(),
    }
}

static NEVER_SET_THEN_SET: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn never_set_then_set_reader(db: &TestDatabase) -> Option<String> {
    NEVER_SET_THEN_SET.fetch_add(1, Ordering::SeqCst);
    db.try_get(File::id_for_key(&"a.txt"))
        .map(|file| file.contents.clone())
}

#[test]
fn set_of_never_set_key_reruns_reader() {
    let mut db = TestDatabase::default();
    assert_eq!(*never_set_then_set_reader(&db), None);
    assert_eq!(NEVER_SET_THEN_SET.load(Ordering::SeqCst), 1);

    db.set(file("a.txt", "a"));

    assert_eq!(*never_set_then_set_reader(&db), "a".to_string().wrap_some());
    assert_eq!(NEVER_SET_THEN_SET.load(Ordering::SeqCst), 2);
}

static UNRELATED_SET: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn unrelated_set_reader(db: &TestDatabase) -> Option<String> {
    UNRELATED_SET.fetch_add(1, Ordering::SeqCst);
    db.try_get(File::id_for_key(&"a.txt"))
        .map(|file| file.contents.clone())
}

#[test]
fn unrelated_set_does_not_rerun_reader_of_never_set_key() {
    let mut db = TestDatabase::default();
    assert_eq!(*unrelated_set_reader(&db), None);

    db.set(file("b.txt", "b"));
    db.set(Unrelated {
        key: "a.txt",
        value: "first".to_string(),
    });
    db.set(Unrelated {
        key: "a.txt",
        value: "second".to_string(),
    });

    assert_eq!(*unrelated_set_reader(&db), None);
    assert_eq!(UNRELATED_SET.load(Ordering::SeqCst), 1);
}

#[test]
fn id_for_key_matches_the_id_set_returns() {
    let mut db = TestDatabase::default();
    let id = db.set(file("a.txt", "a"));

    assert_eq!(File::id_for_key(&"a.txt"), id);
    assert_eq!(db.try_get(id), file("a.txt", "a").reference().wrap_some());
}
