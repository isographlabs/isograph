use std::sync::atomic::{AtomicUsize, Ordering};

use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico_core::{database::Database, source::SourceId};
use pico_macros::{memo, Source};

mod calc;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn return_reference() {
    let mut db = Database::default();

    let input = db.set(Input {
        key: "input",
        value: "2 + 2 * 2".to_string(),
    });

    let value_ref = evaluate_input_ref(&db, input);
    assert_eq!(*value_ref, Value(6));
    // assert that the value was not cloned
    assert_eq!(COUNTER.load(Ordering::SeqCst), 0);

    let value = evaluate_input(&db, input);
    assert_eq!(value, Value(6));
    // assert that the value was cloned this time
    assert_eq!(COUNTER.load(Ordering::SeqCst), 1);
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
        Self(self.0)
    }
}

#[memo]
fn parse_ast(db: &Database, id: SourceId<Input>) -> Result<Program> {
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo]
fn evaluate_input(db: &Database, id: SourceId<Input>) -> Value {
    let ast = parse_ast(db, id).expect("ast must be correct");
    let result = eval(ast.expression).expect("value must be evaluated");
    Value(result)
}

#[memo(reference)]
fn evaluate_input_ref(db: &Database, id: SourceId<Input>) -> Value {
    let ast = parse_ast(db, id).expect("ast must be correct");
    let result = eval(ast.expression).expect("value must be evaluated");
    Value(result)
}
