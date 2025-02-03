use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, Mutex,
    },
};

use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico_core::{database::Database, source::SourceId};
use pico_macros::{memo, Source};

mod calc;

static SUM_COUNTER: AtomicUsize = AtomicUsize::new(0);
static EVAL_COUNTER: LazyLock<Mutex<HashMap<SourceId<Input>, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[test]
fn memoization() {
    let mut db = Database::default();

    let left = db.set(Input {
        key: "left",
        value: "2 + 2 * 2".to_string(),
    });

    let right = db.set(Input {
        key: "right",
        value: "(2 + 2) * 2".to_string(),
    });

    let result = sum(&db, left, right);
    assert_eq!(result, 14);

    // every functions has been called once on the first run
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 1);
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 1);

    // change "left" input with the same eval result
    let left = db.set(Input {
        key: "left",
        value: "3 * 2".to_string(),
    });

    // changing source doesn't cause any recalculation
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 1);
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 1);

    let result = sum(&db, left, right);
    assert_eq!(result, 14);

    // "left" must be called again because the input value has been changed
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 2);
    // "right" must not be called again
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    // "left" and "right" values are the same, so no call
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 1);

    // change "left" input to produce a new value
    let left = db.set(Input {
        key: "left",
        value: "3 * 3".to_string(),
    });
    let result = sum(&db, left, right);
    assert_eq!(result, 17);

    // "left" must be called again because the input value has been changed
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 3);
    // "right" must not be called again
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    // "left" value is different now, so "sum" must be called
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 2);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn parse_ast(db: &Database, id: SourceId<Input>) -> Result<Program> {
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo]
fn evaluate_input(db: &Database, id: SourceId<Input>) -> i64 {
    *EVAL_COUNTER.lock().unwrap().entry(id).or_insert(0) += 1;
    let ast = parse_ast(db, id).expect("ast must be correct");
    eval(ast.expression).expect("value must be evaluated")
}

#[memo]
fn sum(db: &Database, left: SourceId<Input>, right: SourceId<Input>) -> i64 {
    SUM_COUNTER.fetch_add(1, Ordering::SeqCst);
    let left = evaluate_input(db, left);
    let right = evaluate_input(db, right);

    left + right
}
