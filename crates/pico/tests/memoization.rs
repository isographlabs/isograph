use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, Mutex,
    },
};

use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico::storage::DefaultStorage;
use pico_core::{
    database::Database,
    source::{Source, SourceId},
};
use pico_macros::{memo, Db, Source};

mod calc;

static SUM_COUNTER: AtomicUsize = AtomicUsize::new(0);
static EVAL_COUNTER: LazyLock<Mutex<HashMap<SourceId<Input>, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[test]
fn memoization() {
    let mut state = TestDatabase::default();

    let left = state.set(Input {
        key: "left",
        value: "2 + 2 * 2".to_string(),
    });

    let right = state.set(Input {
        key: "right",
        value: "(2 + 2) * 2".to_string(),
    });

    eprintln!("1");
    let result = sum(&mut state, left, right);
    eprintln!("2");
    assert_eq!(result, 14);

    // every functions has been called once on the first run
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 1);
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 1);

    // change "left" input with the same eval result
    let left = state.set(Input {
        key: "left",
        value: "3 * 2".to_string(),
    });

    // changing source doesn't cause any recalculation
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 1);
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 1);

    let result = sum(&mut state, left, right);
    eprintln!("3");
    assert_eq!(result, 14);

    // "left" must be called again because the input value has been changed
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 2);
    // "right" must not be called again
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    // "left" and "right" values are the same, so no call
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 1);

    // change "left" input to produce a new value
    let left = state.set(Input {
        key: "left",
        value: "3 * 3".to_string(),
    });
    let result = sum(&mut state, left, right);
    assert_eq!(result, 17);

    // "left" must be called again because the input value has been changed
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 3);
    // "right" must not be called again
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    // "left" value is different now, so "sum" must be called
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 2);
}

#[derive(Debug, Default, Db)]
struct TestDatabase {
    pub storage: DefaultStorage<Self>,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
pub fn parse_ast(db: &mut TestDatabase, id: SourceId<Input>) -> Result<Program> {
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo]
pub fn evaluate_input(db: &mut TestDatabase, id: SourceId<Input>) -> i64 {
    *EVAL_COUNTER.lock().unwrap().entry(id).or_insert(0) += 1;
    let ast = parse_ast(db, id).expect("ast must be correct");
    eval(ast.expression).expect("value must be evaluated")
}

#[memo]
pub fn sum(db: &mut TestDatabase, left: SourceId<Input>, right: SourceId<Input>) -> i64 {
    eprintln!("calling sum");
    SUM_COUNTER.fetch_add(1, Ordering::SeqCst);
    let left = evaluate_input(db, left);
    let right = evaluate_input(db, right);

    left + right
}

#[memo]
fn if_block(db: &mut TestDatabase, condition: SourceId<Input>, input: SourceId<Input>) -> bool {
    let condition = db.get(condition);
    eprintln!("if block {condition:?}");
    if condition.value == "true" {
        inner(db, input);
    }
    return true;
}

#[memo]
fn unconditional(db: &mut TestDatabase, input: SourceId<Input>) -> usize {
    inner(db, input);
    123
}

#[memo]
fn inner(db: &mut TestDatabase, input: SourceId<Input>) -> usize {
    let x = db.get(input);
    eprintln!("inner block val: {}", x.value);
    return 12;
}

#[test]
fn if_test() {
    let mut state = TestDatabase::default();

    let input_id = state.set(Input {
        key: "input",
        value: "first time input".to_string(),
    });

    let condition_id = state.set(Input {
        key: "condition",
        value: "false".to_string(),
    });

    eprintln!("call if with false (no reading of then)");
    if_block(&mut state, condition_id /* false */, input_id);
    eprintln!("called if with false (no reading of then)");

    eprintln!("call then");
    inner(&mut state, input_id);
    eprintln!("called then");

    state.set(Input {
        key: "input",
        value: "second time input".to_string(),
    });
    eprintln!("Updated input epoch {:?}", state.current_epoch());

    state.set(Input {
        key: "condition",
        value: "true".to_string(),
    });

    eprintln!("call if with true (read then)");
    if_block(&mut state, condition_id /* TRUE */, input_id);
    eprintln!("called if with true (read then)");
    // now, the if_block updated_at time is
}

#[memo]
fn square(db: &mut TestDatabase, left: SourceId<Input>, right: SourceId<Input>) -> i64 {
    eprintln!("calling square");
    let val = sum(db, left, right);
    val * val
}

#[test]
fn square_test() {
    let mut state = TestDatabase::default();

    let left = state.set(Input {
        key: "left",
        value: "2 + 2 * 2".to_string(),
    });

    let right = state.set(Input {
        key: "right",
        value: "(2 + 2) * 2".to_string(),
    });

    eprintln!("1");
    let square_result = square(&mut state, left, right);
    eprintln!("2 square {}", square_result);

    // // every functions has been called once on the first run
    // assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 1);
    // assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    // assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 1);

    // change "left" input with the same eval result
    let left = state.set(Input {
        key: "left",
        value: "3 * 2".to_string(),
    });

    // // changing source doesn't cause any recalculation
    // assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 1);
    // assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    // assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 1);

    let result = square(&mut state, left, right);
    eprintln!("3 square {:?}", result);
    // assert_eq!(result, 14);

    // "left" must be called again because the input value has been changed
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 2);
    // "right" must not be called again
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    // "left" and "right" values are the same, so no call
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 1);

    // change "left" input to produce a new value
    let left = state.set(Input {
        key: "left",
        value: "3 * 3".to_string(),
    });
    let result = sum(&mut state, left, right);
    assert_eq!(result, 17);

    // "left" must be called again because the input value has been changed
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 3);
    // "right" must not be called again
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    // "left" value is different now, so "sum" must be called
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 2);
}
