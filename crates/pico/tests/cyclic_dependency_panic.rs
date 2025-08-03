use pico::Storage;
use pico_macros::{memo, Db};

#[derive(Db, Default)]
struct TestDatabase {
    pub storage: Storage<Self>,
}

#[test]
#[should_panic]
fn test_calls_itself() {
    let db = TestDatabase::default();

    // calls_itself calls calls_itself, which panics
    calls_itself(&db);
}

#[test]
#[should_panic]
fn test_calls_calls_itself() {
    let db = TestDatabase::default();

    // calls_calls_itself calls calls_itself, which calls calls_itself, which panics
    calls_calls_itself(&db);
}

#[test]
#[should_panic]
fn test_a() {
    let db = TestDatabase::default();

    // a calls b, which calls a, which panics
    a(&db);
}

#[test]
#[should_panic]
fn test_calls_a() {
    let db = TestDatabase::default();

    // calls_a calls a, which calls b, which calls a, which panics
    calls_a(&db);
}

#[memo]
fn calls_itself(database: &TestDatabase) {
    calls_itself(database);
}

#[memo]
fn calls_calls_itself(database: &TestDatabase) {
    calls_itself(database);
}

#[memo]
fn a(database: &TestDatabase) {
    b(database);
}

#[memo]
fn b(database: &TestDatabase) {
    a(database);
}

#[memo]
fn calls_a(database: &TestDatabase) {
    a(database);
}
