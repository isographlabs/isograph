use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        LazyLock, Mutex,
    },
};

use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico::storage::DefaultStorage;
use pico_core::{container::Container, database::Database, dyn_eq::DynEq, source::SourceId};
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

    let result = sum(&mut state, left, right);
    assert_eq!(result, 14);

    // it must be safe to remove a root derived_node, it should be recalculeted
    let derived_nodes_count = state.storage.derived_nodes.iter().count();
    state.storage.derived_nodes = state
        .storage
        .derived_nodes
        .into_iter()
        .filter(|(_id, node)| node.value != Box::new(result) as Box<dyn DynEq>)
        .collect();
    // derived_node should be removed
    assert_ne!(
        derived_nodes_count,
        state.storage.derived_nodes.iter().count()
    );
    let result = sum(&mut state, left, right);
    assert_eq!(result, 14);
    // removed derived_node should be restored
    assert_eq!(
        derived_nodes_count,
        state.storage.derived_nodes.iter().count()
    );

    // it must be also safe to remove any intermediate derived_node
    let derived_nodes_count = state.storage.derived_nodes.iter().count();
    state.storage.derived_nodes = state
        .storage
        .derived_nodes
        .into_iter()
        // remove "right" evaluate_input node by its value (8)
        .filter(|(_id, node)| node.value != Box::new(8i64) as Box<dyn DynEq>)
        .collect();
    // node should be removed
    assert_ne!(
        derived_nodes_count,
        state.storage.derived_nodes.iter().count()
    );
    let result = sum(&mut state, left, right);
    assert_eq!(result, 14);
    // but we didn't increase current_epoch! So we've got the result from cache
    // and node was not restored
    assert_ne!(
        derived_nodes_count,
        state.storage.derived_nodes.iter().count()
    );

    // now update "left" input to force dependencies to be recalculated
    let left = state.set(Input {
        key: "left",
        value: "3 * 3".to_string(),
    });
    let result = sum(&mut state, left, right);
    assert_eq!(result, 17);
    // removed derived_node should be restored now
    assert_eq!(
        derived_nodes_count,
        state.storage.derived_nodes.iter().count()
    );
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
    SUM_COUNTER.fetch_add(1, Ordering::SeqCst);
    let left = evaluate_input(db, left);
    let right = evaluate_input(db, right);
    left + right
}
