use pico::Database;
use pico_macros::memo;

#[test]
#[should_panic]
fn test_calls_itself() {
    let db = Database::default();

    // calls_itself calls calls_itself, which panics
    calls_itself(&db);
}

#[test]
#[should_panic]
fn test_calls_calls_itself() {
    let db = Database::default();

    // calls_calls_itself calls calls_itself, which calls calls_itself, which panics
    calls_calls_itself(&db);
}

#[test]
#[should_panic]
fn test_a() {
    let db = Database::default();

    // a calls b, which calls a, which panics
    a(&db);
}

#[test]
#[should_panic]
fn test_calls_a() {
    let db = Database::default();

    // calls_a calls a, which calls b, which calls a, which panics
    calls_a(&db);
}

#[memo]
fn calls_itself(database: &Database) {
    calls_itself(database);
}

#[memo]
fn calls_calls_itself(database: &Database) {
    calls_itself(database);
}

#[memo]
fn a(database: &Database) {
    b(database);
}

#[memo]
fn b(database: &Database) {
    a(database);
}

#[memo]
fn calls_a(database: &Database) {
    a(database);
}
