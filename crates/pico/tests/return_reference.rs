use std::sync::atomic::{AtomicUsize, Ordering};

use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico::storage::DefaultStorage;
use pico_core::{database::Database, source::SourceId};
use pico_macros::{memo, Db, Source};

mod calc;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn return_reference() {
    let mut state = TestDatabase::default();

    let input = state.set(Input {
        key: "input",
        value: "2 + 2 * 2".to_string(),
    });

    let value_ref = evaluate_input_ref(&mut state, input);
    assert_eq!(*value_ref, Value(6));
    // assert that the value was not cloned
    assert_eq!(COUNTER.load(Ordering::SeqCst), 0);

    let value = evaluate_input(&mut state, input);
    assert_eq!(value, Value(6));
    // assert that the value was cloned this time
    assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
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

#[derive(Debug, PartialEq, Eq)]
struct Value(pub i64);

impl Clone for Value {
    fn clone(&self) -> Self {
        COUNTER.fetch_add(1, Ordering::SeqCst);
        Self(self.0.clone())
    }
}

#[memo]
pub fn parse_ast(db: &mut TestDatabase, id: SourceId<Input>) -> Result<Program> {
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo]
pub fn evaluate_input(db: &mut TestDatabase, id: SourceId<Input>) -> Value {
    let ast = parse_ast(db, id).expect("ast must be correct");
    let result = eval(ast.expression).expect("value must be evaluated");
    Value(result)
}

#[memo(reference)]
pub fn evaluate_input_ref(db: &mut TestDatabase, id: SourceId<Input>) -> Value {
    let ast = parse_ast(db, id).expect("ast must be correct");
    let result = eval(ast.expression).expect("value must be evaluated");
    Value(result)
}
