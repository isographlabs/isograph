use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, Mutex,
    },
};

use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico_core::{database::Database, epoch::Epoch, source::SourceId, storage::Storage};
use pico_macros::{memo, Db, Source};

mod calc;

static SUM_COUNTER: AtomicUsize = AtomicUsize::new(0);
static EVAL_COUNTER: LazyLock<Mutex<HashMap<SourceId<Input>, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[test]
fn drop() {
    let mut state = TestDatabase::default();

    let left = state.set(Input {
        key: "left",
        value: "2 + 2 * 2".to_string(),
    });

    let right = state.set(Input {
        key: "right",
        value: "(2 + 2) * 2".to_string(),
    });

    let result = sum(&mut state, left, right);
    assert_eq!(result, 14);

    // every functions has been called once on the first run
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 1);
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 1);
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 1);

    // it must be safe to drop a generation, it should be recalculeted
    let generations_count = state.storage().map.read_only_view().len();
    state.storage_mut().map.remove(&Epoch::from(1));

    // generation should be removed
    assert_ne!(
        generations_count,
        state.storage().map.read_only_view().len(),
    );
    // but we need to create a new generation anyway
    state.increment_epoch();

    let result = sum(&mut state, left, right);
    assert_eq!(result, 14);

    // every functions has been called againd due to empty storage
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 2);
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 2);
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 2);

    // let's update the "left" source with the same eval result.
    // It must create only one new derived node in the current generation.
    let left = state.set(Input {
        key: "left",
        value: "3 * 2".to_string(),
    });

    // changing source doesn't cause any recalculation
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 2);
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 2);
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 2);

    let result = sum(&state, left, right);
    assert_eq!(result, 14);

    // "left" must be called again because the input value has been changed
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 3);
    // "right" must not be called again
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 2);
    // "left" and "right" values are the same, so no call
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 2);

    // drop the oldest generation
    let min_epoch = *state.storage.map.read_only_view().keys().min().unwrap();
    state.storage_mut().map.remove(&min_epoch);

    let result = sum(&mut state, left, right);
    assert_eq!(result, 14);

    // "left" exists in the latest epoch, it must not be called again
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&left).unwrap(), 3);
    // "right" was dropped, we don't know the previous value
    assert_eq!(*EVAL_COUNTER.lock().unwrap().get(&right).unwrap(), 3);
    // "sum" node was dropped as well
    assert_eq!(SUM_COUNTER.load(Ordering::SeqCst), 3);
}

#[derive(Debug, Default, Db)]
struct TestDatabase {
    pub storage: Storage<Self>,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn parse_ast(db: &TestDatabase, id: SourceId<Input>) -> Result<Program> {
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo]
fn evaluate_input(db: &TestDatabase, id: SourceId<Input>) -> i64 {
    *EVAL_COUNTER.lock().unwrap().entry(id).or_insert(0) += 1;
    let ast = parse_ast(db, id).expect("ast must be correct");
    eval(ast.expression).expect("value must be evaluated")
}

#[memo]
fn sum(db: &TestDatabase, left: SourceId<Input>, right: SourceId<Input>) -> i64 {
    SUM_COUNTER.fetch_add(1, Ordering::SeqCst);
    let left = evaluate_input(db, left);
    let right = evaluate_input(db, right);
    left + right
}
