use std::sync::atomic::{AtomicUsize, Ordering};

use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico::{Database, SourceId};
use pico_macros::{memo, Source};

mod calc;

static VALUE: AtomicUsize = AtomicUsize::new(0);

#[test]
fn macro_params() {
    let mut db = Database::default();

    let input = db.set(Input {
        key: "input",
        value: "2 + 2 * 2".to_string(),
    });
    let expected = Value(6);

    let value = evaluate_input(&db, input);
    assert_eq!(*value, expected);
    // assert that the value was not cloned
    assert_eq!(VALUE.load(Ordering::SeqCst), 0);

    let value_inner_ref = evaluate_input_inner_ref(&db, input);
    assert_eq!(value_inner_ref, &expected);
    // assert that the value was not cloned
    assert_eq!(VALUE.load(Ordering::SeqCst), 0);

    let value_inner = evaluate_input_inner(&db, input);
    assert_eq!(value_inner, expected);
    // assert that the value was cloned once
    assert_eq!(VALUE.load(Ordering::SeqCst), 1);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Value(pub i64);

impl Clone for Value {
    fn clone(&self) -> Self {
        VALUE.fetch_add(1, Ordering::SeqCst);
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
    let ast = parse_ast(db, id).to_owned().expect("ast must be correct");
    let result = eval(ast.expression).expect("value must be evaluated");
    Value(result)
}

#[memo(inner)]
fn evaluate_input_inner(db: &Database, id: SourceId<Input>) -> Value {
    let ast = parse_ast(db, id).to_owned().expect("ast must be correct");
    let result = eval(ast.expression).expect("value must be evaluated");
    Value(result)
}

#[memo(inner_ref)]
fn evaluate_input_inner_ref(db: &Database, id: SourceId<Input>) -> Value {
    let ast = parse_ast(db, id).to_owned().expect("ast must be correct");
    let result = eval(ast.expression).expect("value must be evaluated");
    Value(result)
}
