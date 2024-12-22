use common::owned::{evaluate_input, Input};
use pico::database::Database;

mod calc;
mod common;

#[test]
fn returns_result() {
    let mut db = Database::default();
    let key = "test_expr";

    let mut input = Input {
        value: "2 + 2 * 2".to_string(),
    };
    input.set(&mut db, key);
    let mut result = evaluate_input(&mut db, key);
    assert_eq!(result, 6);

    input = Input {
        value: "(2 + 2) * 2".to_string(),
    };
    input.set(&mut db, key);
    result = evaluate_input(&mut db, key);
    assert_eq!(result, 8);
}
