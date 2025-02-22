use std::sync::atomic::{AtomicUsize, Ordering};

use pico::{Database, SourceId};
use pico_macros::{memo, Source};

static A_COUNTER: AtomicUsize = AtomicUsize::new(0);
static B_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn basic_gc() {
    // When we garbage collect, we will only keep the most recently called top-level field
    let mut db = Database::new_with_capacity(1.try_into().unwrap());

    let input_a = db.set(InputA {
        key: "source",
        value: "asdf".to_string(),
    });
    let input_b = db.set(InputB {
        key: "source",
        value: "this is b".to_string(),
    });

    memoized_a(&db, input_a);
    assert_eq!(A_COUNTER.load(Ordering::SeqCst), 1);

    memoized_b(&db, input_b);
    assert_eq!(B_COUNTER.load(Ordering::SeqCst), 1);

    // // Run GC. memoized_b is retained, but not memoized_a.
    // db.run_garbage_collection();

    // memoized_a(&db, input_a);
    // assert_eq!(A_COUNTER.load(Ordering::SeqCst), 2);

    // memoized_b(&db, input_b);
    // assert_eq!(B_COUNTER.load(Ordering::SeqCst), 1);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct InputA {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct InputB {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn memoized_a(db: &Database, input: SourceId<InputA>) -> char {
    dbg!(db.get(input));
    A_COUNTER.fetch_add(1, Ordering::SeqCst);
    'a'
}

#[memo]
fn memoized_b(db: &Database, input: SourceId<InputB>) -> char {
    dbg!(db.get(input));
    B_COUNTER.fetch_add(1, Ordering::SeqCst);
    'b'
}
